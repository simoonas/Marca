use crate::db::Bookmark;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonBookmark {
    #[serde(alias = "title", alias = "name")]
    pub title: Option<String>,
    #[serde(alias = "uri", alias = "url")]
    pub uri: String,
    #[serde(alias = "desc", alias = "note")]
    pub desc: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Parse JSON bookmarks
/// Format: [{"title": "...", "uri": "...", "desc": "...", "tags": ["..."]}, ...]
pub fn parse_json_bookmarks(json: &str) -> Result<Vec<(Bookmark, Vec<String>)>, String> {
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut result = Vec::new();

    for entry in entries {
        // Try to parse each entry, ignore if it doesn't match uri requirement
        if let Ok(jb) = serde_json::from_value::<JsonBookmark>(entry) {
            if jb.uri.is_empty() {
                continue;
            }

            let mut bookmark = Bookmark::new(jb.title.unwrap_or_else(|| jb.uri.clone()), jb.uri);
            bookmark.note = jb.desc;
            result.push((bookmark, jb.tags));
        }
    }

    Ok(result)
}

/// Export bookmarks to JSON string
pub fn export_to_json(bookmarks: &[crate::db::BookmarkWithTags]) -> Result<String, String> {
    let export: Vec<JsonBookmark> = bookmarks
        .iter()
        .map(|b| JsonBookmark {
            title: Some(b.bookmark.title.clone()),
            uri: b.bookmark.url.clone(),
            desc: b.bookmark.note.clone(),
            tags: b.tags.iter().map(|t| t.title.clone()).collect(),
        })
        .collect();

    serde_json::to_string_pretty(&export).map_err(|e| format!("Failed to serialize to JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        let json = r#"[
            {"title": "Joe's blog", "uri": "http://joe.com", "desc": "description", "tags": ["dev", "web"]},
            {"uri": "http://minimal.com"}
        ]"#;

        let result = parse_json_bookmarks(json).unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].0.title, "Joe's blog");
        assert_eq!(result[0].0.url, "http://joe.com");
        assert_eq!(result[0].0.note, Some("description".to_string()));
        assert_eq!(result[0].1, vec!["dev", "web"]);

        assert_eq!(result[1].0.title, "http://minimal.com");
        assert_eq!(result[1].0.url, "http://minimal.com");
        assert!(result[1].0.note.is_none());
        assert!(result[1].1.is_empty());
    }

    #[test]
    fn test_backwards_compatibility() {
        let json = r#"[
            {"title": "Extra", "uri": "http://extra.com", "unknown_key": "ignore me"}
        ]"#;

        let result = parse_json_bookmarks(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.url, "http://extra.com");
    }
}
