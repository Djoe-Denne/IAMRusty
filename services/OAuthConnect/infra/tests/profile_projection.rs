//! T2 / G-2: independent examples from the accepted `OAuthConnect` contract §2.3.
//! No HTTP, parser reimplementation, production-derived oracle or runtime fixture.
//!
//! Obligations -> risks -> scenarios -> oracles:
//! - AST policy -> executable/unbounded mappings -> allowed selectors and exact
//!   bounds versus forbidden syntax -> compilation and exact projected ID/refusal.
//! - Typed claims -> identity corruption -> missing/null/wrong/multiple values ->
//!   exact five-field profile or refusal, never a first-match/coercion fallback.
//! - Email provenance -> false verification -> distinct/equal-valued objects,
//!   primary absence and ambiguous primaries -> localized pairing or refusal.
//! - Operation isolation -> stale identity -> reuse across disjoint responses,
//!   including a failure -> each exact profile, with no retained JSON fallback.
//! Every test below traces to G-2. Positive cases must fail against the T0 stub;
//! standalone refusals are only characterizations until positive cases pass.

use oauth_connect_infra::{
    compile_mapping, project_profile, ApiResponses, ClaimSource, DomainError, OAuthClaims,
    ProviderUserProfile,
};
use serde_json::{json, Value};

const PRIVATE_SUB: &str = "private-subject-g2-canary";
const PRIVATE_EMAIL: &str = "private-email-g2@example.invalid";
const PRIVATE_JSON: &str = "private-response-g2-canary";

fn source(api: &str, path: &str) -> ClaimSource {
    ClaimSource {
        api: api.to_owned(),
        json_path: path.to_owned(),
    }
}

fn claims() -> OAuthClaims {
    OAuthClaims {
        sub: source("user", "$.id"),
        email: source("user", "$.email"),
        email_verified: source("user", "$.verified"),
        preferred_username: source("user", "$.login"),
        picture: Some(source("user", "$.avatar_url")),
    }
}

fn github_claims() -> OAuthClaims {
    let mut mapping = claims();
    mapping.email = source("emails", "$[?(@.primary == true)].email");
    mapping.email_verified = source("emails", "$[?(@.primary == true)].verified");
    mapping
}

fn responses(user: Value) -> ApiResponses {
    ApiResponses::from([("user".to_owned(), user)])
}

fn github_responses(emails: Value) -> ApiResponses {
    ApiResponses::from([
        (
            "user".to_owned(),
            json!({"id": 42, "login": "octocat", "email": "public@example.invalid",
                "avatar_url": "https://images.example.invalid/octocat.png"}),
        ),
        ("emails".to_owned(), emails),
    ])
}

fn expected(
    id: &str,
    username: &str,
    email: Option<&str>,
    verified: bool,
    avatar: Option<&str>,
) -> ProviderUserProfile {
    ProviderUserProfile {
        id: id.to_owned(),
        username: username.to_owned(),
        email: email.map(str::to_owned),
        email_verified: verified,
        avatar_url: avatar.map(str::to_owned),
    }
}

fn assert_profile(actual: &ProviderUserProfile, wanted: &ProviderUserProfile) {
    assert_eq!(actual.id, wanted.id, "G-2: exact external identity");
    assert_eq!(
        actual.username, wanted.username,
        "G-2: descriptive username"
    );
    assert_eq!(actual.email, wanted.email, "G-2: selected email only");
    assert_eq!(
        actual.email_verified, wanted.email_verified,
        "G-2: verification belongs to the selected email"
    );
    assert_eq!(actual.avatar_url, wanted.avatar_url, "G-2: avatar string");
}

fn assert_redacted(error: &DomainError) {
    for diagnostic in [error.to_string(), format!("{error:?}")] {
        for sentinel in [PRIVATE_SUB, PRIVATE_EMAIL, PRIVATE_JSON] {
            assert!(
                !diagnostic.contains(sentinel),
                "G-2: diagnostic must not contain vendor values or response payload"
            );
        }
    }
}

fn assert_refused<T>(result: Result<T, DomainError>, scenario: &str) {
    match result {
        Ok(_) => panic!("G-2: expected refusal for {scenario}"),
        Err(error) => assert_redacted(&error),
    }
}

