//! Apparatus P1 — T6 compatibilité HTTP (GREEN, TDD Phase C).
//!
//! Serveur live + `ComponentServiceMockService` (catalogue wiremock déjà arrangé
//! par `setup_test_server` : pas de `reset()`, pas de ré-arrange).
//! Harness unique (5-tuple). `#[serial]` partout en live.
//! Contrat sérialisé gelé : `ComponentResponse` = id, component_type, status,
//! added_at, configured_at, activated_at, disabled_at ; liste = {data: [...]}.
//! Alias binding GREEN : query `?binding=<même component_id>` sur le GET existant
//! (même ressource, pas de seconde ressource, pas de renommage route).

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use rustycog::permission::{Permission, ResourceRef, Subject};
use serial_test::serial;
use uuid::Uuid;

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

/// Clés triées d'un objet JSON (contrat gelé).
fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("objet JSON")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

#[test]
fn t6_binding_alias_code_exists() {
    // GREEN : alias binding exigé dans la couche HTTP (pas seulement infra T1-T4).
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hits = Vec::new();
    for sub in ["http/src", "application/src", "domain/src", "infra/src"] {
        scan(&root.join(sub), &mut hits);
    }
    assert!(
        !hits.is_empty(),
        "RED T6 : aucun code d'alias binding (binding/alias) dans le prod Manifesto"
    );
    assert!(
        hits.iter().any(|h| h.contains("http")),
        "T6 : alias binding absent de la couche HTTP (vert incident infra seul)"
    );
}

fn scan(dir: &std::path::Path, hits: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan(&path, hits);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let lower = content.to_lowercase();
                // `binding` seul suffit ; `alias` générique (ex. « Type alias »)
                // est exclu — seul un alias apparatus compte.
                if lower.contains("binding")
                    || (lower.contains("alias") && lower.contains("apparatus"))
                {
                    hits.push(path.display().to_string());
                }
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn t6_post_get_components_contract_bytes_frozen() {
    // Contrat sérialisé gelé : GET/POST /components identiques au legacy.
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("projet");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant admin")
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant read");
    let jwt = create_test_jwt_token(owner_id);

    let post = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "taskboard"}))
        .send()
        .await
        .expect("POST component");
    assert_eq!(post.status(), 201);
    let created: serde_json::Value = post.json().await.expect("JSON POST");
    assert_eq!(
        sorted_keys(&created),
        vec![
            "activated_at",
            "added_at",
            "component_type",
            "configured_at",
            "disabled_at",
            "id",
            "status"
        ],
        "T6 : contrat POST gelé"
    );

    let get = client
        .get(format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            created["id"].as_str().expect("id")
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET component");
    assert_eq!(get.status(), 200);
    let fetched: serde_json::Value = get.json().await.expect("JSON GET");
    assert_eq!(
        sorted_keys(&fetched),
        sorted_keys(&created),
        "T6 : contrat GET identique au POST"
    );
    assert_eq!(fetched["id"], created["id"], "T6 : même component_id");

    let list = client
        .get(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET list");
    assert_eq!(list.status(), 200);
    let listed: serde_json::Value = list.json().await.expect("JSON list");
    assert_eq!(sorted_keys(&listed), vec!["data"], "T6 : liste = {{data}}");
    assert_eq!(
        listed["data"][0]["id"], created["id"],
        "T6 : liste cohérente"
    );
}

#[tokio::test]
#[serial]
async fn t6_alias_resolves_same_component_id_no_second_resource() {
    // GREEN : `?binding=<même id>` résout vers le même component_id, mêmes bytes.
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "wiki")
            .await
            .expect("composant");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant read");
    let jwt = create_test_jwt_token(owner_id);
    let resp = client
        .get(format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET component");
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.expect("JSON");
    assert_eq!(json["id"], component.id().to_string(), "id canonique");
    assert!(
        json.get("binding_id").is_none() && json.get("apparatus_id").is_none(),
        "T6 : pas de seconde ressource exposée"
    );

    // Alias `?binding=<même id>` : même id, mêmes clés (bytes gelés), sans seconde ressource.
    let aliased = client
        .get(format!(
            "{}/api/projects/{}/components/{}?binding={}",
            base_url,
            project.id(),
            component.id(),
            component.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET alias");
    assert_eq!(aliased.status(), 200);
    let alias_json: serde_json::Value = aliased.json().await.expect("JSON alias");
    assert_eq!(alias_json["id"], json["id"], "T6 : alias = même id");
    assert_eq!(
        sorted_keys(&alias_json),
        sorted_keys(&json),
        "T6 : alias mêmes bytes gelés"
    );
    assert!(
        alias_json.get("binding_id").is_none() && alias_json.get("apparatus_id").is_none(),
        "T6 : alias sans seconde ressource"
    );

    // Alias distinct : 404, jamais une autre ressource.
    let mismatch = client
        .get(format!(
            "{}/api/projects/{}/components/{}?binding={}",
            base_url,
            project.id(),
            component.id(),
            Uuid::new_v4()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET alias distinct");
    assert_eq!(mismatch.status(), 404, "T6 : alias distinct rejeté");
}

#[tokio::test]
#[serial]
async fn t6_catalog_still_served_over_external_http() {
    // Le catalogue vit toujours via HTTP externe (wiremock observé).
    let (fixture, base_url, client, openfga, components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("projet");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant admin");
    let jwt = create_test_jwt_token(owner_id);
    let before = components.catalog_call_count().await;
    let resp = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "wiki"}))
        .send()
        .await
        .expect("POST component");
    assert_eq!(resp.status(), 201);
    let after = components.catalog_call_count().await;
    assert!(
        after > before,
        "T6 : POST /components doit appeler le catalogue HTTP externe"
    );
}
