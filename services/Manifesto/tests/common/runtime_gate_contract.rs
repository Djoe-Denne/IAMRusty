//! Closed lexical witnesses for ratified runtime control data, not file exemptions.
use std::{path::Path, sync::OnceLock};

#[derive(serde::Deserialize)]
struct Rule {
    path: String,
    #[serde(default)]
    identifiers: Vec<String>,
    #[serde(default)]
    fragments: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Corpus {
    rules: Vec<Rule>,
    cases: Vec<Case>,
}

#[derive(serde::Deserialize)]
struct Case {
    path: String,
    line: String,
    needle: String,
    forbidden: bool,
}

fn corpus() -> &'static Corpus {
    static CORPUS: OnceLock<Corpus> = OnceLock::new();
    CORPUS.get_or_init(|| {
        serde_json::from_str(include_str!("../fixtures/runtime_gate_contract.json"))
            .expect("closed contract witnesses")
    })
}

fn identifier_character(c: char) -> bool {
    // Conservative Rust-identifier boundary: never let a Unicode suffix split
    // an unapproved identifier into an approved ASCII name plus a separator.
    c.is_ascii_alphanumeric() || c == '_' || !c.is_ascii()
}

fn remove_fragment(line: &str, fragment: &str) -> String {
    assert!(!fragment.is_empty(), "empty contract fragment is forbidden");
    let mut output = String::new();
    let mut last = 0;
    for (start, _) in line.match_indices(fragment) {
        let end = start + fragment.len();
        let left_ok = !identifier_character(fragment.chars().next().unwrap())
            || line[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !identifier_character(c));
        let right_ok = !identifier_character(fragment.chars().next_back().unwrap())
            || line[end..]
                .chars()
                .next()
                .is_none_or(|c| !identifier_character(c));
        if left_ok && right_ok {
            output.push_str(&line[last..start]);
            output.push(' ');
            last = end;
        }
    }
    output.push_str(&line[last..]);
    output
}

pub fn forbidden(path: &Path, line: &str, needle: &str) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let rule = corpus()
        .rules
        .iter()
        .find(|r| normalized == r.path || normalized.ends_with(&format!("/{}", r.path)));
    let Some(rule) = rule else {
        return line.to_lowercase().contains(needle);
    };
    let mut remaining = line.to_string();
    for fragment in &rule.fragments {
        remaining = remove_fragment(&remaining, fragment);
    }
    let mut output = String::new();
    let mut word = String::new();
    for c in remaining.chars().chain(std::iter::once(' ')) {
        if identifier_character(c) {
            word.push(c);
        } else {
            if !rule.identifiers.contains(&word) {
                output.push_str(&word);
            } else {
                output.push(' ');
            }
            word.clear();
            output.push(c);
        }
    }
    output.to_lowercase().contains(needle)
}

#[test]
fn exact_contract_context_keeps_unapproved_code_visible() {
    assert!(!corpus().cases.is_empty());
    for case in &corpus().cases {
        assert_eq!(
            forbidden(Path::new(&case.path), &case.line, &case.needle),
            case.forbidden,
            "path={} line={}",
            case.path,
            case.line
        );
    }
}
