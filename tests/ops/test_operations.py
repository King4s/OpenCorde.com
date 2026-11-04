"""Failure cases must be rejected before Docker can mutate an installation."""
import argparse
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def module(name, relative):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


ops = module("ops", "scripts/ops.py")
guard = module("guard", "deploy/containers/netguard.py")


class RecoveryValidation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name)
        for name in ("objects.tar", "identity.tar"):
            with tarfile.open(self.path / name, "w"):
                pass
        (self.path / "database.dump").write_bytes(b"synthetic-dump")
        for name in ("objects-verification.json", "installation.env", "compose.yaml"):
            (self.path / name).write_text("{}")

    def manifest(self):
        value = {"format": 1, "complete": True, "files": {
            p.name: ops.digest(p) for p in self.path.iterdir() if p.name != "manifest.json"}}
        (self.path / "manifest.json").write_text(json.dumps(value))
        return value

    def test_corrupted_dump_refused(self):
        self.manifest()
        (self.path / "database.dump").write_bytes(b"corrupted")
        with self.assertRaisesRegex(ValueError, "checksum"):
            ops.validate_backup(self.path)

    def test_incomplete_backup_refused(self):
        m = self.manifest()
        m["complete"] = False
        (self.path / "manifest.json").write_text(json.dumps(m))
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            ops.validate_backup(self.path)

    def test_archive_traversal_refused_even_with_valid_checksum(self):
        with tarfile.open(self.path / "objects.tar", "w") as archive:
            member = tarfile.TarInfo("../../outside")
            member.size = 1
            archive.addfile(member, io.BytesIO(b"x"))
        self.manifest()
        with self.assertRaisesRegex(ValueError, "Unsafe"):
            ops.validate_backup(self.path)

    def test_archive_link_refused(self):
        with tarfile.open(self.path / "identity.tar", "w") as archive:
            member = tarfile.TarInfo("server.key")
            member.type = tarfile.SYMTYPE
            member.linkname = "/etc/passwd"
            archive.addfile(member)
        self.manifest()
        with self.assertRaisesRegex(ValueError, "Unsafe"):
            ops.validate_backup(self.path)

    def test_valid_checkpoint_passes(self):
        self.manifest()
        self.assertTrue(ops.validate_backup(self.path)["complete"])

    def exercise_empty_restore(self, *, fail_after_import=False):
        target = self.path / "target"
        config = {"services": {"backendguard": {"image": "fixture"}},
                  "volumes": {name: {"name": f"target_{name}"} for name in ops.DATA}}
        manifest = dict(self.manifest(), installation={"id": "source"}, images={"backendguard": "fixture"})
        (self.path / "manifest.json").write_text(json.dumps(manifest))
        target.mkdir()
        initialized, imported, storage_started = False, [], False
        ok = subprocess.CompletedProcess([], 0, b"", b"")

        def compose(*args, **_kwargs):
            nonlocal initialized, storage_started
            if args == ("run", "--rm", "--no-deps", "volume-init"):
                self.assertEqual(json.loads((target / ".restore-pending.json").read_text())["state"], "pending")
                initialized = True
            if args[:3] == ("up", "-d", "--wait"):
                if "volume-init" in args and "minio" not in args:
                    raise RuntimeError("container volume-init exited (0)")  # Compose 5.5.1
                if "minio" in args:
                    self.assertEqual(imported, ["target_objects:/target", "target_identity:/target", "pg_restore"])
                    storage_started = True
            if args[-1] == "verify-objects":
                self.assertTrue(storage_started)
                if fail_after_import:
                    raise RuntimeError("injected object verification failure")
                return subprocess.CompletedProcess([], 0, b"{}", b"")
            return ok

        def restore_process(argv, **_kwargs):
            self.assertTrue(initialized)
            self.assertFalse(storage_started)
            imported.append("pg_restore" if "pg_restore" in argv else argv[argv.index("-v") + 1])
            return ok

        missing = subprocess.CompletedProcess([], 1, b"", b"")
        with patch.object(ops, "ROOT", target), \
                patch.object(ops, "installation", return_value=({"id": "target"}, config)), \
                patch.object(ops, "run", return_value=missing), \
                patch.object(ops, "dc", side_effect=compose), \
                patch.object(ops.subprocess, "run", side_effect=restore_process), \
                patch("sys.stdout", new=io.StringIO()):
            args = argparse.Namespace(backup=str(self.path))
            if fail_after_import:
                with self.assertRaisesRegex(RuntimeError, "object verification failure"):
                    ops.restore(args)
            else:
                ops.restore(args)
        return target

    def assert_startup_blocked(self, target):
        with patch.object(ops, "ROOT", target), patch.object(ops, "installation") as installation, \
                patch.object(ops, "dc") as compose:
            with self.assertRaisesRegex(RuntimeError, "Incomplete restore"):
                ops.up(None)
            installation.assert_not_called()
            compose.assert_not_called()

    @unittest.skipUnless(os.name == "posix", "restore durability requires POSIX filesystem semantics")
    def test_empty_restore_initializes_volumes_before_import_while_storage_is_offline(self):
        target = self.exercise_empty_restore()
        verified = json.loads((target / ".restore-verified.json").read_text())
        self.assertEqual(verified["state"], "verified")
        self.assertEqual(verified["targetInstallationId"], "target")
        self.assertEqual(verified["backupManifestSha256"], ops.digest(self.path / "manifest.json"))
        self.assertFalse((target / ".restore-pending.json").exists())
        with patch.object(ops, "ROOT", target), patch.object(ops, "installation"), \
                patch.object(ops, "dc") as compose, patch("sys.stdout", new=io.StringIO()):
            ops.up(None)
            compose.assert_any_call("up", "-d", "--wait")

    @unittest.skipUnless(os.name == "posix", "restore durability requires POSIX filesystem semantics")
    def test_failed_post_import_verification_blocks_subsequent_startup(self):
        target = self.exercise_empty_restore(fail_after_import=True)
        self.assertTrue((target / ".restore-pending.json").is_file())
        self.assertFalse((target / ".restore-verified.json").exists())
        self.assert_startup_blocked(target)

    @unittest.skipUnless(os.name == "posix", "restore durability requires POSIX filesystem semantics")
    def test_crash_after_success_record_write_before_rename_keeps_startup_blocked(self):
        checkpoint = {"targetInstallationId": "fixture", "backupManifestSha256": "a" * 64}
        with patch.object(ops, "ROOT", self.path):
            ops.record_restore_state(checkpoint, verified=False)
            with patch.object(ops.os, "replace", side_effect=KeyboardInterrupt), self.assertRaises(KeyboardInterrupt):
                ops.record_restore_state(checkpoint, verified=True)
        self.assertEqual(json.loads((self.path / ".restore-pending.json").read_text())["state"], "verified")
        self.assertFalse((self.path / ".restore-verified.json").exists())
        self.assert_startup_blocked(self.path)


