use crate::db::models::{SortDirection, SortField};
use crate::db::{Bookmark, BookmarkWithTags, Tag};
use rusqlite::{params, Connection, OptionalExtension, Result};

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

pub fn insert_tag(conn: &Connection, tag: &Tag) -> Result<i64> {
    conn.execute("INSERT INTO tags (title) VALUES (?1)", params![tag.title])?;
    Ok(conn.last_insert_rowid())
}

pub fn get_or_create_tag(conn: &Connection, title: &str) -> Result<i64> {
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

fn map_bookmark_with_favicon(row: &rusqlite::Row) -> Result<(Bookmark, Option<Vec<u8>>)> {
    let bookmark = Bookmark {
        id: Some(row.get(0)?),
        title: row.get(1)?,
        url: row.get(2)?,
        note: row.get(3)?,
        content: row.get(4)?,
        created: row.get(5)?,
        changed: row.get(6)?,
        favicon_hash: row.get(8)?,
    };
    let favicon_data: Option<Vec<u8>> = row.get(7)?;
    Ok((bookmark, favicon_data))
}

fn load_bookmark_with_tags(
    conn: &Connection,
    bookmark: Bookmark,
    favicon_data: Option<Vec<u8>>,
) -> Result<BookmarkWithTags> {
    let bookmark_id = bookmark.id.unwrap();
    let tags = get_tags_for_bookmark(conn, bookmark_id)?;
    Ok(BookmarkWithTags {
        bookmark,
        tags,
        favicon_data,
    })
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

    let query = format!(
        "SELECT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash
         FROM bookmarks b
         LEFT JOIN favicons f ON b.favicon_hash = f.hash
         {}",
        order_clause
    );

    let mut stmt = conn.prepare(&query)?;

    let bookmark_iter = stmt.query_map([], map_bookmark_with_favicon)?;

    let mut results = Vec::new();
    for result in bookmark_iter {
        let (bookmark, favicon_data) = result?;
        results.push(load_bookmark_with_tags(conn, bookmark, favicon_data)?);
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
    sort_field: SortField,
    sort_direction: SortDirection,
) -> Result<Vec<BookmarkWithTags>> {
    if query.is_none() && tag_ids.is_empty() {
        return get_all_bookmarks(conn, sort_field, sort_direction);
    }

    let mut results = Vec::new();

    if let Some(search_text) = query {
        // Enclose query in quotes for exact substring match across multiple words, escaping internal quotes
        let fts_query = format!("\"{}\"", search_text.replace("\"", "\"\""));

        // Determine order clause based on sort field
        let order_clause = if sort_field == SortField::Relevance {
            "ORDER BY rank".to_string()
        } else {
            format!(
                "ORDER BY b.{} {}",
                sort_field.column_name(),
                sort_direction.sql_keyword()
            )
        };

        if tag_ids.is_empty() {
            let query_str = format!(
                "SELECT DISTINCT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash
                 FROM bookmarks b
                 JOIN bookmarks_fts fts ON b.id = fts.rowid
                 LEFT JOIN favicons f ON b.favicon_hash = f.hash
                 WHERE bookmarks_fts MATCH ?1
                 {}",
                order_clause
            );

            let mut stmt = conn.prepare(&query_str)?;

            let bookmark_iter = stmt.query_map(params![fts_query], map_bookmark_with_favicon)?;

            for result in bookmark_iter {
                let (bookmark, favicon_data) = result?;
                results.push(load_bookmark_with_tags(conn, bookmark, favicon_data)?);
            }
        } else {
            let tag_ids_json = serde_json::to_string(tag_ids).unwrap();
            let query_str = format!(
                "SELECT DISTINCT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash
                 FROM bookmarks b
                 JOIN bookmarks_fts fts ON b.id = fts.rowid
                 JOIN bookmark_tags bt ON b.id = bt.bookmark_id
                 LEFT JOIN favicons f ON b.favicon_hash = f.hash
                 WHERE bookmarks_fts MATCH ?1 AND bt.tag_id IN (SELECT value FROM json_each(?2))
                 GROUP BY b.id
                 HAVING COUNT(DISTINCT bt.tag_id) = ?3
                 {}",
                order_clause
            );

            let mut stmt = conn.prepare(&query_str)?;

            let bookmark_iter = stmt.query_map(
                params![fts_query, tag_ids_json, tag_ids.len()],
                map_bookmark_with_favicon,
            )?;

            for result in bookmark_iter {
                let (bookmark, favicon_data) = result?;
                results.push(load_bookmark_with_tags(conn, bookmark, favicon_data)?);
            }
        }
    } else {
        let tag_ids_json = serde_json::to_string(tag_ids).unwrap();
        let order_clause = format!(
            "ORDER BY b.{} {}",
            sort_field.column_name(),
            sort_direction.sql_keyword()
        );

        let query_str = format!(
            "SELECT DISTINCT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash
             FROM bookmarks b
             JOIN bookmark_tags bt ON b.id = bt.bookmark_id
             LEFT JOIN favicons f ON b.favicon_hash = f.hash
             WHERE bt.tag_id IN (SELECT value FROM json_each(?1))
             GROUP BY b.id
             HAVING COUNT(DISTINCT bt.tag_id) = ?2
             {}",
            order_clause
        );

        let mut stmt = conn.prepare(&query_str)?;

        let bookmark_iter = stmt.query_map(
            params![tag_ids_json, tag_ids.len()],
            map_bookmark_with_favicon,
        )?;

        for result in bookmark_iter {
            let (bookmark, favicon_data) = result?;
            results.push(load_bookmark_with_tags(conn, bookmark, favicon_data)?);
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
    Ok(())
}

pub fn delete_bookmark(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_bookmark_by_id(conn: &Connection, id: i64) -> Result<BookmarkWithTags> {
    let (bookmark, favicon_data) = conn.query_row(
        "SELECT b.id, b.title, b.url, b.note, b.content, b.created, b.changed, f.favicon, b.favicon_hash
         FROM bookmarks b
         LEFT JOIN favicons f ON b.favicon_hash = f.hash
         WHERE b.id = ?1",
        params![id],
        map_bookmark_with_favicon,
    )?;

    load_bookmark_with_tags(conn, bookmark, favicon_data)
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
