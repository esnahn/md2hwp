# SPDX-License-Identifier: AGPL-3.0-only
"""Mocked process ownership checks; never interact with native UI or processes."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/development"))
import hancom_math_capture as runner
import render_hancom_equation_gallery as capture


class NativeCleanupTests(unittest.TestCase):
    def test_native_state_reports_healthy_hwp_identity(self):
        for image in ("hwp.exe", "hwp64.exe"):
            with self.subTest(image=image), \
                 patch.object(runner, "process_creation", return_value=123) as creation, \
                 patch.object(runner, "process_name", return_value=image) as name:
                state = runner.native_process_state(200, 123)
                self.assertEqual(state, {"pid": 200, "alive": True, "state": "running",
                                         "expected_creation": 123, "actual_creation": 123,
                                         "name": image})
                creation.assert_called_once_with(200)
                name.assert_called_once_with(200)

    def test_native_state_never_reports_reused_pid_or_wrong_image_alive(self):
        for stamp, image in ((124, "hwp.exe"), (123, "python.exe")):
            with self.subTest(stamp=stamp, image=image), \
                 patch.object(runner, "process_creation", return_value=stamp), \
                 patch.object(runner, "process_name", return_value=image):
                state = runner.native_process_state(200, 123)
                self.assertFalse(state["alive"])
                self.assertEqual(state["state"], "identity_changed")
                self.assertEqual(state["pid"], 200)
                self.assertEqual(state["expected_creation"], 123)
                self.assertEqual(state["actual_creation"], stamp)
                self.assertEqual(state["name"], image)

    def test_native_state_reports_exit_at_either_lookup(self):
        for missing_at in ("creation", "name"):
            with self.subTest(missing_at=missing_at), \
                 patch.object(runner, "process_creation", return_value=123,
                              side_effect=ProcessLookupError if missing_at == "creation" else None), \
                 patch.object(runner, "process_name", return_value="hwp.exe",
                              side_effect=ProcessLookupError if missing_at == "name" else None):
                state = runner.native_process_state(200, 123)
                self.assertFalse(state["alive"])
                self.assertEqual(state["state"], "exited")
                self.assertEqual(state["pid"], 200)
                self.assertEqual(state["expected_creation"], 123)


    def test_exact_native_identity_targets_only_recorded_pid(self):
        for image in ("hwp.exe", "hwp64.exe"):
            with self.subTest(image=image), \
                 patch.object(runner, "process_creation", return_value=123) as creation, \
                 patch.object(runner, "process_name", return_value=image) as name, \
                 patch.object(runner.subprocess, "run") as kill:
                self.assertEqual(runner.terminate_recorded_hwp(200, 123),
                                 "identity_checked_native_termination")
                creation.assert_called_once_with(200)
                name.assert_called_once_with(200)
                kill.assert_called_once_with(["taskkill.exe", "/PID", "200", "/F"],
                                             check=True, capture_output=True)

    def test_reused_pid_or_unexpected_image_is_never_terminated(self):
        for stamp, image in ((124, "hwp.exe"), (123, "python.exe"),
                             (123, "other.exe")):
            with self.subTest(stamp=stamp, image=image), \
                 patch.object(runner, "process_creation", return_value=stamp), \
                 patch.object(runner, "process_name", return_value=image), \
                 patch.object(runner.subprocess, "run") as kill:
                with self.assertRaisesRegex(runner.UnsafeDesktop, "identity mismatch"):
                    runner.terminate_recorded_hwp(200, 123)
                kill.assert_not_called()

    def test_process_gone_before_or_during_lookup_needs_no_termination(self):
        for missing_at in ("creation", "name"):
            with self.subTest(missing_at=missing_at), \
                 patch.object(runner, "process_creation", return_value=123,
                              side_effect=ProcessLookupError if missing_at == "creation" else None), \
                 patch.object(runner, "process_name", return_value="hwp.exe",
                              side_effect=ProcessLookupError if missing_at == "name" else None), \
                 patch.object(runner.subprocess, "run") as kill:
                self.assertEqual(runner.terminate_recorded_hwp(200, 123), "already_exited")
                kill.assert_not_called()

    def test_failed_termination_propagates_instead_of_claiming_cleanup(self):
        with patch.object(runner, "process_creation", return_value=123), \
             patch.object(runner, "process_name", return_value="hwp.exe"), \
             patch.object(runner.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "taskkill")):
            with self.assertRaises(subprocess.CalledProcessError):
                runner.terminate_recorded_hwp(200, 123)


class FixtureAndOutputTests(unittest.TestCase):
    def fixture(self, folder, cases, **extra):
        path = Path(folder) / 'fixture.json'
        path.write_text(json.dumps({'cases': cases, **extra}), encoding='utf-8')
        return path

    def test_both_explicit_fixture_shapes_normalize_without_com(self):
        for case in ({'id': '5-1', 'hancom_script': 'root {3} of {27}'},
                     {'case_id': '001', 'script': 'a^b', 'name': '첨자', 'category': '수식'}):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as folder:
                path = self.fixture(folder, [case])
                loaded = capture.load_cases(path)
                normalized = loaded['cases'][0]
                self.assertEqual(normalized['case_id'], case.get('id', case.get('case_id')))
                self.assertEqual(normalized['script'], case.get('script', case.get('hancom_script')))
                self.assertEqual(loaded['source']['sha256'], hashlib.sha256(path.read_bytes()).hexdigest())

    def test_case_ids_cannot_escape_output_or_use_windows_devices(self):
        for identifier in ('..', '.', '../escape', 'a/b', r'a\b', 'C:escape', 'NUL', 'CON.txt', 'lpt1', 'a.', 'a ', ''):
            with self.subTest(identifier=identifier), tempfile.TemporaryDirectory() as folder:
                path = self.fixture(folder, [{'id': identifier, 'hancom_script': 'a'}])
                with self.assertRaises(ValueError):
                    capture.load_cases(path)

    def test_duplicate_or_missing_equations_are_rejected_before_com(self):
        for cases in ([], [{'id': 'a', 'hancom_script': ''}], [{'id': 'a', 'hancom_script': 'a'}] * 2,
                      [{'script': 'a'}], [{'id': 'a', 'script': 'a', 'name': 3}]):
            with self.subTest(cases=cases), tempfile.TemporaryDirectory() as folder:
                path = self.fixture(folder, cases)
                with self.assertRaises(ValueError):
                    capture.run(path, Path(folder) / 'out')

    def test_case_only_duplicate_ids_are_rejected_before_native_setup(self):
        with tempfile.TemporaryDirectory() as folder:
            path = self.fixture(folder, [
                {'case_id': 'xleftrightarrow', 'script': 'a'},
                {'id': 'xLeftrightarrow', 'hancom_script': 'b'},
            ])
            output = Path(folder) / 'out'
            with patch.object(capture.winreg, 'OpenKey') as open_key:
                with self.assertRaisesRegex(ValueError, 'Duplicate case ID: xLeftrightarrow'):
                    capture.run(path, output)
                open_key.assert_not_called()
            self.assertFalse(output.exists())

    def test_optional_resource_hash_is_still_mandatory_when_declared(self):
        with tempfile.TemporaryDirectory() as folder:
            resource = Path(folder) / 'resource.bin'
            resource.write_bytes(b'original local resource')
            source = {'path': str(resource), 'sha256': hashlib.sha256(resource.read_bytes()).hexdigest()}
            path = self.fixture(folder, [{'id': 'a', 'hancom_script': 'a'}], source=source)
            self.assertEqual(capture.load_cases(path)['source'], source)
            resource.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'resource has changed'):
                capture.load_cases(path)

    def test_atomic_immutable_publication_preserves_existing_output_on_conflict(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'out.json'
            runner.write_atomic(path, runner.json_bytes({'value': '한글'}), immutable=True)
            before = path.read_bytes()
            runner.write_atomic(path, before, immutable=True)
            with self.assertRaisesRegex(ValueError, 'inconsistent existing file'):
                runner.write_atomic(path, b'changed', immutable=True)
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(list(Path(folder).glob('.math-capture-*')), [])

    def test_equation_extraction_requires_one_native_equation_and_script(self):
        xml = '<DOC xmlns="urn:hwp"><EQUATION><SCRIPT>a &lt; b</SCRIPT></EQUATION></DOC>'
        self.assertEqual(runner.equation_script(xml), 'a < b')
        for xml in ('<DOC/>', '<DOC><EQUATION/><EQUATION/></DOC>',
                    '<DOC><EQUATION><SCRIPT>a</SCRIPT><SCRIPT>b</SCRIPT></EQUATION></DOC>'):
            with self.subTest(xml=xml), self.assertRaises(ValueError):
                runner.equation_script(xml)

    def test_cli_requires_explicit_input_without_opening_native_app(self):
        command = [sys.executable, '-B', str(Path(capture.__file__))]
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('--input', result.stderr)


if __name__ == "__main__":
    unittest.main()
