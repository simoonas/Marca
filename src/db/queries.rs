use crate::db::{Bookmark, BookmarkWithTags, Tag};
use rusqlite::{params, Connection, OptionalExtension, Result};

pub fn insert_bookmark(conn: &Connection, bookmark: &Bookmark) -> Result<i64> {
    conn.execute(
        "INSERT INTO bookmarks (title, url, note, content, created, changed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            bookmark.title,
            bookmark.url,
            bookmark.note,
            bookmark.content,
            bookmark.created,
            bookmark.changed,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_tag(conn: &Connection, tag: &Tag) -> Result<i64> {
    conn.execute("INSERT INTO tags (title) VALUES (?1)", params![tag.title])?;
    Ok(conn.last_insert_rowid())
}

pub fn get_or_create_tag(conn: &Connection, title: &str) -> Result<i64> {
    // Try to find existing tag
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
        // Create new tag
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

pub fn get_all_bookmarks(conn: &Connection) -> Result<Vec<BookmarkWithTags>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, url, note, content, created, changed FROM bookmarks ORDER BY created DESC"
    )?;

    let bookmark_iter = stmt.query_map([], |row| {
        Ok(Bookmark {
            id: Some(row.get(0)?),
            title: row.get(1)?,
            url: row.get(2)?,
            note: row.get(3)?,
            content: row.get(4)?,
            created: row.get(5)?,
            changed: row.get(6)?,
        })
    })?;

    let mut results = Vec::new();
    for bookmark_result in bookmark_iter {
        let bookmark = bookmark_result?;
        let tags = get_tags_for_bookmark(conn, bookmark.id.unwrap())?;
        results.push(BookmarkWithTags { bookmark, tags });
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

fn get_tags_for_bookmark(conn: &Connection, bookmark_id: i64) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.title FROM tags t
         JOIN bookmark_tags bt ON t.id = bt.tag_id
         WHERE bt.bookmark_id = ?1
         ORDER BY t.title",
    )?;

    let tag_iter = stmt.query_map(params![bookmark_id], |row| {
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
) -> Result<Vec<BookmarkWithTags>> {
    // No filters - return all
    if query.is_none() && tag_ids.is_empty() {
        return get_all_bookmarks(conn);
    }

    let mut results = Vec::new();

    // Helper to map row to bookmark
    let map_row = |row: &rusqlite::Row| -> Result<Bookmark> {
        Ok(Bookmark {
            id: Some(row.get(0)?),
            title: row.get(1)?,
            url: row.get(2)?,
            note: row.get(3)?,
            content: row.get(4)?,
            created: row.get(5)?,
            changed: row.get(6)?,
        })
    };

    if let Some(search_text) = query {
        if tag_ids.is_empty() {
            // Text search only
            let mut stmt = conn.prepare(
                "SELECT DISTINCT b.id, b.title, b.url, b.note, b.content, b.created, b.changed
                 FROM bookmarks b
                 JOIN bookmarks_fts fts ON b.id = fts.rowid
                 WHERE bookmarks_fts MATCH ?1
                 ORDER BY rank",
            )?;

            let bookmark_iter = stmt.query_map(params![search_text], map_row)?;

            for bookmark_result in bookmark_iter {
                let bookmark = bookmark_result?;
                let tags = get_tags_for_bookmark(conn, bookmark.id.unwrap())?;
                results.push(BookmarkWithTags { bookmark, tags });
            }
        } else {
            // Text search + tag filtering
            let tag_ids_json = serde_json::to_string(tag_ids).unwrap();
            let mut stmt = conn.prepare(
                "SELECT DISTINCT b.id, b.title, b.url, b.note, b.content, b.created, b.changed
                 FROM bookmarks b
                 JOIN bookmarks_fts fts ON b.id = fts.rowid
                 JOIN bookmark_tags bt ON b.id = bt.bookmark_id
                 WHERE bookmarks_fts MATCH ?1 AND bt.tag_id IN (SELECT value FROM json_each(?2))
                 GROUP BY b.id
                 HAVING COUNT(DISTINCT bt.tag_id) = ?3
                 ORDER BY rank",
            )?;

            let bookmark_iter =
                stmt.query_map(params![search_text, tag_ids_json, tag_ids.len()], map_row)?;

            for bookmark_result in bookmark_iter {
                let bookmark = bookmark_result?;
                let tags = get_tags_for_bookmark(conn, bookmark.id.unwrap())?;
                results.push(BookmarkWithTags { bookmark, tags });
            }
        }
    } else {
        // Tag filtering only
        let tag_ids_json = serde_json::to_string(tag_ids).unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT b.id, b.title, b.url, b.note, b.content, b.created, b.changed
             FROM bookmarks b
             JOIN bookmark_tags bt ON b.id = bt.bookmark_id
             WHERE bt.tag_id IN (SELECT value FROM json_each(?1))
             GROUP BY b.id
             HAVING COUNT(DISTINCT bt.tag_id) = ?2
             ORDER BY b.created DESC",
        )?;

        let bookmark_iter = stmt.query_map(params![tag_ids_json, tag_ids.len()], map_row)?;

        for bookmark_result in bookmark_iter {
            let bookmark = bookmark_result?;
            let tags = get_tags_for_bookmark(conn, bookmark.id.unwrap())?;
            results.push(BookmarkWithTags { bookmark, tags });
        }
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

pub fn update_bookmark_tags(
    conn: &Connection,
    bookmark_id: i64,
    tag_titles: &[String],
) -> Result<()> {
    // Start transaction
    let tx = conn.unchecked_transaction()?;

    // Remove all existing tags for this bookmark
    tx.execute(
        "DELETE FROM bookmark_tags WHERE bookmark_id = ?1",
        params![bookmark_id],
    )?;

    // Add new tags
    for title in tag_titles {
        let tag_id = get_or_create_tag(&tx, title)?;
        add_tag_to_bookmark(&tx, bookmark_id, tag_id)?;
    }

    tx.commit()?;
    Ok(())
}

pub fn delete_bookmark(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_bookmark_by_id(conn: &Connection, id: i64) -> Result<BookmarkWithTags> {
    let bookmark = conn.query_row(
        "SELECT id, title, url, note, content, created, changed FROM bookmarks WHERE id = ?1",
        params![id],
        |row| {
            Ok(Bookmark {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                url: row.get(2)?,
                note: row.get(3)?,
                content: row.get(4)?,
                created: row.get(5)?,
                changed: row.get(6)?,
            })
        },
    )?;

    let tags = get_tags_for_bookmark(conn, id)?;
    Ok(BookmarkWithTags { bookmark, tags })
}