// KEEP: catches blanket rejection, dot-only parsing and first-index assumptions.
#[test]
fn g2_compiles_child_bracket_and_index_selectors() -> Result<(), DomainError> {
    for (path, user) in [
        ("$.id", json!({"id": "child"})),
        ("$['id']", json!({"id": "child"})),
        ("$.account.id", json!({"account": {"id": "child"}})),
        (
            "$.accounts[1].id",
            json!({"accounts": [{"id": "decoy"}, {"id": "child"}]}),
        ),
        (
            "$.accounts[-1].id",
            json!({"accounts": [{"id": "decoy"}, {"id": "child"}]}),
        ),
    ] {
        let mut mapping = claims();
        mapping.sub = source("user", path);
        let compiled = compile_mapping(&mapping)?;
        let profile = compiled.project(&responses(user))?;
        assert_profile(&profile, &expected("child", "", None, false, None));
    }
    Ok(())
}

// KEEP: scalar comparisons and logical composition are contractual, not extensions.
#[test]
fn g2_compiles_and_projects_scalar_logical_filters() -> Result<(), DomainError> {
    for (predicate, decoy_score, decoy_label, decoy_enabled) in [
        ("@.score == 7", 0, "selected", true),
        ("@.score != 0", 0, "selected", true),
        ("@.score > 6", 0, "selected", true),
        ("@.score >= 7", 6, "selected", true),
        ("@.score < 8", 8, "selected", true),
        ("@.score <= 7", 8, "selected", true),
        ("@.label == 'selected'", 7, "decoy", true),
        ("@.enabled == true && @.score == 7", 7, "selected", false),
        ("@.score == 7 || @.score == 99", 0, "selected", true),
        ("!(@.score == 0)", 0, "selected", true),
        (
            "(@.enabled == true && @.score >= 7) && @.label != 'decoy'",
            7,
            "decoy",
            true,
        ),
    ] {
        let mut mapping = claims();
        mapping.sub = source("user", &format!("$.items[?({predicate})].id"));
        let input = responses(json!({"items": [
            {"id": "must-not-select", "score": decoy_score,
                "label": decoy_label, "enabled": decoy_enabled},
            {"id": "filtered", "score": 7, "label": "selected", "enabled": true}
        ]}));
        let profile = compile_mapping(&mapping)?.project(&input)?;
        assert_profile(&profile, &expected("filtered", "", None, false, None));
    }
    Ok(())
}

fn long_selector(bytes: usize) -> (String, String) {
    // Bracket child selector has five ASCII bytes of syntax; Unicode makes the
    // byte-count boundary distinguishable from a character-count implementation.
    let field = format!("é{}", "x".repeat(bytes - 7));
    (format!("$['{field}']"), field)
}

#[test]
fn g2_accepts_a_valid_512_byte_expression() -> Result<(), DomainError> {
    let (path, field) = long_selector(512);
    assert_eq!(path.len(), 512, "fixture byte boundary");
    let mut mapping = claims();
    mapping.sub = source("user", &path);
    let mut user = json!({});
    user[&field] = json!("at-byte-limit");
    let profile = compile_mapping(&mapping)?.project(&responses(user))?;
    assert_profile(&profile, &expected("at-byte-limit", "", None, false, None));
    Ok(())
}

#[test]
fn g2_refuses_a_513_byte_expression_even_with_fewer_characters() {
    let (path, _) = long_selector(513);
    assert_eq!(path.len(), 513, "fixture byte boundary");
    assert!(path.chars().count() < 513, "fixture is not ASCII-only");
    let mut mapping = claims();
    mapping.sub = source("user", &path);
    assert_refused(compile_mapping(&mapping), "513-byte JSONPath");
}

fn nested_subject(segments: usize) -> (String, Value) {
    let mut value = json!("at-depth-limit");
    for _ in 0..segments {
        value = json!({"a": value});
    }
    (format!("${}", ".a".repeat(segments)), value)
}

#[test]
fn g2_accepts_16_child_segments() -> Result<(), DomainError> {
    let (path, user) = nested_subject(16);
    let mut mapping = claims();
    mapping.sub = source("user", &path);
    let profile = compile_mapping(&mapping)?.project(&responses(user))?;
    assert_profile(&profile, &expected("at-depth-limit", "", None, false, None));
    Ok(())
}

#[test]
fn g2_refuses_17_child_segments() {
    let (path, _) = nested_subject(17);
    let mut mapping = claims();
    mapping.sub = source("user", &path);
    assert_refused(compile_mapping(&mapping), "17 child segments");
}

