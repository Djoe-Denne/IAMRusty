//! Apparatus P1 — T3 collisions et concurrence (GREEN, TDD Phase C).
//!
//! Harness unique `common::setup_test_server` (5-tuple). `#[serial]` sur tout
//! test touchant `TestOpenFga`/wiremock. Arrange : `project` + Admin/Read uniquement.
//! GREEN : le vrai mapping injectif vit dans `manifesto_infra::apparatus_mapping`
//! (table canonique + rejet `MappingCollision`, code stable versionné) ; la
//! contrainte DB `project_components_unique` tranche sous course réelle (409).

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use manifesto_infra::apparatus_mapping::{
    check_pairs_injective, legacy_to_apparatus, APPARATUS_MAPPING_COLLISION_CODE,
    APPARATUS_MAPPING_CONTRACT_VERSION,
};
use rustycog::permission::{Permission, ResourceRef, Subject};
use serial_test::serial;
use std::collections::HashSet;
use uuid::Uuid;

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

// ---------------------------------------------------------------------------
// Unitaires (sans Docker)
// ---------------------------------------------------------------------------

#[test]
fn t3_mapping_is_injective_two_legacy_types_rejected() {
    // GREEN T3 : table canonique injective — taskboard et wiki pointent vers
    // des cibles distinctes (pas de fusion silencieuse).
    let mut seen = HashSet::new();
    for legacy in ["taskboard", "wiki"] {
        let mapped = legacy_to_apparatus(legacy).expect("type legacy connu");
        assert!(
            seen.insert(mapped),
            "T3 : collision legacy→Apparatus — deux types vers {mapped}, rejet attendu (pas de fusion silencieuse)"
        );
    }
    // Et deux paires explicites vers la même cible sont rejetées.
    let err = check_pairs_injective(&[
        ("taskboard", "io.aiforall.generic"),
        ("wiki", "io.aiforall.generic"),
    ])
    .expect_err("T3 : 2 legacy vers 1 cible = rejet, pas de fusion silencieuse");
    assert_eq!(
        err.code(),
        APPARATUS_MAPPING_COLLISION_CODE,
        "T3 : rejet avec code versionné stable"
    );
}

#[test]
fn t3_collision_error_is_stable_versioned_code() {
    // GREEN T3 (test-only) : l'erreur versionnée vit dans le module de mapping
    // Manifesto (contrats P0 intacts) — code stable + version de contrat.
    let mapping_rs =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("infra/src/apparatus_mapping.rs");
    let content = std::fs::read_to_string(&mapping_rs).expect("apparatus_mapping.rs lisible");
    assert!(
        content.to_lowercase().contains("collision"),
        "T3 : apparatus_mapping doit exposer une erreur versionnée de collision de mapping"
    );
    assert_eq!(
        APPARATUS_MAPPING_COLLISION_CODE, "APPARATUS_MAPPING_COLLISION",
        "T3 : code stable versionné"
    );
    assert_eq!(
        APPARATUS_MAPPING_CONTRACT_VERSION, 1,
        "T3 : version de contrat documentée"
    );
}

// ---------------------------------------------------------------------------
// Intégration DB concurrence (Postgres réel)
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn t3_concurrent_double_add_rejected_exactly_once() {
    // Contrainte DB (project_components_unique), pas `if` applicatif :
    // deux POST concurrents du même type ⇒ exactement un 201 et un 409.
    // GREEN = contrainte prouvée (le `if` est contourné par la course, la DB tranche).
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
        .expect("grant admin");
    let jwt = create_test_jwt_token(owner_id);
    let url = format!("{}/api/projects/{}/components", base_url, project.id());
    let body = serde_json::json!({"component_type": "taskboard"});
    let (r1, r2) = tokio::join!(
        client
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send(),
        client
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
    );
    let mut statuses = vec![
        r1.expect("requête 1").status().as_u16(),
        r2.expect("requête 2").status().as_u16(),
    ];
    statuses.sort_unstable();
    assert_eq!(
        statuses,
        vec![201, 409],
        "T3 : double ajout concurrent refusé exactement une fois par la DB"
    );
}

#[tokio::test]
#[serial]
async fn t3_no_second_public_uuid_on_component_resource() {
    // Garde verte : la ressource exposée porte un seul `id` (pas de second UUID public).
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
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
    assert_eq!(json["id"], component.id().to_string(), "id unique");
    for forbidden in ["apparatus_id", "binding_id", "uuid", "component_uuid"] {
        assert!(
            json.get(forbidden).is_none(),
            "T3 : pas de second UUID public ({forbidden} absent)"
        );
    }
}
