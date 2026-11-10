"""Offline native Redis TLS profile: same backend/PVC, no plaintext port."""
import unittest

from ops.tests import test_minimal_native_openbao as bao


class NativeRedisDeclarationsTests(unittest.TestCase):
    render = bao.NativeOpenBaoDeclarationsTests.render

    def test_redis_create_keeps_data_backend_and_disables_plaintext(self):
        objects = self.render("redis.create=true")
        self.assertEqual(len(objects), 4)
        self.assertEqual({obj["kind"] for obj in objects}, {"ConfigMap", "Service", "PersistentVolumeClaim", "Deployment"})
        deployment = next(obj for obj in objects if obj["kind"] == "Deployment")
        self.assertEqual(deployment["metadata"]["name"], "redis")
        self.assertEqual(deployment["spec"]["replicas"], 1)
        self.assertEqual(deployment["spec"]["strategy"], {"type": "Recreate"})
        pod = deployment["spec"]["template"]["spec"]
        self.assertFalse(pod["automountServiceAccountToken"])
        self.assertEqual(pod["securityContext"]["runAsUser"], 999)
        container = pod["containers"][0]
        self.assertIn("redis:7.2.7-alpine@sha256:", container["image"])
        self.assertEqual(container["command"], ["redis-server", "/etc/redis/redis.conf"])
        config = next(obj for obj in objects if obj["kind"] == "ConfigMap")["data"]["redis.conf"]
        for setting in ("port 0", "tls-port 6379", "tls-auth-clients no", "tls-cert-file /tls/tls.crt", "tls-key-file /tls/tls.key", "tls-ca-cert-file /tls/ca.crt", "dir /data", "appendonly yes", "appendfsync always"):
            self.assertIn(setting, config.splitlines())
        self.assertEqual([line for line in config.splitlines() if line.startswith("protected-mode ")],
                         ["protected-mode no"], "no-auth lab must accept Lazaret cross-Pod, not only the loopback probe")
        self.assertEqual(next(v for v in pod["volumes"] if v["name"] == "data")["persistentVolumeClaim"]["claimName"], "redis-data")
        self.assertFalse(any(obj["kind"] == "Secret" for obj in objects))
        probe = " ".join(container["readinessProbe"]["exec"]["command"])
        for part in ("redis-cli", "--tls", "--cacert /tls/ca.crt", "--sni redis.aiforall-local-full.svc.cluster.local", "-h 127.0.0.1", "PING", "PONG"):
            self.assertIn(part, probe)
        self.assertNotIn("--insecure", probe)

    def test_redis_tls_references_are_read_only_and_same_stable_claim(self):
        objects = self.render("redis.create=true")
        pod = next(obj for obj in objects if obj["kind"] == "Deployment")["spec"]["template"]["spec"]
        tls = next(v for v in pod["volumes"] if v["name"] == "tls")["projected"]
        self.assertEqual(tls["defaultMode"], 0o440)
        self.assertEqual({s["secret"]["name"] for s in tls["sources"]}, {"redis-server-tls", "transport-ca"})
        mount = next(v for v in pod["containers"][0]["volumeMounts"] if v["name"] == "tls")
        self.assertTrue(mount["readOnly"])
        claim = next(obj for obj in objects if obj["kind"] == "PersistentVolumeClaim")
        self.assertEqual(claim["metadata"]["name"], "redis-data")

    def test_redis_schema_rejects_unpinned_images(self):
        self.render("redis.create=true", "redis.image=redis:latest", expected=1)


if __name__ == "__main__":
    unittest.main()
