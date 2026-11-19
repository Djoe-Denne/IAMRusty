import contextlib
import importlib.util
import io
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from urllib.error import HTTPError


path = Path(__file__).resolve().parent / "k6/ci/provision.py"
spec = importlib.util.spec_from_file_location("k6_ci_provision", path)
provision = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provision)


class CliProvisionTests(unittest.TestCase):
    def test_recovers_from_a_timeout_without_logging_urls(self):
        output = io.StringIO()
        with patch.object(provision.urllib.request, "urlopen", side_effect=[
            TimeoutError("private download URL"), contextlib.nullcontext(io.BytesIO(b"tool")),
        ]) as get, patch.object(provision.time, "sleep"), contextlib.redirect_stdout(output):
            self.assertEqual(provision.download("kubectl", "https://example.invalid/"), b"tool")
        self.assertEqual(get.call_count, 2)
        self.assertIn("kubectl after TimeoutError", output.getvalue())
        self.assertNotIn("private download URL", output.getvalue())

    def test_permanent_http_error_is_not_retried_or_rendered(self):
        error = HTTPError("https://example.invalid/private", 404, "private", {}, None)
        with patch.object(provision.urllib.request, "urlopen", side_effect=error) as get:
            with self.assertRaisesRegex(ValueError, "^CI_CLI_DOWNLOAD helm HTTP_404$"):
                provision.download("helm", "https://example.invalid/")
        get.assert_called_once()

    def test_transient_failures_have_a_bounded_retry_budget(self):
        with patch.object(provision.urllib.request, "urlopen", side_effect=TimeoutError()) as get, \
                patch.object(provision.time, "sleep") as sleep, contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(ValueError, "^CI_CLI_DOWNLOAD kind TimeoutError$"):
                provision.download("kind", "https://example.invalid/")
        self.assertEqual(get.call_count, 3)
        self.assertEqual(sleep.call_count, 2)

    def test_retries_do_not_bypass_the_pin(self):
        with patch.object(provision, "download", return_value=b"not the pinned kind") as get, \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(ValueError, "^CI_CLI_CHECKSUM$"):
                provision.acquire(SimpleNamespace())
        get.assert_called_once()


if __name__ == "__main__":
    unittest.main()
