//! §14 real PostgreSQL admission/HTTP publisher regressions, parent-final IT only.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "../../../workers/ext-authz/tests/support/registry.rs"]
mod registry;
mod utils;

use iam_domain::{
    entity::signing_key::{
        SigningKey, SigningKeyAdmissionReason as Reason, SigningKeyLifecyclePolicy,
        SigningKeyStatus as Status,
    },
    error::DomainError,
    port::repository::SigningKeyRegistry,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use uuid::Uuid;

fn candidate(org: Option<Uuid>, status: Status) -> SigningKey {
    let mut key = registry::registry_key(status, org);
    key.kid = Uuid::new_v4().simple().to_string();
    key
}

fn denied(result: &Result<SigningKey, DomainError>, expected: Reason) {
    assert!(
        matches!(result, Err(DomainError::SigningKeyAdmissionDenied { reason, .. }) if *reason == expected),
        "must fail at the specific admission frontier, not parsing/SQL/issuer setup"
    );
}

#[tokio::test]
#[serial]
async fn configure_full_binding_noop_preserves_epoch_but_credential_and_material_changes_admit() {
    let (fixture, _, _) = common::setup_test_server()
        .await
        .expect("real primary harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, ttl) = common::fixture_signing_registry(&fixture)
            .await
            .expect("root writer and publisher TTL");
        let org = Uuid::new_v4();
        let initial = writer
            .replace_active_organization_key(&candidate(Some(org), Status::Active), None)
            .await
            .expect("first admission");
        let before = writer
            .jwks_publication_snapshot()
            .await
            .expect("primary snapshot");
        let mut repeated = initial.clone();
        repeated.id = Uuid::new_v4();
        repeated.kid = Uuid::new_v4().simple().to_string();
        let kept = writer
            .replace_active_organization_key(&repeated, Some(&initial.kid))
            .await
            .expect("full binding noop");
        assert_eq!(
            (kept.id, &kept.kid, kept.created_at, kept.updated_at),
            (
                initial.id,
                &initial.kid,
                initial.created_at,
                initial.updated_at
            )
        );
        let after = writer
            .jwks_publication_snapshot()
            .await
            .expect("snapshot after noop");
        assert_eq!(before.keys.len(), after.keys.len());
        let policy = SigningKeyLifecyclePolicy::new().expect("ratified bounded policy");
        assert_eq!(
            policy
                .publication_usage(&before.keys, ttl, before.as_of)
                .expect("before reservation")
                .reserved_bytes,
            policy
                .publication_usage(&after.keys, ttl, after.as_of)
                .expect("after reservation")
                .reserved_bytes
        );
        denied(
            &writer
                .replace_active_organization_key(&repeated, Some("stale-epoch"))
                .await,
            Reason::EpochConflict,
        );
        let mut changed = repeated.clone();
        changed.credential_ref = Some("isolated-changed-credential-reference".into());
        let second = writer
            .replace_active_organization_key(&changed, Some(&initial.kid))
            .await
            .expect("same n/e but changed credential must admit");
        assert_ne!(second.kid, initial.kid);
        // A genuine second RSA public key, not malformed fake n/e.
        use rsa::pkcs8::EncodePublicKey;
        let private = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
            .expect("isolated alternate RSA material");
        let mut material = second.clone();
        material.id = Uuid::new_v4();
        material.kid = Uuid::new_v4().simple().to_string();
        material.public_key = private
            .to_public_key()
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .expect("valid SPKI");
        let third = writer
            .replace_active_organization_key(&material, Some(&second.kid))
            .await
            .expect("new material admission");
        assert_ne!(third.kid, second.kid);
        let count = fixture
            .db()
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM signing_keys WHERE organization_id=$1",
                [org.into()],
            ))
            .await
            .expect("SQL history")
            .expect("count row");
        assert_eq!(
            count.try_get::<i64>("", "n").expect("count"),
            3,
            "noop and stale expected epoch must not insert a row"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn pending_promotion_is_not_double_charged_and_rejected_churn_keeps_previous_active() {
    let (fixture, _, _) = common::setup_test_server()
        .await
        .expect("real primary harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, ttl) = common::fixture_signing_registry(&fixture)
            .await
            .expect("root writer");
        let org = Uuid::new_v4();
        let pending = candidate(Some(org), Status::Pending);
        writer
            .insert(&pending)
            .await
            .expect("first Pending admission");
        let before = writer
            .jwks_publication_snapshot()
            .await
            .expect("Pending reservation");
        let mut promoted = pending.clone();
        promoted.status = Status::Active;
        let mut active = writer
            .replace_active_organization_key(&promoted, None)
            .await
            .expect("same-kid promotion");
        assert_eq!(active.kid, pending.kid);
        let after = writer
            .jwks_publication_snapshot()
            .await
            .expect("promoted reservation");
        let policy = SigningKeyLifecyclePolicy::new().expect("bounded policy");
        assert_eq!(
            policy
                .publication_usage(&before.keys, ttl, before.as_of)
                .expect("Pending bytes")
                .reserved_bytes,
            policy
                .publication_usage(&after.keys, ttl, after.as_of)
                .expect("Active bytes")
                .reserved_bytes,
            "promotion cannot charge a second reservation"
        );
        for number in 2..=4 {
            let mut next = candidate(Some(org), Status::Active);
            next.credential_ref = Some(format!("credential-{number}"));
            active = writer
                .replace_active_organization_key(&next, Some(&active.kid))
                .await
                .expect("four admissions including Pending");
        }
        let mut rejected = candidate(Some(org), Status::Active);
        rejected.credential_ref = Some("fifth-credential".into());
        denied(
            &writer
                .replace_active_organization_key(&rejected, Some(&active.kid))
                .await,
            Reason::ChurnRate,
        );
        let still = writer
            .find_by_kid(&active.kid)
            .await
            .expect("primary read")
            .expect("old Active intact");
        assert_eq!(still.status, Status::Active);
        assert_eq!(
            still.updated_at, active.updated_at,
            "rollback must not retire or refresh the old row"
        );
        writer
            .revoke_organization_keys(org)
            .await
            .expect("revoke does not erase admission history");
        denied(
            &writer
                .replace_active_organization_key(&rejected, None)
                .await,
            Reason::ChurnRate,
        );
        let other = candidate(Some(Uuid::new_v4()), Status::Active);
        writer
            .replace_active_organization_key(&other, None)
            .await
            .expect("tenant rate is scoped, not global denial");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn simultaneous_last_rate_slot_has_one_winner_and_four_persisted_epochs() {
    let (fixture, _, _) = common::setup_test_server()
        .await
        .expect("real primary harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, _) = common::fixture_signing_registry(&fixture)
            .await
            .expect("root writer");
        let org = Uuid::new_v4();
        // Pending inserts test independent admission writers without expected-kid
        // conflicts masking the rate frontier. Three of four slots consumed.
        for _ in 0..3 {
            writer
                .insert(&candidate(Some(org), Status::Pending))
                .await
                .expect("initial rate slots");
        }
        let left = candidate(Some(org), Status::Pending);
        let right = candidate(Some(org), Status::Pending);
        let (a, b) = tokio::join!(writer.insert(&left), writer.insert(&right));
        assert_eq!(
            usize::from(a.is_ok()) + usize::from(b.is_ok()),
            1,
            "atomic shared primary rate check"
        );
        let error = if a.is_err() { a } else { b };
        assert!(matches!(
            error,
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: Reason::ChurnRate,
                ..
            })
        ));
        let snapshot = writer
            .jwks_publication_snapshot()
            .await
            .expect("committed snapshot");
        assert_eq!(
            snapshot
                .keys
                .iter()
                .filter(|k| k.organization_id == Some(org))
                .count(),
            4
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cross_org_last_budget_slot_is_atomic_platform_reserve_and_publisher_survive_rejection() {
    let (fixture, base, client) = common::setup_test_server()
        .await
        .expect("real primary HTTP harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, ttl) = common::fixture_signing_registry(&fixture)
            .await
            .expect("root writer and actual TTL");
        let access = common::fixture_platform_access_token(&fixture)
            .await
            .expect("persisted account, actual shared platform mint");
        assert_eq!(
            client
                .get(format!("{base}/api/me"))
                .bearer_auth(&access)
                .send()
                .await
                .expect("positive account HTTP")
                .status(),
            200
        );
        let policy = SigningKeyLifecyclePolicy::new().expect("ratified policy");
        let large = |org| {
            let mut key = candidate(Some(org), Status::Pending);
            key.issuer = format!("https://issuer.example/{}/{}", org, "x".repeat(900));
            key
        };
        let left = large(Uuid::new_v4());
        let right = large(Uuid::new_v4());
        let cost = policy
            .reserved_entry_bytes(&left)
            .expect("exact escaped DTO max")
            + 1;
        assert_eq!(
            cost,
            policy.reserved_entry_bytes(&right).expect("same size") + 1
        );
        let mut reached = false;
        for _ in 0..720 {
            let snapshot = writer
                .jwks_publication_snapshot()
                .await
                .expect("primary budget snapshot");
            let usage = policy
                .publication_usage(&snapshot.keys, ttl, snapshot.as_of)
                .expect("actual compact publication budget");
            let remaining = 720_896_usize
                .checked_sub(usage.organization_reserved_bytes)
                .expect("ratified org ceiling");
            if remaining < 2 * cost {
                assert!(remaining >= cost, "one slot remains");
                reached = true;
                break;
            }
            writer
                .insert(&large(Uuid::new_v4()))
                .await
                .expect("bounded tenant fill with unique organizations");
        }
        assert!(
            reached,
            "finite setup must reach exact one-slot frontier, no skips"
        );
        let (a, b) = tokio::join!(writer.insert(&left), writer.insert(&right));
        assert_eq!(
            usize::from(a.is_ok()) + usize::from(b.is_ok()),
            1,
            "cross-org writers share a global primary capacity lock"
        );
        let error = if a.is_err() { a } else { b };
        assert!(matches!(
            error,
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: Reason::Capacity,
                ..
            })
        ));
        // Tenant exhaustion must not steal the ratified platform reserve.
        let platform = candidate(None, Status::Pending);
        writer
            .insert(&platform)
            .await
            .expect("platform reserve admission despite exhausted org budget");
        for _ in 0..3 {
            let result = writer.insert(&large(Uuid::new_v4())).await;
            assert!(matches!(
                result,
                Err(DomainError::SigningKeyAdmissionDenied {
                    reason: Reason::Capacity,
                    ..
                })
            ));
        }
        let snapshot = writer
            .jwks_publication_snapshot()
            .await
            .expect("publisher primary snapshot");
        let usage = policy
            .publication_usage(&snapshot.keys, ttl, snapshot.as_of)
            .expect("publication usage");
        let response = client
            .get(format!("{base}/.well-known/jwks.json"))
            .send()
            .await
            .expect("actual publisher HTTP");
        assert_eq!(response.status(), 200);
        let bytes = response
            .bytes()
            .await
            .expect("actual compact publisher bytes");
        assert!(bytes.len() <= usage.reserved_bytes && usage.reserved_bytes <= 786_432);
        let published: serde_json::Value = serde_json::from_slice(&bytes).expect("publisher JSON");
        let expected = iam_domain::entity::signing_key::filter_jwks_publication_keys_at(
            snapshot.keys,
            ttl,
            snapshot.as_of,
        );
        assert_eq!(
            published["keys"].as_array().expect("keys").len(),
            expected.len(),
            "no silent key omission to fit budget"
        );
        for key in expected {
            assert!(published["keys"]
                .as_array()
                .expect("keys")
                .iter()
                .any(|entry| entry["kid"] == key.kid));
        }
        assert_eq!(
            client
                .get(format!("{base}/api/me"))
                .bearer_auth(&access)
                .send()
                .await
                .expect("unrelated platform remains usable after rejected churn")
                .status(),
            200
        );
    })
    .await;
}
