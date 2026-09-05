//! Minimal `OpenFGA` HTTP client used by sentinel-sync to write and delete
//! relation tuples. Keeps only the surface the sync worker needs.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::config::OpenFgaConfig;

/// One relation tuple, expressed in the `OpenFGA` wire format
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

    /// Public-read subject `user:*` (`OpenFGA` wildcard).
    pub fn wildcard_user(
        object_type: impl Into<String>,
        object_id: Uuid,
        relation: impl Into<String>,
    ) -> Self {
        Self {
            object_type: object_type.into(),
            object_id: object_id.to_string(),
            relation: relation.into(),
            user_type: "user".to_string(),
            user_id: "*".to_string(),
        }
    }

    /// Userset subject, e.g. `project:123#viewer@organization:456#member`.
    pub fn userset(
        object_type: impl Into<String>,
        object_id: Uuid,
        relation: impl Into<String>,
        user_type: impl Into<String>,
        user_id: Uuid,
        user_relation: impl AsRef<str>,
    ) -> Self {
        Self {
            object_type: object_type.into(),
            object_id: object_id.to_string(),
            relation: relation.into(),
            user_type: user_type.into(),
            user_id: format!("{}#{}", user_id, user_relation.as_ref()),
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

/// Thin HTTP client around `OpenFGA`'s `/stores/{id}/write` endpoint.
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
    /// `OpenFGA`'s write endpoint is atomic per call, so the caller can fuse
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

    /// Apply desired `viewer@user:*` presence for each project id.
    ///
    /// `true` writes the public wildcard; `false` deletes it. Both sides are
    /// idempotent (`already exists` / missing tuple are treated as success).
    ///
    /// # Errors
    ///
    /// Returns an error if OpenFGA rejects a write or delete for a reason
    /// other than an idempotent no-op.
    #[allow(dead_code)]
    pub async fn reconcile_wildcards(&self, desired: Vec<(Uuid, bool)>) -> Result<()> {
        let (writes, deletes) = wildcard_reconcile_ops(&desired);
        for tuple in writes {
            self.write_idempotent(&[tuple], &[]).await?;
        }
        for tuple in deletes {
            self.write_idempotent(&[], &[tuple]).await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn write_idempotent(&self, writes: &[Tuple], deletes: &[Tuple]) -> Result<()> {
        match self.write(writes, deletes).await {
            Ok(()) => Ok(()),
            Err(error) => {
                let text = error.to_string();
                if text.contains("cannot_write_tuple_which_already_exists")
                    || text.contains("cannot_delete_tuple_which_does_not_exist")
                    || text.contains("cannot_delete_unknown_tuple")
                {
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }
}

/// Build the write/delete batches for a wildcard sweep.
#[must_use]
pub fn wildcard_reconcile_ops(desired: &[(Uuid, bool)]) -> (Vec<Tuple>, Vec<Tuple>) {
    let mut writes = Vec::new();
    let mut deletes = Vec::new();
    for &(project_id, wants_wildcard) in desired {
        let tuple = Tuple::wildcard_user("project", project_id, "viewer");
        if wants_wildcard {
            writes.push(tuple);
        } else {
            deletes.push(tuple);
        }
    }
    (writes, deletes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_ops_writes_wanted_wildcards_and_deletes_unwanted() {
        let keep = Uuid::new_v4();
        let drop = Uuid::new_v4();
        let (writes, deletes) = wildcard_reconcile_ops(&[(keep, true), (drop, false)]);
        assert_eq!(
            writes,
            vec![Tuple::wildcard_user("project", keep, "viewer")]
        );
        assert_eq!(
            deletes,
            vec![Tuple::wildcard_user("project", drop, "viewer")]
        );
    }
}
