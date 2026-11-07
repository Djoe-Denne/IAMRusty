//! G-2 / S-1 private evidence added AFTER the budget implementation, not TDD.
//!
//! Obligations -> risks -> scenarios -> oracles (budget contract §§3–5):
//! - Arithmetic caps -> overflow admitted -> equality / +1 / overflow operands ->
//!   fixed ceiling accepted, excess sentinel strictly above that ceiling.
//! - Cumulative estimate -> forgotten work/path/parent -> independent small ASTs
//!   and the annex fixture -> hand-computed costs, maxima and admission outcomes.
//! - Borrowed capability -> evaluation outside preflight -> own/foreign claims,
//!   missing/over-budget inputs -> original response identity or redacted refusal.
//!
//! All 20 tests are KEEP. They complement static control-flow review; no fake
//! evaluator, public counter, copied estimator or elapsed-time assertion is used.

use super::estimate::{estimate_query, QueryBound};
use super::scan::{scan_api, ApiStats};
use super::{
    admit, capped_add, capped_mul, check_limits, estimate_mapping, ProjectionBound,
    MAX_PROJECTION_COPY_BYTES, MAX_PROJECTION_WORK,
};
use crate::{
    compile_mapping, invalid_mapping, ApiResponses, ClaimSource, DomainError, OAuthClaims,
};
use jsonpath_rust::parser::parse_json_path;
use serde_json::{json, Value};

const NESTED: &str = "$.items[?($.items[?($.items[?($.items[?(@.ok == true)])])])].id";
const NESTED_USERNAME: &str = "$.items[?($.items[?($.items[?($.items[?(@.ok == true)])])])].u";

fn source(api: &str, path: &str) -> ClaimSource {
    ClaimSource {
        api: api.to_owned(),
        json_path: path.to_owned(),
    }
}

fn claims(sub: &str) -> OAuthClaims {
    OAuthClaims {
        sub: source("user", sub),
        email: source("user", "$.p.e"),
        email_verified: source("user", "$.p.v"),
        preferred_username: source("user", "$.p.u"),
        picture: None,
    }
}

fn user() -> Value {
    json!({"id": "stable", "p": {"e": "one@example.test", "v": true, "u": "name"}})
}

fn responses(user: Value) -> ApiResponses {
    ApiResponses::from([("user".to_owned(), user)])
}

fn nested_user(count: usize) -> Value {
    let items: Vec<Value> = (0..count)
        .map(|index| {
            if index == 0 {
                json!({"ok": true, "id": "stable"})
            } else {
                json!({"ok": true})
            }
        })
        .collect();
    json!({"items": items, "p": {"e": "one@example.test", "v": true, "u": "name"}})
}

fn query_bound(path: &str, stats: &ApiStats) -> Result<QueryBound, DomainError> {
    // Use the pinned maintained parser, never a handwritten test AST parser.
    // A syntax error here is a fixture failure, not S-1 evidence.
    let query = parse_json_path(path).map_err(|_| invalid_mapping())?;
    estimate_query(&query, stats)
}

fn assert_refused<T>(result: Result<T, DomainError>, scenario: &str) {
    match result {
        Ok(_) => panic!("S-1 private: expected refusal for {scenario}"),
        Err(error) => {
            for diagnostic in [error.to_string(), format!("{error:?}")] {
                for private in ["stable", "one@example.test", NESTED, "\"items\""] {
                    assert!(
                        !diagnostic.contains(private),
                        "S-1 private: diagnostic must not expose response values or expression"
                    );
                }
            }
        }
    }
}

#[test]
fn s1_capped_add_accepts_fixed_ceilings_and_marks_excess_or_overflow() {
    for cap in [MAX_PROJECTION_WORK, MAX_PROJECTION_COPY_BYTES] {
        assert_eq!(
            capped_add(cap - 1, 1, cap),
            cap,
            "equality must remain admissible"
        );
        assert_eq!(
            capped_add(cap, 0, cap),
            cap,
            "zero must not reject equality"
        );
        assert_eq!(
            capped_add(cap, 1, cap),
            cap + 1,
            "excess is not saturated to admission"
        );
        assert_eq!(
            capped_add(u64::MAX, 1, cap),
            cap + 1,
            "overflow must not wrap to zero"
        );
    }
}

