"""Offline check of the closed optional-auth route exceptions."""

from pathlib import Path
import re
import unittest
from urllib.parse import urlsplit

import yaml

ROOT = Path(__file__).resolve().parents[2]
PROJECT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
COMPONENT = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
OPTIONAL_ROUTES = {
    "/hive/api/organizations/search": "hive",
    "/manifesto/api/projects": "manifesto",
    f"/manifesto/api/projects/{PROJECT}": "manifesto",
    f"/manifesto/api/projects/{PROJECT}/details": "manifesto",
    f"/manifesto/api/projects/{PROJECT}/components": "manifesto",
    f"/manifesto/api/projects/{PROJECT}/components/{COMPONENT}": "manifesto",
}


def routes(config):
    listener, = config["static_resources"]["listeners"]
    chain, = listener["filter_chains"]
    manager, = chain["filters"]
    host, = manager["typed_config"]["route_config"]["virtual_hosts"]
    return host["routes"]


class OptionalAdmissionTests(unittest.TestCase):
    def test_both_envoys_exempt_only_anonymous_optional_reads(self):
        mesh = ROOT / "ops/deploy/mesh/envoy.yaml"
        overlay = ROOT / "ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml"
        kind, = [obj for obj in yaml.safe_load_all(overlay.read_text(encoding="utf-8"))
                 if obj and obj["kind"] == "ConfigMap" and obj["metadata"]["name"] == "envoy-mesh-config"]
        for config in (yaml.safe_load(mesh.read_text(encoding="utf-8")),
                       yaml.safe_load(kind["data"]["envoy.yaml"])):
            ordered = routes(config)
            exceptions = [item for item in ordered if item.get("name", "").startswith("anonymous_optional_")]
            self.assertCountEqual([item["route"]["cluster"] for item in exceptions], ["hive", "manifesto"])
            for item in exceptions:
                self.assertEqual(item["match"]["headers"], [
                    {"name": ":method", "string_match": {"safe_regex": {"google_re2": {}, "regex": "^(GET|HEAD)$"}}},
                    {"name": "authorization", "present_match": False},
                ])
                self.assertTrue(item["typed_per_filter_config"]["envoy.filters.http.ext_authz"]["disabled"])
                cluster = item["route"]["cluster"]
                catchall, = [route for route in ordered if route["match"].get("prefix") == f"/{cluster}/"]
                self.assertLess(ordered.index(item), ordered.index(catchall))
                self.assertNotIn("typed_per_filter_config", catchall)

                def matches(url):
                    path = urlsplit(url).path
                    spec = item["match"]
                    if "path" in spec:
                        return spec["path"] == path
                    return re.fullmatch(spec["safe_regex"]["regex"], path) is not None

                for path, service in OPTIONAL_ROUTES.items():
                    self.assertEqual(matches(path + "?test=1"), cluster == service, path)
                    self.assertFalse(matches(path + "/extra/" + PROJECT), path)
                for protected in (f"/hive/api/organizations/{PROJECT}", "/hive/api/organizations",
                                  f"/manifesto/api/projects/{PROJECT}/members",
                                  f"/manifesto/api/projects/{PROJECT}/bindings/{COMPONENT}",
                                  "/iam/api/me", "/telegraph/api/notifications"):
                    self.assertFalse(matches(protected), protected)


if __name__ == "__main__":
    unittest.main()
