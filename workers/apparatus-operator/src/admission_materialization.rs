//! Privileged Adm-A maintenance exporter (§13.5), never a controller signer.
//!
//! The qualified store is deliberately NOT the legacy JSON snapshot. It has an
//! exclusive OS file lock for its whole lifetime, append+fsync events, stable
//! record IDs and revisions including negative tombstones. Reopening suspends
//! emission until independent maintenance reconciles its exact history tip.

use crate::{
    admit::{verify_materialization_envelope, AdmitTarget},
    conformance::{evaluate_conformance, POLICY_ID},
    digest::ReleaseDigest,
};
use apparatus_contracts::runtime::*;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use rsa::{
    pkcs1v15::{Signature, VerifyingKey},
    signature::Verifier as _,
    traits::PublicKeyParts,
    RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const MAX_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;
type Failure = RuntimeErrorCodeV1;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    record_id: Uuid,
    revision: RuntimeCounterV1,
    release: RuntimeDigestV1,
    policy_id: String,
    report_sha256: RuntimeDigestV1,
    envelope_reference: String,
    cri_image: String,
    withdrawn: bool,
    emission_suspended: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    schema_version: u16,
    authority_id: Uuid,
    epoch: Uuid,
    sequence: u64,
    previous_sha256: String,
    decision: Option<Decision>,
}

/// Independently verified recovery fact supplied by the trusted maintenance
/// channel, never parsed as a self-authenticating JSON receipt.
pub struct AdmissionHistoryCheckpoint {
    pub authority_id: Uuid,
    pub epoch: Uuid,
    pub sequence: u64,
    pub tip_sha256: String,
}

/// Setup injects the actual recovery/provenance channel. An old local file or a
/// boolean cannot implement its obligation. No permissive default is supplied.
pub trait AdmissionHistoryAuthority {
    fn verified_checkpoint(&self) -> Result<AdmissionHistoryCheckpoint, Failure>;
}

/// Single process owning the authority file. Not Clone; OS locking also excludes
/// another instance/process, including while slow verification/signing occurs.
pub struct DurableAdmissionAuthority {
    file: File,
    authority_id: Uuid,
    epoch: Uuid,
    sequence: u64,
    tip: String,
    decisions: BTreeMap<String, Decision>,
    reconciled: bool,
    poisoned: bool,
    history_hash: Sha256,
}

impl DurableAdmissionAuthority {
    /// Explicit first provisioning only. It never imports legacy JSON as v1.
    /// Created authority stays suspended until a trusted recovery checkpoint.
    pub fn create(path: &Path, authority_id: Uuid, epoch: Uuid) -> Result<Self, Failure> {
        if authority_id.is_nil() || epoch.is_nil() {
            return Err(Failure::RuntimeInvalidRequest);
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        // Windows inherits the B-provisioned directory ACL. An OS lock is not
        // an ACL and does not qualify an unprotected/copied volume as Adm-A.
        let file = options
            .open(path)
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        file.try_lock()
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        let mut store = Self {
            file,
            authority_id,
            epoch,
            sequence: 0,
            tip: String::new(),
            decisions: BTreeMap::new(),
            reconciled: false,
            poisoned: false,
            history_hash: Sha256::new(),
        };
        store.append(None)?;
        Ok(store)
    }

    /// Missing, truncated, reset, duplicated or malformed history never becomes
    /// an empty usable authority. Holding this handle retains the OS lock.
    pub fn open(path: &Path, authority_id: Uuid, epoch: Uuid) -> Result<Self, Failure> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        file.try_lock()
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        let mut raw = Vec::new();
        (&mut file)
            .take(MAX_JOURNAL_BYTES + 1)
            .read_to_end(&mut raw)
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        if raw.is_empty() || raw.len() as u64 > MAX_JOURNAL_BYTES || raw.last() != Some(&b'\n') {
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        let mut store = Self {
            file,
            authority_id,
            epoch,
            sequence: 0,
            tip: String::new(),
            decisions: BTreeMap::new(),
            reconciled: false,
            poisoned: false,
            history_hash: Sha256::new(),
        };
        for bytes in raw
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let event: Event =
                parse_runtime_json(bytes).map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
            if event.schema_version != 1
                || event.authority_id != authority_id
                || event.epoch != epoch
                || event.sequence
                    != store
                        .sequence
                        .checked_add(1)
                        .ok_or(Failure::RuntimeAuthorityUnavailable)?
                || event.previous_sha256 != store.tip
                || (store.sequence == 0) != event.decision.is_none()
            {
                return Err(Failure::RuntimeAuthorityUnavailable);
            }
            if let Some(decision) = event.decision {
                let previous = store.decisions.get(decision.release.as_str());
                if decision.record_id.is_nil()
                    || decision.policy_id != POLICY_ID
                    || previous.is_some_and(|old| {
                        old.record_id != decision.record_id
                            || old.revision.get().checked_add(1) != Some(decision.revision.get())
                    })
                    || (previous.is_none() && decision.revision.get() != 1)
                {
                    return Err(Failure::RuntimeAuthorityUnavailable);
                }
                store
                    .decisions
                    .insert(decision.release.as_str().to_owned(), decision);
            }
            store.sequence = event.sequence;
            store.tip = hex::encode(Sha256::digest(bytes));
        }
        store.history_hash.update(&raw);
        Ok(store)
    }

