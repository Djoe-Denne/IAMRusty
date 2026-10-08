//! §14 real PostgreSQL admission/HTTP publisher regressions, parent-final IT only.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "../../../workers/ext-authz/tests/support/registry.rs"]
mod registry;
#[path = "support/signing_prefill.rs"]
mod signing_prefill;
mod utils;
#[path = "support/writer_barrier.rs"]
mod writer_barrier;

use iam_domain::{
    entity::signing_key::{
        SigningKey, SigningKeyAdmissionReason as Reason, SigningKeyLifecyclePolicy,
        SigningKeyStatus as Status,
    },
    error::DomainError,
    port::repository::SigningKeyRegistry,
};
use rsa::pkcs8::EncodePublicKey;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serial_test::serial;
use std::{collections::BTreeMap, sync::Arc};
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

async fn independent_registry(
    db: &DatabaseConnection,
    ttl: u64,
) -> iam_infra::repository::SeaOrmSigningKeyRegistry {
    iam_infra::repository::SeaOrmSigningKeyRegistry::new(
        Arc::new(writer_barrier::independent_writer(db).await),
        Arc::new(SigningKeyLifecyclePolicy::new().expect("same bounded slot/churn policy")),
        ttl,
    )
    .expect("independent primary registry; no shared app/provider authority")
}