#[test]
fn g2_filter_operand_cannot_hide_a_17_segment_query() {
    let mut mapping = claims();
    let path = format!("$.items[?(@{} == 1)].id", ".a".repeat(17));
    mapping.sub = source("user", &path);
    assert_refused(
        compile_mapping(&mapping),
        "17 child segments inside a filter operand",
    );
}

#[test]
fn g2_refuses_recursion_regex_functions_extensions_and_code() {
    for path in [
        "$..id",
        "$.items..id",
        "$.items[?(@.id =~ /private/)].id",
        "$.items[?(match(@.id, '.*'))].id",
        "$.items[?(length(@.id) > 0)].id",
        "$.items[?(count(@.id) == 1)].id",
        "$.items[?(@.id.size() > 0)].id",
        "$.items[(@.length-1)].id",
        "$.items[?(@.id == `${process.env.TOKEN}`)].id",
    ] {
        let mut mapping = claims();
        mapping.sub = source("user", path);
        assert_refused(compile_mapping(&mapping), path);
    }
}

#[test]
fn g2_refuses_malformed_jsonpath_before_projection() {
    for path in ["", "$.items[", "$[?(@.primary == )].id", "$.id trailing"] {
        let mut mapping = claims();
        mapping.sub = source("user", path);
        assert_refused(compile_mapping(&mapping), "malformed JSONPath");
    }
}

#[test]
fn g2_ast_policy_applies_to_every_configured_claim_not_only_subject() {
    for field in ["email", "verified", "login", "avatar_url"] {
        let mut mapping = claims();
        let forbidden = source("user", "$..private");
        match field {
            "email" => mapping.email = forbidden,
            "verified" => mapping.email_verified = forbidden,
            "login" => mapping.preferred_username = forbidden,
            _ => mapping.picture = Some(forbidden),
        }
        assert_refused(compile_mapping(&mapping), field);
    }
}

#[test]
fn g2_refuses_email_and_verified_from_different_apis_at_compile() {
    let mut mapping = claims();
    mapping.email_verified.api = "other-user".to_owned();
    assert_refused(compile_mapping(&mapping), "cross-API email verification");
}

#[test]
fn g2_refuses_structurally_disjoint_fixed_parents_at_compile() {
    for (email, verified) in [
        ("$.primary.email", "$.secondary.verified"),
        ("$.records[0].email", "$.records[1].verified"),
        ("$.email", "$.account.verified"),
    ] {
        let mut mapping = claims();
        mapping.email = source("user", email);
        mapping.email_verified = source("user", verified);
        assert_refused(compile_mapping(&mapping), "fixed disjoint email parents");
    }
}

#[test]
fn g2_github_primary_projection_ignores_order_and_public_email() -> Result<(), DomainError> {
    let input = github_responses(json!([
        {"email": "public@example.invalid", "primary": false, "verified": true},
        {"email": "primary@example.invalid", "primary": true, "verified": false}
    ]));
    let profile = project_profile(&github_claims(), &input)?;
    assert_profile(
        &profile,
        &expected(
            "42",
            "octocat",
            Some("primary@example.invalid"),
            false,
            Some("https://images.example.invalid/octocat.png"),
        ),
    );
    Ok(())
}

#[test]
fn g2_subject_strings_preserve_whitespace_leading_zeros_and_unicode() -> Result<(), DomainError> {
    for id in ["000042", " 000042 ", " ", "subject-λ"] {
        let profile = project_profile(&claims(), &responses(json!({"id": id})))?;
        assert_profile(&profile, &expected(id, "", None, false, None));
    }
    Ok(())
}

#[test]
fn g2_subject_unsigned_integers_have_exact_decimal_representation() -> Result<(), DomainError> {
    for (id, decimal) in [
        (0_u64, "0"),
        (42, "42"),
        (9_007_199_254_740_993, "9007199254740993"),
        (u64::MAX, "18446744073709551615"),
    ] {
        let profile = project_profile(&claims(), &responses(json!({"id": id})))?;
        assert_profile(&profile, &expected(decimal, "", None, false, None));
    }
    Ok(())
}

