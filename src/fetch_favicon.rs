use std::time::Duration;

/// Fetch favicon for a given domain
/// Tries multiple sources in order:
/// 1. favicon.ico at domain root
/// 2. HTML head for <link rel="icon">
/// 3. Google favicon service as fallback
pub async fn fetch_favicon(url: &str) -> Option<Vec<u8>> {
    let domain = extract_domain(url)?;
    
    // Try multiple sources
    if let Some(data) = try_favicon_ico(&domain).await {
        return Some(data);
    }
    
    if let Some(data) = try_html_favicon(&domain).await {
        return Some(data);
    }
    
    if let Some(data) = try_google_favicon(&domain).await {
        return Some(data);
    }
    
    None
}

/// Extract domain from URL (e.g., "example.com" from "https://example.com/path")
fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed) = url::Url::parse(url) {
        parsed.host_str().map(|s| s.to_string())
    } else {
        None
    }
}

/// Try to fetch favicon.ico from domain root
async fn try_favicon_ico(domain: &str) -> Option<Vec<u8>> {
    let domain = domain.to_string();
    
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        
        let url = format!("https://{}/favicon.ico", domain);
        let response = client.get(&url).send().ok()?;
        if response.status().is_success() {
            response.bytes().ok().map(|b| b.to_vec())
        } else {
            None
        }
    }).await.ok().flatten()
}

/// Try to extract favicon from HTML <link rel="icon"> tag
async fn try_html_favicon(domain: &str) -> Option<Vec<u8>> {
    let domain_clone = domain.to_string();
    
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        
        let url = format!("https://{}", domain_clone);
        let response = client.get(&url).send().ok()?;
        let html = response.text().ok()?;
        
        // Try to extract favicon URL from HTML
        let favicon_url = extract_favicon_url_from_html(&html)?;
        
        // Fetch the favicon
        let response = client.get(&favicon_url).send().ok()?;
        if response.status().is_success() {
            response.bytes().ok().map(|b| b.to_vec())
        } else {
            None
        }
    }).await.ok().flatten()
}

/// Extract favicon URL from HTML head
fn extract_favicon_url_from_html(html: &str) -> Option<String> {
    // Look for common icon link patterns
    let patterns = [
        r#"<link[^>]*rel="icon"[^>]*href="([^"]+)"#,
        r#"<link[^>]*href="([^"]+)"[^>]*rel="icon""#,
        r#"<link[^>]*rel="shortcut icon"[^>]*href="([^"]+)"#,
        r#"<link[^>]*href="([^"]+)"[^>]*rel="shortcut icon""#,
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

/// Fallback: Use Google's favicon service
async fn try_google_favicon(domain: &str) -> Option<Vec<u8>> {
    let domain_clone = domain.to_string();
    
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        
        let url = format!("https://s2.googleusercontent.com/s2/favicons?domain={}&sz=64", domain_clone);
        let response = client.get(&url).send().ok()?;
        if response.status().is_success() {
            response.bytes().ok().map(|b| b.to_vec())
        } else {
            None
        }
    }).await.ok().flatten()
}
