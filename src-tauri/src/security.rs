use url::Url;

pub fn validate_open_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let parsed = Url::parse(trimmed).map_err(|_| "Invalid URL.".to_string())?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("Only http:// and https:// URLs are supported.".into());
    }

    if parsed.username() != "" || parsed.password().is_some() {
        return Err("URLs with embedded credentials are not allowed.".into());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "URL must include a host.".to_string())?
        .to_lowercase();

    if matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1") {
        return Err("Localhost URLs cannot be opened from search results.".into());
    }

    Ok(trimmed.to_string())
}

pub const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'";