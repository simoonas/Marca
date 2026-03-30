use crate::db::Bookmark;

pub fn generate_sample_bookmarks() -> Vec<(Bookmark, Vec<String>)> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    vec![
        (
            Bookmark {
                id: None,
                title: "The Rust Programming Language".to_string(),
                url: "https://doc.rust-lang.org/book/".to_string(),
                note: Some(
                    "The official Rust book - comprehensive guide to learning Rust".to_string(),
                ),
                content: None,
                created: now - 86400 * 20, // 20 days ago
                changed: now - 86400 * 20,
            },
            vec![
                "rust".to_string(),
                "docs".to_string(),
                "tutorial".to_string(),
            ],
        ),
        (
            Bookmark {
                id: None,
                title: "Tokio - Asynchronous Rust Runtime".to_string(),
                url: "https://tokio.rs/".to_string(),
                note: Some(
                    "Event-driven, non-blocking I/O platform for writing asynchronous applications"
                        .to_string(),
                ),
                content: None,
                created: now - 86400 * 15,
                changed: now - 86400 * 15,
            },
            vec![
                "rust".to_string(),
                "library".to_string(),
                "async".to_string(),
            ],
        ),
        (
            Bookmark {
                id: None,
                title: "Serde - Serialization Framework".to_string(),
                url: "https://serde.rs/".to_string(),
                note: Some(
                    "Serde is a framework for serializing and deserializing Rust data structures"
                        .to_string(),
                ),
                content: None,
                created: now - 86400 * 12,
                changed: now - 86400 * 12,
            },
            vec!["rust".to_string(), "library".to_string()],
        ),
        (
            Bookmark {
                id: None,
                title: "Relm4 Book".to_string(),
                url: "https://relm4.org/book/stable/".to_string(),
                note: Some("Build native applications with Rust and GTK 4".to_string()),
                content: None,
                created: now - 86400 * 10,
                changed: now - 86400 * 10,
            },
            vec![
                "rust".to_string(),
                "docs".to_string(),
                "gtk".to_string(),
                "gui".to_string(),
            ],
        ),
        (
            Bookmark {
                id: None,
                title: "GTK 4 API Documentation".to_string(),
                url: "https://docs.gtk.org/gtk4/".to_string(),
                note: Some("Official GTK 4 reference documentation".to_string()),
                content: None,
                created: now - 86400 * 8,
                changed: now - 86400 * 8,
            },
            vec!["gtk".to_string(), "docs".to_string(), "api".to_string()],
        ),
        (
            Bookmark {
                id: None,
                title: "Rust by Example".to_string(),
                url: "https://doc.rust-lang.org/rust-by-example/".to_string(),
                note: Some("Learn Rust with examples (Live code editor included)".to_string()),
                content: None,
                created: now - 86400 * 6,
                changed: now - 86400 * 6,
            },
            vec![
                "rust".to_string(),
                "tutorial".to_string(),
                "examples".to_string(),
            ],
        ),
        (
            Bookmark {
                id: None,
                title: "SQLite FTS5 Extension".to_string(),
                url: "https://www.sqlite.org/fts5.html".to_string(),
                note: Some("Full-text search extension for SQLite".to_string()),
                content: None,
                created: now - 86400 * 5,
                changed: now - 86400 * 5,
            },
            vec![
                "docs".to_string(),
                "database".to_string(),
                "search".to_string(),
            ],
        ),
        (
            Bookmark {
                id: None,
                title: "Awesome Rust".to_string(),
                url: "https://github.com/rust-unofficial/awesome-rust".to_string(),
                note: Some("A curated list of Rust code and resources".to_string()),
                content: None,
                created: now - 86400 * 3,
                changed: now - 86400 * 3,
            },
            vec![
                "rust".to_string(),
                "github".to_string(),
                "resources".to_string(),
            ],
        ),
        (
            Bookmark {
                id: None,
                title: "GNOME Human Interface Guidelines".to_string(),
                url: "https://developer.gnome.org/hig/".to_string(),
                note: Some("Design patterns and guidelines for GNOME applications".to_string()),
                content: None,
                created: now - 86400 * 2,
                changed: now - 86400 * 2,
            },
            vec!["gnome".to_string(), "design".to_string(), "gui".to_string()],
        ),
        (
            Bookmark {
                id: None,
                title: "Libadwaita Demo".to_string(),
                url: "https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/".to_string(),
                note: Some("Building blocks for modern GNOME applications".to_string()),
                content: None,
                created: now - 86400,
                changed: now - 86400,
            },
            vec![
                "gnome".to_string(),
                "gtk".to_string(),
                "docs".to_string(),
                "library".to_string(),
            ],
        ),
    ]
}
