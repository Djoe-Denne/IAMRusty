exit_after_auth = false
pid_file = "/run/oauth-openbao/agent.pid"
vault {
  address = "https://openbao.aiforall-local-full.svc.cluster.local:8443"
  ca_cert = "/run/openbao-ca/ca.crt"
}
auto_auth {
  method "kubernetes" {
    mount_path = "auth/kubernetes"
    min_backoff = "1s"
    max_backoff = "10s"
    config = {
      role = "oauth-connect"
      token_path = "/run/identity/jwt"
    }
  }
  sink "file" {
    config = {
      path = "/run/oauth-openbao/token"
      mode = 0600
    }
  }
}
