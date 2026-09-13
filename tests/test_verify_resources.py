"""Run with python3 -B -m unittest discover -s tests -p 'test_*.py'.

Fixtures contain synthetic bytes only; resource-set selection must not hash or
pass the base files when the caller requests Ex (including case-sensitive names).
"""
import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "verify_resources", Path(__file__).resolve().parents[1] / "scripts/verify_resources.py"
)
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)


class ResourceSetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="xglib-audit-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "bin/pal").mkdir(parents=True)
        (self.root / "map").mkdir()
        (self.root / "bin/pal/palet_00.cgp").write_bytes(bytes(708))
        (self.root / "map/synthetic.dat").write_bytes(b"synthetic map")
        for name in audit.RESOURCE_SETS["ex"]:
            (self.root / "bin" / name).write_bytes(name.encode())

    def run_main(self, argv, runner):
        with patch("sys.argv", ["verify_resources.py", str(self.root), *argv]), \
             patch.object(audit.subprocess, "run", side_effect=runner) as run, \
             contextlib.redirect_stdout(io.StringIO()) as out:
            code = audit.main()
        return code, out.getvalue(), run

    def test_ex_inventory_uses_exact_case_and_does_not_require_base_files(self):
        entries = audit.inventory(self.root, audit.RESOURCE_SETS["ex"])
        by_path = {entry["path"]: entry for entry in entries}
        self.assertEqual(len(entries), 6)
        name = "AnimeInfoEx_1.Bin"
        entry = by_path["bin/" + name]
        self.assertEqual(entry["sha256"], hashlib.sha256(name.encode()).hexdigest())
        self.assertNotIn("bin/AnimeInfo_4.bin", by_path)

    def test_ex_cli_passes_the_same_four_files_that_were_hashed(self):
        code, output, run = self.run_main(
            ["--set", "ex"], lambda *a, **kw: subprocess.CompletedProcess(a[0], 1)
        )
        self.assertEqual(code, 1)  # Preserve compatibility failure, not just success.
        self.assertEqual(run.call_args.args[0][-4:], list(audit.RESOURCE_SETS["ex"]))
        self.assertIn("inputs_unchanged=true", output)
        self.assertIn("audit_exit_code=1", output)

    def test_base_remains_the_default(self):
        for name in audit.RESOURCE_SETS["base"]:
            (self.root / "bin" / name).write_bytes(b"base fixture")
        code, output, run = self.run_main(
            [], lambda *a, **kw: subprocess.CompletedProcess(a[0], 0)
        )
        self.assertEqual(code, 0)
        self.assertEqual(run.call_args.args[0][-4:], list(audit.RESOURCE_SETS["base"]))
        self.assertNotIn('"path": "bin/GraphicEx_5.bin"', output)

    def test_changed_selected_input_overrides_success(self):
        def changed(command, **kwargs):
            (self.root / "bin/AnimeEx_1.Bin").write_bytes(b"changed synthetic input")
            return subprocess.CompletedProcess(command, 0)
        code, output, _ = self.run_main(["--set", "ex"], changed)
        self.assertEqual(code, 2)
        self.assertIn("inputs_unchanged=false", output)

    def test_unknown_set_fails_before_reading_files_or_starting_cargo(self):
        with patch("sys.argv", ["verify_resources.py", str(self.root), "--set", "unknown"]), \
             patch.object(audit, "inventory") as inventory, \
             patch.object(audit.subprocess, "run") as run, \
             contextlib.redirect_stderr(io.StringIO()), \
             self.assertRaises(SystemExit) as error:
            audit.main()
        self.assertEqual(error.exception.code, 2)
        inventory.assert_not_called()
        run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