#[test]
fn s1_capped_mul_accepts_fixed_ceilings_and_marks_excess_or_overflow() {
    for cap in [MAX_PROJECTION_WORK, MAX_PROJECTION_COPY_BYTES] {
        assert_eq!(capped_mul(cap / 2, 2, cap), cap, "exact fixed ceiling");
        assert_eq!(
            capped_mul(cap + 1, 1, cap),
            cap + 1,
            "strict excess remains refused"
        );
        assert_eq!(
            capped_mul(u64::MAX, 2, cap),
            cap + 1,
            "overflow must not wrap"
        );
        assert_eq!(capped_mul(u64::MAX, 0, cap), 0, "zero product is exact");
    }
}

#[test]
fn s1_check_limits_admits_both_equalities_and_refuses_either_excess() -> Result<(), DomainError> {
    check_limits(MAX_PROJECTION_WORK, MAX_PROJECTION_COPY_BYTES)?;
    assert_refused(
        check_limits(MAX_PROJECTION_WORK + 1, 0),
        "work ceiling plus one",
    );
    assert_refused(
        check_limits(0, MAX_PROJECTION_COPY_BYTES + 1),
        "copy ceiling plus one",
    );
    assert_refused(check_limits(u64::MAX, u64::MAX), "overflow sentinels");
    Ok(())
}

#[test]
fn s1_finish_admits_exact_copy_ceiling_and_refuses_the_next_path_byte() -> Result<(), DomainError> {
    // 8 * 1024 * (2016 + 0 + 32) = 16_777_216, without JSON allocations.
    let exact = ProjectionBound {
        work: 1024,
        max_path: 2016,
        ..ProjectionBound::default()
    };
    assert_eq!(
        exact.finish()?.copy_bytes,
        16_777_216,
        "exact copy ceiling is admitted"
    );
    let excess = ProjectionBound {
        max_path: 2017,
        ..exact
    };
    assert_refused(excess.finish(), "one extra path byte over the copy ceiling");
    Ok(())
}

#[test]
fn s1_finish_refuses_addition_and_multiplication_overflow() {
    for bound in [
        ProjectionBound {
            work: 1,
            max_path: u64::MAX,
            literal_bytes: u64::MAX,
            copy_bytes: 0,
        },
        ProjectionBound {
            work: u64::MAX,
            max_path: 1,
            ..ProjectionBound::default()
        },
        ProjectionBound {
            work: MAX_PROJECTION_WORK,
            max_path: MAX_PROJECTION_COPY_BYTES,
            ..ProjectionBound::default()
        },
    ] {
        assert_refused(
            bound.finish(),
            "overflow or oversized product in final copy bound",
        );
    }
}

#[test]
fn s1_query_accumulation_sums_work_and_preserves_path_and_literal_maxima() -> Result<(), DomainError>
{
    let mut total = ProjectionBound::default();
    total.add_query(&QueryBound {
        references: 1,
        work: 11,
        path: 10,
        max_path: 101,
        literal_bytes: 2,
    })?;
    total.add_query(&QueryBound {
        references: 1,
        work: 13,
        path: 9,
        max_path: 9,
        literal_bytes: 7,
    })?;
    let finished = total.finish()?;
    assert_eq!(
        (finished.work, finished.max_path, finished.literal_bytes),
        (24, 101, 7)
    );
    assert_eq!(finished.copy_bytes, 26_880, "8 * 24 * (101 + 7 + 32)");
    total.add_query(&QueryBound {
        references: 1,
        work: MAX_PROJECTION_WORK - 24,
        path: 1,
        max_path: 1,
        literal_bytes: 0,
    })?;
    assert_eq!(
        total.work, MAX_PROJECTION_WORK,
        "cumulative equality admitted at work stage"
    );
    assert_refused(
        total.add_query(&QueryBound {
            references: 1,
            work: 1,
            path: 1,
            max_path: 1,
            literal_bytes: 0,
        }),
        "one more cumulative work unit",
    );
    Ok(())
}

