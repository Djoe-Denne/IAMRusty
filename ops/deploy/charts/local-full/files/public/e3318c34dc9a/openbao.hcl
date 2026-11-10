disable_mlock = true
api_addr = "https://openbao.aiforall-local-full.svc.cluster.local:8443"
cluster_addr = "https://127.0.0.1:8201"
listener "tcp" {
  address = "0.0.0.0:8443"
  cluster_address = "127.0.0.1:8444"
  tls_min_version = "tls12"
  tls_cert_file = "/tls/tls.crt"
  tls_key_file = "/tls/tls.key"
}
