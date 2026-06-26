use crate::db::models::{SortDirection, SortField, TRASHED_TAG_ID, TagFilterMode, UNTAGGED_TAG_ID};
use crate::db::{Bookmark, BookmarkWithTags, Tag};
use rusqlite::{Connection, OptionalExtension, Result, params};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn insert_bookmark(conn: &Connection, bookmark: &Bookmark) -> Result<i64> {
    conn.execute(
        "INSERT INTO bookmarks (title, url, note, content, created, changed, favicon_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            bookmark.title,
            bookmark.url,
            bookmark.note,
            bookmark.content,
            bookmark.created,
            bookmark.changed,
            bookmark.favicon_hash,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn count_bookmarks(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM bookmarks", [], |r| {
        r.get(0)
    })
}

pub fn insert_tag(conn: &Connection, tag: &Tag) -> Result<i64> {
    conn.execute("INSERT INTO tags (title) VALUES (?1)", params![tag.title])?;
    Ok(conn.last_insert_rowid())
}

pub fn get_or_create_tag(conn: &Connection, title: &str) -> Result<i64> {
    // Ensure all ancestor tags exist
    let parts: Vec<&str> = title.split('/').collect();
    let mut current_path = String::new();

    // Create all ancestors up to, but not including, the final tag
    for i in parts.iter().take(parts.len().saturating_sub(1)) {
        if !current_path.is_empty() {
            current_path.push('/');
        }
        current_path.push_str(i);

        if conn
            .execute(
                "INSERT OR IGNORE INTO tags (title) VALUES (?1)",
                params![&current_path],
            )
            .is_err()
        {
            // Ignore error if it already exists
        }
    }

    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM tags WHERE title = ?1",
            params![title],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        Ok(id)
    } else {
        let tag = Tag::new(title.to_string());
        insert_tag(conn, &tag)
    }
}

pub fn add_tag_to_bookmark(conn: &Connection, bookmark_id: i64, tag_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO bookmark_tags (bookmark_id, tag_id) VALUES (?1, ?2)",
        params![bookmark_id, tag_id],
    )?;
    Ok(())
}

fn parse_tags_from_concat(tags_concat: Option<String>) -> Vec<Tag> {
    match tags_concat {
        None => Vec::new(),
        Some(s) if s.is_empty() => Vec::new(),
        Some(s) => s
            .split(',')
            .filter_map(|pair| {
                let mut parts = pair.split('|');
                match (parts.next(), parts.next()) {
                    (Some(id_str), Some(title)) => id_str.parse::<i64>().ok().map(|id| Tag {
                        id: Some(id),
                        title: title.to_string(),
                    }),
                    _ => None,
                }
            })
            .collect(),
    }
}

fn map_bookmark_with_favicon_and_tags(
    row: &rusqlite::Row,
) -> Result<(Bookmark, Option<Vec<u8>>, Vec<Tag>)> {
    let bookmark = Bookmark {
        id: Some(row.get(0)?),
        title: row.get(1)?,
        url: row.get(2)?,
        note: row.get(3)?,
        content: row.get(4)?,
        created: row.get(5)?,
        changed: row.get(6)?,
        favicon_hash: row.get(8)?,
        deleted: row.get(10)?,
    };
    let favicon_data: Option<Vec<u8>> = row.get(7)?;
    let tags_concat: Option<String> = row.get(9)?;
    let tags = parse_tags_from_concat(tags_concat);
    Ok((bookmark, favicon_data, tags))
}