#[test]
fn g2_subject_missing_null_empty_noninteger_and_wrong_types_are_refused(
) -> Result<(), serde_json::Error> {
    for raw in [
        "{}",
        "{\"id\":null}",
        "{\"id\":\"\"}",
        "{\"id\":-1}",
        "{\"id\":1.5}",
        "{\"id\":1.0}",
        "{\"id\":18446744073709551616}",
        "{\"id\":true}",
        "{\"id\":{}}",
        "{\"id\":[]}",
    ] {
        let user: Value = serde_json::from_str(raw)?;
        assert_refused(project_profile(&claims(), &responses(user)), raw);
    }
    Ok(())
}

#[test]
fn g2_missing_optional_claims_produce_an_incomplete_exact_profile() -> Result<(), DomainError> {
    let profile = project_profile(&claims(), &responses(json!({"id": "only-id"})))?;
    assert_profile(&profile, &expected("only-id", "", None, false, None));
    Ok(())
}

#[test]
fn g2_null_optional_claims_are_absent_not_type_errors() -> Result<(), DomainError> {
    let profile = project_profile(
        &claims(),
        &responses(json!({
            "id": "null-optionals", "email": null, "verified": null,
            "login": null, "avatar_url": null
        })),
    )?;
    assert_profile(&profile, &expected("null-optionals", "", None, false, None));
    Ok(())
}

#[test]
fn g2_present_email_with_absent_null_or_false_verified_is_unverified() -> Result<(), DomainError> {
    for user in [
        json!({"id": "email-id", "email": "selected@example.invalid"}),
        json!({"id": "email-id", "email": "selected@example.invalid", "verified": null}),
        json!({"id": "email-id", "email": "selected@example.invalid", "verified": false}),
    ] {
        let profile = project_profile(&claims(), &responses(user))?;
        assert_profile(
            &profile,
            &expected(
                "email-id",
                "",
                Some("selected@example.invalid"),
                false,
                None,
            ),
        );
    }
    Ok(())
}

#[test]
fn g2_true_verified_without_email_is_always_false() -> Result<(), DomainError> {
    for user in [
        json!({"id": "no-email", "verified": true}),
        json!({"id": "no-email", "email": null, "verified": true}),
    ] {
        let profile = project_profile(&claims(), &responses(user))?;
        assert_profile(&profile, &expected("no-email", "", None, false, None));
    }
    Ok(())
}

#[test]
fn g2_username_absence_never_falls_back_to_email_or_subject() -> Result<(), DomainError> {
    let profile = project_profile(
        &claims(),
        &responses(json!({
            "id": "not-a-username", "email": "not-a-username@example.invalid", "verified": true
        })),
    )?;
    assert_profile(
        &profile,
        &expected(
            "not-a-username",
            "",
            Some("not-a-username@example.invalid"),
            true,
            None,
        ),
    );
    Ok(())
}

#[test]
fn g2_optional_strings_preserve_exact_content_and_picture_is_descriptive() -> Result<(), DomainError>
{
    let profile = project_profile(
        &claims(),
        &responses(json!({
            "id": "42", "email": " exact@example.invalid ", "verified": true,
            "login": " 000octocat ", "avatar_url": "https://unreachable.example.invalid/avatar?size=7"
        })),
    )?;
    assert_profile(
        &profile,
        &expected(
            "42",
            " 000octocat ",
            Some(" exact@example.invalid "),
            true,
            Some("https://unreachable.example.invalid/avatar?size=7"),
        ),
    );
    Ok(())
}

#[test]
fn g2_empty_descriptive_strings_are_not_subject_or_email_errors() -> Result<(), DomainError> {
    let profile = project_profile(
        &claims(),
        &responses(json!({
            "id": "42", "login": "", "avatar_url": ""
        })),
    )?;
    assert_profile(&profile, &expected("42", "", None, false, Some("")));
    Ok(())
}

#[test]
fn g2_optional_wrong_types_are_errors_not_absence_or_coercion() {
    for field in ["email", "login", "avatar_url"] {
        for value in [json!(true), json!(42), json!({}), json!([])] {
            let mut user = json!({"id": "typed-id", "email": "valid@example.invalid"});
            user[field] = value;
            assert_refused(project_profile(&claims(), &responses(user)), field);
        }
    }
    for value in [json!("true"), json!(1), json!({}), json!([])] {
        let user = json!({"id": "typed-id", "email": "valid@example.invalid", "verified": value});
        assert_refused(
            project_profile(&claims(), &responses(user)),
            "nonboolean verified",
        );
    }
}

