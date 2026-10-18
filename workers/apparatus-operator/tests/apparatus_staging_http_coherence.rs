//! PH-1 / C-1: validate the three HTTP staging producers without executing them.
//!
//! The oracle is workspace inheritance from the root and reference-kv manifests,
//! not a copy of a producer. Only the selected single-line TOML declarations are
//! decoded; unsupported syntax fails loudly rather than silently losing coverage.

use std::collections::{BTreeMap, BTreeSet};

const ROOT: &str = include_str!("../../../Cargo.toml");
const REFERENCE: &str = include_str!("../../../crates/apparatus-reference-kv/Cargo.toml");
const WORKSPACE_DEPS: &str = "[workspace.dependencies]";
const PRODUCERS: [(&str, &str); 3] = [
    ("chain.rs", include_str!("fixtures/chain.rs")),
    (
        "apparatus_m5_invoke_isolated.rs",
        include_str!("apparatus_m5_invoke_isolated.rs"),
    ),
    (
        "schedule-reference-kv.ps1",
        include_str!(
            "../../../ops/deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1"
        ),
    ),
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Dependency {
    options: BTreeMap<String, String>,
    features: BTreeSet<String>,
}

struct Requirement {
    dependency: Dependency,
    verbatim: Option<String>,
}

fn entries<'a>(source: &'a str, header: &str) -> Result<BTreeMap<&'a str, &'a str>, String> {
    let mut lines = source.lines().skip_while(|line| line.trim() != header);
    if lines.next().is_none() {
        return Err(format!("missing table {header}"));
    }
    let mut result = BTreeMap::new();
    for line in lines.take_while(|line| !line.trim_start().starts_with('[')) {
        if let Some((name, value)) = line.split_once('=') {
            let name = name.trim();
            if !name.starts_with('#') && result.insert(name, value.trim()).is_some() {
                return Err(format!("duplicate {header} dependency {name}"));
            }
        }
    }
    Ok(result)
}

fn quoted(value: &str) -> Result<String, String> {
    value
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .filter(|value| !value.contains(['"', '\\']))
        .map(str::to_owned)
        .ok_or_else(|| "unsupported string declaration".to_owned())
}

fn features(value: &str) -> Result<BTreeSet<String>, String> {
    let array = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| "unsupported feature array".to_owned())?;
    array
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(quoted)
        .collect()
}

fn inline_fields(value: &str) -> Result<Vec<&str>, String> {
    let body = value
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| "unsupported dependency declaration".to_owned())?;
    let mut fields = Vec::new();
    let mut in_string = false;
    let mut in_array = false;
    let mut start = 0;
    for (index, ch) in body.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '[' if !in_string => in_array = true,
            ']' if !in_string => in_array = false,
            ',' if !in_string && !in_array => {
                fields.push(&body[start..index]);
                start = index + 1;
            }
            '\\' | '{' | '}' => return Err("unsupported inline declaration".to_owned()),
            _ => {}
        }
    }
    if in_string || in_array {
        return Err("unclosed inline declaration".to_owned());
    }
    fields.push(&body[start..]);
    Ok(fields)
}

fn dependency(value: &str) -> Result<Dependency, String> {
    let mut options = BTreeMap::new();
    let mut enabled = BTreeSet::new();
    if value.starts_with('"') {
        quoted(value)?;
        options.insert("version".to_owned(), value.to_owned());
    } else {
        for field in inline_fields(value)? {
            let (key, value) = field
                .trim()
                .split_once('=')
                .ok_or_else(|| "unsupported inline field".to_owned())?;
            let key = key.trim();
            let value = value.trim();
            if key == "features" {
                enabled = features(value)?;
            } else if options.insert(key.to_owned(), value.to_owned()).is_some() {
                return Err(format!("duplicate option {key}"));
            }
        }
    }
    Ok(Dependency {
        options,
        features: enabled,
    })
}

