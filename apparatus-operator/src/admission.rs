//! Store d'admission (register ≠ admit).
//!
//! Un [`AdmissionRecord`] `Valid` n'existe que s'il a été écrit par
//! [`AdmissionStore::admit`] : politique T5 ([`POLICY_ID`]), rapport de
//! conformance, et signature vérifiée. Le register Manifesto, le harness, le
//! publisher et le plugin n'écrivent pas cette ligne. Une revendication
//! publisher n'est pas une admission. Pas d'HTTP. Produit M2 = fichier JSON
//! ([`PersistentAdmissionStore`]) ; CR Kind = apply optionnel T10.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::conformance::POLICY_ID;
use crate::digest::ReleaseDigest;

/// Variable d'environnement du chemin JSON produit ([`PersistentAdmissionStore`]).
///
/// Le chemin est TCB : volume exclusif `apparatus-admit`. Absente ou vide →
/// [`admission_store_path_from_env`] échoue (pas de défaut `{temp_dir}`).
/// M3 lira ce fichier.
pub const ADMISSION_STORE_PATH_ENV: &str = "APPARATUS_ADMISSION_STORE_PATH";

const DISK_VALID_PHASE: &str = "VALID";
const MAX_STORE_BYTES: u64 = 1024 * 1024;

/// Chemin du store JSON produit (TCB), lu depuis [`ADMISSION_STORE_PATH_ENV`].
///
/// Pas de défaut partagé : le volume doit être exclusif au binaire
/// `apparatus-admit`. M3 lira ce fichier.
///
/// # Errors
///
/// Variable absente ou vide.
pub fn admission_store_path_from_env() -> io::Result<PathBuf> {
    admission_store_path_from_var(std::env::var(ADMISSION_STORE_PATH_ENV))
}

fn admission_store_path_from_var(value: Result<String, std::env::VarError>) -> io::Result<PathBuf> {
    match value {
        Ok(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "APPARATUS_ADMISSION_STORE_PATH empty; exclusive apparatus-admit volume required",
        )),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "APPARATUS_ADMISSION_STORE_PATH unset; exclusive apparatus-admit volume required",
        )),
    }
}

/// Statut d'un enregistrement d'admission.
///
/// Seul [`AdmissionStore::admit`] écrit [`AdmissionStatus::Valid`]. Ce n'est
/// pas un statut publisher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionStatus {
    /// Artifact admis : politique + rapport + signature.
    Valid,
}

/// Enregistrement d'admission (mémoire T7, JSON M2 ; apply cluster = T10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRecord {
    /// Identité catalogue ([`ReleaseDigest`] descripteur ADR-0002).
    pub descriptor_digest: ReleaseDigest,
    /// Version de politique runtime (T5 [`POLICY_ID`]).
    pub policy_version: String,
    /// Empreinte du rapport de conformance T5.
    pub report_digest: ReleaseDigest,
    /// Toujours [`AdmissionStatus::Valid`] pour une ligne écrite par admit.
    pub status: AdmissionStatus,
    /// Pin CRI persisté par [`PersistentAdmissionStore::bind_cri`] (absent après admit).
    pub cri_image: Option<String>,
}

/// Entrée du chemin d'admission (booléens T5/T6, pas une enveloppe signée T6).
#[derive(Debug, Clone)]
pub struct AdmitInput {
    /// Identité catalogue à admettre.
    pub descriptor_digest: ReleaseDigest,
    /// Empreinte observée (bytes / enveloppe). Mismatch → [`AdmitRefuse::TamperedDigest`].
    pub observed_descriptor: ReleaseDigest,
    /// Identifiant de politique ; doit égaler [`POLICY_ID`].
    pub policy_id: String,
    /// Digest du rapport de conformance associé.
    pub report_digest: ReleaseDigest,
    /// Résultat T5 : `false` → [`AdmitRefuse::ManifestNonconformant`].
    pub conformance_passed: bool,
    /// Résultat T6 (booléen, pas d'enveloppe) : `false` → [`AdmitRefuse::UnexpectedSignature`].
    pub signature_verified: bool,
    /// Revendication publisher ignorée : elle n'accorde pas l'admission.
    pub claimed_verified: bool,
}

/// Refus d'admission (jeu T8).
///
/// Ordre de garde dans [`evaluate_admit`] / [`AdmissionStore::admit`] : digest
/// mismatch, politique manquante, manifeste non conforme, signature inattendue.
/// T7 4→2→3 est conservé après le garde digest. `claimed_verified` ne saute
/// aucun garde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmitRefuse {
    /// Empreinte observée ≠ identité catalogue.
    TamperedDigest,
    /// Suite de conformance T5 en échec.
    ManifestNonconformant,
    /// Signature absente ou invalide.
    UnexpectedSignature,
    /// `policy_id` vide ou distinct de [`POLICY_ID`].
    MissingRuntimePolicy,
}

