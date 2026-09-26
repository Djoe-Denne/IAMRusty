output "cluster_name" {
  description = "Nom du cluster (contrat 0600)."
  value       = var.cluster_name
}

output "kubernetes_host" {
  description = "Hôte API Kubernetes (contrat 0600)."
  value       = var.kubernetes_host
}

output "cluster_ca" {
  description = "CA cluster (contrat 0600)."
  value       = var.cluster_ca
  sensitive   = true
}

output "oidc_issuer" {
  description = "Issuer OIDC ; null autorisé (contrat 0600)."
  value       = var.oidc_issuer
}

output "auth_exec" {
  description = "Bloc auth exec client Kubernetes (contrat 0600). Pas de kubeconfig dans git."
  value       = var.auth_exec
}
