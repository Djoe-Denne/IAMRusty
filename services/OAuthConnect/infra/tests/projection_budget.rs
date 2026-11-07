//! G-2 / S-1: pre-evaluation budget contract, `oauthconnect-jsonpath-budget.md` §5.
//!
//! Obligations -> risks -> scenarios -> oracles:
//! - Bound repeated traversal -> vendor amplification -> identical valid absolute
//!   query on 2/4/32 items with one logical ID -> exact profile / budget refusal.
//! - Bound borrowed input -> unbounded scan -> exact UTF-8 volume and depth edges,
//!   referenced versus extra APIs -> exact profile or redacted refusal.
//! - Conservative, cumulative admission -> blanket rejection or per-claim reset
//!   -> small selectors/comparables and individually admissible costly claims ->
//!   exact profiles individually, refusal when combined, not a cardinality error.
//!
//! These tests do not measure time or independently prove that refusal precedes
//! evaluation. Static control-flow review and private boundary/overflow witnesses
//! remain required after the private budget API exists. Original 44 tests stay
//! unchanged. Every scenario below is KEEP; no production estimator is copied.

use oauth_connect_infra::{
    compile_mapping, ApiResponses, ClaimSource, DomainError, OAuthClaims, ProviderUserProfile,
};
use serde_json::{json, Value};

const SCAN_LIMIT: usize = 1_048_576;
const NESTED_ABSOLUTE: &str = "$.items[?($.items[?($.items[?($.items[?(@.ok == true)])])])].id";
const NESTED_USERNAME: &str = "$.items[?($.items[?($.items[?($.items[?(@.ok == true)])])])].u";

fn source(api: &str, path: &str) -> ClaimSource {
    ClaimSource {
        api: api.to_owned(),
        json_path: path.to_owned(),
    }
}

fn mapping(sub: &str) -> OAuthClaims {
    OAuthClaims {
        sub: source("user", sub),
        email: source("user", "$.p.e"),
        email_verified: source("user", "$.p.v"),
        preferred_username: source("user", "$.p.u"),
        picture: None,
    }
}

fn base_user() -> Value {
    json!({"id": "stable", "p": {"e": "one@example.test", "v": true, "u": "name"}})
}

fn responses(user: Value) -> ApiResponses {
    ApiResponses::from([("user".to_owned(), user)])
}

fn nested_user(count: usize, ok: bool) -> Value {
    // Exactly the annex fixture: root items/p; only item zero has an ID.
    // K=5, O=3, F=max(3,count). No extra metadata inflates these statistics.
    let items: Vec<Value> = (0..count)
        .map(|index| {
            if index == 0 {
                json!({"ok": ok, "id": "stable"})
            } else {
                json!({"ok": ok})
            }
        })
        .collect();
    json!({"items": items, "p": {"e": "one@example.test", "v": true, "u": "name"}})
}

fn nested_input(count: usize, ok: bool) -> ApiResponses {
    responses(nested_user(count, ok))
}

fn assert_profile(profile: &ProviderUserProfile, avatar: Option<&str>) {
    assert_eq!(profile.id, "stable", "S-1: exact singleton external ID");
    assert_eq!(profile.username, "name", "S-1: exact descriptive username");
    assert_eq!(
        profile.email.as_deref(),
        Some("one@example.test"),
        "S-1: exact email"
    );
    assert!(
        profile.email_verified,
        "S-1: selected email remains verified"
    );
    assert_eq!(
        profile.avatar_url.as_deref(),
        avatar,
        "S-1: exact optional avatar"
    );
}

fn assert_refused(result: Result<ProviderUserProfile, DomainError>, scenario: &str) {
    match result {
        Ok(_) => panic!("S-1: expected refusal for {scenario}"),
        Err(error) => {
            for diagnostic in [error.to_string(), format!("{error:?}")] {
                for private in ["stable", "one@example.test", NESTED_ABSOLUTE, "\"items\""] {
                    assert!(
                        !diagnostic.contains(private),
                        "S-1: refusal must not echo vendor values, JSON or configured expression"
                    );
                }
            }
        }
    }
}

// Characterization: rejects a blanket ban on absolute queries. Annex §5 gives
// C_total=13_720 and U_total=12_841_920 at four items, below both ceilings.
#[test]
fn s1_nested_absolute_queries_accept_two_and_four_items() -> Result<(), DomainError> {
    let compiled = compile_mapping(&mapping(NESTED_ABSOLUTE))?;
    for count in [2, 4] {
        let profile = compiled.project(&nested_input(count, true))?;
        assert_profile(&profile, None);
    }
    Ok(())
}

// RED expected before mitigation: compilation succeeds and the unguarded engine
// would return one ID. 32 is a bounded fixture, not a timing or stress assertion.
#[test]
fn s1_nested_absolute_queries_refuse_32_items_with_one_logical_id() -> Result<(), DomainError> {
    let compiled = compile_mapping(&mapping(NESTED_ABSOLUTE))?;
    assert_refused(
        compiled.project(&nested_input(32, true)),
        "four collection traversals at 32 items",
    );
    Ok(())
}

