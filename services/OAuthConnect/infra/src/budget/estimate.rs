//! Conservative arithmetic over the already validated, maintained AST.

use jsonpath_rust::parser::model::{
    Comparable, Comparison, Filter, FilterAtom, JpQuery, Literal, Segment, Selector, SingularQuery,
    SingularQuerySegment, Test,
};

use super::scan::ApiStats;
use super::{capped_add, capped_mul, length, MAX_PROJECTION_COPY_BYTES, MAX_PROJECTION_WORK};
use crate::{invalid_mapping, DomainError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryBound {
    pub references: u64,
    pub work: u64,
    pub path: u64,
    pub max_path: u64,
    pub literal_bytes: u64,
}

impl QueryBound {
    const fn start(path: u64) -> Self {
        Self {
            references: 1,
            work: 4,
            path,
            max_path: path,
            literal_bytes: 0,
        }
    }

    fn include_cost(&mut self, other: &Self) -> Result<(), DomainError> {
        self.work = add(self.work, other.work);
        self.max_path = self.max_path.max(other.max_path);
        self.literal_bytes = self.literal_bytes.max(other.literal_bytes);
        self.check()
    }

    fn check(&self) -> Result<(), DomainError> {
        if self.work > MAX_PROJECTION_WORK
            || self.references > MAX_PROJECTION_WORK
            || self.max_path > MAX_PROJECTION_COPY_BYTES
            || self.literal_bytes > MAX_PROJECTION_COPY_BYTES
        {
            return Err(invalid_mapping());
        }
        Ok(())
    }
}

const fn add(left: u64, right: u64) -> u64 {
    capped_add(left, right, MAX_PROJECTION_WORK)
}

const fn mul(left: u64, right: u64) -> u64 {
    capped_mul(left, right, MAX_PROJECTION_WORK)
}

const fn path_add(left: u64, right: u64) -> u64 {
    capped_add(left, right, MAX_PROJECTION_COPY_BYTES)
}

const fn name_bytes(bytes: u64) -> u64 {
    path_add(4, capped_mul(6, bytes, MAX_PROJECTION_COPY_BYTES))
}

/// # Errors
/// Refuses an unsupported AST variant or an already excessive intermediate cost.
pub fn estimate_query(query: &JpQuery, stats: &ApiStats) -> Result<QueryBound, DomainError> {
    sequence(&query.segments, stats, 1)
}

fn sequence(segments: &[Segment], stats: &ApiStats, path: u64) -> Result<QueryBound, DomainError> {
    let mut bound = QueryBound::start(path);
    for segment in segments {
        let step = match segment {
            Segment::Selector(selector) => {
                selector_bound(selector, bound.references, bound.path, stats)?
            }
            Segment::Selectors(selectors) => {
                union_bound(selectors, bound.references, bound.path, stats)?
            }
            Segment::Descendant(_) => return Err(invalid_mapping()),
        };
        bound.include_cost(&step)?;
        bound.references = step.references;
        bound.path = step.path;
    }
    Ok(bound)
}

const fn name_bound(references: u64, path: u64, bytes: u64, stats: &ApiStats) -> QueryBound {
    let path = path_add(path, name_bytes(bytes));
    QueryBound {
        references,
        work: mul(mul(4, references), add(stats.max_object, 1)),
        path,
        max_path: path,
        literal_bytes: 0,
    }
}

const fn index_bound(references: u64, path: u64) -> QueryBound {
    let path = path_add(path, 22);
    QueryBound {
        references,
        work: mul(4, references),
        path,
        max_path: path,
        literal_bytes: 0,
    }
}

fn selector_bound(
    selector: &Selector,
    references: u64,
    path: u64,
    stats: &ApiStats,
) -> Result<QueryBound, DomainError> {
    let bound = match selector {
        Selector::Name(name) => name_bound(references, path, length(name.len()), stats),
        Selector::Index(_) => index_bound(references, path),
        Selector::Wildcard | Selector::Slice(..) | Selector::Filter(_) => {
            let fanout = stats.fanout();
            let path = path_add(path, name_bytes(stats.max_key).max(22));
            let mut expansion = QueryBound {
                references: mul(references, fanout),
                work: mul(mul(4, references), add(1, fanout)),
                path,
                max_path: path,
                literal_bytes: 0,
            };
            if let Selector::Filter(filter) = selector {
                let mut predicate = filter_bound(filter, stats)?;
                // One symbolic predicate estimate, charged per candidate. Do not
                // run it against the document or discount an early false/true.
                predicate.work = mul(expansion.references, predicate.work);
                expansion.include_cost(&predicate)?;
            }
            expansion
        }
    };
    bound.check()?;
    Ok(bound)
}

fn union_bound(
    selectors: &[Selector],
    references: u64,
    path: u64,
    stats: &ApiStats,
) -> Result<QueryBound, DomainError> {
    if selectors.is_empty() {
        return Err(invalid_mapping());
    }
    let mut union = QueryBound {
        references: 0,
        work: 0,
        path,
        max_path: path,
        literal_bytes: 0,
    };
    for selector in selectors {
        // Alternatives share the same input; count the preceding sequence once.
        let branch = selector_bound(selector, references, path, stats)?;
        union.include_cost(&branch)?;
        union.references = add(union.references, branch.references);
        union.path = union.path.max(branch.path);
    }
    let copies = mul(
        mul(4, length(selectors.len())),
        add(references, union.references),
    );
    union.work = add(union.work, copies);
    union.check()?;
    Ok(union)
}

fn filter_bound(filter: &Filter, stats: &ApiStats) -> Result<QueryBound, DomainError> {
    let mut cost = QueryBound::start(0);
    match filter {
        Filter::And(filters) | Filter::Or(filters) => {
            for filter in filters {
                cost.include_cost(&filter_bound(filter, stats)?)?;
            }
        }
        Filter::Atom(FilterAtom::Filter { expr, .. }) => {
            cost.include_cost(&filter_bound(expr, stats)?)?;
        }
        Filter::Atom(FilterAtom::Test { expr, not }) => {
            let query = match expr.as_ref() {
                Test::RelQuery(segments) => sequence(segments, stats, 0)?,
                Test::AbsQuery(query) => estimate_query(query, stats)?,
                Test::Function(_) => return Err(invalid_mapping()),
            };
            cost.include_cost(&query)?;
            if *not {
                cost.work = add(4, cost.work);
            }
        }
        Filter::Atom(FilterAtom::Comparison(comparison)) => {
            return comparison_bound(comparison, stats);
        }
    }
    cost.check()?;
    Ok(cost)
}

fn comparison_bound(comparison: &Comparison, stats: &ApiStats) -> Result<QueryBound, DomainError> {
    let (left, right) = comparison.vals();
    let mut cost = QueryBound::start(0);
    cost.include_cost(&comparable_bound(left, stats)?)?;
    cost.include_cost(&comparable_bound(right, stats)?)?;
    let size = if matches!(left, Comparable::Literal(_)) || matches!(right, Comparable::Literal(_))
    {
        cost.literal_bytes
    } else {
        // A singular query can still select a deeply compared object or array.
        stats.units
    };
    cost.work = add(cost.work, add(4, mul(2, size)));
    cost.check()?;
    Ok(cost)
}

fn comparable_bound(comparable: &Comparable, stats: &ApiStats) -> Result<QueryBound, DomainError> {
    match comparable {
        Comparable::Literal(literal) => {
            let mut cost = QueryBound::start(0);
            if let Literal::String(value) = literal {
                cost.literal_bytes = length(value.len());
            }
            Ok(cost)
        }
        Comparable::SingularQuery(query) => singular_bound(query, stats),
        Comparable::Function(_) => Err(invalid_mapping()),
    }
}

fn singular_bound(query: &SingularQuery, stats: &ApiStats) -> Result<QueryBound, DomainError> {
    let (segments, path) = match query {
        SingularQuery::Current(segments) => (segments, 0),
        SingularQuery::Root(segments) => (segments, 1),
    };
    let mut bound = QueryBound::start(path);
    for segment in segments {
        let step = match segment {
            SingularQuerySegment::Name(name) => {
                name_bound(1, bound.path, length(name.len()), stats)
            }
            SingularQuerySegment::Index(_) => index_bound(1, bound.path),
        };
        bound.include_cost(&step)?;
        bound.path = step.path;
    }
    Ok(bound)
}
