use crate::models::LanguageStat;
use anyhow::Result;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use std::collections::HashMap;
use std::fmt;

/// Normalize a language name for comparison (case-insensitive).
fn normalize_language_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Decode one exclude token from the query string.
///
/// Query strings decode `+` as space, so `?exclude=C++` arrives as `"C  "`.
/// GitHub language names never contain spaces, so spaces are treated as `+`.
/// A single leading/trailing `+` (from outer whitespace padding) is stripped.
fn decode_exclude_token(token: &str) -> String {
    let mut decoded = token.replace(' ', "+");
    decoded = decoded.trim_start_matches('+').to_string();
    if decoded.ends_with('+') {
        let body = &decoded[..decoded.len() - 1];
        if !body.contains('+') {
            decoded.pop();
        }
    }
    decoded
}

pub fn parse_excludes(exclude_param: Option<&str>) -> Vec<String> {
    exclude_param
        .map(|value| {
            value
                .split(',')
                .map(decode_exclude_token)
                .filter(|part| !part.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Merge exclude values from one or more query parameters.
///
/// Supports both comma-separated lists and repeated params:
/// `?exclude=Python,TypeScript` or `?exclude=Python&exclude=TypeScript`
///
/// Use repeated params (and percent-encoding) for names with URL-special characters:
/// `?exclude=C++&exclude=C%23&exclude=HLSL` — a literal `#` in the URL is treated
/// as a fragment delimiter and is never sent to the server.
pub fn parse_excludes_from_params(params: &[String]) -> Vec<String> {
    let mut excludes: Vec<String> = params
        .iter()
        .flat_map(|value| parse_excludes(Some(value.as_str())))
        .collect();
    excludes.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    excludes.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    excludes
}

/// Accept `exclude` as either a single value or repeated query params.
///
/// `?exclude=Python` and `?exclude=Python&exclude=TypeScript` both deserialize
/// into a `Vec<String>`.
pub fn deserialize_exclude_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ExcludeListVisitor;

    impl<'de> Visitor<'de> for ExcludeListVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a sequence of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                values.push(value);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_any(ExcludeListVisitor)
}

pub fn exclude_cache_key(excludes: &[String]) -> String {
    let mut names: Vec<String> = excludes
        .iter()
        .map(|name| normalize_language_name(&decode_exclude_token(name)))
        .filter(|name| !name.is_empty())
        .collect();
    names.sort();
    names.join(",")
}

pub fn apply_excludes(
    totals: HashMap<String, u64>,
    excludes: &[String],
) -> Result<HashMap<String, u64>> {
    let exclude_normalized: Vec<String> = excludes
        .iter()
        .map(|name| normalize_language_name(&decode_exclude_token(name)))
        .filter(|name| !name.is_empty())
        .collect();

    if exclude_normalized.is_empty() {
        return Ok(totals);
    }

    let filtered: HashMap<String, u64> = totals
        .into_iter()
        .filter(|(lang, _)| !exclude_normalized.contains(&normalize_language_name(lang)))
        .collect();

    if filtered.is_empty() {
        anyhow::bail!("no languages remain after applying excludes");
    }

    Ok(filtered)
}

pub fn language_stats_from_map(totals: &HashMap<String, u64>) -> Vec<LanguageStat> {
    let mut entries: Vec<(String, u64)> = totals
        .iter()
        .map(|(name, bytes)| (name.clone(), *bytes))
        .collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let total_bytes: u64 = entries.iter().map(|(_, b)| b).sum();
    entries
        .into_iter()
        .map(|(name, bytes)| LanguageStat {
            name,
            bytes,
            percentage: if total_bytes == 0 {
                0.0
            } else {
                (bytes as f64 / total_bytes as f64) * 100.0
            },
        })
        .collect()
}

pub fn aggregate_top_six(totals: HashMap<String, u64>) -> Result<Vec<LanguageStat>> {
    if totals.is_empty() {
        anyhow::bail!("no language data to chart");
    }

    let mut entries: Vec<(String, u64)> = totals.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let top: Vec<(String, u64)> = entries.into_iter().take(6).collect();
    let partial: HashMap<String, u64> = top.into_iter().collect();

    Ok(language_stats_from_map(&partial))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_totals() -> HashMap<String, u64> {
        HashMap::from([
            ("Rust".to_string(), 100),
            ("JavaScript".to_string(), 80),
            ("CSS".to_string(), 20),
            ("C++".to_string(), 500),
            ("C".to_string(), 10),
        ])
    }

    #[test]
    fn parse_excludes_splits_and_trims() {
        assert_eq!(
            parse_excludes(Some(" Rust , JavaScript , ,CSS ")),
            vec!["Rust", "JavaScript", "CSS"]
        );
        assert!(parse_excludes(None).is_empty());
    }

    #[test]
    fn exclude_cache_key_is_order_independent() {
        assert_eq!(
            exclude_cache_key(&["CSS".into(), "Rust".into()]),
            exclude_cache_key(&["Rust".into(), "CSS".into()])
        );
    }

    #[test]
    fn apply_excludes_is_case_insensitive() {
        let filtered = apply_excludes(sample_totals(), &["javascript".into()]).unwrap();
        assert_eq!(filtered.len(), 4);
        assert!(!filtered.contains_key("JavaScript"));
    }

    #[test]
    fn parse_excludes_decodes_plus_as_space() {
        assert_eq!(parse_excludes(Some("C  ")), vec!["C++"]);
        assert_eq!(parse_excludes(Some("Python,C  ")), vec!["Python", "C++"]);
    }

    #[test]
    fn apply_excludes_handles_plus_decoded_as_space() {
        // Simulates ?exclude=C++ where query parsing turns '+' into spaces.
        let filtered = apply_excludes(sample_totals(), &["C  ".into()]).unwrap();
        assert!(!filtered.contains_key("C++"));
        assert!(filtered.contains_key("C"));
        assert_eq!(filtered.len(), 4);
    }

    #[test]
    fn apply_excludes_handles_c_plus_plus_literal() {
        let filtered = apply_excludes(sample_totals(), &["C++".into()]).unwrap();
        assert!(!filtered.contains_key("C++"));
    }

    #[test]
    fn parse_excludes_from_params_merges_repeated_values() {
        let merged =
            parse_excludes_from_params(&["Python,TypeScript".into(), "CSS".into()]);
        assert_eq!(merged.len(), 3);
        assert!(merged.iter().any(|v| v.eq_ignore_ascii_case("Python")));
        assert!(merged.iter().any(|v| v.eq_ignore_ascii_case("TypeScript")));
        assert!(merged.iter().any(|v| v.eq_ignore_ascii_case("CSS")));

        assert_eq!(
            parse_excludes_from_params(&["Python".into(), "TypeScript".into()]),
            vec!["Python", "TypeScript"]
        );
    }

    #[test]
    fn apply_excludes_handles_c_sharp() {
        let totals = HashMap::from([("C#".to_string(), 100), ("Rust".to_string(), 50)]);
        let filtered = apply_excludes(totals, &["C#".into()]).unwrap();
        assert!(!filtered.contains_key("C#"));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn aggregate_top_six_omits_other_bucket() {
        let totals = HashMap::from([
            ("A".to_string(), 100),
            ("B".to_string(), 90),
            ("C".to_string(), 80),
            ("D".to_string(), 70),
            ("E".to_string(), 60),
            ("F".to_string(), 50),
            ("G".to_string(), 40),
            ("H".to_string(), 30),
        ]);
        let stats = aggregate_top_six(totals).unwrap();
        assert_eq!(stats.len(), 6);
        assert!(stats.iter().all(|s| s.name != "Other"));
        assert!(stats.iter().any(|s| s.name == "A"));
        assert!(!stats.iter().any(|s| s.name == "G" || s.name == "H"));
        let pct_sum: f64 = stats.iter().map(|s| s.percentage).sum();
        assert!((pct_sum - 100.0).abs() < 0.01);
    }

    #[test]
    fn apply_excludes_errors_when_nothing_remains() {
        let err = apply_excludes(
            sample_totals(),
            &[
                "Rust".into(),
                "JavaScript".into(),
                "CSS".into(),
                "C++".into(),
                "C".into(),
            ],
        )
        .unwrap_err();
        assert!(err.to_string().contains("no languages remain"));
    }
}