fn requirements() -> Result<BTreeMap<String, Requirement>, String> {
    let root = entries(ROOT, WORKSPACE_DEPS)?;
    let feature_table = entries(REFERENCE, "[features]")?;
    let http = features(feature_table.get("http").ok_or("missing http feature")?)?;
    if http.iter().any(|feature| !feature.starts_with("dep:")) {
        return Err("http feature expansion requires an explicit oracle update".to_owned());
    }
    let mut required = BTreeMap::new();
    for (name, declaration) in entries(REFERENCE, "[dependencies]")? {
        let mut inherited = dependency(declaration)?;
        if inherited.options.remove("workspace").as_deref() != Some("true") {
            continue;
        }
        let optional = inherited.options.remove("optional").as_deref() == Some("true");
        if optional && !http.contains(&format!("dep:{name}")) {
            continue;
        }
        if !inherited.options.is_empty() {
            return Err(format!("unsupported reference-kv override for {name}"));
        }
        let value = root
            .get(name)
            .ok_or_else(|| format!("root missing inherited dependency {name}"))?;
        let mut expected = dependency(value)?;
        expected.features.extend(inherited.features);
        required.insert(
            name.to_owned(),
            Requirement {
                dependency: expected,
                verbatim: matches!(name, "reqwest" | "rcgen").then(|| (*value).to_owned()),
            },
        );
    }
    if !required.contains_key("reqwest") || !required.contains_key("rcgen") {
        return Err("http inheritance must include reqwest and rcgen".to_owned());
    }
    Ok(required)
}

fn validate(
    producer: &str,
    source: &str,
    required: &BTreeMap<String, Requirement>,
) -> Result<(), String> {
    let staged = entries(source, WORKSPACE_DEPS)?;
    for (name, requirement) in required {
        let value = staged
            .get(name.as_str())
            .ok_or_else(|| format!("{producer}: missing inherited dependency {name}"))?;
        let actual = dependency(value).map_err(|error| format!("{producer}: {name}: {error}"))?;
        if actual != requirement.dependency {
            return Err(format!(
                "{producer}: inherited definition differs for {name}"
            ));
        }
        if requirement
            .verbatim
            .as_deref()
            .is_some_and(|root| root != *value)
        {
            return Err(format!(
                "{producer}: root verbatim definition differs for {name}"
            ));
        }
    }
    Ok(())
}

fn rewrite(source: &str, name: &str, replacement: Option<&str>) -> String {
    let mut in_table = false;
    let mut changed = 0;
    let mut output = Vec::new();
    for line in source.lines() {
        if line.trim_start().starts_with('[') {
            in_table = line.trim() == WORKSPACE_DEPS;
        }
        if in_table
            && line
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == name)
        {
            changed += 1;
            if let Some(value) = replacement {
                output.push(format!("{name} = {value}"));
            }
        } else {
            output.push(line.to_owned());
        }
    }
    assert_eq!(changed, 1, "mutation must change exactly one {name} entry");
    output.join("\n")
}

fn render(value: &Dependency) -> String {
    let mut fields: Vec<String> = value
        .options
        .iter()
        .map(|(key, value)| format!("{key} = {value}"))
        .collect();
    if !value.features.is_empty() {
        let enabled: Vec<String> = value
            .features
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect();
        fields.push(format!("features = [{}]", enabled.join(", ")));
    }
    format!("{{ {} }}", fields.join(", "))
}

fn assert_definition_rejected(
    producer: &str,
    source: &str,
    name: &str,
    mutation: &Dependency,
    required: &BTreeMap<String, Requirement>,
) {
    let mutated = rewrite(source, name, Some(&render(mutation)));
    assert_eq!(
        validate(producer, &mutated, required),
        Err(format!(
            "{producer}: inherited definition differs for {name}"
        )),
        "PH-1: mutated {name} definition must be rejected by the staging validator"
    );
}