    /// A crash/reopen/restore always requires explicit external reconciliation;
    /// equality with a checkpoint read from this same file is NOT provenance.
    pub fn reconcile(&mut self, authority: &dyn AdmissionHistoryAuthority) -> Result<(), Failure> {
        if self.poisoned {
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        self.recheck_disk_tip()?;
        let checkpoint = authority.verified_checkpoint()?;
        if checkpoint.authority_id != self.authority_id
            || checkpoint.epoch != self.epoch
            || checkpoint.sequence != self.sequence
            || checkpoint.tip_sha256 != self.tip
        {
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        self.reconciled = true;
        Ok(())
    }

    /// Exposes a non-secret checkpoint to the separate trusted recovery channel.
    /// This returned DATA cannot, on its own, re-enable this store after restore.
    pub fn checkpoint(&self) -> AdmissionHistoryCheckpoint {
        AdmissionHistoryCheckpoint {
            authority_id: self.authority_id,
            epoch: self.epoch,
            sequence: self.sequence,
            tip_sha256: self.tip.clone(),
        }
    }

    fn recheck_disk_tip(&mut self) -> Result<(), Failure> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        let mut raw = Vec::new();
        (&mut self.file)
            .take(MAX_JOURNAL_BYTES + 1)
            .read_to_end(&mut raw)
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        if raw.len() as u64 > MAX_JOURNAL_BYTES || raw.last() != Some(&b'\n') {
            self.reconciled = false;
            self.poisoned = true;
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        let last = raw
            .split(|b| *b == b'\n')
            .rev()
            .find(|line| !line.is_empty())
            .ok_or(Failure::RuntimeAuthorityUnavailable)?;
        if hex::encode(Sha256::digest(last)) != self.tip
            || Sha256::digest(&raw) != self.history_hash.clone().finalize()
        {
            self.reconciled = false;
            self.poisoned = true;
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        Ok(())
    }

    fn append(&mut self, decision: Option<Decision>) -> Result<(), Failure> {
        if self.poisoned {
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        // An uncertain write poisons this process until a full reopen/recovery.
        let was_reconciled = self.reconciled;
        self.reconciled = false;
        self.poisoned = true;
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or(Failure::RuntimeAuthorityUnavailable)?;
        let event = Event {
            schema_version: 1,
            authority_id: self.authority_id,
            epoch: self.epoch,
            sequence,
            previous_sha256: self.tip.clone(),
            decision: decision.clone(),
        };
        let bytes = serde_jcs::to_vec(&event).map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        let length = self
            .file
            .seek(SeekFrom::End(0))
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        if length + bytes.len() as u64 + 1 > MAX_JOURNAL_BYTES {
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        self.file
            .write_all(&bytes)
            .and_then(|()| self.file.write_all(b"\n"))
            .and_then(|()| self.file.sync_all())
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
        self.sequence = sequence;
        self.tip = hex::encode(Sha256::digest(&bytes));
        self.history_hash.update(&bytes);
        self.history_hash.update(b"\n");
        if let Some(decision) = decision {
            self.decisions
                .insert(decision.release.as_str().to_owned(), decision);
        }
        self.reconciled = was_reconciled;
        self.poisoned = false;
        Ok(())
    }

    /// Real Adm-A admission, never an import of an AdmissionRecord/VALID flag.
    /// Re-runs conformance and actual envelope signature/content binding before
    /// committing a new durable revision under this authority's process lock.
    pub async fn admit(
        &mut self,
        candidate_toml: String,
        envelope_reference: String,
        cri_image: String,
        target: AdmitTarget,
    ) -> Result<(), Failure> {
        if !self.reconciled {
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        self.recheck_disk_tip()?;
        validate_runtime_oci_pin(&envelope_reference)?;
        validate_runtime_oci_pin(&cri_image)?;
        let candidate = crate::build::parse_manifest(&candidate_toml)
            .map_err(|_| Failure::RuntimeReleaseNotAdmitted)?;
        let report = evaluate_conformance(&candidate_toml);
        if !report.passed || report.policy_id != POLICY_ID {
            return Err(Failure::RuntimeReleaseNotAdmitted);
        }
        let release = RuntimeDigestV1::new(candidate.digest.as_str().to_owned())?;
        let previous = self.decisions.get(release.as_str()).cloned();
        let checked_release =
            ReleaseDigest::new(release.as_str()).map_err(|_| Failure::RuntimeReleaseNotAdmitted)?;
        let checked_envelope = envelope_reference.clone();
        let checked_image = cri_image.clone();
        tokio::task::spawn_blocking(move || {
            verify_materialization_envelope(
                &target,
                &checked_envelope,
                &checked_release,
                &checked_image,
            )
        })
        .await
        .map_err(|_| Failure::RuntimeAuthorityUnavailable)?
        .map_err(|_| Failure::RuntimeReleaseNotAdmitted)?;
        self.recheck_disk_tip()?;
        let decision = Decision {
            record_id: previous
                .as_ref()
                .map_or_else(Uuid::new_v4, |old| old.record_id),
            revision: match previous {
                Some(old) => old.revision.checked_next()?,
                None => RuntimeCounterV1::new(1)?,
            },
            release,
            policy_id: POLICY_ID.to_owned(),
            report_sha256: RuntimeDigestV1::new(report.report_digest.as_str().to_owned())?,
            envelope_reference,
            cri_image,
            withdrawn: false,
            emission_suspended: false,
        };
        self.append(Some(decision))
    }

    /// First withdrawal step. Persist suspension BEFORE asking writer C to fence
    /// every consumer. It cannot be undone by replaying an old JSON record.
    pub fn suspend_emission(&mut self, release: &RuntimeDigestV1) -> Result<(), Failure> {
        self.recheck_disk_tip()?;
        let mut record = self
            .decisions
            .get(release.as_str())
            .cloned()
            .ok_or(Failure::RuntimeReleaseNotAdmitted)?;
        record.emission_suspended = true;
        record.revision = record.revision.checked_next()?;
        self.append(Some(record))
    }

    /// Final withdrawal requires the trusted channel to prove the durable writer
    /// barrier. No caller-supplied boolean or maintenance receipt is accepted.
    pub fn commit_withdrawal(
        &mut self,
        release: &RuntimeDigestV1,
        barrier: &dyn AdmissionWithdrawalBarrier,
    ) -> Result<(), Failure> {
        self.recheck_disk_tip()?;
        let mut record = self
            .decisions
            .get(release.as_str())
            .cloned()
            .ok_or(Failure::RuntimeReleaseNotAdmitted)?;
        if !record.emission_suspended {
            return Err(Failure::RuntimeInvalidRequest);
        }
        barrier.verify_all_consumers_closed(
            self.authority_id,
            release,
            record.record_id,
            record.revision,
        )?;
        self.recheck_disk_tip()?;
        record.withdrawn = true;
        record.revision = record.revision.checked_next()?;
        self.append(Some(record))
    }
}

pub trait AdmissionWithdrawalBarrier {
    fn verify_all_consumers_closed(
        &self,
        authority_id: Uuid,
        release: &RuntimeDigestV1,
        record_id: Uuid,
        revision: RuntimeCounterV1,
    ) -> Result<(), Failure>;
}

/// Dedicated configured Transit RSA key. It must not equal the envelope Cosign
/// key; no token/private key is passed to controller/startup/Manifesto.
pub struct AdmissionMaterializationExporter {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    token: String,
    public_key: RsaPublicKey,
    kid: String,
    installation_id: Uuid,
    target: AdmitTarget,
}

impl AdmissionMaterializationExporter {
    pub fn new(
        client: reqwest::Client,
        endpoint: reqwest::Url,
        token: String,
        key_name: &str,
        public_key: RsaPublicKey,
        installation_id: Uuid,
        target: AdmitTarget,
    ) -> Result<Self, Failure> {
        if endpoint.scheme() != "https"
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
            || token.is_empty()
            || installation_id.is_nil()
            || key_name.is_empty()
            || key_name == target.transit_key
            || !key_name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || public_key.n().bits() != 3072
            || public_key.e().to_bytes_be() != [1, 0, 1]
        {
            return Err(Failure::RuntimeKeyUnavailable);
        }
        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let jwk = format!("{{\"e\":\"AQAB\",\"kty\":\"RSA\",\"n\":\"{n}\"}}");
        let kid = format!(
            "ara-v1-{}",
            URL_SAFE_NO_PAD.encode(Sha256::digest(jwk.as_bytes()))
        );
        let endpoint = endpoint
            .join(&format!("v1/transit/sign/{key_name}/sha2-256"))
            .map_err(|_| Failure::RuntimeKeyUnavailable)?;
        Ok(Self {
            client,
            endpoint,
            token,
            public_key,
            kid,
            installation_id,
            target,
        })
    }

    /// Sign only a freshly rechecked authoritative decision for a writer intent.
    /// The caller supplies no payload/VALID flag. The OS lock remains held across
    /// all slow checks and the final <=2s signing operation.
    pub async fn export(
        &self,
        store: &mut DurableAdmissionAuthority,
        challenge: &RuntimeAdmissionChallengeV1,
    ) -> Result<String, Failure> {
        challenge
            .validate()
            .map_err(|_| Failure::RuntimeInvalidProof)?;
        if !store.reconciled
            || challenge.authority_id != store.authority_id
            || challenge.installation_id != self.installation_id
            || challenge.schema_version != 1
        {
            return Err(Failure::RuntimeAuthorityUnavailable);
        }
        let decision = store
            .decisions
            .get(challenge.release.as_str())
            .cloned()
            .ok_or(Failure::RuntimeReleaseNotAdmitted)?;
        if decision.withdrawn || decision.emission_suspended || decision.policy_id != POLICY_ID {
            return Err(Failure::RuntimeReleaseNotAdmitted);
        }
        let target = self.target.clone();
        let checked = decision.clone();
        tokio::task::spawn_blocking(move || {
            let release = ReleaseDigest::new(checked.release.as_str())
                .map_err(|_| Failure::RuntimeReleaseNotAdmitted)?;
            verify_materialization_envelope(
                &target,
                &checked.envelope_reference,
                &release,
                &checked.cri_image,
            )
            .map_err(|_| Failure::RuntimeReleaseNotAdmitted)
        })
        .await
        .map_err(|_| Failure::RuntimeAuthorityUnavailable)??;
        // Last authoritative check is after the costly verification, under the
        // SAME OS mutation lock. No process-local cache is accepted as authority.
        store.recheck_disk_tip()?;
        if store.decisions.get(challenge.release.as_str()) != Some(&decision) {
            return Err(Failure::RuntimeReleaseNotAdmitted);
        }
        let observed = clock_ms()?;
        let expiry = challenge.expires_at_unix_ms.get();
        if challenge
            .issued_at_unix_ms
            .get()
            .checked_add(ADMISSION_CHALLENGE_TTL_MS)
            != Some(expiry)
            || challenge.issued_at_unix_ms.get() > observed
            || observed >= expiry
        {
            return Err(Failure::RuntimeInvalidProof);
        }
        let until = observed
            .checked_add(ADMISSION_PROOF_TTL_MS)
            .ok_or(Failure::RuntimeInvalidProof)?
            .min(expiry);
        if until / 1000 <= observed / 1000 {
            return Err(Failure::RuntimeInvalidProof);
        }
        let claims = RuntimeAdmissionAttestationV1 {
            schema_version: 1,
            iss: ADMISSION_ISSUER.to_owned(),
            aud: ADMISSION_AUDIENCE.to_owned(),
            purpose: ADMISSION_PURPOSE.to_owned(),
            jti: challenge.challenge_id,
            iat: observed / 1000,
            exp: until / 1000,
            challenge: challenge.clone(),
            record_id: decision.record_id,
            admission_revision: decision.revision,
            policy_id: decision.policy_id,
            report_sha256: decision.report_sha256,
            envelope_reference: decision.envelope_reference,
            cri_image: decision.cri_image,
            observed_at_unix_ms: RuntimeUnixMsV1::new(observed)?,
            valid_until_unix_ms: RuntimeUnixMsV1::new(until)?,
        };
        let header = serde_jcs::to_vec(
            &serde_json::json!({"alg":"RS256","typ":ADMISSION_TYP,"kid":self.kid}),
        )
        .map_err(|_| Failure::RuntimeInvalidProof)?;
        let payload = serde_jcs::to_vec(&claims).map_err(|_| Failure::RuntimeInvalidProof)?;
        let signed = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header),
            URL_SAFE_NO_PAD.encode(payload)
        );
        let compact = tokio::time::timeout(Duration::from_secs(2), async {
            let mut response = self.client.post(self.endpoint.clone()).header("X-Vault-Token", &self.token)
                .json(&serde_json::json!({"input":STANDARD.encode(signed.as_bytes()),"prehashed":false,"signature_algorithm":"pkcs1v15"}))
                .send().await.map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
            if !response.status().is_success() || response.content_length().is_some_and(|n| n > 16_384) { return Err(Failure::RuntimeAuthorityUnavailable); }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| Failure::RuntimeAuthorityUnavailable)? {
                if bytes.len().saturating_add(chunk.len()) > 16_384 { return Err(Failure::RuntimeAuthorityUnavailable); }
                bytes.extend_from_slice(&chunk);
            }
            let response: serde_json::Value = parse_runtime_json(&bytes).map_err(|_| Failure::RuntimeAuthorityUnavailable)?;
            let signature = response["data"]["signature"].as_str().and_then(|value| value.strip_prefix("vault:v1:"))
                .ok_or(Failure::RuntimeAuthorityUnavailable)?;
            let bytes = STANDARD.decode(signature).map_err(|_| Failure::RuntimeInvalidProof)?;
            let signature = Signature::try_from(bytes.as_slice()).map_err(|_| Failure::RuntimeInvalidProof)?;
            VerifyingKey::<Sha256>::new(self.public_key.clone()).verify(signed.as_bytes(), &signature).map_err(|_| Failure::RuntimeInvalidProof)?;
            Ok(format!("{signed}.{}", URL_SAFE_NO_PAD.encode(bytes)))
        }).await.map_err(|_| Failure::RuntimeAuthorityUnavailable)??;
        let now = clock_ms()?;
        if now < observed
            || now >= until
            || now / 1000 >= claims.exp
            || compact.len() > MAX_ADMISSION_PROOF_BYTES
        {
            return Err(Failure::RuntimeInvalidProof);
        }
        Ok(compact)
    }
}

fn clock_ms() -> Result<i64, Failure> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Failure::RuntimeAuthorityUnavailable)?
            .as_millis(),
    )
    .map_err(|_| Failure::RuntimeAuthorityUnavailable)
}
