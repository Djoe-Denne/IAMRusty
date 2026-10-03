//! Fixtures Docker service-local T6 (zot + `OpenBao` Transit).
//!
//! Placement local : pas de flag `has_openbao` / `has_k8s` sur un descripteur partagé.

pub mod zot;

#[path = "openbao-transit/mod.rs"]
pub mod openbao_transit;

/// Réseau Docker user-defined partagé (DNS par nom de conteneur).
pub const NETWORK_NAME: &str = "apparatus_operator_test-net";

/// Pile zot + Transit prête pour les IT T6.
pub struct SignRegistryStack {
    /// Registry OCI (push signer-only, pull anonyme).
    pub zot: std::sync::Arc<zot::TestZot>,
    /// `OpenBao` avec engine Transit (pas un mount KV).
    pub transit: std::sync::Arc<openbao_transit::TestOpenBaoTransit>,
}

impl SignRegistryStack {
    /// Démarre (ou réutilise) zot puis Transit sur [`NETWORK_NAME`].
    ///
    /// # Errors
    ///
    /// Échoue bruyamment si Docker n'est pas joignable, si un pull d'image
    /// échoue, ou si un service ne devient pas prêt avant le délai.
    pub async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        assert_docker_running()?;
        let zot = zot::TestZot::new().await?;
        let transit = openbao_transit::TestOpenBaoTransit::new().await?;
        Ok(Self { zot, transit })
    }

    /// Nom du réseau Docker des fixtures et des helpers ORAS/Cosign.
    #[must_use]
    pub const fn network_name() -> &'static str {
        NETWORK_NAME
    }
}

/// Vérifie que le démon Docker répond. Pas de skip silencieux.
///
/// # Errors
///
/// Retourne une erreur citant Docker si `docker info` échoue.
pub fn assert_docker_running() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("docker").args(["info"]).output();
    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!(
                "Docker daemon must be running (docker info failed, status={:?}): {stderr}",
                out.status.code()
            )
            .into())
        }
        Err(err) => {
            Err(format!("Docker daemon must be running (cannot execute docker: {err})").into())
        }
    }
}

fn docker_rm(container_name: &str) {
    let _ = std::process::Command::new("docker")
        .args(["stop", container_name])
        .output();
    let _ = std::process::Command::new("docker")
        .args(["rm", "-f", container_name])
        .output();
}