#[test]
fn s1_parent_reservation_is_exact_and_uses_the_shared_work_ceiling() -> Result<(), DomainError> {
    let stats = ApiStats {
        max_object: 3,
        ..ApiStats::default()
    };
    let mut total = ProjectionBound::default();
    total.reserve_parents(&stats)?;
    assert_eq!(total.work, 260, "parent reserve = 4 + 64 * (3 + 1)");
    let mut at_limit = ProjectionBound {
        work: MAX_PROJECTION_WORK - 260,
        ..ProjectionBound::default()
    };
    at_limit.reserve_parents(&stats)?;
    assert_eq!(
        at_limit.work, MAX_PROJECTION_WORK,
        "parent reservation reaches equality"
    );
    assert_refused(
        at_limit.reserve_parents(&stats),
        "parent work is not a separate allowance",
    );
    let mut overflow = ProjectionBound::default();
    assert_refused(
        overflow.reserve_parents(&ApiStats {
            max_object: u64::MAX,
            ..ApiStats::default()
        }),
        "overflow in parent reservation",
    );
    Ok(())
}

#[test]
fn s1_scan_counts_nodes_utf8_keys_and_strings_once_with_exact_fanout() -> Result<(), DomainError> {
    let stats = scan_api(&json!({"é": ["λ", true, null], "x": {"long": "🙂"}}))?;
    // Seven Values; keys 2+1+4 bytes, strings 2+4 bytes, independent of JSON encoding.
    assert_eq!(
        stats,
        ApiStats {
            nodes: 7,
            bytes: 13,
            units: 20,
            max_array: 3,
            max_object: 2,
            max_key: 4
        }
    );
    assert_eq!(stats.fanout(), 3, "array/object maximum, not their sum");
    Ok(())
}

#[test]
fn s1_scan_scalar_and_empty_roots_count_the_root_and_have_minimum_fanout_one(
) -> Result<(), DomainError> {
    for value in [Value::Null, json!(false), json!([]), json!({})] {
        let stats = scan_api(&value)?;
        assert_eq!(
            stats,
            ApiStats {
                nodes: 1,
                units: 1,
                ..ApiStats::default()
            }
        );
        assert_eq!(
            stats.fanout(),
            1,
            "empty input still has symbolic multiplicity one"
        );
    }
    let string = scan_api(&json!("é"))?;
    assert_eq!((string.nodes, string.bytes, string.units), (1, 2, 3));
    Ok(())
}

#[test]
fn s1_annex_nested_absolute_reference_has_exact_cost_and_rejects_32() -> Result<(), DomainError> {
    let compiled = compile_mapping(&claims(NESTED))?;
    let two = estimate_mapping(&compiled, &responses(nested_user(2)))?;
    assert!(two.work < MAX_PROJECTION_WORK && two.copy_bytes < MAX_PROJECTION_COPY_BYTES);
    let four = estimate_mapping(&compiled, &responses(nested_user(4)))?;
    assert_eq!(
        (
            four.work,
            four.max_path,
            four.literal_bytes,
            four.copy_bytes
        ),
        (13_720, 85, 0, 12_841_920),
        "independent numeric reference from annex §5.2"
    );
    assert_refused(
        estimate_mapping(&compiled, &responses(nested_user(32))),
        "annex 32-item estimate",
    );
    Ok(())
}

