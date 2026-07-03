pub mod import;
pub mod models;
pub mod queries;
mod schema;
pub mod seed;

pub use import::ImportResult;
pub use models::{
    Bookmark, BookmarkWithTags, SortDirection, SortField, Tag, TagFilterMode, UpsertAction,
};

use rusqlite::{Connection, Result, Transaction};
use std::path::PathBuf;

const SCHEMA_VERSION: i32 = 1;

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

        // Enable WAL mode for better concurrency and set a busy timeout
        // to prevent SQLITE_BUSY errors when the UI and background threads access the DB.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

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

        // Run migrations
        let current_version: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if current_version < SCHEMA_VERSION {
            let tx = self.conn.transaction()?;
            Self::run_migrations(&tx, current_version)?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            tx.commit()?;
            eprintln!("Database migrated from v{current_version} to v{SCHEMA_VERSION}");
        }

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

    fn run_migrations(_tx: &Transaction, from_version: i32) -> Result<()> {
        for _version in (from_version + 1)..=SCHEMA_VERSION {
            // match _version {
            //     1 => { /* initial schema — no migration needed */ }
            //     _ => {}
            // }
        }
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

    pub fn count_bookmarks(&self) -> Result<i64> {
        queries::count_bookmarks(&self.conn)
    }

    pub fn insert_bookmark(&self, title: &str, url: &str, note: Option<&str>) -> Result<i64> {
        let mut bookmark = Bookmark::new(title.to_string(), url.to_string());
        bookmark.note = note.map(|s| s.to_string());
        queries::insert_bookmark(&self.conn, &bookmark)
    }

    pub fn find_bookmark_by_url(&self, url: &str) -> Result<Option<Bookmark>> {
        queries::find_bookmark_by_url(&self.conn, url)
    }

    /// Insert or update a bookmark by URL. Returns the bookmark id and what action was taken.
    /// - URL not found: creates a new bookmark → `(id, UpsertAction::Created)`
    /// - URL found and trashed: restores and updates metadata → `(id, UpsertAction::Restored)`
    /// - URL found and active: updates metadata → `(id, UpsertAction::Updated)`
    pub fn upsert_bookmark(
        &self,
        title: &str,
        url: &str,
        note: Option<&str>,
    ) -> Result<(i64, UpsertAction)> {
        let existing = self.find_bookmark_by_url(url)?;
        if let Some(bm) = existing {
            let id = bm.id.expect("bookmark id should be set");
            if bm.deleted {
                self.restore_bookmark(id)?;
                self.update_bookmark(id, title, url, note)?;
                Ok((id, UpsertAction::Restored))
            } else {
                self.update_bookmark(id, title, url, note)?;
                Ok((id, UpsertAction::Updated))
            }
        } else {
            let id = self.insert_bookmark(title, url, note)?;
            Ok((id, UpsertAction::Created))
        }
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

    pub fn get_favicon_hash_for_domain(&self, domain: &str) -> Result<Option<i32>> {
        queries::get_favicon_hash_for_domain(&self.conn, domain)
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

    pub fn clear_trashed_bookmarks(&self) -> Result<usize> {
        queries::clear_trashed_bookmarks(&self.conn)
    }
}
