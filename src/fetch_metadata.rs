use readability::extractor::scrape;
use std::error::Error;

/// Metadata extraction result
/// Returns (title, description, had_extraction_error)
pub async fn fetch_url_metadata(url: &str) -> Result<(String, Option<String>, bool), Box<dyn Error + Send + Sync>> {
    let url_string = url.to_string();
    
    // Spawn blocking task since readability uses blocking operations
    tokio::task::spawn_blocking(move || {
        // Try to fetch and extract using readability
        match scrape(&url_string) {
            Ok(product) => {
                let is_title_empty = product.title.is_empty();
                let title = if !is_title_empty {
                    product.title
                } else {
                    // Fallback to URL if title is empty
                    extract_domain_from_url(&url_string)
                };
                
                // Extract description from the extracted text
                let description = extract_description_from_text(&product.text);
                
                // Check if we had to use defaults
                let had_error = is_title_empty || description.is_none();
                
                Ok::<_, Box<dyn Error + Send + Sync>>((title, description, had_error))
            }
            Err(_e) => {
                // Fallback when extraction completely fails
                let title = extract_domain_from_url(&url_string);
                Ok::<_, Box<dyn Error + Send + Sync>>((title, None, true))
            }
        }
    })
    .await?
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
