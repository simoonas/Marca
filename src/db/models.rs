/// Virtual tag ID for "Untagged" bookmarks (bookmarks with no tags)
pub const UNTAGGED_TAG_ID: i64 = -1;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortField {
    Relevance, // Only available when searching
    Created,
    Modified,
    Title,
    Url,
}

impl SortField {
    pub fn column_name(&self) -> &'static str {
        match self {
            Self::Relevance => "rank",
            Self::Created => "created",
            Self::Modified => "changed",
            Self::Title => "title",
            Self::Url => "url",
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Title | Self::Url)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Relevance => "Relevance",
            Self::Created => "Created",
            Self::Modified => "Modified",
            Self::Title => "Title",
            Self::Url => "URL",
        }
    }

    pub fn next(&self, has_query: bool) -> Self {
        match self {
            Self::Relevance => Self::Created,
            Self::Created => Self::Modified,
            Self::Modified => Self::Title,
            Self::Title => Self::Url,
            Self::Url => {
                if has_query {
                    Self::Relevance
                } else {
                    Self::Created
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub fn icon(&self, is_text: bool) -> &'static str {
        if is_text {
            match self {
                Self::Ascending => "A→Z",
                Self::Descending => "Z→A",
            }
        } else {
            match self {
                Self::Ascending => "↑",
                Self::Descending => "↓",
            }
        }
    }

    pub fn sql_keyword(&self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TagFilterMode {
    All, // AND - show bookmarks with ALL selected tags
    Any, // OR - show bookmarks with ANY of the selected tags
}

impl TagFilterMode {
    pub fn toggle(&self) -> Self {
        match self {
            Self::All => Self::Any,
            Self::Any => Self::All,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Any => "any",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::All => "Bookmarks matching all selected tags",
            Self::Any => "Bookmarks matching any selected tags",
        }
    }
}
