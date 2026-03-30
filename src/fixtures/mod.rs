mod sample_data;

use crate::db::{queries, Database};
use rusqlite::Result;

pub fn load_sample_data(db: &Database) -> Result<()> {
    eprintln!("Loading sample bookmark data...");

    let conn = db.conn();
    let bookmarks = sample_data::generate_sample_bookmarks();

    for (bookmark, tag_names) in bookmarks {
        // Insert bookmark
        let bookmark_id = queries::insert_bookmark(conn, &bookmark)?;
        eprintln!(
            "  Inserted bookmark: {} (id: {})",
            bookmark.title, bookmark_id
        );

        // Insert/get tags and link them
        for tag_name in tag_names {
            let tag_id = queries::get_or_create_tag(conn, &tag_name)?;
            queries::add_tag_to_bookmark(conn, bookmark_id, tag_id)?;
        }
    }

    eprintln!("Sample data loaded successfully!");
    Ok(())
}
