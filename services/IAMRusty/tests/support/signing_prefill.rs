//! Nonproduction arrangement ONLY for the175-slot admission regression.
//! Bounded canonical Active rows, one fresh owner/binding/admission per org.
//! No tested admission, promotion or race goes through this helper. All database
//! triggers stay enabled; the real primary GET rematerializes the dirty snapshot.

use anyhow::{ensure, Context};
use chrono::{DateTime, Utc};
use iam_domain::entity::{
    signing_key::{
        SigningKey, SigningKeyLifecyclePolicy, SigningKeyStatus, SigningProviderType, TrustScope,
    },
    signing_publication::{PreparedSigningPublicKey, ValidatedJwksPublication},
};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait, Value,
};
use std::collections::BTreeSet;

/// Fail closed unless this is the exact pristine fixture (one platform Active).
/// Binding/material, fixed-slot capacity and actual complete DTO are validated
/// before INSERT. Bulk INSERT avoids174 repeated full snapshot writer builds;
/// history and epochs are actual SQL evidence, never invented helper counters.
pub async fn prefill_organization_frontier(
    db: &DatabaseConnection,
    root: &SigningKey,
    keys: &mut [SigningKey],
    ttl: u64,
) -> anyhow::Result<()> {
    ensure!(
        keys.len() == 174,
        "fixture prefill is bounded to the exact174-row arrangement"
    );
    let tx = db.begin().await?;
    let result=async {
        tx.execute(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtextextended('iam-signing-jwks-admission-v1',0))")).await?;
        let baseline=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS total,count(*) FILTER(WHERE id=$1 AND kid=$2 AND trust_scope='platform' AND organization_id IS NULL AND status='active')::bigint AS root FROM signing_keys",
            [root.id.into(),root.kid.clone().into()])).await?.context("missing baseline")?;
        ensure!(baseline.try_get::<i64>("","total")?==1 && baseline.try_get::<i64>("","root")?==1,"prefill requires a pristine one-platform-key fixture, no preserved state");
        let singleton=tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT revision,dirty FROM signing_jwks_publication WHERE singleton=1 FOR UPDATE")).await?.context("missing publication")?;
        ensure!(!singleton.try_get::<bool>("","dirty")?,"baseline must already be materialized");
        let revision=singleton.try_get::<i64>("","revision")?;
        let clock=tx.query_one(Statement::from_string(DatabaseBackend::Postgres,"SELECT statement_timestamp() AS as_of")).await?.context("missing DB clock")?;
        let now:DateTime<chrono::FixedOffset>=clock.try_get("","as_of")?;let now=now.with_timezone(&Utc);
        let mut ids=BTreeSet::from([root.id]);let mut kids=BTreeSet::from([root.kid.clone()]);
        let mut owners=BTreeSet::new();let mut issuers=BTreeSet::from([root.issuer.clone()]);
        let mut publics=Vec::with_capacity(keys.len());
        for key in keys.iter_mut() {
            ensure!(key.trust_scope==TrustScope::Organization && key.status==SigningKeyStatus::Active && key.organization_id.is_some(),"only distinct-owner Active filler bindings are arranged; no Pending receipt fabricated");
            ensure!(key.provider_type==SigningProviderType::PemFile && key.provider_key_version.is_none() && key.credential_ref.is_none() && key.public_key==rustycog::testing::http::jwt::TEST_RS256_PUBLIC_PEM,"only the explicit nonproduction neutral public material may be prefilled");
            ensure!(ids.insert(key.id) && kids.insert(key.kid.clone()) && owners.insert(key.organization_id.unwrap()) && issuers.insert(key.issuer.clone()),"prefill IDs/kids/owners/issuers must be unique, including platform");
            key.created_at=now;key.updated_at=now;
            let public=PreparedSigningPublicKey::prepare(key)?;
            let restored=PreparedSigningPublicKey::from_persisted(key,public.n().into(),public.e().into(),public.binding_fingerprint(),public.longest_entry_bytes())?;
            ensure!(restored.n()==public.n() && restored.e()==public.e() && restored.binding_fingerprint()==public.binding_fingerprint() && restored.longest_entry_bytes()==public.longest_entry_bytes(),"persisted attestation must restore to the identical validated binding");
            publics.push(public);
        }
        let mut complete=vec![root.clone()];complete.extend_from_slice(keys);
        // Uses exactly the new-binding validator/capacity/serializer, not a
        // hand-maintained size/slot counter or bypass flag in production.
        SigningKeyLifecyclePolicy::new()?.check_admission(&complete,&[],None,ttl,now)?;
        let mut entries=vec![PreparedSigningPublicKey::prepare(root)?.project(root)?];
        for (key,public) in keys.iter().zip(&publics) {entries.push(public.project(key)?);}
        let expected=ValidatedJwksPublication::from_entries(entries)?;
        ensure!(expected.counts().global==175 && expected.counts().global_organization==174 && expected.counts().platform==1,"literal prefill partition oracle");
        let mut values:Vec<Value>=Vec::new();let mut tuples=Vec::new();
        for key in keys.iter() {
            let start=values.len();
            values.extend([key.id.into(),key.kid.clone().into(),key.issuer.clone().into(),key.provider_key_ref.clone().into(),key.public_key.clone().into(),key.organization_id.into(),now.naive_utc().into(),now.into()]);
            tuples.push(format!("(${},${},'RS256','organization',${},'pem_file',${},NULL,NULL,${},'active',${},${},${},${})",start+1,start+2,start+3,start+4,start+5,start+6,start+7,start+7,start+8));
        }
        let inserted=tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            format!("INSERT INTO signing_keys(id,kid,algorithm,trust_scope,issuer,provider_type,provider_key_ref,provider_key_version,credential_ref,public_key,status,organization_id,created_at,updated_at,lifecycle_admitted_at) VALUES {}",tuples.join(",")),values)).await?;
        ensure!(inserted.rows_affected()==174,"all validated filler rows must be inserted atomically");
        let mut values:Vec<Value>=Vec::new();let mut tuples=Vec::new();
        for (key,public) in keys.iter().zip(&publics) {
            let start=values.len();values.extend([key.id.into(),public.n().to_string().into(),public.e().to_string().into(),public.binding_fingerprint().to_vec().into(),i32::try_from(public.longest_entry_bytes())?.into()]);
            tuples.push(format!("(${},${},${},${},${})",start+1,start+2,start+3,start+4,start+5));
        }
        let inserted=tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            format!("INSERT INTO signing_public_entries(signing_key_id,public_n,public_e,binding_fingerprint,longest_entry_bytes) VALUES {}",tuples.join(",")),values)).await?;
        ensure!(inserted.rows_affected()==174,"all prepared records must commit with their keys");
        // Read each stored attestation back (no PEM parse), not merely a count.
        let rows=tx.query_all(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT k.*,p.public_n,p.public_e,p.binding_fingerprint,p.longest_entry_bytes,e.revision FROM signing_keys k JOIN signing_public_entries p ON p.signing_key_id=k.id JOIN signing_scope_epochs e ON e.scope_id='organization:'||k.organization_id::text WHERE k.organization_id IS NOT NULL ORDER BY k.id")).await?;
        ensure!(rows.len()==174,"DB must contain exactly the arranged owners");
        for row in rows {
            let id=row.try_get::<uuid::Uuid>("","id")?;let key=keys.iter().find(|key|key.id==id).context("unexpected stored filler")?;
            ensure!(row.try_get::<String>("","kid")?==key.kid && row.try_get::<String>("","algorithm")?==key.algorithm
                && row.try_get::<String>("","trust_scope")?=="organization" && row.try_get::<String>("","status")?=="active"
                && row.try_get::<Option<uuid::Uuid>>("","organization_id")?==key.organization_id
                && row.try_get::<String>("","issuer")?==key.issuer && row.try_get::<String>("","provider_type")?=="pem_file"
                && row.try_get::<String>("","provider_key_ref")?==key.provider_key_ref && row.try_get::<Option<i64>>("","provider_key_version")?.is_none()
                && row.try_get::<Option<String>>("","credential_ref")?.is_none() && row.try_get::<String>("","public_key")?==key.public_key,
                "stored full binding must equal the validated input BEFORE attestation restoration");
            let restored=PreparedSigningPublicKey::from_persisted(key,row.try_get("","public_n")?,row.try_get("","public_e")?,&row.try_get::<Vec<u8>>("","binding_fingerprint")?,usize::try_from(row.try_get::<i32>("","longest_entry_bytes")?)?)?;
            ensure!(serde_json::to_value(restored.project(key)?)?==serde_json::to_value(expected.jwks().keys.iter().find(|entry|entry.kid==key.kid).context("expected public missing")?)?,"stored public entry mismatch");
            ensure!(row.try_get::<chrono::NaiveDateTime>("","created_at")?==now.naive_utc() && row.try_get::<chrono::NaiveDateTime>("","updated_at")?==now.naive_utc(),"canonical row timestamps must use the final DB clock");
            ensure!(row.try_get::<DateTime<chrono::FixedOffset>>("","lifecycle_admitted_at")?.with_timezone(&Utc)==now && row.try_get::<i64>("","revision")?==1,"one immutable admission and one real trigger-generated scope epoch per owner");
        }
        let facts=tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT revision,dirty,(SELECT count(*)::bigint FROM signing_key_prepublications) AS receipts FROM signing_jwks_publication WHERE singleton=1")).await?.context("missing final publication")?;
        ensure!(facts.try_get::<i64>("","revision")?==revision && facts.try_get::<bool>("","dirty")? && facts.try_get::<i64>("","receipts")?==0,"enabled triggers dirty the existing snapshot; no receipt invented for arranged Active bindings");
        Ok::<(),anyhow::Error>(())
    }.await;
    match result {
        Ok(()) => {
            tx.commit().await?;
            Ok(())
        }
        Err(error) => {
            tx.rollback().await?;
            Err(error)
        }
    }
}

