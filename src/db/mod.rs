pub mod models;
pub mod queries;
mod schema;

pub use models::{Bookmark, BookmarkWithTags, Tag};

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
        self.conn.execute(schema::CREATE_BOOKMARKS_FTS, [])?;

        // Create indexes
        for index_sql in schema::CREATE_INDEXES {
            self.conn.execute(index_sql, [])?;
        }

        // Create triggers
        for trigger_sql in schema::CREATE_TRIGGERS {
            self.conn.execute(trigger_sql, [])?;
        }

        eprintln!("Database schema initialized successfully");

        Ok(())
    }

    pub fn is_empty(&self) -> Result<bool> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM bookmarks", [], |row| row.get(0))?;
        Ok(count == 0)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn get_all_bookmarks(&self) -> Result<Vec<BookmarkWithTags>> {
        queries::get_all_bookmarks(&self.conn)
    }

    pub fn search_bookmarks(
        &self,
        query: Option<&str>,
        tag_ids: &[i64],
    ) -> Result<Vec<BookmarkWithTags>> {
        queries::search_bookmarks(&self.conn, query, tag_ids)
    }
}
