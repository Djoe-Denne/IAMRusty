//! PH-6 closed public proof schema. Never dump arbitrary response/config/error text.

#[derive(Clone, Copy)]
pub enum ApiObservation {
    PluginImage,
    AdmissionDigest,
}

impl ApiObservation {
    pub const fn failure_category(self) -> &'static str {
        match self {
            Self::PluginImage => "M6 FAIL plugin image mismatch",
            Self::AdmissionDigest => "M6 FAIL AdmissionRecord digest mismatch",
        }
    }
}

/// S-2: retain strict equality without reflecting either API operand in a panic.
///
/// # Panics
/// On unequal values, with only a closed failure category (never `Debug` operands).
#[track_caller]
pub fn assert_api_equal(actual: &str, expected: &str, observation: ApiObservation) {
    assert!(actual == expected, "{}", observation.failure_category());
}

fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    })
}

pub fn h1(catalog: &str, envelope: &str, image: &str) -> Result<String, &'static str> {
    let Some((name, pin)) = image.split_once('@') else {
        return Err("H1 image pin invalid");
    };
    if !matches!(
        name,
        "apparatus-reference-kv" | "docker.io/library/apparatus-reference-kv"
    ) || !digest(pin)
        || !digest(catalog)
        || !digest(envelope)
    {
        return Err("H1 digest or image invalid");
    }
    Ok(format!(
        "M6 H1 catalog={catalog} envelope={envelope} image={image}"
    ))
}

pub fn h2(catalog: &str, status: &str, scheduled: &str) -> Result<String, &'static str> {
    if !digest(catalog) || status != "VALID" || scheduled != "Scheduled" {
        return Err("H2 observation invalid");
    }
    Ok(format!(
        "M6 H2 catalog={catalog} status={status} schedule={scheduled}"
    ))
}

pub fn h3(
    denied: u16,
    allowed: u16,
    denied_hops: usize,
    allowed_hops: usize,
    hostname: &str,
    expected: &str,
) -> Result<String, &'static str> {
    if denied != 403
        || allowed != 200
        || denied_hops != 0
        || allowed_hops == 0
        || hostname != expected
        || !hostname.strip_prefix("plugin-").is_some_and(|suffix| {
            suffix.len() == 32
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        })
    {
        return Err("H3 observation invalid");
    }
    Ok(format!("M6 H3 denied_http={denied} allowed_http={allowed} denied_hops={denied_hops} allowed_hops={allowed_hops} hostname={hostname}"))
}
