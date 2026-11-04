"""Failure evidence must survive helper cleanup without persisting environment."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import MagicMock, patch


spec = importlib.util.spec_from_file_location("phase1_egress", Path(__file__).with_name("phase1_egress.py"))
egress = importlib.util.module_from_spec(spec)
spec.loader.exec_module(egress)


class HelperFailureEvidence(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.h = egress.Harness.__new__(egress.Harness)
        self.h.directory = Path(self.temp.name)
        self.h.run_id = "a" * 32
        self.h.owned = ["b" * 64]
        self.h.failure_diagnostics = None
        self.info = {"Name": "/oc-egress-fixture-mediaguard-optin",
                     "State": {"Running": True, "ExitCode": 0},
                     "Config": {"Env": ["MUST_NOT_PERSIST=private-sentinel"]}}

    def test_readiness_failure_names_stage_and_preserves_bounded_private_logs(self):
        def run(*args, **_kwargs):
            if args[:2] == ("docker", "logs"):
                return subprocess.CompletedProcess(args, 0, b"x" * 20000 + b"invalid nft rule", b"")
            if args[-1] == "health":
                return subprocess.CompletedProcess(args, 1, b"", b"policy not installed")
            return subprocess.CompletedProcess(args, 0, b'{"guard-exit":"{\"returncode\":1}"}', b"")

        with patch.object(self.h, "owned_info", return_value=self.info), \
                patch.object(self.h, "inspect", return_value=self.info), \
                patch.object(egress, "_run", side_effect=run), \
                patch.object(egress.time, "monotonic", side_effect=[100, 100, 116]), \
                patch.object(egress.time, "sleep"):
            with self.assertRaisesRegex(RuntimeError, "mediaguard-optin failed at guard-policy-readiness"):
                self.h.wait_ready(self.h.owned[0], guard=True)
        path = self.h.failure_diagnostics
        content = path.read_text()
        evidence = json.loads(content)
        self.assertNotIn("private-sentinel", content)
        self.assertEqual(evidence["last_readiness_probe"]["exit_code"], 1)
        self.assertEqual(evidence["stage"], "guard-policy-readiness")
        self.assertTrue(evidence["helpers"][0]["logs"].endswith("invalid nft rule"))
        self.assertLessEqual(len(evidence["helpers"][0]["logs"]), 16384)
        if os.name != "nt":
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertEqual(path.parent.stat().st_mode & 0o777, 0o700)

    def test_unverified_helper_logs_are_never_read(self):
        with patch.object(self.h, "owned_info", side_effect=RuntimeError("ownership mismatch")), \
                patch.object(egress, "_run") as run:
            path = self.h.save_failure("guard-policy-readiness")
        run.assert_not_called()
        self.assertEqual(json.loads(path.read_text())["helpers"][0]["inspection"],
                         "unavailable or ownership mismatch")

    def test_smoke_retains_evidence_before_cleanup(self):
        h = MagicMock()
        h.create.side_effect = RuntimeError("synthetic helper startup failure")
        h.save_failure.return_value = Path("private-evidence.json")
        with patch.object(egress, "Harness", return_value=h):
            with self.assertRaisesRegex(RuntimeError, "synthetic helper startup failure.*private-evidence"):
                egress.smoke(Path("unused-fixture"))
        names = [call[0] for call in h.mock_calls]
        self.assertLess(names.index("save_failure"), names.index("cleanup"))


if __name__ == "__main__":
    unittest.main()
