use std::sync::Arc;

use rustycog_permission::{casbin::CasbinPermissionEngine, Permission, PermissionsFetcher, PermissionEngine, ResourceId};
use rustycog_core::error::DomainError;
use uuid::Uuid;

struct MockFetcher {
    // Map of (user_id, resource_key) -> permissions
    // resource_key is a comma-separated list of UUIDs, built the same way as the engine
    rules: std::collections::HashMap<(Option<Uuid>, String), Vec<Permission>>,
}

impl MockFetcher {
    fn new() -> Self {
        Self { rules: std::collections::HashMap::new() }
    }

    fn set(&mut self, user: Uuid, resource_ids: &[ResourceId], perms: Vec<Permission>) {
        let key = resource_ids.iter().map(|u| u.id().to_string()).collect::<Vec<_>>().join(",");
        self.rules.insert((Some(user), key), perms);
    }
}

#[async_trait::async_trait]
impl PermissionsFetcher for MockFetcher {
    async fn fetch_permissions(&self, user_id: Option<Uuid>, resource_ids: Vec<ResourceId>) -> Result<Vec<Permission>, DomainError> {
        let key = resource_ids.iter().map(|u| u.id().to_string()).collect::<Vec<_>>().join(",");
        Ok(self.rules.get(&(user_id, key)).cloned().unwrap_or_default())
    }
}

fn fixture_model_path_1() -> String { "tests/fixtures/model_1level.conf".to_string() }
fn fixture_model_path_2() -> String { "tests/fixtures/model_2level.conf".to_string() }
fn fixture_model_path_3() -> String { "tests/fixtures/model_3level.conf".to_string() }

#[tokio::test]
async fn allows_direct_read() {
    let user = Uuid::new_v4();
    let resources = vec![ResourceId::new_v4()];
    let mut fetcher = MockFetcher::new();
    fetcher.set(user, &resources, vec![Permission::Read]);

    let engine = CasbinPermissionEngine::new(fixture_model_path_1(), Arc::new(fetcher))
        .await
        .unwrap();

    let ok = engine
        .has_permission(Some(user), resources.clone(), Permission::Read, serde_json::json!({}))
        .await
        .unwrap();
    assert!(ok);

    let nok = engine
        .has_permission(Some(user), resources.clone(), Permission::Write, serde_json::json!({}))
        .await
        .unwrap();
    assert!(!nok);
}

#[tokio::test]
async fn hierarchy_allows_owner_all() {
    let user = Uuid::new_v4();
    let resources = vec![ResourceId::new_v4(), ResourceId::new_v4()];
    let mut fetcher = MockFetcher::new();
    fetcher.set(user, &resources, vec![Permission::Owner]);

    let engine = CasbinPermissionEngine::new(fixture_model_path_2(), Arc::new(fetcher))
        .await
        .unwrap();

    for p in [Permission::Read, Permission::Write, Permission::Admin, Permission::Owner] {
        let ok = engine
            .has_permission(Some(user), resources.clone(), p.clone(), serde_json::json!({}))
            .await
            .unwrap();
        assert!(ok, "owner should allow {:?}", p);
    }
}

#[tokio::test]
async fn denies_when_no_policy() {
    let user = Uuid::new_v4();
    let resources = vec![ResourceId::new_v4()];
    let fetcher = MockFetcher::new();

    let engine = CasbinPermissionEngine::new(fixture_model_path_1(), Arc::new(fetcher))
        .await
        .unwrap();

    for p in [Permission::Read, Permission::Write, Permission::Admin, Permission::Owner] {
        let ok = engine
            .has_permission(Some(user), resources.clone(), p.clone(), serde_json::json!({}))
            .await
            .unwrap();
        assert!(!ok, "no policy should deny {:?}", p);
    }
}

