//! Maintained `JSONPath` AST policy and localized provenance checks.

use jsonpath_rust::parser::{
    model::{JpQuery, Segment, Selector},
    parse_json_path,
};
use jsonpath_rust::query::{js_path_process, QueryRef};
use serde_json::Value;

use crate::budget::Admission;
use crate::{invalid_mapping, ApiId, ClaimSource, DomainError};

mod policy;

#[derive(Clone)]
pub struct CompiledClaim {
    api: ApiId,
    query: JpQuery,
}

impl CompiledClaim {
    pub(super) fn compile(source: &ClaimSource) -> Result<Self, DomainError> {
        // Bound input before invoking the recursive parser. The pinned parser also
        // guards grouping depth before AST construction; our stricter policy is AST-based.
        if source.api.is_empty() || source.json_path.len() > 512 {
            return Err(invalid_mapping());
        }
        let query = parse_expression(&source.json_path)?;
        policy::validate_segments(&query.segments, 0)?;
        Ok(Self {
            api: source.api.clone(),
            query,
        })
    }

    pub(super) fn select<'a>(
        &self,
        admission: &Admission<'a>,
    ) -> Result<Option<QueryRef<'a, Value>>, DomainError> {
        let response = admission.response_for(self)?;
        // This public library entry point evaluates the very AST validated above
        // and preserves every match, including duplicates and nulls.
        let mut matches = js_path_process(&self.query, response).map_err(|_| invalid_mapping())?;
        if matches.len() > 1 {
            return Err(invalid_mapping());
        }
        Ok(matches.pop())
    }

    pub(super) fn ensure_compatible_parent(&self, other: &Self) -> Result<(), DomainError> {
        let left = parent_segments(&self.query)?;
        let right = parent_segments(&other.query)?;
        if self.api != other.api
            || left.len() != right.len()
            || !left
                .iter()
                .zip(right)
                .all(|(left, right)| segments_overlap(left, right))
        {
            return Err(invalid_mapping());
        }
        Ok(())
    }

    pub(super) fn ensure_same_parent(
        &self,
        email: &QueryRef<'_, Value>,
        verified: &QueryRef<'_, Value>,
        admission: &Admission<'_>,
    ) -> Result<(), DomainError> {
        let response = admission.parent_response_for(self)?;
        // Parse the library's normalized paths with its own parser. Splitting on
        // dots/brackets would confuse literal property names with path syntax.
        let parent = localized_parent(&email.path)?;
        let other = localized_parent(&verified.path)?;
        if parent != other {
            return Err(invalid_mapping());
        }
        let matches = js_path_process(&parent, response).map_err(|_| invalid_mapping())?;
        match matches.as_slice() {
            [selected] if selected.val.is_object() => Ok(()),
            _ => Err(invalid_mapping()),
        }
    }

    pub(super) fn api(&self) -> &str {
        &self.api
    }

    pub(super) const fn query(&self) -> &JpQuery {
        &self.query
    }
}

fn parse_expression(expression: &str) -> Result<JpQuery, DomainError> {
    let query = parse_json_path(expression).map_err(|_| invalid_mapping())?;
    // The pinned grammar's non-atomic segments enable implicit Pest whitespace
    // even inside shorthand names: `$.id trailing` becomes Name("id trailing").
    // Layout outside literals must not change the maintained parser's AST.
    // Keep the original AST; this is a validation check, not a query rewrite.
    let compact = without_layout(expression);
    if compact != expression && parse_json_path(&compact).map_err(|_| invalid_mapping())? != query {
        return Err(invalid_mapping());
    }
    Ok(query)
}

fn without_layout(expression: &str) -> String {
    let mut compact = String::with_capacity(expression.len());
    let mut quote = None;
    let mut escaped = false;
    for character in expression.chars() {
        if let Some(delimiter) = quote {
            compact.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
            compact.push(character);
        } else if !matches!(character, ' ' | '\t' | '\r' | '\n') {
            compact.push(character);
        }
    }
    compact
}

fn parent_segments(query: &JpQuery) -> Result<&[Segment], DomainError> {
    query
        .segments
        .split_last()
        .map(|(_, parent)| parent)
        .ok_or_else(invalid_mapping)
}

fn localized_parent(path: &str) -> Result<JpQuery, DomainError> {
    // Localized names can be longer than the configured expression (wildcards),
    // so the 512-byte configuration bound does not apply to this library output.
    let mut query = parse_json_path(path).map_err(|_| invalid_mapping())?;
    if query.segments.len() > 16
        || !query.segments.iter().all(|segment| {
            matches!(
                segment,
                Segment::Selector(Selector::Name(_) | Selector::Index(0..))
            )
        })
    {
        return Err(invalid_mapping());
    }
    query.segments.pop().ok_or_else(invalid_mapping)?;
    Ok(query)
}

fn segments_overlap(left: &Segment, right: &Segment) -> bool {
    match (left, right) {
        (Segment::Selector(left), Segment::Selector(right)) => selectors_overlap(left, right),
        (Segment::Selectors(left), _) => left
            .iter()
            .any(|left| selector_overlaps_segment(left, right)),
        (_, Segment::Selectors(right)) => right
            .iter()
            .any(|right| selector_overlaps_segment(right, left)),
        _ => false,
    }
}

fn selector_overlaps_segment(left: &Selector, right: &Segment) -> bool {
    match right {
        Segment::Selector(right) => selectors_overlap(left, right),
        Segment::Selectors(right) => right.iter().any(|right| selectors_overlap(left, right)),
        Segment::Descendant(_) => false,
    }
}

fn selectors_overlap(left: &Selector, right: &Selector) -> bool {
    match (left, right) {
        (Selector::Name(left), Selector::Name(right)) => left == right,
        (Selector::Index(left), Selector::Index(right)) => {
            // Positive and negative indices can designate the same array element.
            left == right || left.is_negative() != right.is_negative()
        }
        (Selector::Name(_), Selector::Index(_) | Selector::Slice(..))
        | (Selector::Index(_) | Selector::Slice(..), Selector::Name(_)) => false,
        // Different predicates/wildcards/slices may overlap for some responses.
        // Do not claim to prove predicate equivalence; actual locations decide.
        _ => true,
    }
}