/// Independent primary SQL cardinalities, deliberately not a registry snapshot
/// or imported production predicate. Caller asserts literal174/175/16/191.
pub async fn assert_sql_slots(
    db: &DatabaseConnection,
    ttl: u64,
    org: i64,
    platform: i64,
) -> anyhow::Result<()> {
    let row=db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*)::bigint AS global,count(*) FILTER(WHERE trust_scope='organization' AND organization_id IS NOT NULL)::bigint AS org,count(*) FILTER(WHERE trust_scope='platform' AND organization_id IS NULL)::bigint AS platform,count(DISTINCT kid)::bigint AS kids FROM signing_keys WHERE status IN ('pending','active') OR (status='retiring' AND updated_at<=statement_timestamp() AT TIME ZONE 'UTC' AND updated_at + make_interval(secs=>$1::double precision) > statement_timestamp() AT TIME ZONE 'UTC')",
        [i64::try_from(ttl)?.checked_add(60).context("TTL overflow")?.into()])).await?.context("missing slot counts")?;
    ensure!(
        row.try_get::<i64>("", "org")? == org
            && row.try_get::<i64>("", "platform")? == platform
            && row.try_get::<i64>("", "global")? == org + platform
            && row.try_get::<i64>("", "kids")? == org + platform,
        "independent SQL partition/kid cardinality mismatch"
    );
    Ok(())
}

/// Byte-stable complete state oracle for rejection: includes immutable history,
/// prepared records, scope epochs, receipts and snapshot metadata/revision/dirty.
pub async fn database_state(db: &DatabaseConnection) -> anyhow::Result<serde_json::Value> {
    let row=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('keys',(SELECT coalesce(jsonb_agg(to_jsonb(k) ORDER BY id),'[]'::jsonb) FROM signing_keys k),'publics',(SELECT coalesce(jsonb_agg(to_jsonb(p) ORDER BY signing_key_id),'[]'::jsonb) FROM signing_public_entries p),'epochs',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY scope_id),'[]'::jsonb) FROM signing_scope_epochs e),'receipts',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY key_id),'[]'::jsonb) FROM signing_key_prepublications r),'snapshot',(SELECT to_jsonb(s) FROM signing_jwks_publication s WHERE singleton=1)) AS state")).await?.context("missing DB state")?;
    Ok(row.try_get("", "state")?)
}