#[tokio::test]
async fn two_level_specificity() {
    let user = Uuid::new_v4();
    let org = Uuid::new_v4();
    let member = Uuid::new_v4();
    let resources = vec![org.into(), member.into()];
    let mut fetcher = MockFetcher::new();
    fetcher.set(user, &resources, vec![Permission::Write]);

    let engine = CasbinPermissionEngine::new(fixture_model_path_2(), Arc::new(fetcher))
        .await
        .unwrap();

    // Write allowed on (org, member)
    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Write, serde_json::json!({}))
        .await
        .unwrap());
    // Read implied
    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Read, serde_json::json!({}))
        .await
        .unwrap());
    // Admin not implied by write
    assert!(!engine
        .has_permission(Some(user), resources.clone(), Permission::Admin, serde_json::json!({}))
        .await
        .unwrap());
}

#[tokio::test]
async fn three_level_owner_allows_all() {
    let user = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let resources = vec![a.into(), b.into(), c.into()];
    let mut fetcher = MockFetcher::new();
    fetcher.set(user, &resources, vec![Permission::Owner]);

    let engine = CasbinPermissionEngine::new(fixture_model_path_3(), Arc::new(fetcher))
        .await
        .unwrap();

    for p in [Permission::Read, Permission::Write, Permission::Admin, Permission::Owner] {
        assert!(engine
            .has_permission(Some(user), resources.clone(), p.clone(), serde_json::json!({}))
            .await
            .unwrap());
    }
}

#[tokio::test]
async fn three_level_read_allows_read_only() {
    let user = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let resources = vec![a.into(), b.into(), c.into()];
    let mut fetcher = MockFetcher::new();
    fetcher.set(user, &resources, vec![Permission::Read]);

    let engine = CasbinPermissionEngine::new(fixture_model_path_3(), Arc::new(fetcher))
        .await
        .unwrap();

    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Read, serde_json::json!({}))
        .await
        .unwrap());

    assert!(!engine
        .has_permission(Some(user), resources.clone(), Permission::Write, serde_json::json!({}))
        .await
        .unwrap());

    assert!(!engine
        .has_permission(Some(user), resources.clone(), Permission::Admin, serde_json::json!({}))
        .await
        .unwrap());

    assert!(!engine
        .has_permission(Some(user), resources.clone(), Permission::Owner, serde_json::json!({}))
        .await
        .unwrap());
}

#[tokio::test]
async fn three_level_write_allows_rw_only() {
    let user = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let resources = vec![a.into(), b.into(), c.into()];
    let mut fetcher = MockFetcher::new();
    fetcher.set(user, &resources, vec![Permission::Write]);

    let engine = CasbinPermissionEngine::new(fixture_model_path_3(), Arc::new(fetcher))
        .await
        .unwrap();

    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Write, serde_json::json!({}))
        .await
        .unwrap());

    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Read, serde_json::json!({}))
        .await
        .unwrap());

    assert!(!engine
        .has_permission(Some(user), resources.clone(), Permission::Admin, serde_json::json!({}))
        .await
        .unwrap());

    assert!(!engine
        .has_permission(Some(user), resources.clone(), Permission::Owner, serde_json::json!({}))
        .await
        .unwrap());
}
#[tokio::test]
async fn three_level_admin_allows_raw() {
    let user = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let resources = vec![a.into(), b.into(), c.into()];
    let mut fetcher = MockFetcher::new();
    fetcher.set(user, &resources, vec![Permission::Admin]);

    let engine = CasbinPermissionEngine::new(fixture_model_path_3(), Arc::new(fetcher))
        .await
        .unwrap();

    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Write, serde_json::json!({}))
        .await
        .unwrap());

    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Admin, serde_json::json!({}))
        .await
        .unwrap());

    assert!(engine
        .has_permission(Some(user), resources.clone(), Permission::Read, serde_json::json!({}))
        .await
        .unwrap());

    assert!(!engine
        .has_permission(Some(user), resources.clone(), Permission::Owner, serde_json::json!({}))
        .await
        .unwrap());
}
