"""Public oracles stay fixed; the declared GitLab-only profile is not a fallback."""
import json
from pathlib import Path
import tomllib
import sys
import unittest


DIRECTORY = Path(__file__).parent


class PublicRouteProfileTests(unittest.TestCase):
    def test_gitlab_profile_preserves_all_33_ids_paths_methods_and_bodies(self):
        base = json.loads((DIRECTORY / "mesh-public-route-cases-local-full.json").read_text())
        oauth = json.loads((DIRECTORY / "mesh-public-route-cases-local-full-oauth-proof.json").read_text())
        self.assertEqual(len(base), 33)
        self.assertEqual(len(oauth), 33)
        self.assertEqual([{k: v for k, v in row.items() if k != "statuses"} for row in base],
                         [{k: v for k, v in row.items() if k != "statuses"} for row in oauth])
        changed = {left["id"]: (left["statuses"], right["statuses"])
                   for left, right in zip(base, oauth) if left["statuses"] != right["statuses"]}
        self.assertEqual(changed, {"login-unavailable-gitlab-head": ([422], [303]),
                                  "callback-get-requires-state": ([400], [422]),
                                  "disabled-relink-gitlab-protected": ([401, 403], [400])})

    def test_gitlab_profile_has_one_declared_connector_and_keeps_github_disabled(self):
        sys.path.insert(0, str(DIRECTORY.parent / "deploy"))
        from local_full_helm import render
        config = tomllib.loads(next(o for o in render("gitlab-proof") if o["kind"] == "ConfigMap"
            and o["metadata"]["name"].startswith("iam-dev-config"))["data"]["development.toml"])
        self.assertEqual([row["id"] for row in config["idp"]["connectors"]], ["gitlab"])
        self.assertIn("http://127.0.0.1:18080/iam/api/auth/gitlab/relink-callback",
                      config["idp"]["connectors"][0]["redirect_uris"])
        cases = {row["id"]: row for row in json.loads(
            (DIRECTORY / "mesh-public-route-cases-local-full-oauth-proof.json").read_text())}
        self.assertEqual(cases["login-unavailable-github"]["statuses"], [422])
        self.assertEqual(cases["disabled-relink-github-protected"]["statuses"], [401, 403])
        # oauth_callback resolves the connector before checking missing state;
        # AuthError::oauth_connector_not_configured is 422 (Rust error_mapping IT).
        self.assertEqual(cases["callback-get-requires-state"]["statuses"], [422])
        self.assertEqual(cases["unknown-provider-login"]["statuses"], [401, 403])
        self.assertEqual(cases["unknown-provider-callback"]["statuses"], [401, 403])


if __name__ == "__main__":
    unittest.main()
