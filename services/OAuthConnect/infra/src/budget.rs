//! One borrowed admission before any exhaustive library evaluation (S-1).

use serde_json::Value;

use crate::{invalid_mapping, ApiResponses, CompiledClaim, CompiledMapping, DomainError};

mod estimate;
mod scan;

const MAX_SCAN_UNITS_PER_API: u64 = 1_048_576;
const MAX_JSON_DEPTH: usize = 128;
const MAX_PROJECTION_WORK: u64 = 1_000_000;
const MAX_PROJECTION_COPY_BYTES: u64 = 16_777_216;

// All callers use the fixed local ceilings above, which leave room for cap+1.
pub const fn capped_add(left: u64, right: u64, cap: u64) -> u64 {
    let result = left.saturating_add(right);
    if result > cap {
        cap + 1
    } else {
        result
    }
}

pub const fn capped_mul(left: u64, right: u64, cap: u64) -> u64 {
    let result = left.saturating_mul(right);
    if result > cap {
        cap + 1
    } else {
        result
    }
}

fn length(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// The fields are private to this module: only a complete preflight creates it.
pub struct Admission<'a> {
    mapping: &'a CompiledMapping,
    responses: &'a ApiResponses,
}

impl<'a> Admission<'a> {
    /// # Errors
    /// Refuses a claim from another mapping or a missing referenced response.
    pub fn response_for(&self, claim: &CompiledClaim) -> Result<&'a Value, DomainError> {
        if !self
            .mapping
            .claims()
            .any(|known| std::ptr::eq(known, claim))
        {
            return Err(invalid_mapping());
        }
        self.responses.get(claim.api()).ok_or_else(invalid_mapping)
    }

    /// # Errors
    /// Only the reserved email-parent check may use this capability.
    pub fn parent_response_for(&self, email: &CompiledClaim) -> Result<&'a Value, DomainError> {
        if !std::ptr::eq(
            std::ptr::from_ref(&self.mapping.email),
            std::ptr::from_ref(email),
        ) {
            return Err(invalid_mapping());
        }
        self.response_for(email)
    }
}

/// # Errors
/// Refuses scan, work or copy excess before returning an evaluation capability.
pub fn admit<'a>(
    mapping: &'a CompiledMapping,
    responses: &'a ApiResponses,
) -> Result<Admission<'a>, DomainError> {
    estimate_mapping(mapping, responses)?;
    Ok(Admission { mapping, responses })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionBound {
    pub work: u64,
    pub max_path: u64,
    pub literal_bytes: u64,
    pub copy_bytes: u64,
}

impl ProjectionBound {
    /// # Errors
    /// Refuses cumulative work excess; maxima never reset between claims.
    pub fn add_query(&mut self, query: &estimate::QueryBound) -> Result<(), DomainError> {
        self.work = capped_add(self.work, query.work, MAX_PROJECTION_WORK);
        self.max_path = self.max_path.max(query.max_path);
        self.literal_bytes = self.literal_bytes.max(query.literal_bytes);
        check_limits(self.work, 0)
    }

    /// # Errors
    /// Refuses excess after reserving parsings, comparison and parent evaluation.
    pub fn reserve_parents(&mut self, stats: &scan::ApiStats) -> Result<(), DomainError> {
        let lookups = capped_mul(
            64,
            capped_add(stats.max_object, 1, MAX_PROJECTION_WORK),
            MAX_PROJECTION_WORK,
        );
        let reserve = capped_add(4, lookups, MAX_PROJECTION_WORK);
        self.work = capped_add(self.work, reserve, MAX_PROJECTION_WORK);
        check_limits(self.work, 0)
    }

    /// # Errors
    /// Refuses either shared ceiling, including overflow represented by cap+1.
    pub fn finish(mut self) -> Result<Self, DomainError> {
        let bytes = capped_add(self.max_path, self.literal_bytes, MAX_PROJECTION_COPY_BYTES);
        let bytes = capped_add(bytes, 32, MAX_PROJECTION_COPY_BYTES);
        let copies = capped_mul(8, self.work, MAX_PROJECTION_COPY_BYTES);
        self.copy_bytes = capped_mul(copies, bytes, MAX_PROJECTION_COPY_BYTES);
        check_limits(self.work, self.copy_bytes)?;
        Ok(self)
    }
}

/// # Errors
/// Equality is admitted; either value strictly above its ceiling is refused.
pub fn check_limits(work: u64, copy_bytes: u64) -> Result<(), DomainError> {
    if work > MAX_PROJECTION_WORK || copy_bytes > MAX_PROJECTION_COPY_BYTES {
        return Err(invalid_mapping());
    }
    Ok(())
}

/// # Errors
/// Refuses missing APIs, excessive scans, unsupported ASTs or cumulative cost.
pub fn estimate_mapping(
    mapping: &CompiledMapping,
    responses: &ApiResponses,
) -> Result<ProjectionBound, DomainError> {
    // At most five borrowed identifiers/statistics, never cloned JSON. Extra API
    // responses are deliberately not inspected, including their size/depth.
    let mut summaries: Vec<(&str, scan::ApiStats)> = Vec::with_capacity(5);
    for claim in mapping.claims() {
        if !summaries.iter().any(|(api, _)| *api == claim.api()) {
            let response = responses.get(claim.api()).ok_or_else(invalid_mapping)?;
            summaries.push((claim.api(), scan::scan_api(response)?));
        }
    }
    let mut total = ProjectionBound::default();
    for claim in mapping.claims() {
        let stats = statistics(&summaries, claim.api())?;
        total.add_query(&estimate::estimate_query(claim.query(), stats)?)?;
    }
    // compile_mapping already requires the email pair to use this same API.
    // The full claim paths already majorize their shorter parent paths.
    total.reserve_parents(statistics(&summaries, mapping.email.api())?)?;
    total.finish()
}

fn statistics<'a>(
    summaries: &'a [(&str, scan::ApiStats)],
    api: &str,
) -> Result<&'a scan::ApiStats, DomainError> {
    summaries
        .iter()
        .find_map(|(id, stats)| (*id == api).then_some(stats))
        .ok_or_else(invalid_mapping)
}

#[cfg(test)]
mod tests;
