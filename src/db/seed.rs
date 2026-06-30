use rusqlite::Result;

use super::{Bookmark, Database, queries};

pub fn seed_sample_data(db: &Database) -> Result<()> {
    let conn = db.conn();

    let samples: Vec<(&str, &str, Option<&str>, Vec<&str>)> = vec![
        (
            "Marca GitHub",
            "https://github.com/simoonas/Marca",
            Some("Marca GitHub repository"),
            vec!["bookmarking"],
        ),
        (
            "Wikipedia",
            "https://en.wikipedia.org/",
            Some("The free encyclopedia"),
            vec!["wiki"],
        ),
    ];

    let count = samples.len();

    for (title, url, note, tags) in samples {
        let mut bookmark = Bookmark::new(title.to_string(), url.to_string());
        bookmark.note = note.map(|s| s.to_string());
        let id = queries::insert_bookmark(conn, &bookmark)?;
        let tag_strings: Vec<String> = tags.into_iter().map(|s| s.to_string()).collect();
        queries::update_bookmark_tags(conn, id, &tag_strings)?;
    }

    eprintln!("Seeded {} sample bookmarks", count);

    Ok(())
}
