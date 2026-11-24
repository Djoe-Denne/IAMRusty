//! Independent authenticator for STORY-004 tests. Never used by the service.

use anyhow::{ensure, Context as _};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

pub const PERIOD: u64 = 30;
const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Parse the provisioning payload without surfacing it in test errors.
pub fn seed_from_uri(uri: &str) -> anyhow::Result<[u8; 20]> {
    let uri = Url::parse(uri).map_err(|_| anyhow::anyhow!("invalid provisioning URI"))?;
    ensure!(
        uri.scheme() == "otpauth" && uri.host_str() == Some("totp") && uri.path().len() > 1,
        "expected a labelled TOTP provisioning URI"
    );
    let mut secret = None;
    for (name, value) in uri.query_pairs() {
        match name.as_ref() {
            "secret" => {
                ensure!(secret.is_none(), "duplicate provisioning seed");
                secret = Some(decode_seed(value.as_bytes())?);
            }
            "algorithm" => ensure!(value == "SHA1", "expected interoperable TOTP algorithm"),
            "digits" => ensure!(value == "6", "expected six-digit TOTP"),
            "period" => ensure!(value == "30", "expected a 30-second TOTP period"),
            _ => {}
        }
    }
    secret.context("provisioning URI has no seed")
}

fn decode_seed(encoded: &[u8]) -> anyhow::Result<[u8; 20]> {
    ensure!(
        encoded.len() == 32,
        "expected a 160-bit unpadded Base32 seed"
    );
    let mut seed = [0; 20];
    for (symbols, bytes) in encoded
        .as_chunks::<8>()
        .0
        .iter()
        .zip(seed.as_chunks_mut::<5>().0)
    {
        let mut bits = 0_u64;
        for &symbol in symbols {
            let value = BASE32
                .iter()
                .position(|&allowed| allowed == symbol)
                .context("invalid Base32 provisioning seed")?;
            bits = (bits << 5) | value as u64;
        }
        bytes.copy_from_slice(&bits.to_be_bytes()[3..]);
    }
    Ok(seed)
}

pub fn clock() -> anyhow::Result<Duration> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?)
}

pub fn code(seed: &[u8; 20], step: u64) -> String {
    format!("{:06}", number(seed, step) % 1_000_000)
}

// Fixed 20-byte RFC 4226 key, eight-byte big-endian counter, then dynamic truncation.
fn number(seed: &[u8; 20], step: u64) -> u32 {
    let mut inner = [0x36; 72];
    let mut outer = [0x5c; 84];
    for (index, &byte) in seed.iter().enumerate() {
        inner[index] ^= byte;
        outer[index] ^= byte;
    }
    inner[64..].copy_from_slice(&step.to_be_bytes());
    outer[64..].copy_from_slice(&sha1(&inner));
    let mac = sha1(&outer);
    let offset = usize::from(mac[19] & 0x0f);
    u32::from_be_bytes([
        mac[offset] & 0x7f,
        mac[offset + 1],
        mac[offset + 2],
        mac[offset + 3],
    ])
}

// Test-only FIPS 180-4 compression; the integration target needs no new
// dependencies or production TOTP helper to drive the red case.
fn sha1(input: &[u8]) -> [u8; 20] {
    let mut data = input.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&(input.len() as u64 * 8).to_be_bytes());
    let mut state = [
        0x6745_2301_u32,
        0xefcd_ab89,
        0x98ba_dcfe,
        0x1032_5476,
        0xc3d2_e1f0,
    ];
    for block in data.as_chunks::<64>().0 {
        let mut schedule = [0_u32; 80];
        for (dst, src) in schedule.iter_mut().zip(block.as_chunks::<4>().0) {
            *dst = u32::from_be_bytes(*src);
        }
        for i in 16..80 {
            schedule[i] = (schedule[i - 3] ^ schedule[i - 8] ^ schedule[i - 14] ^ schedule[i - 16])
                .rotate_left(1);
        }
        let [mut work_a, mut work_b, mut work_c, mut work_d, mut work_e] = state;
        for (i, word) in schedule.into_iter().enumerate() {
            let (mix, constant) = match i {
                0..=19 => ((work_b & work_c) | (!work_b & work_d), 0x5a82_7999),
                20..=39 => (work_b ^ work_c ^ work_d, 0x6ed9_eba1),
                40..=59 => (
                    (work_b & work_c) | (work_b & work_d) | (work_c & work_d),
                    0x8f1b_bcdc,
                ),
                _ => (work_b ^ work_c ^ work_d, 0xca62_c1d6),
            };
            let next = work_a
                .rotate_left(5)
                .wrapping_add(mix)
                .wrapping_add(work_e)
                .wrapping_add(constant)
                .wrapping_add(word);
            [work_a, work_b, work_c, work_d, work_e] =
                [next, work_a, work_b.rotate_left(30), work_c, work_d];
        }
        for (acc, value) in state
            .iter_mut()
            .zip([work_a, work_b, work_c, work_d, work_e])
        {
            *acc = acc.wrapping_add(value);
        }
    }
    let mut digest = [0; 20];
    for (dst, value) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        dst.copy_from_slice(&value.to_be_bytes());
    }
    digest
}

#[test]
fn app_oracle_matches_published_rfc6238_sha1_vectors() -> anyhow::Result<()> {
    // RFC 6238 Appendix B public test key, represented in a standard app URI.
    let seed = seed_from_uri(
        "otpauth://totp/AIForAll:test?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=AIForAll",
    )?;
    for (time, expected) in [
        (59, "94287082"),
        (1_111_111_109, "07081804"),
        (1_111_111_111, "14050471"),
        (1_234_567_890, "89005924"),
        (2_000_000_000, "69279037"),
        (20_000_000_000, "65353130"),
    ] {
        assert_eq!(
            format!("{:08}", number(&seed, time / PERIOD) % 100_000_000),
            expected
        );
    }
    // The same authenticator configuration with six digits truncates modulo 10^6.
    assert_eq!(code(&seed, 59 / PERIOD), "287082");
    Ok(())
}
