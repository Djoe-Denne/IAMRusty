use std::collections::HashSet;

use async_trait::async_trait;
use manifesto_application::{ApplicationError, OrgScopeLookup};
use rustycog::permission::{OpenFgaPermissionChecker, RelationshipTuple};
use uuid::Uuid;

/// Resolves org viewer/admin scopes from stored OpenFGA tuples.
pub struct OpenFgaOrgScopeLookup {
    checker: OpenFgaPermissionChecker,
}

impl OpenFgaOrgScopeLookup {
    #[must_use]
    pub fn new(checker: OpenFgaPermissionChecker) -> Self {
        Self { checker }
    }

    async fn org_tuples_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<RelationshipTuple>, ApplicationError> {
        let user = format!("user:{user_id}");
        match self
            .checker
            .read_tuples(Some(user.as_str()), None, Some("organization:"))
            .await
        {
            Ok(tuples) => Ok(tuples),
            Err(_) => {
                let tuples = self
                    .checker
                    .read_tuples(Some(user.as_str()), None, None)
                    .await
                    .map_err(ApplicationError::from)?;
                Ok(tuples
                    .into_iter()
                    .filter(|tuple| tuple.object.starts_with("organization:"))
                    .collect())
            }
        }
    }
}

fn parse_org_id(object: &str) -> Option<Uuid> {
    object
        .strip_prefix("organization:")
        .and_then(|id| Uuid::parse_str(id).ok())
}

fn is_viewer_relation(relation: &str) -> bool {
    matches!(relation, "owner" | "admin" | "member" | "viewer")
}

fn is_admin_relation(relation: &str) -> bool {
    matches!(relation, "owner" | "admin")
}

fn org_ids_matching(tuples: &[RelationshipTuple], predicate: fn(&str) -> bool) -> Vec<Uuid> {
    let mut ids = HashSet::new();
    for tuple in tuples {
        if predicate(tuple.relation.as_str()) {
            if let Some(org_id) = parse_org_id(&tuple.object) {
                ids.insert(org_id);
            }
        }
    }
    ids.into_iter().collect()
}

#[async_trait]
impl OrgScopeLookup for OpenFgaOrgScopeLookup {
    async fn org_ids_where_viewer(&self, user_id: Uuid) -> Result<Vec<Uuid>, ApplicationError> {
        let tuples = self.org_tuples_for_user(user_id).await?;
        Ok(org_ids_matching(&tuples, is_viewer_relation))
    }

    async fn org_ids_where_admin(&self, user_id: Uuid) -> Result<Vec<Uuid>, ApplicationError> {
        let tuples = self.org_tuples_for_user(user_id).await?;
        Ok(org_ids_matching(&tuples, is_admin_relation))
    }
}
