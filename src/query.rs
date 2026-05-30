use crate::stats::deserialize_exclude_list;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LanguagesQuery {
    #[serde(default, deserialize_with = "deserialize_exclude_list")]
    pub exclude: Vec<String>,
    #[serde(default = "default_include_org", rename = "includeOrg")]
    pub include_org: bool,
    #[serde(default = "default_include_private", rename = "includePrivate")]
    pub include_private: bool,
    #[serde(default = "default_show_username", rename = "showUsername")]
    pub show_username: bool,
    #[serde(default)]
    pub minimal: bool,
    #[serde(default)]
    pub username: Option<String>,
}

fn default_include_org() -> bool {
    true
}

fn default_include_private() -> bool {
    true
}

fn default_show_username() -> bool {
    true
}

impl<S> FromRequestParts<S> for LanguagesQuery
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let raw = parts.uri.query().unwrap_or("");
        let normalized = normalize_query_commas(raw);
        serde_urlencoded::from_str(&normalized).map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to deserialize query string: {err}"),
            )
        })
    }
}

/// Expand comma-separated query params into ampersand-separated pairs.
///
/// Badge URLs sometimes join flags with commas:
/// `minimal=true,showUsername=false&exclude=HTML,CSS`
/// becomes
/// `minimal=true&showUsername=false&exclude=HTML,CSS`
pub fn normalize_query_commas(query: &str) -> String {
    query
        .split('&')
        .map(expand_comma_separated_params)
        .collect::<Vec<_>>()
        .join("&")
}

fn expand_comma_separated_params(segment: &str) -> String {
    let Some(eq_idx) = segment.find('=') else {
        return segment.to_string();
    };

    let mut current_key = segment[..eq_idx].to_string();
    let value = &segment[eq_idx + 1..];

    let mut out = String::new();
    let mut current_value = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == ',' {
            let remaining: String = chars.clone().collect();
            if let Some(next_eq) = remaining.find('=') {
                let potential_key = remaining[..next_eq].to_string();
                if is_param_name(&potential_key) {
                    out.push_str(&format!("{current_key}={current_value}&"));
                    current_key = potential_key;
                    current_value.clear();
                    for _ in 0..next_eq + 1 {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        current_value.push(ch);
    }

    out.push_str(&format!("{current_key}={current_value}"));
    out
}

fn is_param_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(
        (chars.next(), chars.all(|c| c.is_ascii_alphanumeric() || c == '_')),
        (Some(c), true) if c.is_ascii_alphabetic()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_comma_separated_bool_params() {
        assert_eq!(
            normalize_query_commas("minimal=true,showUsername=false&includeOrg=false"),
            "minimal=true&showUsername=false&includeOrg=false"
        );
    }

    #[test]
    fn preserves_commas_in_exclude_values() {
        assert_eq!(
            normalize_query_commas("exclude=HTML,CSS,Java&minimal=true"),
            "exclude=HTML,CSS,Java&minimal=true"
        );
    }

    #[test]
    fn minimal_first_with_comma_separated_flags() {
        assert_eq!(
            normalize_query_commas("minimal=true,showUsername=false,includeOrg=false&exclude=HTML,CSS"),
            "minimal=true&showUsername=false&includeOrg=false&exclude=HTML,CSS"
        );
    }

    #[test]
    fn ampersand_separated_unchanged() {
        let query = "minimal=true&showUsername=false&includeOrg=false&exclude=HTML,CSS";
        assert_eq!(normalize_query_commas(query), query);
    }

    #[test]
    fn deserializes_comma_joined_minimal_first() {
        let normalized = normalize_query_commas(
            "minimal=true,showUsername=false,includeOrg=false&exclude=HTML,CSS",
        );
        let query: LanguagesQuery = serde_urlencoded::from_str(&normalized).unwrap();
        assert!(query.minimal);
        assert!(!query.show_username);
        assert!(!query.include_org);
        assert_eq!(query.exclude, vec!["HTML,CSS"]);
    }

    #[test]
    fn deserializes_exact_comma_joined_url_from_screenshot() {
        let raw = "minimal=true,showUsername=false&includeOrg=false&exclude=HTML,CSS,Java,Blade,PHP,Jupyter%20Notebook";
        let normalized = normalize_query_commas(raw);
        let query: LanguagesQuery = serde_urlencoded::from_str(&normalized).unwrap();
        assert!(query.minimal);
        assert!(!query.show_username);
        assert!(!query.include_org);
        let excludes = crate::stats::parse_excludes_from_params(&query.exclude);
        assert_eq!(excludes.len(), 6);
        assert!(excludes.iter().any(|v| v == "HTML"));
        assert!(excludes.iter().any(|v| v == "Jupyter Notebook"));
    }
}
