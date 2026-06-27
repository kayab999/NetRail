use url::Url;

const DDG_HOSTS: &[&str] = &["duckduckgo.com", "duck.com"];

fn is_ddg_host(host: &str) -> bool {
    let host = host.to_lowercase();
    DDG_HOSTS
        .iter()
        .any(|&h| host == h || host.ends_with(&format!(".{h}")))
}

/// Unwrap DuckDuckGo redirect links (`/l/?uddg=…`) to the destination URL.
pub fn resolve_result_url(raw: &str, depth: u8) -> String {
    if depth > 4 {
        return raw.trim().to_string();
    }

    let trimmed = raw.trim();
    let absolute = if trimmed.starts_with("//") {
        format!("https:{trimmed}")
    } else {
        trimmed.to_string()
    };
    let Ok(parsed) = Url::parse(&absolute) else {
        return absolute;
    };

    if let Some(host) = parsed.host_str() {
        if is_ddg_host(host) {
            if let Some((_, uddg)) = parsed.query_pairs().find(|(k, _)| k == "uddg") {
                return resolve_result_url(&uddg, depth + 1);
            }
        }
    }

    absolute
}

/// Heuristic: title is useless when it mirrors a raw/encoded redirect URL.
pub fn clean_result_title(title: &str, url: &str, display_url_hint: Option<&str>) -> String {
    let title = title.trim();
    let resolved = resolve_result_url(url, 0);

    if let Some(hint) = display_url_hint {
        let hint = hint.trim();
        if !hint.is_empty() && !looks_like_encoded_url(hint) {
            return hint.to_string();
        }
    }

    if title.is_empty() || title == url || title == resolved || looks_like_encoded_url(title) {
        return title_from_url(&resolved);
    }

    title.to_string()
}

fn looks_like_encoded_url(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("%2f")
        || lower.contains("%3a")
        || lower.contains("uddg=")
        || lower.contains("duckduckgo.com/l/")
}

fn title_from_url(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else {
        return url.to_string();
    };
    let host = parsed.host_str().unwrap_or(url);
    let path = parsed.path();
    if path.is_empty() || path == "/" {
        host.to_string()
    } else {
        format!("{host}{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_ddg_redirect() {
        let raw = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org%2F&rut=abc";
        assert_eq!(
            resolve_result_url(raw, 0),
            "https://rust-lang.org/"
        );
    }

    #[test]
    fn cleans_encoded_title() {
        let raw = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fdoc";
        let title = clean_result_title(raw, raw, Some("example.com › doc"));
        assert_eq!(title, "example.com › doc");
    }

    #[test]
    fn unwraps_protocol_relative_ddg_redirect() {
        let raw = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2F";
        assert_eq!(resolve_result_url(raw, 0), "https://example.com/");
    }
}