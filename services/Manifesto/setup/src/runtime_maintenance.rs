//! Explicit composition of local maintenance. No generic HTTP/queue endpoint,
//! no private key, no cluster client, and no fabricated D/provenance adapter.
use apparatus_contracts::runtime::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use manifesto_configuration::AppConfig;
use manifesto_infra::apparatus_runtime::{
    admission_proof::{PinnedRuntimeAdmissionKey, PinnedRuntimeAdmissionProofVerifier},
    runtime_control::maintenance::{
        RuntimeMaintenanceChannel, RuntimeMaintenanceProfile, SqlRuntimeMaintenance,
    },
};
use rsa::{pkcs8::DecodePublicKey, traits::PublicKeyParts, RsaPublicKey};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use sha2::{Digest as _, Sha256};
use std::sync::Arc;

/// Non-deserializable composition dependency. No default verified channel.
#[derive(Default)]
pub struct RuntimeMaintenanceOverrides {
    pub channel: Option<Arc<dyn RuntimeMaintenanceChannel>>,
}

/// Private command registry, not merged into the HTTP command registry. One
/// execution only; an uncertain response requires journal/state re-reading.
pub struct RuntimeMaintenanceRunner {
    service: rustycog::command::GenericCommandService,
}
impl RuntimeMaintenanceRunner {
    pub(crate) fn new(executor: Arc<SqlRuntimeMaintenance>) -> Self {
        use rustycog::command::{CommandRegistryBuilder, GenericCommandService, RegistryConfig};
        let registry = CommandRegistryBuilder::with_config(RegistryConfig::default())
            .register::<manifesto_application::RuntimeMaintenanceCommand, _>(
                "apparatus_runtime_maintenance_v1".to_owned(),
                Arc::new(manifesto_application::RuntimeMaintenanceCommandHandler::new(executor)),
                Arc::new(manifesto_application::RuntimeControlCommandErrorMapper),
            )
            .build();
        Self {
            service: GenericCommandService::new(Arc::new(registry)),
        }
    }

    pub async fn execute(
        &self,
        action: manifesto_application::RuntimeMaintenanceAction,
    ) -> Result<manifesto_application::RuntimeMaintenanceReply, RuntimeControlErrorV1> {
        self.service
            .execute_once(
                manifesto_application::RuntimeMaintenanceCommand::new(action),
                rustycog::command::CommandContext::new(),
            )
            .await
            .map_err(|_| {
                RuntimeErrorCodeV1::RuntimeAuthorityUnavailable.error(
                    RuntimeServiceRoleV1::Projector,
                    RuntimeEffectOutcomeV1::NotStarted,
                )
            })?
    }
}

pub(crate) async fn compose(
    db: Arc<DatabaseConnection>,
    config: &AppConfig,
    overrides: RuntimeMaintenanceOverrides,
) -> anyhow::Result<Arc<SqlRuntimeMaintenance>> {
    let runtime = &config.apparatus.runtime_control;
    let exists = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_regclass('apparatus_runtime_bindings') IS NOT NULL AS present".to_owned(),
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("runtime schema status unavailable"))?
        .try_get::<bool>("", "present")?;
    let fenced = if exists {
        db.query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM apparatus_runtime_bindings) AS fenced".to_owned(),
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("runtime fence status unavailable"))?
        .try_get::<bool>("", "fenced")?
    } else {
        false
    };
    if !runtime.enabled && fenced {
        return Err(anyhow::anyhow!(
            "runtime disabled with persistent fences; legacy fallback forbidden"
        ));
    }
    if runtime.enabled && !exists {
        return Err(anyhow::anyhow!(
            "runtime enabled without additive writer schema"
        ));
    }
    if !runtime.enabled && (runtime.maintenance_enabled || overrides.channel.is_some()) {
        return Err(anyhow::anyhow!(
            "maintenance requires explicit runtime mode"
        ));
    }
    let disabled = SqlRuntimeMaintenance::disabled(db);
    if !runtime.maintenance_enabled {
        if overrides.channel.is_some() {
            return Err(anyhow::anyhow!(
                "verified maintenance channel supplied while maintenance disabled"
            ));
        }
        return Ok(Arc::new(disabled));
    }
    let channel = overrides.channel.ok_or_else(|| {
        anyhow::anyhow!("maintenance enabled without verified operator/D/provenance channel")
    })?;
    let required = |value: &Option<String>| {
        value
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("runtime maintenance trust configuration incomplete"))
    };
    let authority_id = uuid::Uuid::parse_str(&required(&runtime.authority_id)?)?;
    let installation_id = uuid::Uuid::parse_str(&required(&runtime.installation_id)?)?;
    let profile = RuntimeMaintenanceProfile {
        authority_id,
        installation_id,
        profile: required(&runtime.profile)?,
        profile_lock_sha256: RuntimeDigestV1::new(required(&runtime.profile_lock_sha256)?)
            .map_err(|_| anyhow::anyhow!("invalid runtime profile lock"))?,
        endpoint_scheme: required(&runtime.endpoint_scheme)?,
        endpoint_port: runtime
            .endpoint_port
            .ok_or_else(|| anyhow::anyhow!("runtime endpoint port absent"))?,
    };
    let mut keys = Vec::new();
    for configured in &runtime.admission_keys {
        if configured.issuer != ADMISSION_ISSUER
            || configured.audience != ADMISSION_AUDIENCE
            || configured.purpose != ADMISSION_PURPOSE
            || uuid::Uuid::parse_str(&configured.authority_id)? != authority_id
            || uuid::Uuid::parse_str(&configured.installation_id)? != installation_id
        {
            return Err(anyhow::anyhow!(
                "admission key scope/purpose is not approved"
            ));
        }
        let pem = std::fs::read_to_string(&configured.public_key_pem_path)
            .map_err(|_| anyhow::anyhow!("pinned admission public key unavailable"))?;
        let public_key = RsaPublicKey::from_public_key_pem(&pem)
            .map_err(|_| anyhow::anyhow!("pinned admission public key malformed"))?;
        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let jwk = format!("{{\"e\":\"AQAB\",\"kty\":\"RSA\",\"n\":\"{n}\"}}");
        let kid = format!(
            "ara-v1-{}",
            URL_SAFE_NO_PAD.encode(Sha256::digest(jwk.as_bytes()))
        );
        if configured.kid != kid {
            return Err(anyhow::anyhow!("pinned admission key thumbprint mismatch"));
        }
        keys.push(PinnedRuntimeAdmissionKey {
            authority_id,
            installation_id,
            public_key,
        });
    }
    let verifier = Arc::new(
        PinnedRuntimeAdmissionProofVerifier::new(keys, runtime.allowed_repositories.clone())
            .map_err(|_| anyhow::anyhow!("pinned admission key/repository policy invalid"))?,
    );
    Ok(Arc::new(
        disabled
            .with_verified_channel(profile, channel, verifier)
            .map_err(|_| anyhow::anyhow!("maintenance profile invalid"))?,
    ))
}