#[test]
fn g2_empty_email_is_an_error_not_absence() {
    assert_refused(
        project_profile(&claims(), &responses(json!({"id": "42", "email": ""}))),
        "empty email",
    );
}

#[test]
fn g2_two_subject_results_are_refused_even_when_equal() {
    let mut mapping = claims();
    mapping.sub = source("user", "$.items[*].id");
    for ids in [
        json!([{"id": "same"}, {"id": "same"}]),
        json!([{"id": "one"}, {"id": "two"}]),
    ] {
        assert_refused(
            project_profile(&mapping, &responses(json!({"items": ids}))),
            "two subject matches",
        );
    }
}

#[test]
fn g2_duplicate_optional_results_are_refused_even_when_equal() {
    for field in ["email", "verified", "login", "avatar_url"] {
        let mut mapping = claims();
        match field {
            "email" => mapping.email = source("user", "$.items[*].email"),
            "verified" => mapping.email_verified = source("user", "$.items[*].verified"),
            "login" => mapping.preferred_username = source("user", "$.items[*].login"),
            _ => mapping.picture = Some(source("user", "$.items[*].avatar_url")),
        }
        // Keep the email pair structurally coherent, so its cardinality is the
        // only reason for failure, not a different fixed parent at compilation.
        if matches!(field, "email" | "verified") {
            mapping.email = source("user", "$.items[*].email");
            mapping.email_verified = source("user", "$.items[*].verified");
        }
        let mut first = json!({});
        first[field] = if field == "verified" {
            json!(true)
        } else {
            json!("same-value")
        };
        let user = json!({"id": "42", "items": [first.clone(), first]});
        assert_refused(project_profile(&mapping, &responses(user)), field);
    }
}

#[test]
fn g2_null_does_not_erase_duplicate_result_cardinality() {
    let mut mapping = claims();
    mapping.email = source("user", "$.items[*].email");
    mapping.email_verified = source("user", "$.items[*].verified");
    let input = responses(json!({"id": "42", "items": [
        {"email": null, "verified": null}, {"email": null, "verified": null}
    ]}));
    assert_refused(
        project_profile(&mapping, &input),
        "two null results are still two matches",
    );
}

#[test]
fn g2_zero_subject_matches_are_refused_without_fallback() {
    let mut mapping = claims();
    mapping.sub = source("user", "$.items[?(@.selected == true)].id");
    assert_refused(
        project_profile(
            &mapping,
            &responses(json!({
                "id": "do-not-fallback", "email": "not-an-id@example.invalid",
                "items": [{"selected": false, "id": "unselected"}]
            })),
        ),
        "zero selected subjects",
    );
}

#[test]
fn g2_no_primary_email_does_not_fall_back_to_public_or_secondary() -> Result<(), DomainError> {
    for emails in [
        json!([]),
        json!([
            {"primary": false, "email": "public@example.invalid", "verified": true}
        ]),
    ] {
        let profile = project_profile(&github_claims(), &github_responses(emails))?;
        assert_profile(
            &profile,
            &expected(
                "42",
                "octocat",
                None,
                false,
                Some("https://images.example.invalid/octocat.png"),
            ),
        );
    }
    Ok(())
}

#[test]
fn g2_two_primary_emails_are_refused_even_when_identical() {
    for second in ["primary@example.invalid", "other@example.invalid"] {
        let input = github_responses(json!([
            {"primary": true, "email": "primary@example.invalid", "verified": true},
            {"primary": true, "email": second, "verified": true}
        ]));
        assert_refused(
            project_profile(&github_claims(), &input),
            "two primary emails",
        );
    }
}

#[test]
fn g2_malformed_primary_claims_do_not_fall_back_to_public_email() {
    for record in [
        json!({"primary": true, "email": 42, "verified": true}),
        json!({"primary": true, "email": "", "verified": true}),
        json!({"primary": true, "email": "primary@example.invalid", "verified": "true"}),
    ] {
        assert_refused(
            project_profile(&github_claims(), &github_responses(json!([record]))),
            "maltyped primary record",
        );
    }
}