impl AdmitRefuse {
    /// Retourne [`Self::TamperedDigest`] si les deux empreintes diffèrent.
    #[must_use]
    pub fn if_digest_mismatch(expected: &ReleaseDigest, observed: &ReleaseDigest) -> Option<Self> {
        (expected != observed).then_some(Self::TamperedDigest)
    }
}

impl fmt::Display for AdmitRefuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TamperedDigest => f.write_str("tampered digest"),
            Self::ManifestNonconformant => f.write_str("manifest nonconformant"),
            Self::UnexpectedSignature => f.write_str("unexpected signature"),
            Self::MissingRuntimePolicy => f.write_str("missing runtime policy"),
        }
    }
}

impl std::error::Error for AdmitRefuse {}

/// Évalue les gardes T7 sans persister. `claimed_verified` est ignoré.
///
/// # Errors
///
/// - [`AdmitRefuse::TamperedDigest`] si `observed_descriptor` ≠ `descriptor_digest`
/// - [`AdmitRefuse::MissingRuntimePolicy`] si `policy_id` est vide ou ≠ [`POLICY_ID`]
/// - [`AdmitRefuse::ManifestNonconformant`] si `conformance_passed` est `false`
/// - [`AdmitRefuse::UnexpectedSignature`] si `signature_verified` est `false`
pub fn evaluate_admit(input: AdmitInput) -> Result<AdmissionRecord, AdmitRefuse> {
    let AdmitInput {
        descriptor_digest,
        observed_descriptor,
        policy_id,
        report_digest,
        conformance_passed,
        signature_verified,
        claimed_verified: _,
    } = input;
    // Guard order: digest mismatch, missing policy, nonconformant, unexpected
    // signature (T7 4→2→3 after digest). claimed_verified never skips.
    if let Some(refuse) = AdmitRefuse::if_digest_mismatch(&descriptor_digest, &observed_descriptor)
    {
        return Err(refuse);
    }
    if policy_id.is_empty() || policy_id != POLICY_ID {
        return Err(AdmitRefuse::MissingRuntimePolicy);
    }
    if !conformance_passed {
        return Err(AdmitRefuse::ManifestNonconformant);
    }
    if !signature_verified {
        return Err(AdmitRefuse::UnexpectedSignature);
    }
    Ok(AdmissionRecord {
        descriptor_digest,
        policy_version: POLICY_ID.to_owned(),
        report_digest,
        status: AdmissionStatus::Valid,
        cri_image: None,
    })
}

/// Persistance d'admission (mémoire en T7, JSON M2, CRD en T10).
pub trait AdmissionStore {
    /// Admet un artifact si digest, politique, conformance et signature sont valides.
    ///
    /// # Errors
    ///
    /// - [`AdmitRefuse::TamperedDigest`] si `observed_descriptor` ≠ `descriptor_digest`
    /// - [`AdmitRefuse::MissingRuntimePolicy`] si `policy_id` est vide ou ≠ [`POLICY_ID`]
    /// - [`AdmitRefuse::ManifestNonconformant`] si `conformance_passed` est `false`
    /// - [`AdmitRefuse::UnexpectedSignature`] si `signature_verified` est `false`
    ///
    /// Aucune ligne n'est écrite en cas de refus. `claimed_verified` est ignoré.
    fn admit(&mut self, input: AdmitInput) -> Result<AdmissionRecord, AdmitRefuse>;

    /// Lecture par identité catalogue (digest descripteur, pas le rapport).
    fn get(&self, descriptor_digest: &ReleaseDigest) -> Option<AdmissionRecord>;
}

/// Store [`HashMap`] : seule [`AdmissionStore::admit`] insère une ligne `Valid`.
#[derive(Debug, Default)]
pub struct InMemoryAdmissionStore {
    records: HashMap<ReleaseDigest, AdmissionRecord>,
}

impl InMemoryAdmissionStore {
    /// Store vide, sans ligne `Valid`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AdmissionStore for InMemoryAdmissionStore {
    fn admit(&mut self, input: AdmitInput) -> Result<AdmissionRecord, AdmitRefuse> {
        let record = evaluate_admit(input)?;
        self.records
            .insert(record.descriptor_digest.clone(), record.clone());
        Ok(record)
    }

