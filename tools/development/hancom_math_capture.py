# SPDX-License-Identifier: AGPL-3.0-only
"""Shared native equation capture helpers; importing this module never opens Hancom.

Process cleanup checks both executable name and Windows creation time, preserving
preexisting sessions and rejecting reused process IDs. No installation or registry
mutation is performed here.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET


class UnsafeDesktop(RuntimeError):
    pass


def process_creation(pid):
    """Windows creation FILETIME, used to reject a reused process ID."""
    import ctypes
    from ctypes import wintypes
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
    handle = kernel.OpenProcess(0x1000, False, pid)
    if not handle:
        raise ProcessLookupError(pid)
    try:
        values = [wintypes.FILETIME() for _ in range(4)]
        if not kernel.GetProcessTimes(handle, *(ctypes.byref(v) for v in values)):
            raise ctypes.WinError(ctypes.get_last_error())
        return (values[0].dwHighDateTime << 32) | values[0].dwLowDateTime
    finally:
        kernel.CloseHandle(handle)


def process_snapshot():
    """Native Unicode process names and parent IDs, without shell decoding."""
    import ctypes
    from ctypes import wintypes

    class PROCESSENTRY32W(ctypes.Structure):
        _fields_ = [("dwSize", wintypes.DWORD), ("cntUsage", wintypes.DWORD),
                    ("th32ProcessID", wintypes.DWORD), ("th32DefaultHeapID", ctypes.c_size_t),
                    ("th32ModuleID", wintypes.DWORD), ("cntThreads", wintypes.DWORD),
                    ("th32ParentProcessID", wintypes.DWORD), ("pcPriClassBase", wintypes.LONG),
                    ("dwFlags", wintypes.DWORD), ("szExeFile", wintypes.WCHAR * 260)]

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
    kernel.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
    for name in ("Process32FirstW", "Process32NextW"):
        action = getattr(kernel, name)
        action.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
        action.restype = wintypes.BOOL
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    handle = kernel.CreateToolhelp32Snapshot(0x2, 0)
    if handle == ctypes.c_void_p(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        entry = PROCESSENTRY32W()
        entry.dwSize = ctypes.sizeof(entry)
        result = {}
        found = kernel.Process32FirstW(handle, ctypes.byref(entry))
        while found:
            result[entry.th32ProcessID] = {"name": entry.szExeFile.lower(),
                                          "parent": entry.th32ParentProcessID}
            found = kernel.Process32NextW(handle, ctypes.byref(entry))
        error = ctypes.get_last_error()
        if error != 18:  # ERROR_NO_MORE_FILES
            raise ctypes.WinError(error)
        return result
    finally:
        kernel.CloseHandle(handle)


def process_table():
    return {pid: record["name"] for pid, record in process_snapshot().items()}


def process_parent(pid):
    record = process_snapshot().get(pid)
    if record is None:
        raise ProcessLookupError(pid)
    return record["parent"]


def process_name(pid):
    record = process_snapshot().get(pid)
    if record is None:
        raise ProcessLookupError(pid)
    return record["name"]


def native_process_state(pid, creation):
    """Check the original HWP identity, never a replacement with the same PID."""
    state = {"pid": pid, "expected_creation": creation, "alive": False}
    try:
        state["actual_creation"] = process_creation(pid)
        state["name"] = process_name(pid)
    except ProcessLookupError:
        return {**state, "state": "exited"}
    if state["actual_creation"] != creation or state["name"] not in {"hwp.exe", "hwp64.exe"}:
        return {**state, "state": "identity_changed"}
    return {**state, "alive": True, "state": "running"}


def terminate_recorded_hwp(pid, creation):
    """Close only this experiment's exact process; never accept a native prompt."""
    try:
        stamp = process_creation(pid)
        name = process_name(pid)
    except ProcessLookupError:
        return "already_exited"
    if stamp != creation or name not in {"hwp.exe", "hwp64.exe"}:
        raise UnsafeDesktop("native cleanup identity mismatch")
    subprocess.run(["taskkill.exe", "/PID", str(pid), "/F"], check=True, capture_output=True)
    return "identity_checked_native_termination"


def equation_script(xml):
    root = ET.fromstring(xml)
    equations = [item for item in root.iter() if item.tag.rsplit("}", 1)[-1] == "EQUATION"]
    if len(equations) != 1:
        raise ValueError(f"expected exactly one native equation, found {len(equations)}")
    scripts = [item for item in equations[0] if item.tag.rsplit("}", 1)[-1] == "SCRIPT"]
    if len(scripts) != 1:
        raise ValueError("native equation must contain exactly one SCRIPT")
    return "".join(scripts[0].itertext())


def json_bytes(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes, *, immutable=False, check=False):
    """Publish complete files; immutable files must match on resume."""
    if path.exists() and (immutable or check):
        if path.read_bytes() != content:
            raise ValueError(f"inconsistent existing file: {path}")
        return
    if check:
        raise ValueError(f"missing file: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=".math-capture-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        if immutable:
            # Windows rename refuses existing targets and needs no hard-link
            # privilege. POSIX uses a hard link for the same no-clobber behavior.
            try:
                if os.name == "nt":
                    os.rename(temporary, path)
                else:
                    os.link(temporary, path)
            except FileExistsError:
                if path.read_bytes() != content:
                    raise ValueError(f"inconsistent existing file: {path}")
        else:
            os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