#[test]
fn s1_unions_preserve_occurrences_and_charge_all_alternatives_and_copy_surcharge(
) -> Result<(), DomainError> {
    let stats = scan_api(&json!({"a": [1, 2]}))?; // O=1, F=2, K=1.
                                                  // Prefix $.a costs 4+8=12. Two indices: 4+4+8*(1+2)=32.
                                                  // Wildcard+index: 12+4+8*(1+3)=48. Slice conservatively expands by F=2.
    for (path, references, work) in [
        ("$.a[0]", 1, 16),
        ("$.a[0,0]", 2, 44),
        ("$.a[0,1]", 2, 44),
        ("$.a[*,0]", 3, 60),
        ("$.a[*]", 2, 24),
        ("$.a[0:1]", 2, 24),
    ] {
        let bound = query_bound(path, &stats)?;
        assert_eq!(
            (
                bound.references,
                bound.work,
                bound.path,
                bound.max_path,
                bound.literal_bytes
            ),
            (references, work, 33, 33, 0),
            "union/expansion hand oracle for {path}"
        );
    }
    Ok(())
}

#[test]
fn s1_maximum_inner_path_survives_later_root_reset() -> Result<(), DomainError> {
    let mut chain = json!(0);
    for key in ["j", "i", "h", "g", "f", "e", "d", "c", "b", "a"] {
        let mut parent = json!({});
        parent[key] = chain;
        chain = parent;
    }
    let stats = scan_api(&json!({"items": [chain], "x": 0}))?;
    for path in [
        "$.items[?(@.a.b.c.d.e.f.g.h.i.j == $.x)].id",
        "$.items[?(@.a.b.c.d.e.f.g.h.i.j && $.x)].id",
    ] {
        let bound = query_bound(path, &stats)?;
        // Ten relative 1-byte names: 100; later Root $.x: 11.
        // Outer final path: 1+34+34+16=85. Keep 100, not 85 or 11.
        assert_eq!(
            (bound.path, bound.max_path),
            (85, 100),
            "maximum before Root reset"
        );
    }
    Ok(())
}

#[test]
fn s1_and_or_charge_both_branches_without_reading_boolean_values() -> Result<(), DomainError> {
    for ok in [false, true] {
        let stats = scan_api(&json!({"items": [{"ok": ok, "a": true}]}))?;
        for path in [
            "$.items[?(@.ok == true && @.a == true)]",
            "$.items[?(@.ok == true || @.a == true)]",
        ] {
            let bound = query_bound(path, &stats)?;
            // O=2,F=2: comparison 28 each, logic 4+28+28,
            // group 4, prefix 16, collection 12 + 2*64 => 156.
            assert_eq!(
                (bound.references, bound.work, bound.literal_bytes),
                (2, 156, 0)
            );
        }
    }
    Ok(())
}

#[test]
fn s1_query_comparables_charge_deep_volume_but_scalar_literals_do_not() -> Result<(), DomainError> {
    for (padding, units, deep_work) in [("", 14, 172), ("0123456789", 24, 212)] {
        let stats = scan_api(&json!({"items": [{"a": {}, "b": {}}], "z": padding}))?;
        assert_eq!(stats.units, units, "independent fixture size");
        let deep = query_bound("$.items[?(@.a == @.b)]", &stats)?;
        let scalar = query_bound("$.items[?(@.a == true)]", &stats)?;
        // O=2,F=2. Deep h=4+2*S; scalar h=4 regardless of unused z.
        assert_eq!(deep.work, deep_work, "both queries may select deep objects");
        assert_eq!(
            scalar.work, 92,
            "literal bool is not charged as a deep comparison"
        );
    }
    Ok(())
}

#[test]
fn s1_admission_returns_the_original_borrowed_response_for_every_owned_claim(
) -> Result<(), DomainError> {
    let mut declarations = claims("$.id");
    declarations.picture = Some(source("aux", "$.avatar"));
    let compiled = compile_mapping(&declarations)?;
    let mut input = responses(user());
    input.insert("aux".to_owned(), json!({"avatar": "image"}));
    let admission = admit(&compiled, &input)?;
    for claim in compiled.claims() {
        let original = input.get(claim.api()).ok_or_else(invalid_mapping)?;
        assert!(
            std::ptr::eq(admission.response_for(claim)?, original),
            "capability must borrow the exact original response, not clone or substitute it"
        );
    }
    Ok(())
}

