pub const CREATE_BOOKMARKS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    note TEXT,
    content TEXT,
    created INTEGER NOT NULL,
    changed INTEGER NOT NULL,
    favicon_hash INTEGER,
    deleted BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (favicon_hash) REFERENCES favicons(hash)
)";

pub const CREATE_TAGS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL UNIQUE
)";

pub const CREATE_BOOKMARK_TAGS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS bookmark_tags (
    bookmark_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (bookmark_id, tag_id),
    FOREIGN KEY (bookmark_id) REFERENCES bookmarks(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
)";

pub const CREATE_FAVICONS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS favicons (
    hash INTEGER PRIMARY KEY,
    favicon BLOB NOT NULL
)";

pub const CREATE_BOOKMARKS_FTS: &str = "
CREATE VIRTUAL TABLE IF NOT EXISTS bookmarks_fts USING fts5(
    title,
    note,
    url,
    content='bookmarks',
    content_rowid='id',
    tokenize='trigram'
)";

pub const CREATE_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_bookmark_tags_bookmark ON bookmark_tags(bookmark_id)",
    "CREATE INDEX IF NOT EXISTS idx_bookmark_tags_tag ON bookmark_tags(tag_id)",
    "CREATE INDEX IF NOT EXISTS idx_bookmark_tags_tag_bookmark ON bookmark_tags(tag_id, bookmark_id)",
    "CREATE INDEX IF NOT EXISTS idx_bookmarks_created ON bookmarks(created DESC)",
    "CREATE INDEX IF NOT EXISTS idx_bookmarks_changed ON bookmarks(changed DESC)",
    "CREATE INDEX IF NOT EXISTS idx_bookmarks_deleted ON bookmarks(deleted)",
    "CREATE INDEX IF NOT EXISTS idx_bookmarks_favicon_hash ON bookmarks(favicon_hash)",
];

pub const CREATE_TRIGGERS: &[&str] = &[
    "CREATE TRIGGER IF NOT EXISTS bookmarks_ai AFTER INSERT ON bookmarks BEGIN
        INSERT INTO bookmarks_fts(rowid, title, note, url)
        VALUES (new.id, new.title, new.note, new.url);
    END",
    "CREATE TRIGGER IF NOT EXISTS bookmarks_ad AFTER DELETE ON bookmarks BEGIN
        DELETE FROM bookmarks_fts WHERE rowid = old.id;
    END",
    "CREATE TRIGGER IF NOT EXISTS bookmarks_au AFTER UPDATE ON bookmarks BEGIN
        UPDATE bookmarks_fts 
        SET title = new.title, note = new.note, url = new.url
        WHERE rowid = new.id;
    END",
];
// TODO:
// CREATE TRIGGER cleanup_unused_tags
// AFTER DELETE/UPDATE ON bookmark_tags
// BEGIN
//     DELETE FROM tags
//     WHERE id = OLD.tag_id
//     AND NOT EXISTS (
//         SELECT 1 FROM bookmark_tags WHERE tag_id = OLD.tag_id
//     );
// END;