#[test]
fn g2_same_parent_can_be_selected_by_distinct_runtime_filters() -> Result<(), DomainError> {
    // Assumption: different predicates are not structurally disjoint by
    // themselves. This permits runtime localized-path correlation, not a DSL
    // restriction requiring textually identical selectors.
    let mut mapping = github_claims();
    mapping.email_verified = source("emails", "$[?(@.verified == true)].verified");
    let compiled = compile_mapping(&mapping)?;
    let profile = compiled.project(&github_responses(json!([
        {"primary": false, "email": "other@example.invalid", "verified": false},
        {"primary": true, "email": "selected@example.invalid", "verified": true}
    ])))?;
    assert_profile(
        &profile,
        &expected(
            "42",
            "octocat",
            Some("selected@example.invalid"),
            true,
            Some("https://images.example.invalid/octocat.png"),
        ),
    );
    Ok(())
}

#[test]
fn g2_equal_email_values_in_distinct_selected_objects_do_not_prove_verification(
) -> Result<(), DomainError> {
    let mut mapping = github_claims();
    mapping.email_verified = source("emails", "$[?(@.verified == true)].verified");
    let compiled = compile_mapping(&mapping)?;
    let input = github_responses(json!([
        {"primary": true, "email": "same@example.invalid", "verified": false},
        {"primary": false, "email": "same@example.invalid", "verified": true}
    ]));
    assert_refused(
        compiled.project(&input),
        "equal values but different localized parents",
    );
    Ok(())
}

#[test]
fn g2_identical_wildcard_parents_can_still_mismatch_at_runtime() -> Result<(), DomainError> {
    let mut mapping = claims();
    mapping.email = source("user", "$.items[*].email");
    mapping.email_verified = source("user", "$.items[*].verified");
    let compiled = compile_mapping(&mapping)?;
    assert_refused(
        compiled.project(&responses(json!({"id": "42", "items": [
            {"email": "unverified@example.invalid"}, {"verified": true}
        ]}))),
        "same selector shape, different result parents",
    );
    Ok(())
}

#[test]
fn g2_a_matched_email_may_have_no_verified_result() -> Result<(), DomainError> {
    let profile = project_profile(
        &github_claims(),
        &github_responses(json!([
            {"primary": true, "email": "primary@example.invalid"}
        ])),
    )?;
    assert_profile(
        &profile,
        &expected(
            "42",
            "octocat",
            Some("primary@example.invalid"),
            false,
            Some("https://images.example.invalid/octocat.png"),
        ),
    );
    Ok(())
}

#[test]
fn g2_missing_referenced_response_is_not_an_absent_optional_claim() -> Result<(), DomainError> {
    let mapping = github_claims();
    let compiled = compile_mapping(&mapping)?;
    for missing in ["user", "emails"] {
        let mut input = github_responses(json!([]));
        input.remove(missing);
        assert_refused(compiled.project(&input), missing);
    }
    let mut picture_mapping = claims();
    picture_mapping.picture = Some(source("pictures", "$.avatar"));
    assert_refused(
        compile_mapping(&picture_mapping)?.project(&responses(json!({"id": "42"}))),
        "missing API used only by an optional mapping",
    );
    Ok(())
}

#[test]
fn g2_compiled_mapping_reuse_never_reuses_previous_json_or_accumulates_results(
) -> Result<(), DomainError> {
    let compiled = compile_mapping(&claims())?;
    let first = compiled.project(&responses(json!({
        "id": "first", "email": "first@example.invalid", "verified": true,
        "login": "first-login", "avatar_url": "https://images.example.invalid/first"
    })))?;
    assert_profile(
        &first,
        &expected(
            "first",
            "first-login",
            Some("first@example.invalid"),
            true,
            Some("https://images.example.invalid/first"),
        ),
    );
    assert_refused(
        compiled.project(&ApiResponses::new()),
        "missing second response",
    );
    let second = compiled.project(&responses(json!({"id": "second"})))?;
    assert_profile(&second, &expected("second", "", None, false, None));
    let third = compiled.project(&responses(json!({"id": 0, "verified": true})))?;
    assert_profile(&third, &expected("0", "", None, false, None));
    Ok(())
}

#[test]
fn g2_no_picture_mapping_ignores_unselected_response_fields() -> Result<(), DomainError> {
    let mut mapping = claims();
    mapping.picture = None;
    let profile = project_profile(
        &mapping,
        &responses(json!({"id": "42", "avatar_url": {"bad": true}})),
    )?;
    assert_profile(&profile, &expected("42", "", None, false, None));
    Ok(())
}

