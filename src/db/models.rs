#[derive(Debug, Clone)]
pub struct Bookmark {
    pub id: Option<i64>,
    pub title: String,
    pub url: String,
    pub note: Option<String>,
    pub content: Option<String>,
    pub created: i64,
    pub changed: i64,
    pub favicon_hash: Option<i32>,
}

impl Bookmark {
    pub fn new(title: String, url: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            id: None,
            title,
            url,
            note: None,
            content: None,
            created: now,
            changed: now,
            favicon_hash: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: Option<i64>,
    pub title: String,
}

impl Tag {
    pub fn new(title: String) -> Self {
        Self { id: None, title }
    }
}

#[derive(Debug, Clone)]
pub struct BookmarkWithTags {
    pub bookmark: Bookmark,
    pub tags: Vec<Tag>,
    pub favicon_data: Option<Vec<u8>>,
}
