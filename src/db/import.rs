use crate::db::{queries, Bookmark};
use rusqlite::{params, Connection, OptionalExtension, Result};

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub imported_urls: Vec<String>,
}

/// Import bookmarks into the database
/// - Skips duplicates (same URL)
/// - Creates tags as needed
/// - Returns statistics about the import
pub fn import_bookmarks(
    conn: &Connection,
    bookmarks: Vec<(Bookmark, Vec<String>)>,
) -> Result<ImportResult> {
    let tx = conn.unchecked_transaction()?;

    let mut result = ImportResult {
        imported: 0,
        skipped: 0,
        errors: Vec::new(),
        imported_urls: Vec::new(),
    };

    for (bookmark, tag_titles) in bookmarks {
        // Check if bookmark with this URL already exists
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM bookmarks WHERE url = ?1",
                params![&bookmark.url],
                |_| Ok(true),
            )
            .optional()?
            .is_some();

        if exists {
            result.skipped += 1;
            continue;
        }

        // Insert the bookmark
        match queries::insert_bookmark(&tx, &bookmark) {
            Ok(bookmark_id) => {
                // Add tags if any
                let mut tag_error = false;
                if !tag_titles.is_empty() {
                    for title in &tag_titles {
                        match queries::get_or_create_tag(&tx, title) {
                            Ok(tag_id) => {
                                if let Err(e) =
                                    queries::add_tag_to_bookmark(&tx, bookmark_id, tag_id)
                                {
                                    result.errors.push(format!(
                                        "Failed to link tag '{}' to '{}': {}",
                                        title, bookmark.title, e
                                    ));
                                    tag_error = true;
                                }
                            }
                            Err(e) => {
                                result.errors.push(format!(
                                    "Failed to create tag '{}' for '{}': {}",
                                    title, bookmark.title, e
                                ));
                                tag_error = true;
                            }
                        }
                    }
                }

                if !tag_error {
                    result.imported += 1;
                    result.imported_urls.push(bookmark.url);
                }
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to import '{}': {}", bookmark.title, e));
            }
        }
    }

    tx.commit()?;

    Ok(result)
}
