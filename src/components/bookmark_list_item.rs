use crate::db::models::Bookmark;
use crate::db::{BookmarkWithTags, Tag};
use adw::gtk;
use adw::gtk::gdk;
use adw::gtk::gdk::gdk_pixbuf;
use adw::gtk::prelude::*;
use relm4::typed_view::list::RelmListItem;
use relm4::RelmWidgetExt;
use std::io::Cursor;

/// Data for a single bookmark in the virtualized list
#[derive(Clone, Debug)]
pub struct BookmarkListItem {
    pub bookmark: Bookmark,
    pub tags: Vec<Tag>,
    pub favicon_texture: Option<gdk::Texture>,
}

impl BookmarkListItem {
    pub fn from_bookmark_with_tags(bwt: BookmarkWithTags) -> Self {
        let favicon_texture = bwt.favicon_data.as_ref().and_then(|data| {
            let cursor = Cursor::new(data.clone());
            gdk_pixbuf::Pixbuf::from_read(cursor)
                .ok()
                .and_then(|pixbuf| {
                    let target_size = 48;
                    let scaled = if pixbuf.width() > target_size || pixbuf.height() > target_size {
                        pixbuf.scale_simple(
                            target_size,
                            target_size,
                            gdk_pixbuf::InterpType::Bilinear,
                        )
                    } else {
                        Some(pixbuf)
                    };
                    scaled.map(|pb| gdk::Texture::for_pixbuf(&pb))
                })
        });

        Self {
            bookmark: bwt.bookmark,
            tags: bwt.tags,
            favicon_texture,
        }
    }
}

/// Widgets that are set up once and reused for each list item
pub struct BookmarkListItemWidgets {
    pub favicon_picture: gtk::Picture,
    pub favicon_placeholder: gtk::Image,
    pub title_label: gtk::Label,
    pub url_label: gtk::Label,
    pub tags_box: gtk::Box,
}

impl RelmListItem for BookmarkListItem {
    type Root = gtk::Box;
    type Widgets = BookmarkListItemWidgets;

    fn setup(_list_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("card");
        root.set_margin_top(3);
        root.set_margin_bottom(3);
        root.set_margin_start(0);
        root.set_margin_end(0);

        let main_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        main_box.set_margin_all(8);

        // Favicon icon on left
        let favicon_picture = gtk::Picture::new();
        favicon_picture.add_css_class("favicon-icon");
        favicon_picture.set_can_shrink(true);

        let favicon_placeholder = gtk::Image::builder()
            .icon_name("image-missing-symbolic")
            .pixel_size(32)
            .build();
        favicon_placeholder.add_css_class("dim-label");

        // Content box (title, URL, tags)
        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        content_box.set_hexpand(true);

        // Title
        let title_label = gtk::Label::new(None);
        title_label.add_css_class("title-4");
        title_label.set_halign(gtk::Align::Start);
        title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        content_box.append(&title_label);

        // URL and tags container
        let url_tags_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);

        let url_label = gtk::Label::new(None);
        url_label.add_css_class("dim-label");
        url_label.add_css_class("caption");
        url_label.set_halign(gtk::Align::Start);
        url_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        url_tags_box.append(&url_label);

        // Tags badges container
        let tags_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        tags_box.set_halign(gtk::Align::Start);
        url_tags_box.append(&tags_box);

        content_box.append(&url_tags_box);

        main_box.append(&favicon_picture);
        main_box.append(&content_box);

        root.append(&main_box);

        (
            root,
            BookmarkListItemWidgets {
                favicon_picture,
                favicon_placeholder,
                title_label,
                url_label,
                tags_box,
            },
        )
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        // Update title
        let truncated_title = if self.bookmark.title.len() > 100 {
            format!("{}...", &self.bookmark.title[..100])
        } else {
            self.bookmark.title.clone()
        };
        widgets.title_label.set_label(&truncated_title);

        // Update URL
        let truncated_url = if self.bookmark.url.len() > 50 {
            format!("{}...", &self.bookmark.url[..50])
        } else {
            self.bookmark.url.clone()
        };
        widgets.url_label.set_label(&truncated_url);

        // Update favicon
        if let Some(ref texture) = self.favicon_texture {
            widgets.favicon_picture.set_paintable(Some(texture));
            widgets.favicon_picture.set_visible(true);
            widgets.favicon_placeholder.set_visible(false);
        } else {
            widgets.favicon_picture.set_visible(false);
            widgets.favicon_placeholder.set_visible(true);
        }

        // Clear existing tags
        while let Some(child) = widgets.tags_box.first_child() {
            widgets.tags_box.remove(&child);
        }

        // Add tag badges
        if !self.tags.is_empty() {
            let max_visible_tags = 3;
            let visible_count = std::cmp::min(max_visible_tags, self.tags.len());

            for (idx, tag) in self.tags.iter().enumerate() {
                if idx < visible_count {
                    let badge = gtk::Label::new(Some(&format!("#{}", tag.title)));
                    badge.add_css_class("tag-badge");
                    badge.add_css_class("accent");
                    badge.set_margin_start(2);
                    badge.set_margin_end(2);
                    widgets.tags_box.append(&badge);
                }
            }

            // Show "+X" if there are more tags
            if self.tags.len() > visible_count {
                let remaining = self.tags.len() - visible_count;
                let more_label = gtk::Label::new(Some(&format!("+{}", remaining)));
                more_label.add_css_class("tag-badge");
                more_label.add_css_class("accent");
                more_label.set_margin_start(2);
                more_label.set_margin_end(2);
                widgets.tags_box.append(&more_label);
            }

            widgets.tags_box.set_visible(true);
        } else {
            widgets.tags_box.set_visible(false);
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        // Clean up if needed
        while let Some(child) = widgets.tags_box.first_child() {
            widgets.tags_box.remove(&child);
        }
    }
}