#[test]
fn g2_runtime_errors_redact_subject_email_and_vendor_payload() -> Result<(), DomainError> {
    let compiled = compile_mapping(&claims())?;
    for user in [
        json!({"id": PRIVATE_SUB, "email": {"value": PRIVATE_EMAIL}, "payload": PRIVATE_JSON}),
        json!({"id": {"value": PRIVATE_SUB}, "email": PRIVATE_EMAIL, "payload": PRIVATE_JSON}),
        json!({"id": PRIVATE_SUB, "email": PRIVATE_EMAIL, "verified": PRIVATE_JSON}),
    ] {
        assert_refused(
            compiled.project(&responses(user)),
            "sensitive invalid vendor response",
        );
    }
    Ok(())
}

#[test]
fn g2_compilation_errors_do_not_echo_sensitive_expression_literals() {
    let mut mapping = claims();
    mapping.sub = source(
        "user",
        &format!("$[?(@.id == '{PRIVATE_SUB}' && @.email == '{PRIVATE_EMAIL}'"),
    );
    assert_refused(
        compile_mapping(&mapping),
        "invalid expression containing private literals",
    );
}

// Post-GREEN characterization, not part of the original 41-case TDD proof.
// KEEP: removing layout must not rename a quoted Unicode member.
#[test]
fn g2_quoted_unicode_child_preserves_spaces_in_the_member_name() -> Result<(), DomainError> {
    let mut mapping = claims();
    mapping.sub = source("user", "$['clé avec espaces']");
    let input = responses(json!({
        "clé avec espaces": " 000quoted ",
        "cléavecespaces": "wrong-member"
    }));
    let profile = compile_mapping(&mapping)?.project(&input)?;
    assert_profile(&profile, &expected(" 000quoted ", "", None, false, None));
    Ok(())
}

// Post-GREEN characterization. RFC strings allow escaped delimiter quotes and
// backslashes (pin 1.0.12 grammar string/escapable and unquote_and_unescape).
// KEEP: stripping literal spaces, trimming, or mishandling an escaped quote
// selects a decoy or loses the sole result instead of preserving the literal.
#[test]
fn g2_filter_string_literals_preserve_spaces_escaped_quotes_and_backslashes(
) -> Result<(), DomainError> {
    for path in [
        r#"$.items[?(@.label == ' l\'adresse C:\\ dossier "privé" ')].id"#,
        r#"$.items[?(@.label == " l'adresse C:\\ dossier \"privé\" ")].id"#,
    ] {
        let mut mapping = claims();
        mapping.sub = source("user", path);
        let input = responses(json!({"items": [
            {"id": "spaces-removed", "label": r#"l'adresseC:\dossier"privé""#},
            {"id": "trimmed-literal", "label": r#"l'adresse C:\ dossier "privé""#},
            {"id": "lost-backslash", "label": r#" l'adresse C: dossier "privé" "#},
            {"id": "exact-literal", "label": r#" l'adresse C:\ dossier "privé" "#}
        ]}));
        let profile = compile_mapping(&mapping)?.project(&input)?;
        assert_profile(&profile, &expected("exact-literal", "", None, false, None));
    }
    Ok(())
}

// Post-GREEN characterization. A literal member containing punctuation is not
// the similarly spelled sequence of child/index selectors in the same response.
// KEEP: protects both successful localized pairing and fixed-parent separation.
#[test]
fn g2_quoted_email_parent_is_not_confused_with_structural_separators() -> Result<(), DomainError> {
    let mut mapping = claims();
    mapping.email = source("user", "$['contact.info[0].details with space'].email");
    mapping.email_verified = source("user", "$['contact.info[0].details with space'].verified");
    let input = responses(json!({
        "id": "quoted-parent",
        "contact.info[0].details with space": {
            "email": "selected@example.invalid", "verified": true
        },
        "contact": {"info": [{"details with space": {
            "email": "decoy@example.invalid", "verified": false
        }}]}
    }));
    let profile = compile_mapping(&mapping)?.project(&input)?;
    assert_profile(
        &profile,
        &expected(
            "quoted-parent",
            "",
            Some("selected@example.invalid"),
            true,
            None,
        ),
    );
    mapping.email_verified = source("user", "$.contact.info[0]['details with space'].verified");
    assert_refused(
        compile_mapping(&mapping),
        "literal quoted member and distinct structural parent",
    );
    Ok(())
}
