// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;

namespace Md2Hwp.HancomIrPreview;

// The Hancom process a session started. Hancom windows the user already had
// open are recorded beforehand and never cleared, quit or stopped.
internal sealed class OwnedHwpProcess
{
    private static readonly string[] Names = ["Hwp", "Hwp64"];

    private OwnedHwpProcess(int id, DateTime startTime)
    {
        Id = id;
        StartTime = startTime;
    }

    internal int Id { get; }
    internal DateTime StartTime { get; }

    internal static IReadOnlyDictionary<int, DateTime> Snapshot()
    {
        var table = new Dictionary<int, DateTime>();
        foreach (var name in Names)
            foreach (var process in Process.GetProcessesByName(name))
                using (process)
                {
                    try { table[process.Id] = process.StartTime; }
                    catch (InvalidOperationException) { } // exited meanwhile
                }
        return table;
    }

    // Identify the process behind a new COM object from its main window.
    // A process that already existed belongs to the user: fail without
    // touching it (the caller only releases its COM reference).
    internal static OwnedHwpProcess Identify(dynamic hwp, IReadOnlyDictionary<int, DateTime> before)
    {
        var handle = new IntPtr(Convert.ToInt64(hwp.XHwpWindows.Item(0).WindowHandle));
        _ = GetWindowThreadProcessId(handle, out var processId);
        if (processId == 0) throw new InvalidOperationException("Could not identify the Hancom process behind the COM object.");
        var id = checked((int)processId);
        if (before.ContainsKey(id))
            throw new InvalidOperationException($"COM returned a preexisting Hancom process ({id}); it was left untouched.");
        using var process = Process.GetProcessById(id);
        if (!Names.Contains(process.ProcessName, StringComparer.OrdinalIgnoreCase))
            throw new InvalidOperationException($"Unexpected Hancom COM server process: {process.ProcessName}");
        return new(id, process.StartTime);
    }

    // PID plus start time, so a reused PID is never mistaken for ours.
    internal bool IsAlive()
    {
        try
        {
            using var process = Process.GetProcessById(Id);
            return !process.HasExited && process.StartTime == StartTime &&
                Names.Contains(process.ProcessName, StringComparer.OrdinalIgnoreCase);
        }
        catch (ArgumentException) { return false; }
        catch (InvalidOperationException) { return false; }
    }

    // Wait for a normal exit after Quit. Only this process may be stopped,
    // and stopping it is reported as a failure so nothing is published.
    internal void WaitForExit()
    {
        var deadline = DateTime.UtcNow.AddSeconds(15);
        while (IsAlive() && DateTime.UtcNow < deadline) Thread.Sleep(200);
        if (!IsAlive()) return;
        using (var process = Process.GetProcessById(Id))
            if (process.StartTime == StartTime) process.Kill();
        deadline = DateTime.UtcNow.AddSeconds(5);
        while (IsAlive() && DateTime.UtcNow < deadline) Thread.Sleep(100);
        if (IsAlive()) throw new InvalidOperationException($"The Hancom process this run started ({Id}) could not be stopped.");
        throw new InvalidOperationException($"The Hancom process this run started ({Id}) did not exit after Quit and was stopped.");
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
}

// Hancom keeps documents it has open locked against ordinary reads. Read
// sources with shared access so they may stay open in the user's Hancom;
// unsaved changes in that window are not included.
internal static class SharedFile
{
    internal static FileStream OpenRead(string path) =>
        new(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);

    internal static string Hash(string path)
    {
        using var stream = OpenRead(path);
        return Convert.ToHexString(SHA256.HashData(stream));
    }

    internal static void Copy(string source, string destination)
    {
        using var reader = OpenRead(source);
        using var writer = new FileStream(destination, FileMode.CreateNew, FileAccess.Write, FileShare.None);
        reader.CopyTo(writer);
    }

    // A private copy for Hancom to open, so a source locked by the user's
    // Hancom can still be read. Delete it with DeleteSnapshot.
    internal static string Snapshot(string source)
    {
        var directory = Path.Combine(Path.GetTempPath(), "md2hwp-source-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        var snapshot = Path.Combine(directory, "source" + Path.GetExtension(source).ToLowerInvariant());
        Copy(source, snapshot);
        return snapshot;
    }

    internal static void DeleteSnapshot(string snapshot)
    {
        if (File.Exists(snapshot)) File.Delete(snapshot);
        var directory = Path.GetDirectoryName(snapshot)!;
        if (Path.GetFileName(directory).StartsWith("md2hwp-source-", StringComparison.Ordinal) && Directory.Exists(directory))
            Directory.Delete(directory, recursive: false);
    }
}
