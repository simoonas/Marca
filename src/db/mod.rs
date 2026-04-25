pub mod import;
pub mod models;
pub mod queries;
mod schema;

pub use import::ImportResult;
pub use models::{Bookmark, BookmarkWithTags, SortDirection, SortField, Tag, TagFilterMode};

use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }

        eprintln!("Opening database at: {}", db_path.display());
        let conn = Connection::open(&db_path)?;

        let mut db = Self { conn };
        db.init_schema()?;

        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf> {
        let mut path = dirs::data_local_dir().ok_or_else(|| {
            rusqlite::Error::InvalidPath("Could not find local data directory".into())
        })?;

        path.push("marca");
        path.push("bookmarks.db");

        Ok(path)
    }

    fn init_schema(&mut self) -> Result<()> {
        eprintln!("Initializing database schema...");

        // Create tables
        self.conn.execute(schema::CREATE_BOOKMARKS_TABLE, [])?;
        self.conn.execute(schema::CREATE_TAGS_TABLE, [])?;
        self.conn.execute(schema::CREATE_BOOKMARK_TAGS_TABLE, [])?;
        self.conn.execute(schema::CREATE_FAVICONS_TABLE, [])?;
        self.conn.execute(schema::CREATE_BOOKMARKS_FTS, [])?;

        // Create indexes
        for index_sql in schema::CREATE_INDEXES {
            self.conn.execute(index_sql, [])?;
        }

        // Create triggers
        for trigger_sql in schema::CREATE_TRIGGERS {
            self.conn.execute(trigger_sql, [])?;
        }

        // Recreate with fixed version (no favicon cleanup)
        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS bookmarks_ad AFTER DELETE ON bookmarks BEGIN
                DELETE FROM bookmarks_fts WHERE rowid = old.id;
            END",
            [],
        )?;

        eprintln!("Database schema initialized successfully");

        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn get_all_bookmarks_with_sort(
        &self,
        sort_field: SortField,
        sort_direction: SortDirection,
    ) -> Result<Vec<BookmarkWithTags>> {
        queries::get_all_bookmarks(&self.conn, sort_field, sort_direction)
    }

    pub fn get_all_tags(&self) -> Result<Vec<Tag>> {
        queries::get_all_tags(&self.conn)
    }

    pub fn search_bookmarks_with_sort(
        &self,
        query: Option<&str>,
        tag_ids: &[i64],
        sort_field: SortField,
        sort_direction: SortDirection,
        tag_filter_mode: TagFilterMode,
    ) -> Result<Vec<BookmarkWithTags>> {
        queries::search_bookmarks(
            &self.conn,
            query,
            tag_ids,
            sort_field,
            sort_direction,
            tag_filter_mode,
        )
    }

    pub fn insert_bookmark(&self, title: &str, url: &str, note: Option<&str>) -> Result<i64> {
        let mut bookmark = Bookmark::new(title.to_string(), url.to_string());
        bookmark.note = note.map(|s| s.to_string());
        queries::insert_bookmark(&self.conn, &bookmark)
    }

    pub fn update_bookmark(
        &self,
        id: i64,
        title: &str,
        url: &str,
        note: Option<&str>,
    ) -> Result<()> {
        queries::update_bookmark(&self.conn, id, title, url, note)
    }

    pub fn update_bookmark_tags(&self, bookmark_id: i64, tag_titles: &[String]) -> Result<()> {
        queries::update_bookmark_tags(&self.conn, bookmark_id, tag_titles)
    }

    pub fn rename_tag(&self, id: i64, new_title: &str) -> Result<()> {
        queries::rename_tag(&self.conn, id, new_title)
    }

    pub fn delete_tag(&self, id: i64) -> Result<()> {
        queries::delete_tag(&self.conn, id)
    }

    pub fn delete_bookmark(&self, id: i64) -> Result<()> {
        queries::delete_bookmark(&self.conn, id)
    }

    pub fn restore_bookmark(&self, id: i64) -> Result<()> {
        queries::restore_bookmark(&self.conn, id)
    }

    pub fn get_bookmark_by_id(&self, id: i64) -> Result<BookmarkWithTags> {
        queries::get_bookmark_by_id(&self.conn, id)
    }

    pub fn insert_favicon_if_new(&self, hash: i32, data: &[u8]) -> Result<()> {
        queries::insert_favicon_if_new(&self.conn, hash, data)
    }

    pub fn update_bookmark_favicon_hash(&self, bookmark_id: i64, hash: i32) -> Result<()> {
        queries::update_bookmark_favicon_hash(&self.conn, bookmark_id, hash)
    }

    pub fn import_bookmarks(
        &self,
        bookmarks: Vec<(Bookmark, Vec<String>)>,
    ) -> Result<ImportResult> {
        import::import_bookmarks(&self.conn, bookmarks)
    }

    pub fn gc_deleted_bookmarks(&self, days: u32) -> Result<usize> {
        queries::gc_deleted_bookmarks(&self.conn, days)
    }
}
