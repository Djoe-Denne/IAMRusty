//! Apparatus P1 — T4 transaction et outbox (RED, TDD Phase C).
//!
//! Postgres réel + faux broker in-memory par défaut (aucun broker réel exigé).
//! Harness unique `common::setup_test_server`. `#[serial]` sur le live.
//! Le helper `FakeBroker` est le minimal test-only pour rendre le RED observable.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use manifesto_infra::apparatus_outbox::persist_binding_atomically;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::Mutex;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Faux broker in-memory (test-only, défaut T4)
// ---------------------------------------------------------------------------

/// Enveloppe publiée vers le faux broker.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FakeEnvelope {
    topic: String,
    payload: String,
}

/// Broker in-memory : `publish` append-only, `published` lecture seule.
/// `fail_next` simule un échec d'écriture pour prouver le rollback atomique.
#[derive(Debug, Default)]
struct FakeBroker {
    published: Mutex<Vec<FakeEnvelope>>,
    fail_next: Mutex<bool>,
}

impl FakeBroker {
    fn new() -> Self {
        Self::default()
    }

    fn publish(&self, topic: &str, payload: &str) -> Result<(), &'static str> {
        if *self.fail_next.lock().expect("verrou") {
            *self.fail_next.lock().expect("verrou") = false;
            return Err("fake broker: injected failure");
        }
        self.published.lock().expect("verrou").push(FakeEnvelope {
            topic: topic.to_owned(),
            payload: payload.to_owned(),
        });
        Ok(())
    }

    fn published(&self) -> Vec<FakeEnvelope> {
        self.published.lock().expect("verrou").clone()
    }

    fn fail_next(&self) {
        *self.fail_next.lock().expect("verrou") = true;
    }
}

// ---------------------------------------------------------------------------
// Unitaires / gardes pures (sans Docker)
// ---------------------------------------------------------------------------

#[test]
fn t4_fake_broker_is_in_memory_and_injectable() {
    let broker = FakeBroker::new();
    broker.publish("t", "p1").expect("publish");
    broker.publish("t", "p2").expect("publish");
    assert_eq!(broker.published().len(), 2, "append-only in-memory");
    broker.fail_next();
    assert!(broker.publish("t", "p3").is_err(), "échec injecté");
    assert_eq!(broker.published().len(), 2, "échec sans effet");
}

#[test]
fn t4_no_outbox_polling_or_worker_in_manifesto_prod() {
    // Garde : T4 = atomicité locale, pas de worker/polling.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hits = Vec::new();
    for sub in [
        "domain/src",
        "application/src",
        "infra/src",
        "http/src",
        "setup/src",
        "migration/src",
    ] {
        scan(&root.join(sub), &mut hits);
    }
    assert!(
        hits.is_empty(),
        "T4 : polling/worker outbox interdit :\n{}",
        hits.join("\n")
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
                for (idx, line) in content.lines().enumerate() {
                    let lower = line.to_lowercase();
                    if lower.contains("outbox_worker")
                        || lower.contains("poll_outbox")
                        || lower.contains("start_polling")
                    {
                        hits.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Intégration DB (Postgres réel) — preuve atomique via le code prod T4
// ---------------------------------------------------------------------------
// NOTE : le consentement/génération sont hors scope (renvoyés à une ADR dédiée
// avant P2, décision orchestrateur). L'atomicité prouvée ici = insertion du
// binding `apparatus_bindings` + étape externe dans la même transaction prod
// (`persist_binding_atomically`) : échec externe ⇒ rollback total, rien persisté.

#[tokio::test]
#[serial]
async fn t4_binding_consent_write_failure_rolls_back_atomically() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    // RED si la table d'extension est absente : rien à rendre atomique (T1/T2 requis).
    let tables = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE '%apparatus%'".to_owned(),
        ))
        .await
        .expect("information_schema lisible");
    assert!(
        !tables.is_empty(),
        "RED T4 : aucune table d'extension '%apparatus%' — persistance binding+consentement non implémentée"
    );
    let table: String = tables[0].try_get("", "table_name").expect("table_name");
    let fk_col: String = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT kcu.column_name FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
                 JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name \
                 WHERE tc.table_name = '{table}' AND tc.constraint_type = 'FOREIGN KEY' \
                 AND ccu.table_name = 'project_components' AND ccu.column_name = 'id'"
            ),
        ))
        .await
        .expect("FK lisible")
        .expect("RED T4 : extension sans FK vers project_components(id)")
        .try_get("", "column_name")
        .expect("colonne FK");

    let owner_id = Uuid::new_v4();
    let (_project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    // La fonction prod écrit dans `apparatus_bindings` (table T1 canonique).
    assert_eq!(
        table, "apparatus_bindings",
        "T4 : la preuve prod vise la table T1 canonique"
    );
    let broker = FakeBroker::new();

    // Cas échec : la fermeture externe échoue ⇒ la fonction prod annule tout.
    broker.fail_next();
    let outcome = persist_binding_atomically(db.as_ref(), component.id(), || {
        broker.publish("apparatus", "binding")
    })
    .await;
    assert!(
        outcome.is_err(),
        "T4 : échec externe ⇒ erreur prod, insertion annulée"
    );

    let count = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) AS n FROM {table} WHERE {fk_col} = '{}'",
                component.id()
            ),
        ))
        .await
        .expect("comptage")
        .expect("comptage présent");
    let n: i64 = count.try_get("", "n").expect("n");
    assert_eq!(n, 0, "T4 : échec outbox ⇒ rollback atomique, rien persisté");
    assert!(
        broker.published().is_empty(),
        "T4 : échec externe ⇒ rien publié"
    );

    // Cas succès : insertion + publication commitées ensemble (preuve commit).
    persist_binding_atomically(db.as_ref(), component.id(), || {
        broker.publish("apparatus", "binding")
    })
    .await
    .expect("commit succès");
    let committed = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT source, digest FROM {table} WHERE {fk_col} = '{}'",
                component.id()
            ),
        ))
        .await
        .expect("lecture binding")
        .expect("T4 : ligne binding commise après succès");
    let source: String = committed.try_get("", "source").expect("source");
    let digest: Option<String> = committed.try_get("", "digest").expect("digest");
    assert_eq!(source, "managed", "binding succès source=managed");
    assert!(digest.is_none(), "digest non résolu (NULL)");
    assert_eq!(
        broker.published().len(),
        1,
        "T4 : succès ⇒ exactement une publication"
    );
}
