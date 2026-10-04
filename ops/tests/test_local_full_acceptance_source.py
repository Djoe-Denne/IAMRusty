"""Pure source/fake-exec checks; no Docker, Kubernetes, sockets or Secrets."""
import ast
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

SOURCE = Path(__file__).with_name("local-full-acceptance.py")


class AcceptanceGuardTests(unittest.TestCase):
    def load(self):
        spec = importlib.util.spec_from_file_location("acceptance_guard_test", SOURCE)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_no_direct_process_or_ambient_kubectl_path(self):
        tree = ast.parse(SOURCE.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                self.assertFalse(any(alias.name in {"subprocess", "os"} for alias in node.names))
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == "kubectl":
                self.assertIsInstance(node.func.value, ast.Name)
                self.assertEqual(node.func.value.id, "runtime")

    def test_remote_timeout_is_reported_not_replaced_by_raw_host_exec(self):
        module = self.load()
        with patch.object(module.runtime, "kubectl", return_value="28 000") as fake:
            self.assertEqual(module.guarded_curl("aiforall-local-full", "owned-probe", ["https://example.test"], 20), (28, "000"))
        self.assertEqual(fake.call_args.args[:7], ("exec", "-n", "aiforall-local-full", "owned-probe", "--", "sh", "-c"))

    def test_anchor_failure_never_falls_back_to_another_transport(self):
        module = self.load()
        with patch.object(module.runtime, "kubectl", side_effect=module.anchor_module.AnchorError("fake changed endpoint")) as fake:
            with self.assertRaises(module.anchor_module.AnchorError):
                module.guarded_curl("aiforall-local-full", "owned-probe", [], 20)
        self.assertEqual(fake.call_count, 1)

    def test_legacy_context_rejected_before_lease_or_api(self):
        module = self.load()
        args = Mock(context="kind-aiforall-local")
        with patch.object(module.runtime, "lease") as lease:
            with self.assertRaises(ValueError):
                module.run(args)
        lease.assert_not_called()


if __name__ == "__main__":
    unittest.main()
