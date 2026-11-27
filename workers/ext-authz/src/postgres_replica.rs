use crate::clock::boot_time;
use crate::replica_proof::{canonical_epoch, Proof, ReplicaAdmission, ReplicaSource, Snapshot};
use reqwest::{header::HeaderValue, Certificate, Client, Identity, StatusCode, Url};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgSslMode};
use std::{
    sync::{Arc, PoisonError, RwLock},
    time::Duration,
};
use tokio::time::{interval, timeout, MissedTickBehavior};
use uuid::Uuid;

const PROBE_INTERVAL: Duration = Duration::from_secs(1);
const PROBE_BUDGET: Duration = Duration::from_millis(900);
const SNAPSHOT_SQL: &str = "
    SELECT r.generation,
           EXISTS (SELECT 1 FROM public.iam_session_liveness_checkpoints WHERE challenge = $2),
           (SELECT auth_epoch FROM public.iam_session_liveness_epochs WHERE subject = $3)
    FROM public.iam_session_liveness_ready r
    WHERE r.configured_generation = $1";

struct PostgresReplica {
    pool: PgPool,
    configured_generation: Uuid,
}

impl PostgresReplica {
    async fn ready_generation(&self) -> Option<Uuid> {
        sqlx::query_scalar(
            "SELECT generation FROM public.iam_session_liveness_ready WHERE configured_generation = $1",
        )
        .bind(self.configured_generation)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
    }
}

#[async_trait::async_trait]
impl ReplicaSource for PostgresReplica {
    async fn snapshot(&self, challenge: Uuid, subject: Uuid) -> Option<Snapshot> {
        let (generation, applied, epoch): (Uuid, bool, Option<String>) =
            sqlx::query_as(SNAPSHOT_SQL)
                .bind(self.configured_generation)
                .bind(challenge)
                .bind(subject)
                .fetch_optional(&self.pool)
                .await
                .ok()??;
        let epoch = match epoch {
            Some(text) => Some(canonical_epoch(&text)?),
            None => None,
        };
        Some(Snapshot {
            ready_generation: Some(generation),
            applied_challenge: applied.then_some(challenge),
            epoch,
        })
    }
}

struct CheckpointClient {
    http: Client,
    endpoint: Url,
    token: HeaderValue,
}

impl CheckpointClient {
    async fn prove(&self, replica: &PostgresReplica) -> Option<Proof> {
        let generation = replica.ready_generation().await?;
        let challenge = Uuid::new_v4();
        let started = boot_time()?;
        let response = self
            .http
            .post(self.endpoint.clone())
            .header("x-iam-checkpoint-token", self.token.clone())
            .json(&serde_json::json!({ "challenge": challenge }))
            .send()
            .await
            .ok()?;
        if response.status() != StatusCode::NO_CONTENT {
            return None;
        }
        let mut ticks = interval(Duration::from_millis(40));
        loop {
            ticks.tick().await;
            let snapshot = replica.snapshot(challenge, Uuid::nil()).await?;
            if snapshot.ready_generation != Some(generation) {
                return None;
            }
            if snapshot.applied_challenge == Some(challenge) {
                return Some(Proof {
                    challenge,
                    generation,
                    started,
                });
            }
        }
    }
}

pub struct ReplicaRuntime {
    replica: Arc<PostgresReplica>,
    checkpoint: CheckpointClient,
}

impl ReplicaRuntime {
    pub(super) fn from_env() -> Result<Option<Self>, String> {
        let dsn = match std::env::var("EXT_AUTHZ_REPLICA_DSN") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(_) => return Err("invalid replica DSN".into()),
        };
        if boot_time().is_none() {
            return Err("replica admission requires a suspend-inclusive clock".into());
        }
        let options = dsn
            .parse::<PgConnectOptions>()
            .map_err(|_| "invalid replica DSN")?;
        if !matches!(options.get_ssl_mode(), PgSslMode::VerifyFull) {
            return Err("replica DSN requires sslmode=verify-full".into());
        }
        let configured_generation = std::env::var("EXT_AUTHZ_REPLICA_GENERATION")
            .ok()
            .and_then(|value| Uuid::parse_str(&value).ok())
            .ok_or("missing or invalid replica generation")?;
        let checkpoint = CheckpointClient::from_env()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_millis(150))
            .connect_lazy_with(options);
        Ok(Some(Self {
            replica: Arc::new(PostgresReplica {
                pool,
                configured_generation,
            }),
            checkpoint,
        }))
    }

    pub(super) fn start(self) -> Arc<ReplicaAdmission> {
        let admission = Arc::new(ReplicaAdmission {
            clock: Arc::new(boot_time),
            proof: RwLock::new(None),
            source: self.replica.clone(),
        });
        let worker = admission.clone();
        tokio::spawn(async move {
            let mut ticks = interval(PROBE_INTERVAL);
            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                ticks.tick().await;
                if let Ok(Some(proof)) =
                    timeout(PROBE_BUDGET, self.checkpoint.prove(&self.replica)).await
                {
                    worker
                        .proof
                        .write()
                        .unwrap_or_else(PoisonError::into_inner)
                        .replace(proof);
                } else {
                    tracing::debug!("replica checkpoint unverified");
                }
            }
        });
        admission
    }
}

impl CheckpointClient {
    fn from_env() -> Result<Self, String> {
        let endpoint = std::env::var("EXT_AUTHZ_CHECKPOINT_URL")
            .ok()
            .and_then(|value| Url::parse(&value).ok())
            .filter(|url| {
                url.scheme() == "https"
                    && url.host().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
                    && url.path() == "/iam/internal/auth/session-liveness/checkpoint"
            })
            .ok_or("checkpoint endpoint requires the private HTTPS IAM checkpoint URL")?;
        let mut token = std::env::var("EXT_AUTHZ_CHECKPOINT_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| HeaderValue::from_str(&value).ok())
            .ok_or("missing or invalid checkpoint capability")?;
        token.set_sensitive(true);
        let http = iam_https_client(
            "EXT_AUTHZ_CHECKPOINT_CA_FILE",
            "EXT_AUTHZ_CHECKPOINT_IDENTITY_FILE",
        )?;
        Ok(Self {
            http,
            endpoint,
            token,
        })
    }
}

pub fn iam_https_client(ca: &str, identity: &str) -> Result<Client, String> {
    let mut http = Client::builder()
        .use_rustls_tls()
        .no_proxy()
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none());
    if let Some(pem) = tls_file(ca)? {
        for cert in Certificate::from_pem_bundle(&pem).map_err(|_| "invalid IAM workload CA")? {
            http = http.add_root_certificate(cert);
        }
        http = http.tls_built_in_root_certs(false);
    }
    if let Some(pem) = tls_file(identity)? {
        let identity = Identity::from_pem(&pem).map_err(|_| "invalid IAM workload TLS identity")?;
        http = http.identity(identity);
    }
    http.build()
        .map_err(|_| "IAM workload HTTP client setup failed".into())
}

fn tls_file(variable: &str) -> Result<Option<Vec<u8>>, String> {
    std::env::var_os(variable)
        .map(|path| std::fs::read(path).map_err(|_| format!("cannot read {variable}")))
        .transpose()
}
