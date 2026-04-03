use readability::extractor::scrape;
use std::error::Error;
use std::time::Duration;

/// Quick metadata fetch result (title and description only, no favicon)
pub struct QuickMetadata {
    pub title: String,
    pub description: Option<String>,
}

/// Fetch title and description quickly (5s timeout)
/// Does NOT fetch favicon - that should be done async after dialog closes
pub async fn fetch_quick_metadata(url: &str) -> Result<QuickMetadata, Box<dyn Error + Send + Sync>> {
    let url_string = url.to_string();
    
    // Use tokio timeout for the entire operation
    tokio::time::timeout(Duration::from_secs(5), tokio::task::spawn_blocking(move || {
        // Try to fetch and extract using readability
        match scrape(&url_string) {
            Ok(product) => {
                let title = if !product.title.is_empty() {
                    product.title
                } else {
                    extract_domain_from_url(&url_string)
                };
                let description = extract_description_from_text(&product.text);
                Ok::<_, Box<dyn Error + Send + Sync>>(QuickMetadata { title, description })
            }
            Err(_e) => {
                let title = extract_domain_from_url(&url_string);
                Ok::<_, Box<dyn Error + Send + Sync>>(QuickMetadata { title, description: None })
            }
        }
    }))
    .await
    .map_err(|_| "Timeout fetching metadata".into())
    .and_then(|r| r.map_err(|e| e.into()))
    .and_then(|r| r)
}

/// Fetch favicon only (async, can be spawned after save)
pub async fn fetch_favicon(url: &str) -> Option<Vec<u8>> {
    let url_string = url.to_string();
    
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .build()
            .ok()?;
        
        fetch_favicon_sync(&client, &url_string)
    })
    .await
    .ok()
    .flatten()
}

/// Synchronous favicon fetching (called within spawn_blocking)
fn fetch_favicon_sync(client: &reqwest::blocking::Client, url: &str) -> Option<Vec<u8>> {
    let domain = extract_domain_from_url_for_favicon(url)?;
    
    // Try favicon.ico first
    if let Some(data) = try_favicon_ico_sync(client, &domain) {
        return Some(data);
    }
    
    // Try HTML link extraction
    if let Some(data) = try_html_favicon_sync(client, &domain) {
        return Some(data);
    }
    
    // Try Google favicon service as fallback
    try_google_favicon_sync(client, &domain)
}

fn extract_domain_from_url_for_favicon(url: &str) -> Option<String> {
    if let Ok(parsed) = url::Url::parse(url) {
        parsed.host_str().map(|s| s.to_string())
    } else {
        None
    }
}

fn try_favicon_ico_sync(client: &reqwest::blocking::Client, domain: &str) -> Option<Vec<u8>> {
    let url = format!("https://{}/favicon.ico", domain);
    let response = client.get(&url).send().ok()?;
    if response.status().is_success() {
        let bytes = response.bytes().ok()?.to_vec();
        // Filter out very small responses (likely error pages)
        if bytes.len() >= 100 {
            return Some(bytes);
        }
    }
    None
}

fn try_html_favicon_sync(client: &reqwest::blocking::Client, domain: &str) -> Option<Vec<u8>> {
    let base_url = format!("https://{}", domain);
    let response = client.get(&base_url).send().ok()?;
    let html = response.text().ok()?;
    
    let favicon_href = extract_favicon_url_from_html(&html)?;
    let favicon_url = resolve_url(&base_url, &favicon_href);
    
    let response = client.get(&favicon_url).send().ok()?;
    if response.status().is_success() {
        let bytes = response.bytes().ok()?.to_vec();
        if bytes.len() >= 100 {
            return Some(bytes);
        }
    }
    None
}

fn try_google_favicon_sync(client: &reqwest::blocking::Client, domain: &str) -> Option<Vec<u8>> {
    let url = format!("https://www.google.com/s2/favicons?domain={}&sz=64", domain);
    let response = client.get(&url).send().ok()?;
    if response.status().is_success() {
        let bytes = response.bytes().ok()?.to_vec();
        // Filter out tiny responses (Google returns 1x1 pixel for unknown domains)
        if bytes.len() >= 100 {
            return Some(bytes);
        }
    }
    None
}

/// Resolve a potentially relative URL against a base URL
fn resolve_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if href.starts_with("//") {
        return format!("https:{}", href);
    }
    if href.starts_with('/') {
        return format!("{}{}", base, href);
    }
    format!("{}/{}", base, href)
}

/// Extract favicon URL from HTML head
fn extract_favicon_url_from_html(html: &str) -> Option<String> {
    let patterns = [
        r#"<link[^>]*rel="icon"[^>]*href="([^"]+)"#,
        r#"<link[^>]*href="([^"]+)"[^>]*rel="icon""#,
        r#"<link[^>]*rel="shortcut icon"[^>]*href="([^"]+)"#,
        r#"<link[^>]*href="([^"]+)"[^>]*rel="shortcut icon""#,
        r#"<link[^>]*rel='icon'[^>]*href='([^']+)'"#,
        r#"<link[^>]*href='([^']+)'[^>]*rel='icon'"#,
        // Also try apple-touch-icon as fallback (usually higher quality)
        r#"<link[^>]*rel="apple-touch-icon"[^>]*href="([^"]+)"#,
        r#"<link[^>]*href="([^"]+)"[^>]*rel="apple-touch-icon""#,
    ];
    
    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(html) {
                if let Some(url) = caps.get(1) {
                    return Some(url.as_str().to_string());
                }
            }
        }
    }
    
    None
}

fn extract_description_from_text(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    
    // Clean and normalize the text
    let cleaned = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    
    // Limit to a reasonable length for the note field
    let max_len = 300;
    
    if cleaned.len() > max_len {
        // Try to break at a sentence boundary for better readability
        if let Some(period_pos) = cleaned[..max_len]
            .rfind('.')
            .filter(|&pos| pos > max_len / 2)
        {
            Some(format!("{}.", &cleaned[..period_pos]))
        } else if let Some(space_pos) = cleaned[..max_len].rfind(' ') {
            Some(format!("{}...", &cleaned[..space_pos]))
        } else {
            Some(format!("{}...", &cleaned[..max_len]))
        }
    } else if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Extract domain name from URL as fallback title
fn extract_domain_from_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            // Remove www. prefix if present
            let domain = host.strip_prefix("www.").unwrap_or(host);
            return domain.to_string();
        }
    }
    // Fallback to the full URL if parsing fails
    url.to_string()
}