fn utf8_padding(bytes: usize) -> String {
    let mut text = "é".repeat(bytes / 2);
    if bytes % 2 == 1 {
        text.push('x');
    }
    text
}

fn volume_user(units: usize, pad_key: bool) -> Value {
    let mut user = base_user();
    if pad_key {
        // Seven values + six ordinary key bytes + 26 value-string bytes.
        // The remaining bytes are one unselected UTF-8 key with a null value.
        let key = utf8_padding(units - 39);
        user[&key] = Value::Null;
    } else {
        // Seven values + seven key bytes (including z) + 26 fixed string bytes.
        user["z"] = Value::String(utf8_padding(units - 40));
    }
    user
}

// Characterization: equality is admitted; this is logical Value volume, not
// serialized JSON size. A large unused key must not inflate a Name-only path.
#[test]
fn s1_scan_accepts_exact_units_for_utf8_keys_and_strings() -> Result<(), DomainError> {
    let compiled = compile_mapping(&mapping("$.id"))?;
    for pad_key in [false, true] {
        let profile = compiled.project(&responses(volume_user(SCAN_LIMIT, pad_key)))?;
        assert_profile(&profile, None);
    }
    Ok(())
}

// RED expected: no selected field changes, so type/cardinality cannot cause the
// refusal. Reusing the same capability must not reuse a previous scan summary.
#[test]
fn s1_scan_refuses_one_unit_excess_for_utf8_keys_and_strings() -> Result<(), DomainError> {
    let compiled = compile_mapping(&mapping("$.id"))?;
    let before = compiled.project(&responses(base_user()))?;
    assert_profile(&before, None);
    for pad_key in [false, true] {
        assert_refused(
            compiled.project(&responses(volume_user(SCAN_LIMIT + 1, pad_key))),
            "per-API logical volume exceeds the ceiling by one",
        );
    }
    let after = compiled.project(&responses(base_user()))?;
    assert_profile(&after, None);
    Ok(())
}

fn user_with_unused_depth(depth: usize) -> Value {
    let mut unused = Value::Null;
    for _ in 1..depth {
        unused = json!([unused]);
    }
    let mut user = base_user();
    user["z"] = unused;
    user
}

// The root is depth zero. Only an UNUSED branch reaches the depth edge: the
// pre-budget RED does not ask the query evaluator to traverse deeply nested data.
#[test]
fn s1_scan_accepts_depth_128_and_refuses_depth_129() -> Result<(), DomainError> {
    let compiled = compile_mapping(&mapping("$.id"))?;
    let profile = compiled.project(&responses(user_with_unused_depth(128)))?;
    assert_profile(&profile, None);
    assert_refused(
        compiled.project(&responses(user_with_unused_depth(129))),
        "unused branch at depth 129",
    );
    Ok(())
}

// Refuse only referenced oversized APIs, independently per API, not a combined
// volume cap. Inputs are borrowed by the same real projection API throughout.
#[test]
fn s1_scan_ignores_extra_apis_and_checks_each_referenced_response() -> Result<(), DomainError> {
    let claims = mapping("$.id");
    let compiled = compile_mapping(&claims)?;
    let mut input = responses(base_user());
    input.insert("aux".to_owned(), json!({"z": utf8_padding(SCAN_LIMIT + 1)}));
    let profile = compiled.project(&input)?;
    assert_profile(&profile, None);

    let mut with_picture = claims;
    with_picture.picture = Some(source("aux", "$.avatar"));
    let checks_both = compile_mapping(&with_picture)?;
    // Absent optional avatar is otherwise valid, so no malformed-data refusal.
    assert_refused(
        checks_both.project(&input),
        "oversized API referenced only by picture",
    );
    let unchanged = compiled.project(&input)?;
    assert_profile(&unchanged, None);

    let two_at_limit = ApiResponses::from([
        ("user".to_owned(), volume_user(SCAN_LIMIT, true)),
        ("aux".to_owned(), volume_user(SCAN_LIMIT, false)),
    ]);
    let profile = checks_both.project(&two_at_limit)?;
    assert_profile(&profile, None);
    Ok(())
}

// Characterization: 16 Name/Index steps are linear even when F is set by a
// separate 128-element array. The literal sub remains exactly one string.
#[test]
fn s1_sixteen_names_and_indices_ignore_unused_array_width() -> Result<(), DomainError> {
    let mut user = json!("stable");
    for _ in 0..8 {
        user = json!({"a": [user]});
    }
    user["p"] = json!({"e": "one@example.test", "v": true, "u": "name"});
    user["z"] = json!(vec![Value::Null; 128]);
    let claims = mapping(&format!("${}", ".a[0]".repeat(8)));
    let profile = compile_mapping(&claims)?.project(&responses(user))?;
    assert_profile(&profile, None);
    Ok(())
}

