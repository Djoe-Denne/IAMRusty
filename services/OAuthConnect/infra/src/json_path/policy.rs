//! Closed, recursively bounded allowlist over jsonpath-rust 1.0.12's AST.

use jsonpath_rust::parser::model::{
    Comparable, Filter, FilterAtom, Segment, Selector, SingularQuery, Test,
};

use crate::{invalid_mapping, DomainError};

const MAX_DEPTH: usize = 16;

fn depth_within_limit(depth: usize) -> Result<(), DomainError> {
    if depth > MAX_DEPTH {
        return Err(invalid_mapping());
    }
    Ok(())
}

pub(super) fn validate_segments(segments: &[Segment], depth: usize) -> Result<(), DomainError> {
    depth_within_limit(depth + segments.len())?;
    for (index, segment) in segments.iter().enumerate() {
        let depth = depth + index + 1;
        match segment {
            Segment::Selector(selector) => validate_selector(selector, depth)?,
            Segment::Selectors(selectors) => {
                for selector in selectors {
                    validate_selector(selector, depth)?;
                }
            }
            Segment::Descendant(_) => return Err(invalid_mapping()),
        }
    }
    Ok(())
}

fn validate_selector(selector: &Selector, depth: usize) -> Result<(), DomainError> {
    match selector {
        Selector::Name(_) | Selector::Index(_) | Selector::Wildcard | Selector::Slice(..) => Ok(()),
        Selector::Filter(filter) => validate_filter(filter, depth),
    }
}

fn validate_filter(filter: &Filter, depth: usize) -> Result<(), DomainError> {
    depth_within_limit(depth)?;
    match filter {
        Filter::Or(filters) | Filter::And(filters) => {
            for filter in filters {
                validate_filter(filter, depth + 1)?;
            }
            Ok(())
        }
        Filter::Atom(FilterAtom::Filter { expr, .. }) => validate_filter(expr, depth + 1),
        Filter::Atom(FilterAtom::Test { expr, not }) => {
            validate_test(expr, depth + usize::from(*not))
        }
        Filter::Atom(FilterAtom::Comparison(comparison)) => {
            let (left, right) = comparison.vals();
            validate_comparable(left, depth)?;
            validate_comparable(right, depth)
        }
    }
}

fn validate_test(test: &Test, depth: usize) -> Result<(), DomainError> {
    match test {
        Test::RelQuery(segments) => validate_segments(segments, depth),
        Test::AbsQuery(query) => validate_segments(&query.segments, depth),
        Test::Function(_) => Err(invalid_mapping()),
    }
}

fn validate_comparable(comparable: &Comparable, depth: usize) -> Result<(), DomainError> {
    match comparable {
        Comparable::Literal(_) => Ok(()),
        Comparable::SingularQuery(
            SingularQuery::Current(segments) | SingularQuery::Root(segments),
        ) => depth_within_limit(depth + segments.len()),
        Comparable::Function(_) => Err(invalid_mapping()),
    }
}
