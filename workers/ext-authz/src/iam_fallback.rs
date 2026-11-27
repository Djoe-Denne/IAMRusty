use crate::postgres_replica::iam_https_client;
use crate::replica_proof::canonical_epoch;
use reqwest::{
    header::{HeaderName, HeaderValue},
    Client, StatusCode, Url,
};
use rustycog::{
    config::SessionLivenessConfig,
    http::{
        SessionLivenessChecker, SessionLivenessDecision, SessionLivenessError,
        SessionLivenessOutcome, SessionLivenessQuery,
    },
};
use serde::Deserialize;
use std::time::Duration;

pub struct IamFallback {
    http: Client,
    endpoint: Url,
    header: HeaderName,
    token: HeaderValue,
    timeout: Duration,
}

impl IamFallback {
    pub(super) fn new(config: &SessionLivenessConfig) -> Result<Self, String> {
        let endpoint = config
            .endpoint
            .as_deref()
            .and_then(|value| Url::parse(value).ok())
            .filter(|url| {
                url.scheme() == "https"
                    && url.host().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
                    && url.path() == "/iam/internal/auth/session-liveness"
            })
            .ok_or("replica mode requires the private HTTPS IAM liveness fallback URL")?;
        let header = HeaderName::from_bytes(
            config
                .token_header
                .as_deref()
                .unwrap_or("x-iam-internal-token")
                .as_bytes(),
        )
        .map_err(|_| "invalid fallback capability header")?;
        let mut token = config
            .token_env
            .as_deref()
            .and_then(|name| std::env::var(name).ok())
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| HeaderValue::from_str(&value).ok())
            .ok_or("missing or invalid fallback capability")?;
        token.set_sensitive(true);
        let timeout = Duration::from_millis(config.timeout_ms.unwrap_or(2_000));
        if timeout.is_zero() {
            return Err("liveness timeout must be positive".into());
        }
        let http = iam_https_client(
            "EXT_AUTHZ_LIVENESS_CA_FILE",
            "EXT_AUTHZ_LIVENESS_IDENTITY_FILE",
        )?;
        Ok(Self {
            http,
            endpoint,
            header,
            token,
            timeout,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Decision {
    Live,
    Revoked,
}

#[derive(Deserialize)]
struct Reply {
    decision: Decision,
    current_epoch: Option<String>,
}

impl Reply {
    fn checked(self, claim: Option<u64>) -> Option<SessionLivenessOutcome> {
        if self.current_epoch.is_none() && matches!(self.decision, Decision::Revoked) {
            return Some(SessionLivenessOutcome::new(
                SessionLivenessDecision::Revoked,
            ));
        }
        let epoch = canonical_epoch(self.current_epoch.as_deref()?)?;
        let live = matches!(self.decision, Decision::Live);
        if live != (claim.unwrap_or(0) == epoch) {
            return None;
        }
        let decision = if live {
            SessionLivenessDecision::Live
        } else {
            SessionLivenessDecision::Revoked
        };
        Some(SessionLivenessOutcome::with_current_epoch(decision, epoch))
    }
}

#[async_trait::async_trait]
impl SessionLivenessChecker for IamFallback {
    async fn check(
        &self,
        query: &SessionLivenessQuery,
    ) -> Result<SessionLivenessOutcome, SessionLivenessError> {
        let mut body = serde_json::json!({"issuer": query.issuer, "subject": query.subject});
        if let Some(epoch) = query.auth_epoch {
            body["auth_epoch"] = serde_json::json!(epoch.to_string());
        }
        let mut response = self
            .http
            .post(self.endpoint.clone())
            .timeout(self.timeout)
            .header(self.header.clone(), self.token.clone())
            .json(&body)
            .send()
            .await
            .map_err(|_| SessionLivenessError::unavailable("liveness request failed"))?;
        if response.status() != StatusCode::OK {
            return Err(SessionLivenessError::unavailable(
                "liveness authority cannot decide",
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| SessionLivenessError::unavailable("liveness body read failed"))?
        {
            if chunk.len() > 1024_usize.saturating_sub(bytes.len()) {
                return Err(SessionLivenessError::unavailable(
                    "liveness body exceeds limit",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice::<Reply>(&bytes)
            .ok()
            .and_then(|reply| reply.checked(query.auth_epoch))
            .ok_or_else(|| SessionLivenessError::unavailable("invalid liveness response"))
    }
}

#[cfg(test)]
mod tests {
    use super::{IamFallback, Reply};
    use axum::{routing::post, Json, Router};
    use reqwest::{
        header::{HeaderName, HeaderValue},
        Client,
    };
    use rustycog::http::{
        SessionLivenessChecker, SessionLivenessDecision, SessionLivenessError, SessionLivenessQuery,
    };
    use std::time::Duration;
    use uuid::Uuid;

    #[tokio::test]
    async fn fallback_does_not_accept_a_positive_reply_after_the_configured_timeout() {
        let app = Router::new().route(
            "/liveness",
            post(|| async {
                tokio::time::sleep(Duration::from_millis(250)).await;
                Json(serde_json::json!({"decision": "live", "current_epoch": "0"}))
            }),
        );
        let server = axum_test::TestServer::builder()
            .http_transport()
            .build(app)
            .expect("local authority");
        let mut fallback = IamFallback {
            http: Client::new(),
            endpoint: server.server_url("/liveness").expect("local authority URL"),
            header: HeaderName::from_static("x-test-workload-token"),
            token: HeaderValue::from_static("fixture"),
            timeout: Duration::from_millis(50),
        };
        let query = SessionLivenessQuery {
            issuer: "https://iam.example.test".into(),
            subject: Uuid::new_v4(),
            auth_epoch: None,
        };
        assert!(matches!(
            fallback.check(&query).await,
            Err(SessionLivenessError::Unavailable { .. })
        ));
        fallback.timeout = Duration::from_secs(2);
        assert_eq!(
            fallback
                .check(&query)
                .await
                .expect("positive control")
                .decision,
            SessionLivenessDecision::Live
        );
    }

    #[test]
    fn fallback_allows_only_a_matching_canonical_epoch() {
        for (body, claim, expected) in [
            (
                r#"{"decision":"live","current_epoch":"0"}"#,
                None,
                Some(SessionLivenessDecision::Live),
            ),
            (r#"{"decision":"live","current_epoch":"1"}"#, Some(2), None),
            (r#"{"decision":"live","current_epoch":"01"}"#, Some(1), None),
            (r#"{"decision":"live"}"#, None, None),
            (r#"{"decision":"not_applicable"}"#, None, None),
            (
                r#"{"decision":"revoked"}"#,
                None,
                Some(SessionLivenessDecision::Revoked),
            ),
            (
                r#"{"decision":"revoked","current_epoch":"1"}"#,
                Some(0),
                Some(SessionLivenessDecision::Revoked),
            ),
        ] {
            let decision = serde_json::from_str::<Reply>(body)
                .ok()
                .and_then(|reply| reply.checked(claim))
                .map(|outcome| outcome.decision);
            assert_eq!(decision, expected);
        }
    }
}