#[test]
fn ph1_http_stagings_match_root_and_reference_kv_inheritance() -> Result<(), String> {
    let required = requirements()?;
    for name in [
        "axum",
        "rcgen",
        "reqwest",
        "serde_json",
        "thiserror",
        "tokio",
        "uuid",
    ] {
        assert!(
            required.contains_key(name),
            "PH-1: oracle lost known inherited dependency {name}"
        );
    }
    for (producer, source) in PRODUCERS {
        assert_eq!(
            validate(producer, source, &required),
            Ok(()),
            "PH-1: {producer} must satisfy HTTP inheritance"
        );
    }
    Ok(())
}

#[test]
fn ph1_each_inherited_dependency_missing_from_each_producer_is_rejected() -> Result<(), String> {
    let required = requirements()?;
    for (producer, source) in PRODUCERS {
        for name in required.keys() {
            let mutated = rewrite(source, name, None);
            assert_eq!(
                validate(producer, &mutated, &required),
                Err(format!("{producer}: missing inherited dependency {name}")),
                "PH-1: deleting {name} from {producer} must fail with its own diagnostic"
            );
        }
    }
    Ok(())
}

#[test]
fn ph1_version_source_and_option_drift_is_rejected() -> Result<(), String> {
    let required = requirements()?;
    for (producer, source) in PRODUCERS {
        for (name, requirement) in &required {
            let mut wrong_version = requirement.dependency.clone();
            wrong_version
                .options
                .insert("version".to_owned(), "\"0.0.0\"".to_owned());
            assert_definition_rejected(producer, source, name, &wrong_version, &required);
        }
        for (key, value) in [
            ("git", "\"https://example.invalid/reference-kv\""),
            ("default-features", "false"),
        ] {
            let mut wrong_option = required["reqwest"].dependency.clone();
            wrong_option
                .options
                .insert(key.to_owned(), value.to_owned());
            assert_definition_rejected(producer, source, "reqwest", &wrong_option, &required);
        }
    }
    Ok(())
}

#[test]
fn ph1_root_and_crate_features_including_tokio_net_cannot_be_removed() -> Result<(), String> {
    let required = requirements()?;
    // Explicit independent sentinel: losing the crate-local net requirement must
    // not turn this into a vacuous loop over root features only.
    assert!(
        required["tokio"].dependency.features.contains("net"),
        "PH-1: reference-kv http needs tokio/net"
    );
    for (producer, source) in PRODUCERS {
        for (name, requirement) in &required {
            for feature in &requirement.dependency.features {
                let mut missing_feature = requirement.dependency.clone();
                missing_feature.features.remove(feature);
                assert_definition_rejected(producer, source, name, &missing_feature, &required);
            }
        }
    }
    Ok(())
}

#[test]
fn ph1_undeclared_features_are_not_accepted_as_inheritance() -> Result<(), String> {
    let required = requirements()?;
    for (producer, source) in PRODUCERS {
        for (name, requirement) in &required {
            let mut extra_feature = requirement.dependency.clone();
            extra_feature
                .features
                .insert("unapproved-staging-feature".to_owned());
            assert_definition_rejected(producer, source, name, &extra_feature, &required);
        }
    }
    Ok(())
}

#[test]
fn ph1_reqwest_and_rcgen_definitions_are_verbatim_from_root() -> Result<(), String> {
    let required = requirements()?;
    for (producer, source) in PRODUCERS {
        let declarations = entries(source, WORKSPACE_DEPS)?;
        for name in ["reqwest", "rcgen"] {
            // Semantically identical TOML: only the stricter verbatim check can
            // reject this drift, not the parsed version/options/features check.
            let spaced = declarations[name].replacen("version =", "version  =", 1);
            let mutated = rewrite(source, name, Some(&spaced));
            assert_eq!(
                validate(producer, &mutated, &required),
                Err(format!(
                    "{producer}: root verbatim definition differs for {name}"
                )),
                "PH-1: {name} must keep the exact root definition in {producer}"
            );
        }
    }
    Ok(())
}