#[test]
fn s1_admission_refuses_identical_claims_owned_by_another_mapping() -> Result<(), DomainError> {
    let compiled = compile_mapping(&claims("$.id"))?;
    let foreign = compiled.clone();
    let input = responses(user());
    let admission = admit(&compiled, &input)?;
    for claim in foreign.claims() {
        assert_refused(
            admission.response_for(claim),
            "equal declaration at a foreign address",
        );
    }
    Ok(())
}

#[test]
fn s1_parent_response_capability_accepts_only_the_reserved_email_claim() -> Result<(), DomainError>
{
    let mut declarations = claims("$.id");
    declarations.picture = Some(source("user", "$.p.a"));
    let compiled = compile_mapping(&declarations)?;
    let foreign = compiled.clone();
    let input = responses(user());
    let admission = admit(&compiled, &input)?;
    let original = input.get("user").ok_or_else(invalid_mapping)?;
    assert!(std::ptr::eq(
        admission.parent_response_for(&compiled.email)?,
        original
    ));
    for claim in compiled.claims() {
        if !std::ptr::eq(claim, std::ptr::from_ref(&compiled.email)) {
            assert_refused(
                admission.parent_response_for(claim),
                "non-email claim parent access",
            );
        }
    }
    assert_refused(
        admission.parent_response_for(&foreign.email),
        "foreign email parent access",
    );
    Ok(())
}

#[test]
fn s1_admission_is_not_created_for_missing_or_scan_excessive_responses() -> Result<(), DomainError>
{
    let compiled = compile_mapping(&claims("$.id"))?;
    assert_refused(
        admit(&compiled, &ApiResponses::new()),
        "missing referenced API",
    );
    // One bounded local allocation; no duplicate large response or evaluation.
    let oversized = responses(Value::String("x".repeat(1_048_576)));
    assert_refused(
        admit(&compiled, &oversized),
        "string bytes plus root node exceed scan limit",
    );
    let valid = responses(user());
    let admission = admit(&compiled, &valid)?;
    assert!(std::ptr::eq(
        admission.response_for(&compiled.sub)?,
        std::ptr::from_ref(&valid["user"])
    ));
    Ok(())
}

#[test]
fn s1_admission_is_not_created_for_work_or_cumulative_copy_excess() -> Result<(), DomainError> {
    let nested = compile_mapping(&claims(NESTED))?;
    let work_excess = responses(nested_user(32));
    assert_refused(admit(&nested, &work_excess), "nested work excess");
    let mut declarations = claims(NESTED);
    declarations.preferred_username = source("user", NESTED_USERNAME);
    declarations.picture = Some(source("user", "$.p.a"));
    let combined = compile_mapping(&declarations)?;
    let mut value = nested_user(4);
    value["items"][0]["u"] = json!("name");
    value["p"]["a"] = json!("avatar");
    let copy_excess = responses(value);
    assert_refused(
        admit(&combined, &copy_excess),
        "combined claims exceed shared copy bound",
    );
    Ok(())
}

#[test]
fn s1_mapping_estimate_sums_five_claims_and_one_parent_reserve_ignoring_extra_apis(
) -> Result<(), DomainError> {
    let mut declarations = claims("$.id");
    declarations.picture = Some(source("user", "$.p.a"));
    let compiled = compile_mapping(&declarations)?;
    let mut value = user();
    value["p"]["a"] = json!("avatar");
    let mut input = responses(value);
    // O=4: $.id costs 24, four two-Name paths cost 44 each,
    // parents reserve 324. C=524, P=21, L=0; U=8*524*(21+32).
    let expected = ProjectionBound {
        work: 524,
        max_path: 21,
        literal_bytes: 0,
        copy_bytes: 222_176,
    };
    assert_eq!(estimate_mapping(&compiled, &input)?, expected);
    let mut unused = Value::Null;
    for _ in 0..129 {
        unused = json!([unused]);
    }
    input.insert("unreferenced".to_owned(), unused);
    assert_eq!(
        estimate_mapping(&compiled, &input)?,
        expected,
        "unreferenced depth-129 API must not enter the scan or cumulative budget"
    );
    Ok(())
}
