//! In-memory ensure_*_identity idempotence (no docker).

use async_trait::async_trait;
use iam_domain::entity::identity::{Identity, IdentityKind};
use iam_domain::error::DomainError;
use iam_domain::port::repository::IdentityRepository;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

struct MemoryIdentityRepo {
    by_iss_sub: Mutex<HashMap<(String, String), Identity>>,
    by_user_issuer_kind: Mutex<HashMap<(Uuid, String, String), Identity>>,
}

impl MemoryIdentityRepo {
    fn new() -> Self {
        Self {
            by_iss_sub: Mutex::new(HashMap::new()),
            by_user_issuer_kind: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl IdentityRepository for MemoryIdentityRepo {
    type Error = DomainError;

    async fn find_by_issuer_subject(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<Identity>, Self::Error> {
        Ok(self
            .by_iss_sub
            .lock()
            .unwrap()
            .get(&(issuer.to_string(), subject.to_string()))
            .cloned())
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Identity>, Self::Error> {
        Ok(self
            .by_iss_sub
            .lock()
            .unwrap()
            .values()
            .filter(|i| i.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn create(&self, identity: &Identity) -> Result<Identity, Self::Error> {
        self.by_iss_sub.lock().unwrap().insert(
            (identity.issuer.clone(), identity.subject.clone()),
            identity.clone(),
        );
        self.by_user_issuer_kind.lock().unwrap().insert(
            (
                identity.user_id,
                identity.issuer.clone(),
                String::from(&identity.kind),
            ),
            identity.clone(),
        );
        Ok(identity.clone())
    }

    async fn ensure_platform_identity(
        &self,
        user_id: Uuid,
        issuer: &str,
    ) -> Result<Identity, Self::Error> {
        let subject = user_id.to_string();
        if let Some(existing) = self.find_by_issuer_subject(issuer, &subject).await? {
            return Ok(existing);
        }
        self.create(&Identity::platform(user_id, issuer)).await
    }

    async fn ensure_organization_managed_identity(
        &self,
        user_id: Uuid,
        issuer: &str,
    ) -> Result<Identity, Self::Error> {
        let key = (
            user_id,
            issuer.to_string(),
            String::from(&IdentityKind::OrganizationManaged),
        );
        if let Some(existing) = self.by_user_issuer_kind.lock().unwrap().get(&key).cloned() {
            return Ok(existing);
        }
        self.create(&Identity::organization_managed(user_id, issuer))
            .await
    }
}

#[tokio::test]
async fn ensure_platform_identity_is_idempotent() {
    let repo = MemoryIdentityRepo::new();
    let user = Uuid::new_v4();
    let issuer = "http://127.0.0.1:8080/iam";

    let first = repo
        .ensure_platform_identity(user, issuer)
        .await
        .expect("create");
    let second = repo
        .ensure_platform_identity(user, issuer)
        .await
        .expect("idempotent");

    assert_eq!(first.id, second.id);
    assert_eq!(first.issuer, issuer);
    assert_eq!(first.subject, user.to_string());
    assert!(matches!(first.kind, IdentityKind::Platform));
    assert_eq!(repo.find_by_user_id(user).await.unwrap().len(), 1);
}

#[tokio::test]
async fn ensure_url_creates_second_row_when_iamrusty_exists() {
    let repo = MemoryIdentityRepo::new();
    let user = Uuid::new_v4();
    repo.create(&Identity::platform(user, "iamrusty"))
        .await
        .expect("seed iamrusty");

    let url = "http://127.0.0.1:8080/iam";
    let created = repo
        .ensure_platform_identity(user, url)
        .await
        .expect("create url identity");
    assert_eq!(created.issuer, url);
    assert_eq!(repo.find_by_user_id(user).await.unwrap().len(), 2);

    let again = repo
        .ensure_platform_identity(user, url)
        .await
        .expect("idempotent url");
    assert_eq!(again.id, created.id);
    assert_eq!(repo.find_by_user_id(user).await.unwrap().len(), 2);
}

#[tokio::test]
async fn organization_managed_create_idempotent_and_subject_differs() {
    let repo = MemoryIdentityRepo::new();
    let user = Uuid::new_v4();
    let issuer = "http://127.0.0.1:8080/iam/orgs/acme";

    let first = repo
        .ensure_organization_managed_identity(user, issuer)
        .await
        .expect("create");
    let second = repo
        .ensure_organization_managed_identity(user, issuer)
        .await
        .expect("idempotent");

    assert_eq!(first.id, second.id);
    assert_eq!(first.subject, second.subject);
    assert_ne!(first.subject, user.to_string());
    assert!(matches!(first.kind, IdentityKind::OrganizationManaged));
    assert_eq!(repo.find_by_user_id(user).await.unwrap().len(), 1);
}
