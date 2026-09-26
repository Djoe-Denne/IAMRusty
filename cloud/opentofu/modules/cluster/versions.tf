# Module contrat d'outputs cluster (ADR-0600 / tranche 0603)
#
# Pas de provider google. Pas d'apply GKE. `tofu validate` sans credentials.
# L'adaptateur GKE (`modules/cluster/gcp/` + `live/.../gcp/`) n'est PAS cette tranche.

terraform {
  required_version = ">= 1.10.0"
}
