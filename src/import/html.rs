use crate::db::Bookmark;
use scraper::{Html, Selector};

/// Parse Netscape Bookmark Format HTML and extract bookmarks with tags
/// Tags are created from all ancestor folder names (H3 elements)
pub fn parse_html_bookmarks(html: &str) -> Result<Vec<(Bookmark, Vec<String>)>, String> {
    let document = Html::parse_document(html);

    // Selectors for navigating bookmark structure
    let dl_selector = Selector::parse("dl").unwrap();

    let mut bookmarks = Vec::new();
    let mut folder_stack: Vec<String> = Vec::new();

    // Find the root DL element (main bookmark list)
    if let Some(root_dl) = document.select(&dl_selector).next() {
        parse_dl_element(root_dl, &mut folder_stack, &mut bookmarks);
    }

    Ok(bookmarks)
}

/// Recursively parse a DL element and its children
fn parse_dl_element(
    dl: scraper::ElementRef,
    folder_stack: &mut Vec<String>,
    bookmarks: &mut Vec<(Bookmark, Vec<String>)>,
) {
    let h3_selector = Selector::parse("h3").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let dl_selector = Selector::parse("dl").unwrap();

    // Process direct children of this DL
    for child in dl.children() {
        if let Some(dt) = scraper::ElementRef::wrap(child) {
            if dt.value().name() != "dt" {
                continue;
            }

            // Check if this DT contains a folder (H3)
            if let Some(h3) = dt.select(&h3_selector).next() {
                let folder_name = h3.text().collect::<String>().trim().to_string();

                if !folder_name.is_empty() {
                    // Push folder to stack
                    folder_stack.push(folder_name);

                    // Look for a nested DL as a child of this DT
                    if let Some(nested_dl) = dt.select(&dl_selector).next() {
                        parse_dl_element(nested_dl, folder_stack, bookmarks);
                    }

                    // Pop folder from stack when done
                    folder_stack.pop();
                }
            }
            // Check if this DT contains a bookmark (A tag) directly
            else if dt.children().any(|c| {
                scraper::ElementRef::wrap(c)
                    .map(|e| e.value().name() == "a")
                    .unwrap_or(false)
            }) {
                if let Some(a) = dt.select(&a_selector).next() {
                    if let Some(bookmark) = parse_bookmark(a, folder_stack) {
                        bookmarks.push(bookmark);
                    }
                }
            }
        }
    }
}

/// Parse a bookmark anchor tag and create a Bookmark with tags
fn parse_bookmark(
    a: scraper::ElementRef,
    folder_stack: &[String],
) -> Option<(Bookmark, Vec<String>)> {
    // Get URL (required)
    let url = a.value().attr("href")?.trim().to_string();

    if url.is_empty() {
        return None;
    }

    // Get title from link text
    let title = a.text().collect::<String>().trim().to_string();
    let title = if title.is_empty() { url.clone() } else { title };

    // Get timestamp from ADD_DATE attribute (Unix timestamp)
    let created = a
        .value()
        .attr("add_date")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        });

    // Use LAST_MODIFIED if available, otherwise use ADD_DATE
    let changed = a
        .value()
        .attr("last_modified")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(created);

    // Create bookmark
    let bookmark = Bookmark {
        id: None,
        title,
        url,
        note: None, // HTML bookmarks don't typically have notes
        content: None,
        created,
        changed,
        favicon_hash: None,
    };

    // Use all ancestor folders as tags
    let tags = folder_stack.to_vec();

    Some((bookmark, tags))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_bookmarks() {
        let html = r#"
<!DOCTYPE NETSCAPE-Bookmark-file-1>
<HTML>
<HEAD>
    <TITLE>Bookmarks</TITLE>
</HEAD>
<BODY>
<H1>Bookmarks</H1>
<DL>
    <DT><A HREF="https://example.com" ADD_DATE="1234567890">Example</A>
    <DT><H3>Dev</H3>
    <DL>
        <DT><A HREF="https://rust-lang.org">Rust</A>
    </DL>
</DL>
</BODY>
</HTML>
        "#;

        let bookmarks = parse_html_bookmarks(html).unwrap();

        assert_eq!(bookmarks.len(), 2);

        // First bookmark (no tags)
        assert_eq!(bookmarks[0].0.url, "https://example.com");
        assert_eq!(bookmarks[0].0.title, "Example");
        assert_eq!(bookmarks[0].1.len(), 0);

        // Second bookmark (Dev tag)
        assert_eq!(bookmarks[1].0.url, "https://rust-lang.org");
        assert_eq!(bookmarks[1].0.title, "Rust");
        assert_eq!(bookmarks[1].1, vec!["Dev"]);
    }

    #[test]
    fn test_parse_nested_folders() {
        let html = r#"
<DL>
    <DT><H3>Programming</H3>
    <DL>
        <DT><H3>Rust</H3>
        <DL>
            <DT><A HREF="https://doc.rust-lang.org">Docs</A>
        </DL>
    </DL>
</DL>
        "#;

        let bookmarks = parse_html_bookmarks(html).unwrap();

        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].1, vec!["Programming", "Rust"]);
    }

    #[test]
    fn test_parse_sample_bookmarks_file() {
        // Test with a more realistic bookmark file structure
        let html = r#"
<!DOCTYPE NETSCAPE-Bookmark-file-1>
<HTML>
<HEAD><TITLE>Bookmarks</TITLE></HEAD>
<BODY>
<H1>Bookmarks</H1>
<DL><p>
    <DT><H3>Dev</H3>
    <DL><p>
        <DT><H3>Rust</H3>
        <DL><p>
            <DT><A HREF="https://doc.rust-lang.org/">Rust Documentation</A>
            <DT><A HREF="https://www.rust-lang.org/">The Rust Programming Language</A>
        </DL><p>
        <DT><H3>GTK</H3>
        <DL><p>
            <DT><A HREF="https://gtk-rs.org/">gtk-rs - Rust bindings for GTK</A>
        </DL><p>
    </DL><p>
    <DT><H3>News</H3>
    <DL><p>
        <DT><A HREF="https://news.ycombinator.com/">Hacker News</A>
        <DT><A HREF="https://lobste.rs/">Lobsters</A>
    </DL><p>
    <DT><A HREF="https://github.com/">GitHub</A>
</DL><p>
</BODY>
</HTML>
        "#;

        let bookmarks = parse_html_bookmarks(html).unwrap();

        // Should have 6 total bookmarks
        assert_eq!(bookmarks.len(), 6);

        // Check specific bookmarks and their tags
        // Rust Documentation should have tags: ["Dev", "Rust"]
        let rust_doc = bookmarks
            .iter()
            .find(|(bm, _)| bm.url == "https://doc.rust-lang.org/")
            .unwrap();
        assert_eq!(rust_doc.1, vec!["Dev", "Rust"]);

        // gtk-rs should have tags: ["Dev", "GTK"]
        let gtk_rs = bookmarks
            .iter()
            .find(|(bm, _)| bm.url == "https://gtk-rs.org/")
            .unwrap();
        assert_eq!(gtk_rs.1, vec!["Dev", "GTK"]);

        // Hacker News should have tags: ["News"]
        let hn = bookmarks
            .iter()
            .find(|(bm, _)| bm.url == "https://news.ycombinator.com/")
            .unwrap();
        assert_eq!(hn.1, vec!["News"]);

        // GitHub should have no tags (top-level bookmark)
        let github = bookmarks
            .iter()
            .find(|(bm, _)| bm.url == "https://github.com/")
            .unwrap();
        assert_eq!(github.1.len(), 0);
    }
}