    fn get(&self, descriptor_digest: &ReleaseDigest) -> Option<AdmissionRecord> {
        self.records.get(descriptor_digest).cloned()
    }
}

/// Fichier JSON local : store produit M2 (lisible hors Kind).
///
/// Champs alignés sur la CR `AdmissionRecord` : `descriptorDigest`,
/// `policyVersion`, `reportDigest`, `status.phase` = `VALID`.
/// Seul [`AdmissionStore::admit`] écrit une ligne `VALID`. Un refus laisse le
/// fichier inchangé (ou sans ce digest).
///
/// Chemin binaire : [`ADMISSION_STORE_PATH_ENV`] (TCB, volume exclusif
/// `apparatus-admit`). M3 lira ce fichier.
///
/// # Panics
///
/// [`AdmissionStore::admit`] panique si l'écriture JSON échoue après une
/// évaluation réussie (rollback mémoire ; le digest n'est pas `VALID` sur disque).
#[derive(Debug)]
pub struct PersistentAdmissionStore {
    path: PathBuf,
    records: HashMap<ReleaseDigest, AdmissionRecord>,
}

/// Pin CRI `name@sha256:<64 hex>` sans tag `latest`.
///
/// # Errors
///
/// Image sans digest, digest non 64 hex minuscules, ou tag `latest`.
pub fn validate_cri_pin(cri_image: &str) -> Result<(), &'static str> {
    let Some((_, digest)) = cri_image.split_once("@sha256:") else {
        return Err("cri image must be pinned as name@sha256:<digest>");
    };
    let valid_digest = digest.len() == 64
        && digest
            .as_bytes()
            .iter()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if !valid_digest {
        return Err("cri image must be pinned as name@sha256:<digest>");
    }
    if cri_image.ends_with(":latest") || cri_image.contains(":latest@") {
        return Err("cri image must not use tag latest");
    }
    Ok(())
}

/// Échec de [`PersistentAdmissionStore::bind_cri`].
#[derive(Debug)]
pub enum BindCriError {
    /// Pas de ligne JSON VALID pour ce digest.
    NotValid,
    /// Pin CRI rejeté (`@sha256` 64 hex, pas `latest`).
    InvalidPin,
    /// Échec d'écriture JSON (mémoire restaurée).
    Persist(io::Error),
}

impl fmt::Display for BindCriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotValid => f.write_str("bind_cri: digest is not VALID"),
            Self::InvalidPin => f.write_str("bind_cri: cri image is not a @sha256 pin"),
            Self::Persist(err) => write!(f, "bind_cri persist: {err}"),
        }
    }
}

impl std::error::Error for BindCriError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Persist(err) => Some(err),
            _ => None,
        }
    }
}

impl PersistentAdmissionStore {
    /// Ouvre le JSON existant, ou un store vide si le fichier est absent.
    ///
    /// # Errors
    ///
    /// Lecture ou parse JSON impossible.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let records = if path.exists() {
            load_records(&path)?
        } else {
            HashMap::new()
        };
        Ok(Self { path, records })
    }

    fn persist(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let file = DiskStore {
            records: self.records.values().map(DiskRecord::from_record).collect(),
        };
        let json = serde_json::to_string_pretty(&file)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let tmp_path = store_tmp_path(&self.path);
        fs::write(&tmp_path, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
        }
        replace_file(&tmp_path, &self.path)
    }

    /// Écrit le pin CRI sur une ligne déjà VALID, puis persiste le JSON.
    ///
    /// N'admet pas : [`AdmissionStore::admit`] laisse `cri_image` à `None`.
    ///
    /// # Errors
    ///
    /// Digest non VALID, pin `@sha256` invalide, ou écriture JSON.
    pub fn bind_cri(
        &mut self,
        digest: &ReleaseDigest,
        cri_image: &str,
    ) -> Result<(), BindCriError> {
        if !self.records.contains_key(digest) {
            return Err(BindCriError::NotValid);
        }
        validate_cri_pin(cri_image).map_err(|_| BindCriError::InvalidPin)?;
        let previous = {
            let record = self.records.get_mut(digest).ok_or(BindCriError::NotValid)?;
            let previous = record.cri_image.clone();
            record.cri_image = Some(cri_image.to_owned());
            previous
        };
        if let Err(err) = self.persist() {
            if let Some(record) = self.records.get_mut(digest) {
                record.cri_image = previous;
            }
            return Err(BindCriError::Persist(err));
        }
        Ok(())
    }
}