pub fn get_all_bookmarks(
    conn: &Connection,
    sort_field: SortField,
    sort_direction: SortDirection,
) -> Result<Vec<BookmarkWithTags>> {
    let order_clause = format!(
        "ORDER BY b.{} {}",
        sort_field.column_name(),
        sort_direction.sql_keyword()
    );

    let _query = format!(
        "SELECT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash,
                GROUP_CONCAT(t.id || \'|\' || t.title, \',\') as tags_concat, b.deleted
         FROM bookmarks b
         LEFT JOIN favicons f ON b.favicon_hash = f.hash
         LEFT JOIN bookmark_tags bt ON b.id = bt.bookmark_id
         LEFT JOIN tags t ON bt.tag_id = t.id
         WHERE b.deleted = 0
         GROUP BY b.id
         {}",
        order_clause
    );

    let mut stmt = conn.prepare(&_query)?;

    let bookmark_iter = stmt.query_map([], map_bookmark_with_favicon_and_tags)?;

    let mut results = Vec::new();
    for result in bookmark_iter {
        let (bookmark, favicon_data, tags) = result?;
        results.push(BookmarkWithTags {
            bookmark,
            tags,
            favicon_data,
        });
    }

    Ok(results)
}

pub fn get_all_tags(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT id, title FROM tags ORDER BY title")?;

    let tag_iter = stmt.query_map([], |row| {
        Ok(Tag {
            id: Some(row.get(0)?),
            title: row.get(1)?,
        })
    })?;

    tag_iter.collect()
}

pub fn search_bookmarks(
    conn: &Connection,
    query: Option<&str>,
    tag_ids: &[i64],
    sort_field: SortField,
    sort_direction: SortDirection,
    tag_filter_mode: TagFilterMode,
) -> Result<Vec<BookmarkWithTags>> {
    if query.is_none() && tag_ids.is_empty() {
        return get_all_bookmarks(conn, sort_field, sort_direction);
    }

    // Check if untagged is in the tag_ids
    let has_untagged = tag_ids.contains(&UNTAGGED_TAG_ID);
    let has_trashed = tag_ids.contains(&TRASHED_TAG_ID);

    // Regular tag_ids (excluding untagged and trashed)
    let regular_tag_ids: Vec<i64> = tag_ids
        .iter()
        .copied()
        .filter(|&id| id != UNTAGGED_TAG_ID && id != TRASHED_TAG_ID)
        .collect();

    // Fetch titles for regular_tag_ids for prefix matching
    let mut selected_tag_titles: Vec<String> = Vec::new();
    if !regular_tag_ids.is_empty() {
        let placeholders: Vec<String> = (0..regular_tag_ids.len())
            .map(|_| "?".to_string())
            .collect();
        let query_str = format!(
            "SELECT title FROM tags WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&query_str)?;
        let params: Vec<&dyn rusqlite::ToSql> = regular_tag_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();
        let iter = stmt.query_map(rusqlite::params_from_iter(params), |row| row.get(0))?;
        for title in iter {
            selected_tag_titles.push(title?);
        }
    }

    let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    let mut base_select = "SELECT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash,
                        GROUP_CONCAT(t.id || \'|\' || t.title, \',\') as tags_concat, b.deleted
                 FROM bookmarks b".to_string();

    let mut where_clauses = Vec::new();

    let mut is_fts_search = false;
    if let Some(search_text) = query {
        if search_text.chars().count() >= 3 {
            let fts = format!("\"{}\"", search_text.replace("\"", "\"\""));
            base_select = format!("{} JOIN bookmarks_fts fts ON b.id = fts.rowid", base_select);
            where_clauses.push("bookmarks_fts MATCH ?".to_string());
            query_params.push(Box::new(fts));
            is_fts_search = true;
        } else {
            // Fallback for short queries (1 or 2 characters) since fts trigram requires >= 3
            // We use ESCAPE \'\\\' for the LIKE clause to handle literal % and _
            let like_query = format!(
                "%{}%",
                search_text
                    .replace("\\", "\\\\")
                    .replace("%", "\\%")
                    .replace("_", "\\_")
            );
            where_clauses.push("(b.title LIKE ? ESCAPE \'\\\' OR b.url LIKE ? ESCAPE \'\\\' OR b.note LIKE ? ESCAPE \'\\\')".to_string());
            query_params.push(Box::new(like_query.clone()));
            query_params.push(Box::new(like_query.clone()));
            query_params.push(Box::new(like_query));
        }
    }

    base_select = format!(
        "{}
         LEFT JOIN favicons f ON b.favicon_hash = f.hash
         LEFT JOIN bookmark_tags bt ON b.id = bt.bookmark_id
         LEFT JOIN tags t ON bt.tag_id = t.id",
        base_select
    );

    if tag_ids.is_empty() {
        // Default to active bookmarks
        where_clauses.push("b.deleted = 0".to_string());
    } else {
        if tag_filter_mode == TagFilterMode::All {
            // All mode: everything is ANDed
            if has_trashed {
                where_clauses.push("b.deleted = 1".to_string());
            } else {
                where_clauses.push("b.deleted = 0".to_string());
            }

            if has_untagged {
                where_clauses.push(
                    "NOT EXISTS (SELECT 1 FROM bookmark_tags bt2 WHERE bt2.bookmark_id = b.id)"
                        .to_string(),
                );
            }

            for title in &selected_tag_titles {
                where_clauses.push(
                    "EXISTS (SELECT 1 FROM bookmark_tags bt2 JOIN tags t2 ON bt2.tag_id = t2.id WHERE bt2.bookmark_id = b.id AND (t2.title = ? OR t2.title LIKE ? || \'/%\'))"
                        .to_string(),
                );
                query_params.push(Box::new(title.clone()));
                query_params.push(Box::new(title.clone()));
            }
        } else {
            // Any mode: OR together the selected "tags"
            let mut or_conditions = Vec::new();

            if has_trashed {
                or_conditions.push("b.deleted = 1".to_string());
            }

            if has_untagged {
                if has_trashed {
                    or_conditions.push(
                        "NOT EXISTS (SELECT 1 FROM bookmark_tags bt2 WHERE bt2.bookmark_id = b.id)"
                            .to_string(),
                    );
                } else {
                    or_conditions.push(
                        "(b.deleted = 0 AND NOT EXISTS (SELECT 1 FROM bookmark_tags bt2 WHERE bt2.bookmark_id = b.id))"
                            .to_string(),
                    );
                }
            }

            if !selected_tag_titles.is_empty() {
                let mut tag_conds = Vec::new();
                for title in &selected_tag_titles {
                    tag_conds.push("(t2.title = ? OR t2.title LIKE ? || \'/%\')".to_string());
                    query_params.push(Box::new(title.clone()));
                    query_params.push(Box::new(title.clone()));
                }

                let combined_tag_cond = format!(
                    "EXISTS (SELECT 1 FROM bookmark_tags bt2 JOIN tags t2 ON bt2.tag_id = t2.id WHERE bt2.bookmark_id = b.id AND ({}))",
                    tag_conds.join(" OR ")
                );

                if has_trashed {
                    or_conditions.push(combined_tag_cond);
                } else {
                    or_conditions.push(format!("(b.deleted = 0 AND {})", combined_tag_cond));
                }
            }

            if !has_trashed {
                // If "Trashed" is NOT one of the options, we must restrict to active bookmarks globally
                where_clauses.push("b.deleted = 0".to_string());
            }

            where_clauses.push(format!("({})", or_conditions.join(" OR ")));
        }
    }

    let order_clause = if is_fts_search && sort_field == SortField::Relevance {
        "ORDER BY rank".to_string()
    } else {
        // If it's a short query (LIKE fallback) but sort_field is Relevance, default to Created
        let safe_sort_field = if !is_fts_search && sort_field == SortField::Relevance {
            SortField::Created
        } else {
            sort_field
        };

        format!(
            "ORDER BY b.{} {}",
            safe_sort_field.column_name(),
            sort_direction.sql_keyword()
        )
    };

    let final_query = format!(
        "{} WHERE {} GROUP BY b.id {}",
        base_select,
        where_clauses.join(" AND "),
        order_clause
    );

    let mut stmt = conn.prepare(&final_query)?;

    let params: Vec<&dyn rusqlite::ToSql> = query_params
        .iter()
        .map(|p| &**p as &dyn rusqlite::ToSql)
        .collect();
    let bookmark_iter = stmt.query_map(
        rusqlite::params_from_iter(params),
        map_bookmark_with_favicon_and_tags,
    )?;

    let mut results = Vec::new();
    for result in bookmark_iter {
        let (bookmark, favicon_data, tags) = result?;
        results.push(BookmarkWithTags {
            bookmark,
            tags,
            favicon_data,
        });
    }

    Ok(results)
}

pub fn update_bookmark(
    conn: &Connection,
    id: i64,
    title: &str,
    url: &str,
    note: Option<&str>,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "UPDATE bookmarks SET title = ?1, url = ?2, note = ?3, changed = ?4 WHERE id = ?5",
        params![title, url, note, now, id],
    )?;
    Ok(())
}

pub fn rename_tag(conn: &Connection, id: i64, new_title: &str) -> Result<()> {
    conn.execute(
        "UPDATE tags SET title = ?1 WHERE id = ?2",
        params![new_title, id],
    )?;
    Ok(())
}

pub fn delete_tag(conn: &Connection, id: i64) -> Result<()> {
    let tx = conn.unchecked_transaction()?;

    // First get the title of the tag so we can delete children
    let title: String =
        tx.query_row("SELECT title FROM tags WHERE id = ?1", params![id], |row| {
            row.get(0)
        })?;

    // Delete the tag and any children (title LIKE 'title/%')
    // ON DELETE CASCADE will handle bookmark_tags
    tx.execute(
        "DELETE FROM tags WHERE id = ?1 OR title LIKE ?2",
        params![id, format!("{}/%", title)],
    )?;

    tx.commit()?;
    Ok(())
}

pub fn cleanup_orphan_tags(conn: &Connection) -> Result<()> {
    loop {
        // Delete tags that:
        // 1. Have no bookmarks
        // 2. Have no child tags (title LIKE tag.title || '/%')
        let deleted = conn.execute(
            "DELETE FROM tags 
             WHERE id NOT IN (SELECT tag_id FROM bookmark_tags)
               AND NOT EXISTS (
                   SELECT 1 FROM tags child 
                   WHERE child.title LIKE tags.title || '/%'
               )",
            [],
        )?;

        if deleted == 0 {
            break;
        }
    }
    Ok(())
}

pub fn update_bookmark_tags(
    conn: &Connection,
    bookmark_id: i64,
    tag_titles: &[String],
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "DELETE FROM bookmark_tags WHERE bookmark_id = ?1",
        params![bookmark_id],
    )?;

    for title in tag_titles {
        let tag_id = get_or_create_tag(&tx, title)?;
        add_tag_to_bookmark(&tx, bookmark_id, tag_id)?;
    }

    tx.commit()?;
    cleanup_orphan_tags(conn)?;
    Ok(())
}

pub fn delete_bookmark(conn: &Connection, id: i64) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    conn.execute(
        "UPDATE bookmarks SET deleted = 1, changed = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    cleanup_orphan_tags(conn)?;
    Ok(())
}

pub fn restore_bookmark(conn: &Connection, id: i64) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    conn.execute(
        "UPDATE bookmarks SET deleted = 0, changed = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

pub fn get_bookmark_by_id(conn: &Connection, id: i64) -> Result<BookmarkWithTags> {
    let (bookmark, favicon_data, tags) = conn.query_row(
        "SELECT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash,
                GROUP_CONCAT(t.id || \'|\' || t.title, \',\') as tags_concat, b.deleted
         FROM bookmarks b
         LEFT JOIN favicons f ON b.favicon_hash = f.hash
         LEFT JOIN bookmark_tags bt ON b.id = bt.bookmark_id
         LEFT JOIN tags t ON bt.tag_id = t.id
         WHERE b.id = ?1
         GROUP BY b.id",
        params![id],
        map_bookmark_with_favicon_and_tags,
    )?;

    Ok(BookmarkWithTags {
        bookmark,
        tags,
        favicon_data,
    })
}

pub fn insert_favicon_if_new(conn: &Connection, hash: i32, data: &[u8]) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO favicons (hash, favicon) VALUES (?1, ?2)",
        params![hash, data],
    )?;
    Ok(())
}

pub fn update_bookmark_favicon_hash(conn: &Connection, bookmark_id: i64, hash: i32) -> Result<()> {
    conn.execute(
        "UPDATE bookmarks SET favicon_hash = ?1 WHERE id = ?2",
        params![hash, bookmark_id],
    )?;
    Ok(())
}

pub fn get_favicon_hash_for_domain(conn: &Connection, domain: &str) -> Result<Option<i32>> {
    let hash: Option<i32> = conn
        .query_row(
            "SELECT favicon_hash FROM bookmarks 
             WHERE (url LIKE 'http://' || ?1 || '/%' 
                 OR url LIKE 'https://' || ?1 || '/%' 
                 OR url = 'http://' || ?1 
                 OR url = 'https://' || ?1) 
               AND favicon_hash IS NOT NULL 
             LIMIT 1",
            params![domain],
            |row| row.get(0),
        )
        .optional()?;
    Ok(hash)
}

pub fn clear_trashed_bookmarks(conn: &Connection) -> Result<usize> {
    let count = conn.execute("DELETE FROM bookmarks WHERE deleted = 1", [])?;
    Ok(count)
}

pub fn gc_deleted_bookmarks(conn: &Connection, days: u32) -> Result<usize> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let threshold = now - (days as i64 * 86400);

    let count = conn.execute(
        "DELETE FROM bookmarks WHERE deleted = 1 AND changed < ?1",
        params![threshold],
    )?;

    Ok(count)
}