fn public_map(set: &serde_json::Value) -> BTreeMap<String, serde_json::Value> {
    let entries = set["keys"].as_array().expect("JWKS keys array");
    let map = entries
        .iter()
        .map(|entry| {
            (
                entry["kid"].as_str().expect("public kid").to_owned(),
                entry.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        map.len(),
        entries.len(),
        "no duplicate or silently overwritten kids"
    );
    map
}

/// Expected entries come from explicit arranged/tested rows and a separate PEM
/// DTO codec, NEVER from the publication under test or a fixture-maintained count.
struct CompletePublicationAssert<'a, W: SigningKeyRegistry<Error = DomainError> + ?Sized> {
    writer: &'a W,
    db: &'a DatabaseConnection,
    client: &'a reqwest::Client,
    base: &'a str,
    expected: &'a [SigningKey],
    ttl: u64,
    org: i64,
    platform: i64,
}

async fn assert_complete_publication<W>(args: CompletePublicationAssert<'_, W>)
where
    W: SigningKeyRegistry<Error = DomainError> + ?Sized,
{
    let CompletePublicationAssert {
        writer,
        db,
        client,
        base,
        expected,
        ttl,
        org,
        platform,
    } = args;
    signing_prefill::assert_sql_slots(db, ttl, org, platform)
        .await
        .expect("independent SQL counts");
    assert_eq!(
        i64::try_from(expected.len()).expect("key count fits i64"),
        org + platform
    );
    let expected = iam_domain::entity::token::JwkSet::from_registry_keys_checked(expected)
        .expect("independent full public DTO oracle");
    let expected = public_map(&serde_json::to_value(expected).unwrap());
    let snapshot = writer
        .jwks_publication_snapshot()
        .await
        .expect("real primary materialization");
    assert_eq!(
        i64::try_from(snapshot.publication.counts().global).expect("global count fits i64"),
        org + platform
    );
    assert_eq!(
        i64::try_from(snapshot.publication.counts().global_organization)
            .expect("org count fits i64"),
        org
    );
    assert_eq!(
        i64::try_from(snapshot.publication.counts().platform).expect("platform count fits i64"),
        platform
    );
    assert_eq!(snapshot.access_token_expiration_seconds, ttl);
    assert!(snapshot.revision > 0);
    assert_eq!(public_map(&serde_json::to_value(snapshot.publication.jwks()).unwrap()),expected,"all canonical components, issuer/status/scope/org and exact kids, not just a self-confirming count");
    let response = client
        .get(format!("{base}/.well-known/jwks.json"))
        .send()
        .await
        .expect("live publisher HTTP");
    assert_eq!(response.status(), 200);
    let bytes = response.bytes().await.expect("complete publisher bytes");
    let usage = snapshot.publication.usage();
    assert!(bytes.len() <= usage.reserved_bytes && usage.reserved_bytes <= 786_432);
    assert!(
        bytes.len() <= 782_538 && bytes.len() < 1_048_576,
        "independent writer/SDK frame limits"
    );
    assert_eq!(
        public_map(&serde_json::from_slice(&bytes).expect("publisher JSON")),
        expected,
        "HTTP must neither truncate nor silently omit or invent a key"
    );
    let metadata=db.query_one(Statement::from_string(DatabaseBackend::Postgres,"SELECT revision,dirty,access_token_ttl,retire_skew,next_expiration,payload FROM signing_jwks_publication WHERE singleton=1")).await.unwrap().unwrap();
    assert!(!metadata.try_get::<bool>("", "dirty").unwrap());
    assert_eq!(
        u64::try_from(metadata.try_get::<i64>("", "revision").unwrap())
            .expect("revision is non-negative"),
        snapshot.revision
    );
    assert_eq!(
        u64::try_from(metadata.try_get::<i64>("", "access_token_ttl").unwrap())
            .expect("ttl is non-negative"),
        ttl
    );
    assert_eq!(metadata.try_get::<i64>("", "retire_skew").unwrap(), 60);
    assert!(
        metadata
            .try_get::<Option<chrono::DateTime<chrono::FixedOffset>>>("", "next_expiration")
            .unwrap()
            .is_none(),
        "these fixtures are only Active/Pending, no artificial retirement deadline"
    );
    assert_eq!(
        public_map(
            &serde_json::from_str(&metadata.try_get::<String>("", "payload").unwrap()).unwrap()
        ),
        expected
    );
}

async fn assert_platform_receiver(base: &str, issuer: &str, access: &str) {
    use rustycog::{
        config::{AuthConfig, JwtAuthConfig},
        http::UserIdExtractor,
    };
    // A fresh, unseeded SDK receiver MUST fetch this live URL. Reconstruction is
    // not a claim that a warm snapshot's60s lease has elapsed/been extended.
    let receiver = UserIdExtractor::new(AuthConfig {
        jwt: JwtAuthConfig {
            allowed_algorithms: vec!["RS256".into()],
            issuer: Some(issuer.into()),
            audience: Some(rustycog::testing::http::jwt::TEST_JWT_AUDIENCE.into()),
            jwks_url: Some(format!("{base}/.well-known/jwks.json")),
            ..JwtAuthConfig::default()
        },
        ..AuthConfig::default()
    })
    .expect("actual URL-bound SDK receiver");
    let principal = receiver
        .extract_principal(access)
        .await
        .expect("platform signature/trust accepted after live JWKS fetch");
    assert_eq!(principal.iss, issuer);
}

#[tokio::test]
#[serial]
async fn configure_full_binding_noop_preserves_epoch_but_credential_and_material_changes_admit() {
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("real primary harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, _ttl) =
            common::fixture_signing_registry(&fixture).expect("root writer and publisher TTL");
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
        assert_eq!(
            before.publication.counts().global,
            after.publication.counts().global
        );
        assert_eq!(
            before.publication.usage().reserved_bytes,
            after.publication.usage().reserved_bytes
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
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("real primary harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, ttl) = common::fixture_signing_registry(&fixture)
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
        assert_eq!(
            before.publication.usage().reserved_bytes,
            after.publication.usage().reserved_bytes,
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
        let after_revoke=signing_prefill::database_state(fixture.db().as_ref()).await.unwrap();
        denied(
            &writer
                .replace_active_organization_key(&rejected, None)
                .await,
            Reason::ChurnRate,
        );
        let restarted=independent_registry(fixture.db().as_ref(),ttl).await;
        denied(&restarted.replace_active_organization_key(&rejected,None).await,Reason::ChurnRate);
        assert_eq!(signing_prefill::database_state(fixture.db().as_ref()).await.unwrap(),after_revoke,"revoke/reconstructed registry/repeated denial retain history, epochs and complete snapshot");
        let history=fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,"SELECT count(*)::bigint AS n,count(*) FILTER(WHERE lifecycle_admitted_at>statement_timestamp()-INTERVAL '3600 seconds')::bigint AS recent FROM signing_keys WHERE organization_id=$1",[org.into()])).await.unwrap().unwrap();
        assert_eq!(history.try_get::<i64>("","n").unwrap(),4);assert_eq!(history.try_get::<i64>("","recent").unwrap(),4);
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
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("real primary harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, ttl) = common::fixture_signing_registry(&fixture).expect("root writer");
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
        let other = independent_registry(fixture.db().as_ref(), ttl).await;
        let (a, b) = tokio::join!(writer.insert(&left), other.insert(&right));
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
                .publication
                .jwks()
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
    let (fixture, base, client) = Box::pin(common::setup_test_server())
        .await
        .expect("real primary HTTP harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, ttl) = common::fixture_signing_registry(&fixture)
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
        let root=writer.find_active_platform_key().await.expect("primary platform").expect("one root Active");
        signing_prefill::assert_sql_slots(fixture.db().as_ref(),ttl,0,1).await.unwrap();
        assert_platform_receiver(&base,&root.issuer,&access).await;
        let large = |org| {
            let mut key = candidate(Some(org), Status::Pending);
            key.issuer = format!("https://issuer.example/{}/{}", org, "x".repeat(900));
            key
        };
        let left = large(Uuid::new_v4());
        let right = large(Uuid::new_v4());
        let cost = iam_domain::entity::signing_publication::SLOT_BYTES_MAX + 1;
        assert_eq!(cost, 4097);
        // Only neutral preparation changes:174 canonical Active bindings, all
        // fresh distinct owners, admitted/history/epochs/attestations in one TX.
        // The helper cannot perform an admission under study or forge receipts.
        let mut fillers=(0..174).map(|_|{let mut key=large(Uuid::new_v4());key.status=Status::Active;key}).collect::<Vec<_>>();
        signing_prefill::prefill_organization_frontier(fixture.db().as_ref(),&root,&mut fillers,ttl).await.expect("bounded canonical validated preparation");
        let mut expected=vec![root.clone()];expected.extend(fillers);
        assert_complete_publication(CompletePublicationAssert { writer: writer.as_ref(), db: fixture.db().as_ref(), client: &client, base: &base, expected: &expected, ttl, org: 174, platform: 1 }).await;
        let head=writer.jwks_publication_snapshot().await.unwrap();
        assert_eq!(175-head.publication.counts().global_organization,1,"exactly one disjoint org slot remains");
        for key in [&left,&right] {
            let row=fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,"SELECT count(*)::bigint AS n FROM signing_keys WHERE organization_id=$1",[key.organization_id.into()])).await.unwrap().unwrap();
            assert_eq!(row.try_get::<i64>("","n").unwrap(),0,"race candidates have quota<8 and history<4; global capacity is the ONLY frontier");
        }
        let other=independent_registry(fixture.db().as_ref(),ttl).await;
        // Separate registry objects/pools on the SAME primary, two independent
        // production transactions. No fixture counter decides the winner.
        let (a, b) = tokio::join!(writer.insert(&left), other.insert(&right));
        assert_eq!(
            usize::from(a.is_ok()) + usize::from(b.is_ok()),
            1,
            "cross-org writers share a global primary capacity lock"
        );
        let (winner,loser,error)=if a.is_ok(){(&left,&right,b)}else{(&right,&left,a)};
        assert!(matches!(
            error,
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: Reason::Capacity,
                ..
            })
        ));
        expected.push(winner.clone());
        assert!(writer.find_by_kid(&loser.kid).await.unwrap().is_none(),"losing key must not persist");
        assert!(writer.find_by_kid(&winner.kid).await.unwrap().is_some());
        assert_complete_publication(CompletePublicationAssert { writer: writer.as_ref(), db: fixture.db().as_ref(), client: &client, base: &base, expected: &expected, ttl, org: 175, platform: 1 }).await;
        assert!(writer.jwks_publication_snapshot().await.unwrap().revision>head.revision);
        // Tenant exhaustion must not steal the ratified platform reserve.
        let mut platform = root.clone();platform.id=Uuid::new_v4();platform.kid=Uuid::new_v4().simple().to_string();platform.status=Status::Pending;
        writer
            .insert(&platform)
            .await
            .expect("platform reserve admission despite exhausted org budget");
        expected.push(platform);
        assert_complete_publication(CompletePublicationAssert { writer: writer.as_ref(), db: fixture.db().as_ref(), client: &client, base: &base, expected: &expected, ttl, org: 175, platform: 2 }).await;
        let last_good=signing_prefill::database_state(fixture.db().as_ref()).await.unwrap();
        for _ in 0..3 {
            let result = writer.insert(&large(Uuid::new_v4())).await;
            assert!(matches!(
                result,
                Err(DomainError::SigningKeyAdmissionDenied {
                    reason: Reason::Capacity,
                    ..
                })
            ));
            assert_eq!(signing_prefill::database_state(fixture.db().as_ref()).await.unwrap(),last_good,"repeated capacity refusal has zero partial key/history/epoch/prepared/receipt/snapshot effects");
        }
        // Every platform admission is still a real production writer operation.
        // Root Active plus15 Pending epochs occupy all16 reserved slots, while
        // none of the175 org slots can borrow them, even with shared neutral n/e.
        for _ in 2..16 {
            let mut key=root.clone();key.id=Uuid::new_v4();key.kid=Uuid::new_v4().simple().to_string();key.status=Status::Pending;
            writer.insert(&key).await.expect("reserved platform slots through16");expected.push(key);
        }
        assert_complete_publication(CompletePublicationAssert { writer: writer.as_ref(), db: fixture.db().as_ref(), client: &client, base: &base, expected: &expected, ttl, org: 175, platform: 16 }).await;
        assert_eq!(expected.len(),191,"full disjoint191=175+16 frontier");
        assert_eq!(writer.find_active_platform_key().await.unwrap().unwrap(),root,"Pending reserve admissions and refusals must not retire the root Active");
        let mut seventeenth=root.clone();seventeenth.id=Uuid::new_v4();seventeenth.kid=Uuid::new_v4().simple().to_string();seventeenth.status=Status::Pending;
        let last_good=signing_prefill::database_state(fixture.db().as_ref()).await.unwrap();
        for _ in 0..3 {
            assert!(matches!(writer.insert(&seventeenth).await,Err(DomainError::SigningKeyAdmissionDenied {reason:Reason::Capacity,..})),"17th platform epoch refused through the real primary writer");
            assert_eq!(signing_prefill::database_state(fixture.db().as_ref()).await.unwrap(),last_good,"platform refusal has zero partial effects");
        }
        assert_complete_publication(CompletePublicationAssert { writer: writer.as_ref(), db: fixture.db().as_ref(), client: &client, base: &base, expected: &expected, ttl, org: 175, platform: 16 }).await;
        assert_platform_receiver(&base,&root.issuer,&access).await;
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

#[tokio::test]
#[serial]
async fn platform_seventeenth_slot_is_refused_even_with175_unused_org_slots() {
    let (fixture, base, client) = Box::pin(common::setup_test_server())
        .await
        .expect("real primary HTTP harness");
    fixture_cleanup::run(&fixture, async {
        let (writer, ttl) = common::fixture_signing_registry(&fixture).unwrap();
        let root = writer.find_active_platform_key().await.unwrap().unwrap();
        let access = common::fixture_platform_access_token(&fixture)
            .await
            .unwrap();
        let mut expected = vec![root.clone()];
        assert_complete_publication(CompletePublicationAssert {
            writer: writer.as_ref(),
            db: fixture.db().as_ref(),
            client: &client,
            base: &base,
            expected: &expected,
            ttl,
            org: 0,
            platform: 1,
        })
        .await;
        assert_eq!(
            client
                .get(format!("{base}/api/me"))
                .bearer_auth(&access)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        for _ in 1..16 {
            let mut pending = root.clone();
            pending.id = Uuid::new_v4();
            pending.kid = Uuid::new_v4().simple().to_string();
            pending.status = Status::Pending;
            writer
                .insert(&pending)
                .await
                .expect("real platform admission through slot16");
            expected.push(pending);
        }
        assert_complete_publication(CompletePublicationAssert {
            writer: writer.as_ref(),
            db: fixture.db().as_ref(),
            client: &client,
            base: &base,
            expected: &expected,
            ttl,
            org: 0,
            platform: 16,
        })
        .await;
        assert_eq!(
            expected.len(),
            16,
            "global cap191 is NOT the platform refusal reason"
        );
        let state = signing_prefill::database_state(fixture.db().as_ref())
            .await
            .unwrap();
        let mut seventeenth = root.clone();
        seventeenth.id = Uuid::new_v4();
        seventeenth.kid = Uuid::new_v4().simple().to_string();
        seventeenth.status = Status::Pending;
        let separate = independent_registry(fixture.db().as_ref(), ttl).await;
        for _ in 0..3 {
            assert!(
                matches!(
                    separate.insert(&seventeenth).await,
                    Err(DomainError::SigningKeyAdmissionDenied {
                        reason: Reason::Capacity,
                        ..
                    })
                ),
                "platform cannot borrow unoccupied org slots"
            );
            assert_eq!(
                signing_prefill::database_state(fixture.db().as_ref())
                    .await
                    .unwrap(),
                state
            );
        }
        assert!(writer
            .find_by_kid(&seventeenth.kid)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            writer.find_active_platform_key().await.unwrap().unwrap(),
            root
        );
        assert_complete_publication(CompletePublicationAssert {
            writer: writer.as_ref(),
            db: fixture.db().as_ref(),
            client: &client,
            base: &base,
            expected: &expected,
            ttl,
            org: 0,
            platform: 16,
        })
        .await;
        assert_platform_receiver(&base, &root.issuer, &access).await;
        assert_eq!(
            client
                .get(format!("{base}/api/me"))
                .bearer_auth(&access)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
    })
    .await;
}