impl AdmissionStore for PersistentAdmissionStore {
    fn admit(&mut self, input: AdmitInput) -> Result<AdmissionRecord, AdmitRefuse> {
        let record = evaluate_admit(input)?;
        let digest = record.descriptor_digest.clone();
        let previous = self.records.insert(digest.clone(), record.clone());
        if let Err(err) = self.persist() {
            match previous {
                Some(old) => {
                    self.records.insert(digest, old);
                }
                None => {
                    self.records.remove(&digest);
                }
            }
            panic!(
                "PersistentAdmissionStore: écriture JSON {} échouée: {err}",
                self.path.display()
            );
        }
        Ok(record)
    }

    fn get(&self, descriptor_digest: &ReleaseDigest) -> Option<AdmissionRecord> {
        self.records.get(descriptor_digest).cloned()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DiskStore {
    #[serde(default)]
    records: Vec<DiskRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiskRecord {
    #[serde(rename = "descriptorDigest")]
    descriptor_digest: String,
    #[serde(rename = "policyVersion")]
    policy_version: String,
    #[serde(rename = "reportDigest")]
    report_digest: String,
    status: DiskStatus,
    #[serde(rename = "criImage", default, skip_serializing_if = "Option::is_none")]
    cri_image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiskStatus {
    phase: String,
}

impl DiskRecord {
    fn from_record(record: &AdmissionRecord) -> Self {
        Self {
            descriptor_digest: record.descriptor_digest.as_str().to_owned(),
            policy_version: record.policy_version.clone(),
            report_digest: record.report_digest.as_str().to_owned(),
            status: DiskStatus {
                phase: DISK_VALID_PHASE.to_owned(),
            },
            cri_image: record.cri_image.clone(),
        }
    }

    fn into_record(self) -> Option<AdmissionRecord> {
        if self.status.phase != DISK_VALID_PHASE {
            return None;
        }
        if self.policy_version != POLICY_ID {
            return None;
        }
        let descriptor_digest = ReleaseDigest::new(&self.descriptor_digest).ok()?;
        let report_digest = ReleaseDigest::new(&self.report_digest).ok()?;
        Some(AdmissionRecord {
            descriptor_digest,
            policy_version: self.policy_version,
            report_digest,
            status: AdmissionStatus::Valid,
            cri_image: self.cri_image,
        })
    }
}

fn store_tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

fn replace_file(tmp: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        if dest.exists() {
            fs::remove_file(dest)?;
        }
        fs::rename(tmp, dest)
    }
    #[cfg(not(windows))]
    {
        fs::rename(tmp, dest)
    }
}

fn load_records(path: &Path) -> io::Result<HashMap<ReleaseDigest, AdmissionRecord>> {
    let file = fs::File::open(path)?;
    let mut raw = Vec::new();
    let n = file
        .take(MAX_STORE_BYTES.saturating_add(1))
        .read_to_end(&mut raw)?;
    if n as u64 > MAX_STORE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "admission store exceeds 1 MiB",
        ));
    }
    let parsed: DiskStore = serde_json::from_slice(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let mut records = HashMap::new();
    for disk in parsed.records {
        if let Some(record) = disk.into_record() {
            records.insert(record.descriptor_digest.clone(), record);
        }
    }
    Ok(records)
}

/// Indique si le contrôleur programmerait le digest : une ligne `Valid` existe.
///
/// T8 / M3 : faux après chaque refus et sans ligne JSON. Pas d'appel cluster.
#[must_use]
pub fn would_schedule<S: AdmissionStore + ?Sized>(store: &S, digest: &ReleaseDigest) -> bool {
    store.get(digest).is_some()
}

/// Enregistre une release comme le ferait Manifesto : n'appelle pas [`AdmissionStore::admit`],
/// n'insère pas de ligne [`AdmissionStatus::Valid`].
pub const fn register_manifesto_release(_store: &InMemoryAdmissionStore, _digest: &ReleaseDigest) {}

/// Variante nommée pour le test T7 : le register Manifesto ne produit pas d'admission.
pub const fn manifesto_register_does_not_admit(
    store: &InMemoryAdmissionStore,
    digest: &ReleaseDigest,
) {
    register_manifesto_release(store, digest);
}

#[cfg(test)]
mod store_path_tests {
    use super::*;

    #[test]
    fn unset_or_empty_env_is_fail_closed() {
        assert!(admission_store_path_from_var(Err(std::env::VarError::NotPresent)).is_err());
        assert!(admission_store_path_from_var(Ok(String::new())).is_err());
        let path = admission_store_path_from_var(Ok("/var/lib/apparatus-admit/store.json".into()))
            .expect("path non vide");
        assert_eq!(path, PathBuf::from("/var/lib/apparatus-admit/store.json"));
    }
}
