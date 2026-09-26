variable "cluster_name" {
  type        = string
  description = "Nom logique du cluster (contrat 0600)."
  default     = "aiforall-local"
}

variable "kubernetes_host" {
  type        = string
  description = "URL API Kubernetes."
  default     = "https://127.0.0.1:6443"
}

variable "cluster_ca" {
  type        = string
  description = "CA du cluster (PEM). Jamais un kubeconfig git."
  default     = ""
  sensitive   = true
}

variable "oidc_issuer" {
  type        = string
  description = "Issuer OIDC workload identity ; null si le cluster n'en a pas."
  default     = null
  nullable    = true
}

variable "auth_exec" {
  type = object({
    api_version = string
    command     = string
    args        = list(string)
  })
  description = "Authentification client Kubernetes par exec (contrat 0600)."
  default = {
    api_version = "client.authentication.k8s.io/v1"
    command     = "kubectl"
    args        = ["oidc-login", "get-token"]
  }
}