class BackupWriterBarrier(unittest.TestCase):
    @staticmethod
    def container(service, *, oneoff="False", project="fixture"):
        return {"State": {"Running": True}, "Config": {"Labels": {
            "com.docker.compose.project": project,
            "com.docker.compose.service": service,
            "com.docker.compose.oneoff": oneoff}}}

    def test_oneoff_refused_before_stop_or_capture(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "checkpoint"
            with patch.object(ops, "installation", return_value=(
                    {"project": "fixture"}, {"services": dict.fromkeys(ops.SERVICES)})), \
                    patch.object(ops, "project_running_containers", return_value=[
                        self.container("api"), self.container("maintenance", oneoff="True")]), \
                    patch.object(ops, "dc") as compose, patch.object(ops, "run") as docker:
                with self.assertRaisesRegex(RuntimeError, "one-off"):
                    ops.backup(argparse.Namespace(output=str(destination)))
                compose.assert_not_called()
                docker.assert_not_called()
                self.assertFalse(destination.exists())

    def test_unknown_service_refused(self):
        with self.assertRaisesRegex(RuntimeError, "unexpected project service"):
            ops.backup_services([self.container("unreviewed-writer")], "fixture", ops.SERVICES)

    def test_post_stop_only_storage_and_guards_allowed(self):
        allowed = [self.container(service) for service in ops.BACKUP_READ_SERVICES]
        self.assertEqual(ops.backup_services(allowed, "fixture", ops.SERVICES, stopped=True),
                         sorted(ops.BACKUP_READ_SERVICES))
        for writer in ("api", "web", "livekit", "maintenance", "redis", "volume-init"):
            with self.subTest(writer=writer), self.assertRaisesRegex(RuntimeError, "writer"):
                ops.backup_services(allowed + [self.container(writer)], "fixture",
                                    ops.SERVICES, stopped=True)

    def test_oneoff_rechecked_after_stop(self):
        with self.assertRaisesRegex(RuntimeError, "one-off"):
            ops.backup_services([self.container("postgres", oneoff="True")],
                                "fixture", ops.SERVICES, stopped=True)

    def test_object_archive_requires_minio_to_remain_stopped(self):
        with patch.object(ops, "project_running_containers", return_value=[self.container("minio")]):
            with self.assertRaisesRegex(RuntimeError, "Object storage is still running"):
                ops.assert_backup_barrier({"project": "fixture"}, {"services": ops.SERVICES},
                                          objects_stopped=True)

    def test_inventory_uses_project_labels_and_inspects_oneoffs(self):
        container = self.container("maintenance", oneoff="True")
        with patch.object(ops, "run", side_effect=[
                subprocess.CompletedProcess([], 0, b"oneoff-id\n"),
                subprocess.CompletedProcess([], 0, json.dumps([container]).encode())]) as docker:
            self.assertEqual(ops.project_running_containers("fixture"), [container])
            self.assertEqual(docker.call_args_list[0].args[0],
                             ["docker", "ps", "--filter", "label=com.docker.compose.project=fixture", "-q"])
            self.assertEqual(docker.call_args_list[1].args[0], ["docker", "inspect", "oneoff-id"])


class TLSMountPermissions(unittest.TestCase):
    @unittest.skipIf(os.name == "nt", "requires POSIX permission semantics")
    def test_tls_mount_traversable_by_both_uids_under_private_host_parent(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source-tls"
            source.mkdir()
            for name in ("fullchain.pem", "privkey.pem"):
                (source / name).write_text("synthetic fixture")
            images = root / "images.json"
            images.write_text(json.dumps({name: "fixture/image@sha256:" + "a" * 64
                                         for name in ("api", "web", "netguard", "minio")}))
            args = argparse.Namespace(project="fixture", domain="example.test",
                                      public_ip="203.0.113.1", private_subnet="172.30.90.0/24",
                                      images=str(images), test_local_images=False,
                                      https_port=443, tls_dir=str(source))
            previous = os.umask(0o077)
            try:
                with patch.object(ops, "ROOT", root), patch("sys.stdout", new=io.StringIO()):
                    ops.prepare(args)
            finally:
                os.umask(previous)
            self.assertEqual(stat.S_IMODE((root / "secrets").stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE((root / "secrets/livekit.yaml").stat().st_mode), 0o444)
            self.assertEqual(stat.S_IMODE((root / "secrets/tls").stat().st_mode), 0o755)
            for name in ("fullchain.pem", "privkey.pem"):
                self.assertEqual(stat.S_IMODE((root / "secrets/tls" / name).stat().st_mode), 0o444)


class LoopbackMediaFixture(unittest.TestCase):
    def test_loopback_permissions_require_both_test_mode_and_loopback_address(self):
        for test_mode, address, permitted in (
                (True, "127.0.0.1", True), (True, "127.0.0.2", True),
                (True, "203.0.113.1", False), (False, "127.0.0.1", False),
                (False, "203.0.113.1", False)):
            with self.subTest(test_mode=test_mode, address=address), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / "source-tls"
                source.mkdir()
                for name in ("fullchain.pem", "privkey.pem"):
                    (source / name).write_text("synthetic fixture")
                images = root / "images.json"
                images.write_text(json.dumps({name: "fixture/image@sha256:" + "a" * 64
                                             for name in ("api", "web", "netguard", "minio")}))
                args = argparse.Namespace(project="fixture", domain="example.test", public_ip=address,
                                          private_subnet="172.30.90.0/24", images=str(images),
                                          test_local_images=test_mode, https_port=443, tls_dir=str(source))
                with patch.object(ops, "ROOT", root), patch("sys.stdout", new=io.StringIO()):
                    ops.prepare(args)
                config = (root / "secrets/livekit.yaml").read_text()
                if permitted:
                    self.assertIn("enable_loopback_candidate: true", config)
                    self.assertIn(f'allow_restricted_peer_cidrs: ["{address}/32"]', config)
                    self.assertIn("BIND_IP=127.0.0.1", (root / ".env").read_text())
                else:
                    self.assertNotIn("enable_loopback_candidate", config)
                    self.assertNotIn("allow_restricted_peer_cidrs", config)
                self.assertNotIn("127.0.0.0/8", config)


class FirewallSnapshotHealth(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.snapshot = Path(self.temp.name) / "policy.json"
        patches = [patch.object(guard, "SNAPSHOT", self.snapshot),
                   patch.dict(os.environ, {"GUARD_ROLE": "backend", "BACKEND_IP": "172.30.90.10",
                                           "MEDIA_IP": "172.30.90.30"}, clear=True)]
        for item in patches:
            item.start()
            self.addCleanup(item.stop)
        self.policy = {"nftables": [
            {"metainfo": {"version": "fixture"}},
            {"table": {"family": "inet", "name": "opencorde", "handle": 1}},
            {"chain": {"family": "inet", "table": "opencorde", "name": "dns", "prio": -150, "policy": "accept", "handle": 2}},
            {"chain": {"family": "inet", "table": "opencorde", "name": "output", "prio": 0, "policy": "drop", "handle": 3}},
            {"chain": {"family": "inet", "table": "opencorde", "name": "forward", "prio": 0, "policy": "drop", "handle": 4}},
        ]}
        for handle, (chain, expression) in enumerate([
                ("dns", [{"match": {"left": "ip daddr", "right": "127.0.0.11"}}, {"drop": None}]),
                ("dns", [{"match": {"left": "udp dport", "right": 53}}, {"drop": None}]),
                ("dns", [{"match": {"left": "tcp dport", "right": 53}}, {"drop": None}]),
                ("output", [{"match": {"left": "oifname", "right": "lo"}}, {"accept": None}]),
                ("output", [{"match": {"left": "ct state", "right": "established,related"}}, {"accept": None}]),
                ("output", [{"match": {"left": "ip daddr", "right": "172.30.90.30"}}, {"accept": None}]),
        ], start=5):
            self.policy["nftables"].append({"rule": {"family": "inet", "table": "opencorde",
                "chain": chain, "expr": expression, "handle": handle}})
        with self.nft(self.policy):
            guard.record_installed_policy()

    def nft(self, policy):
        return patch.object(guard.subprocess, "run", return_value=
                            subprocess.CompletedProcess([], 0, json.dumps(policy).encode()))

    def test_complete_policy_matches(self):
        with self.nft(self.policy):
            self.assertTrue(guard.healthy())

    def test_removed_dns_rule_is_unhealthy_even_with_five_rules(self):
        changed = copy.deepcopy(self.policy)
        changed["nftables"] = [entry for entry in changed["nftables"]
                               if entry.get("rule", {}).get("handle") != 6]
        self.assertEqual(sum("rule" in entry for entry in changed["nftables"]), 5)
        with self.nft(changed):
            self.assertFalse(guard.healthy())

    def test_added_accept_rule_is_unhealthy(self):
        changed = copy.deepcopy(self.policy)
        changed["nftables"].append({"rule": {"family": "inet", "table": "opencorde",
            "chain": "output", "expr": [{"accept": None}], "handle": 999}})
        with self.nft(changed):
            self.assertFalse(guard.healthy())

    def test_extra_owned_chain_is_unhealthy(self):
        changed = copy.deepcopy(self.policy)
        changed["nftables"].append({"chain": {"family": "inet", "table": "opencorde",
                                            "name": "unexpected", "handle": 999}})
        with self.nft(changed):
            self.assertFalse(guard.healthy())

    def test_handles_and_version_metadata_are_not_policy(self):
        changed = copy.deepcopy(self.policy)
        changed["nftables"][0]["metainfo"]["version"] = "different fixture"
        for entry in changed["nftables"][1:]:
            next(iter(entry.values()))["handle"] += 100
        with self.nft(changed):
            self.assertTrue(guard.healthy())

    def test_changed_config_requires_new_successful_install(self):
        with patch.dict(os.environ, {"BACKEND_IP": "172.30.90.99"}), self.nft(self.policy):
            self.assertFalse(guard.healthy())

    def test_missing_snapshot_fails_closed(self):
        self.snapshot.unlink()
        with self.nft(self.policy):
            self.assertFalse(guard.healthy())

    def test_failed_atomic_install_cannot_reuse_old_snapshot(self):
        with patch.object(guard.sys, "argv", ["netguard"]), \
                patch.object(guard.subprocess, "run", side_effect=[
                    subprocess.CompletedProcess([], 0),
                    subprocess.CalledProcessError(1, "nft")]):
            with self.assertRaises(subprocess.CalledProcessError):
                guard.main()
        self.assertFalse(self.snapshot.exists())


class PrivacyConfiguration(unittest.TestCase):
    def rules(self, role, **extra):
        with patch.dict(os.environ, {"GUARD_ROLE": role, "BACKEND_IP": "172.30.90.10",
                                     "MEDIA_IP": "172.30.90.30", **extra}, clear=True):
            return guard.rules()

    def test_default_never_grants_external_media(self):
        for role in ("web", "backend", "media"):
            rules = self.rules(role)
            self.assertNotIn("ip daddr 0.0.0.0/0 meta l4proto udp accept", rules)
            self.assertIn("policy drop", rules)
            self.assertLess(rules.index("ip daddr 127.0.0.11 drop"), rules.index('oifname "lo" accept'))
            self.assertIn("priority -150", rules)

    def test_media_opt_in_does_not_open_web_namespace(self):
        self.assertIn("ip daddr 0.0.0.0/0 meta l4proto udp accept", self.rules("media", MEDIA_EGRESS="true"))
        self.assertNotIn("ip daddr 0.0.0.0/0 meta l4proto udp accept", self.rules("web", MEDIA_EGRESS="true"))

    def test_malformed_opt_in_is_not_truthy(self):
        with self.assertRaises(ValueError):
            self.rules("media", MEDIA_EGRESS="yes")

    def test_firewall_injection_is_rejected(self):
        with self.assertRaises(ValueError):
            self.rules("media", MEDIA_EGRESS="true", MEDIA_CLIENT_CIDRS="0.0.0.0/0; accept")


if __name__ == "__main__":
    unittest.main()
