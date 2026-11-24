//! Cryptographic primitives for the interoperable SHA-1 TOTP profile.

use aws_lc_rs::{
    aead::{Aad, Nonce, RandomizedNonceKey, AES_256_GCM},
    constant_time, hmac, rand,
};
use data_encoding::BASE32_NOPAD;
use iam_domain::port::totp::TotpError;
use std::{fs::File, io::Read, path::Path};
use uuid::Uuid;
use zeroize::Zeroizing;

const SEED_LEN: usize = 20;
const KEY_LEN: usize = 32;
const SEALED_LEN: usize = 12 + SEED_LEN + 16;
pub const STEP_SECONDS: i64 = 30;

/// An instance-only dedicated secret key; never expose it via Debug or config.
pub struct TotpCipher(RandomizedNonceKey);

impl TotpCipher {
    pub(crate) fn from_mounted_file(path: &Path) -> Result<Self, TotpError> {
        let mut bytes = Zeroizing::new(Vec::with_capacity(KEY_LEN + 1));
        File::open(path)
            .and_then(|file| file.take((KEY_LEN + 1) as u64).read_to_end(&mut bytes))
            .map_err(|_| TotpError::Unavailable)?;
        if bytes.len() != KEY_LEN {
            return Err(TotpError::Unavailable);
        }
        RandomizedNonceKey::new(&AES_256_GCM, &bytes)
            .map(Self)
            .map_err(|_| TotpError::Unavailable)
    }

    pub(crate) fn provision(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<(String, Vec<u8>), TotpError> {
        let mut seed = Zeroizing::new([0; SEED_LEN]);
        rand::fill(seed.as_mut()).map_err(|_| TotpError::Unavailable)?;
        // Only this provisioning response may contain the seed.
        let encoded = Zeroizing::new(BASE32_NOPAD.encode(seed.as_ref()));
        let uri = format!(
            "otpauth://totp/AIForAll:{user_id}?secret={}&issuer=AIForAll&algorithm=SHA1&digits=6&period=30",
            encoded.as_str()
        );
        let mut ciphertext = Zeroizing::new(Vec::with_capacity(SEED_LEN + 16));
        ciphertext.extend_from_slice(seed.as_ref());
        let nonce = self
            .0
            .seal_in_place_append_tag(Aad::from(binding(user_id, id)), &mut *ciphertext)
            .map_err(|_| TotpError::Unavailable)?;
        let mut sealed = Vec::with_capacity(SEALED_LEN);
        sealed.extend_from_slice(nonce.as_ref());
        sealed.extend_from_slice(&ciphertext);
        Ok((uri, sealed))
    }

    pub(crate) fn verify(
        &self,
        user_id: Uuid,
        id: Uuid,
        sealed: &[u8],
        code: &str,
        step: i64,
        last_used: Option<i64>,
    ) -> Result<i64, TotpError> {
        if sealed.len() != SEALED_LEN {
            return Err(TotpError::Unavailable);
        }
        let (nonce, encrypted) = sealed.split_at(12);
        let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| TotpError::Unavailable)?;
        let mut buffer = Zeroizing::new(encrypted.to_vec());
        let seed = self
            .0
            .open_in_place(nonce, Aad::from(binding(user_id, id)), &mut buffer)
            .map_err(|_| TotpError::Unavailable)?;
        match_step(seed, code, step, last_used).ok_or(TotpError::InvalidProof)
    }
}

fn binding(user_id: Uuid, id: Uuid) -> [u8; 40] {
    let mut binding = [0; 40];
    binding[..8].copy_from_slice(b"IAM-TOTP");
    binding[8..24].copy_from_slice(user_id.as_bytes());
    binding[24..].copy_from_slice(id.as_bytes());
    binding
}

fn match_step(seed: &[u8], code: &str, current: i64, last_used: Option<i64>) -> Option<i64> {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) || current < 1 {
        return None;
    }
    // RFC 6238 interoperability only. No SHA-1 is used to protect stored seeds.
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, seed);
    let mut matched = None;
    let mut replayed = false;
    // Two bounded compares: current and one prior step for transmission delay.
    for step in [current - 1, current] {
        let mut output = Zeroizing::new([0_u8; 20]);
        let mac = hmac::sign_to_buffer(&key, &step.to_be_bytes(), output.as_mut()).ok()?;
        let start = usize::from(mac[19] & 0x0f);
        let mut number = u32::from_be_bytes(mac[start..start + 4].try_into().ok()?) & 0x7fff_ffff;
        number %= 1_000_000;
        let mut digits = Zeroizing::new([b'0'; 6]);
        for digit in digits.iter_mut().rev() {
            *digit += u8::try_from(number % 10).ok()?;
            number /= 10;
        }
        if constant_time::verify_slices_are_equal(digits.as_ref(), code.as_bytes()).is_ok() {
            if last_used.is_some_and(|used| used >= step) {
                replayed = true;
            } else {
                matched = Some(step);
            }
        }
    }
    if replayed {
        None
    } else {
        matched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_rfc_step_and_rejects_replay_or_outside_the_window() {
        let seed = b"12345678901234567890";
        assert_eq!(match_step(seed, "287082", 1, None), Some(1));
        assert_eq!(match_step(seed, "287082", 2, None), Some(1));
        assert_eq!(match_step(seed, "287082", 2, Some(1)), None);
        assert_eq!(match_step(seed, "287082", 3, None), None);
    }

    #[test]
    fn seed_cannot_be_opened_for_another_account_or_enrollment() {
        let cipher = TotpCipher(RandomizedNonceKey::new(&AES_256_GCM, &[0x7b; 32]).unwrap());
        let user = Uuid::new_v4();
        let id = Uuid::new_v4();
        let (_, sealed) = cipher.provision(user, id).unwrap();
        assert!(matches!(
            cipher.verify(Uuid::new_v4(), id, &sealed, "123456", 1, None),
            Err(TotpError::Unavailable)
        ));
        assert!(matches!(
            cipher.verify(user, Uuid::new_v4(), &sealed, "123456", 1, None),
            Err(TotpError::Unavailable)
        ));
    }

    #[test]
    fn configured_key_file_requires_exact_length_and_rejects_an_absent_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ephemeral-totp-key");
        assert!(TotpCipher::from_mounted_file(&path).is_err());
        for len in [KEY_LEN - 1, KEY_LEN, KEY_LEN + 1] {
            std::fs::write(&path, vec![0x7b; len]).unwrap();
            assert_eq!(TotpCipher::from_mounted_file(&path).is_ok(), len == KEY_LEN);
        }
    }
}
