//! Minimal OpenFGA HTTP client used by sentinel-sync to write and delete
//! relation tuples. Keeps only the surface the sync worker needs.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::config::OpenFgaConfig;

/// One relation tuple, expressed in the OpenFGA wire format
/// `{object_type}:{object_id}#{relation}@{user_type}:{user_id}`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct Tuple {
    pub object_type: String,
    pub object_id: String,
    pub relation: String,
    pub user_type: String,
    pub user_id: String,
}

impl Tuple {
    /// Tuple with a `user:{uuid}` subject.
    pub fn user(
        object_type: impl Into<String>,
        object_id: Uuid,
        relation: impl Into<String>,
        user_id: Uuid,
    ) -> Self {
        Self {
            object_type: object_type.into(),
            object_id: object_id.to_string(),
            relation: relation.into(),
            user_type: "user".to_string(),
            user_id: user_id.to_string(),
        }
    }

    /// Tuple pointing at another object (parent relation), e.g.
    /// `project:123#organization@organization:456`.
    pub fn object(
        object_type: impl Into<String>,
        object_id: Uuid,
        relation: impl Into<String>,
        parent_type: impl Into<String>,
        parent_id: Uuid,
    ) -> Self {
        Self {
            object_type: object_type.into(),
            object_id: object_id.to_string(),
            relation: relation.into(),
            user_type: parent_type.into(),
            user_id: parent_id.to_string(),
        }
    }
}

#[derive(Serialize)]
struct TupleKey<'a> {
    user: String,
    relation: &'a str,
    object: String,
}

impl<'a> From<&'a Tuple> for TupleKey<'a> {
    fn from(t: &'a Tuple) -> Self {
        TupleKey {
            user: format!("{}:{}", t.user_type, t.user_id),
            relation: &t.relation,
            object: format!("{}:{}", t.object_type, t.object_id),
        }
    }
}

#[derive(Serialize)]
struct WriteRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    writes: Option<TupleKeyList<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deletes: Option<TupleKeyList<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization_model_id: Option<&'a str>,
}

#[derive(Serialize)]
struct TupleKeyList<'a> {
    tuple_keys: Vec<TupleKey<'a>>,
}

/// Thin HTTP client around OpenFGA's `/stores/{id}/write` endpoint.
#[derive(Clone)]
pub struct OpenFgaWriteClient {
    config: OpenFgaConfig,
    http: reqwest::Client,
}

impl OpenFgaWriteClient {
    pub fn new(config: OpenFgaConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build OpenFGA HTTP client")?;
        Ok(Self { config, http })
    }

    fn write_url(&self) -> String {
        format!(
            "{}/stores/{}/write",
            self.config.api_url().trim_end_matches('/'),
            self.config.store_id
        )
    }

    /// Write and/or delete a batch of tuples atomically.
    ///
    /// OpenFGA's write endpoint is atomic per call, so the caller can fuse
    /// "remove old, add new" transitions into one request.
    pub async fn write(&self, writes: &[Tuple], deletes: &[Tuple]) -> Result<()> {
        if writes.is_empty() && deletes.is_empty() {
            return Ok(());
        }

        let writes_payload = if writes.is_empty() {
            None
        } else {
            Some(TupleKeyList {
                tuple_keys: writes.iter().map(TupleKey::from).collect(),
            })
        };
        let deletes_payload = if deletes.is_empty() {
            None
        } else {
            Some(TupleKeyList {
                tuple_keys: deletes.iter().map(TupleKey::from).collect(),
            })
        };

        let body = WriteRequest {
            writes: writes_payload,
            deletes: deletes_payload,
            authorization_model_id: self.config.authorization_model_id.as_deref(),
        };

        let mut req = self.http.post(self.write_url()).json(&body);
        if let Some(token) = &self.config.api_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await.context("OpenFGA write request failed")?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            warn!(status = %status, body = %text, "OpenFGA write returned non-success status");
            return Err(anyhow!("OpenFGA write returned {status}: {text}"));
        }

        debug!(
            writes = writes.len(),
            deletes = deletes.len(),
            "OpenFGA write succeeded"
        );
        Ok(())
    }
}