// Characterization, not a public collection-size quota. Normal primary-only
// scalar comparisons stay admitted, for both values of the selected bool.
#[test]
fn s1_github_32_emails_with_one_primary_remain_admitted() -> Result<(), DomainError> {
    let mut claims = mapping("$.id");
    claims.email = source("emails", "$[?(@.primary == true)].email");
    claims.email_verified = source("emails", "$[?(@.primary == true)].verified");
    let compiled = compile_mapping(&claims)?;
    for verified in [false, true] {
        let emails: Vec<Value> = (0..32)
            .map(|index| {
                json!({
                    "email": "one@example.test", "primary": index == 17, "verified": verified
                })
            })
            .collect();
        let mut input = responses(base_user());
        input.insert("emails".to_owned(), Value::Array(emails));
        let profile = compiled.project(&input)?;
        assert_eq!(profile.id, "stable", "S-1: GitHub ID remains exact");
        assert_eq!(profile.username, "name", "S-1: username unchanged");
        assert_eq!(
            profile.email.as_deref(),
            Some("one@example.test"),
            "S-1: primary email"
        );
        assert_eq!(
            profile.email_verified, verified,
            "S-1: selected bool unchanged"
        );
        assert_eq!(profile.avatar_url, None, "S-1: no extra avatar mapping");
    }
    Ok(())
}

// Small admissible families must not become forbidden to satisfy the budget.
// Duplicate union occurrences still fail cardinality: this last assertion is
// historical functionality, NOT evidence that budget admission ran first.
#[test]
fn s1_small_unions_expansions_and_relative_logical_queries_keep_semantics(
) -> Result<(), DomainError> {
    let input = nested_input(2, true);
    for path in [
        "$.items[*].id",
        "$.items[0:2].id",
        "$.items[0,1].id",
        "$.items[?(@.ok)].id",
        "$.items[?($.p)].id",
        "$.items[?((@.ok == true && $.p.v == true) || @.ok == false)].id",
        "$.items[?(!(@.ok == false))].id",
    ] {
        let profile = compile_mapping(&mapping(path))?.project(&input)?;
        assert_profile(&profile, None);
    }
    let duplicate = compile_mapping(&mapping("$.items[0,0].id"))?;
    assert_refused(
        duplicate.project(&input),
        "union preserves two equal occurrences",
    );
    Ok(())
}

// Query/query equality may compare objects deeply; tiny objects remain admitted.
// Unlike a scalar literal comparison, its cost must later account for API S.
// The escaped quote/backslash member also contributes an inner query path.
#[test]
fn s1_small_relative_object_comparables_remain_admitted() -> Result<(), DomainError> {
    let mut user = base_user();
    user["items"] = json!([
        {"id": "stable", "l'\\hs": {"a": [1, 2]}, "rhs": {"a": [1, 2]}},
        {"l'\\hs": {"a": [1, 2]}, "rhs": {"a": [1, 3]}}
    ]);
    let claims = mapping(r"$.items[?(@['l\'\\hs'] == @.rhs)].id");
    let profile = compile_mapping(&claims)?.project(&responses(user))?;
    assert_profile(&profile, None);
    Ok(())
}

// Each costly claim separately stays under the annex work/copy bounds; their
// combination exceeds the shared copy allowance. All five claims and the parent
// check belong to one admission. This does NOT isolate the parent reserve alone:
// its exact-ceiling witness awaits a private API, not a public tuning parameter.
#[test]
fn s1_individually_admissible_claims_share_one_projection_budget() -> Result<(), DomainError> {
    let mut user = nested_user(4, true);
    user["items"][0]["u"] = json!("name");
    user["p"]["a"] = json!("avatar");
    let input = responses(user);
    let mut baseline = mapping("$.items[0].id");
    baseline.picture = Some(source("user", "$.p.a"));
    let mut expensive_sub = baseline.clone();
    expensive_sub.sub = source("user", NESTED_ABSOLUTE);
    let mut expensive_username = baseline;
    expensive_username.preferred_username = source("user", NESTED_USERNAME);
    for claims in [&expensive_sub, &expensive_username] {
        let profile = compile_mapping(claims)?.project(&input)?;
        assert_profile(&profile, Some("avatar"));
    }
    expensive_sub.preferred_username = source("user", NESTED_USERNAME);
    assert_refused(
        compile_mapping(&expensive_sub)?.project(&input),
        "two separately admissible costly claims combined with three other claims",
    );
    Ok(())
}

// Refusal must not depend on reading actual predicate truth/early existence.
// With ok=false, the unguarded optional username is simply absent: all mandatory
// identity fields remain valid, so missing-sub/cardinality cannot mask the gap.
#[test]
fn s1_false_predicates_do_not_discount_the_structural_budget() -> Result<(), DomainError> {
    let mut claims = mapping("$.items[0].id");
    claims.preferred_username = source("user", NESTED_ABSOLUTE);
    let compiled = compile_mapping(&claims)?;
    for ok in [false, true] {
        assert_refused(
            compiled.project(&nested_input(32, ok)),
            "oversized repeated traversal regardless of actual predicate truth",
        );
    }
    Ok(())
}
