use crate::db::models::{Bookmark, BookmarkWithTags, Tag};
use adw::prelude::*;
use gtk::gdk;
use gtk::gdk_pixbuf;
use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender};
use relm4::prelude::*;
use std::io::Cursor;

#[derive(Debug)]
pub struct BookmarkRow {
    bookmark: Bookmark,
    tags: Vec<Tag>,
    hovered: bool,
    favicon_data: Option<Vec<u8>>,
}

impl BookmarkRow {
    /// Convert favicon data to a GdkTexture for display
    fn get_favicon_texture(&self) -> Option<gdk::Texture> {
        self.favicon_data.as_ref().and_then(|data| {
            let cursor = Cursor::new(data.clone());
            match gdk_pixbuf::Pixbuf::from_read(cursor) {
                Ok(pixbuf) => {
                    // Scale pixbuf to 48x48 if needed
                    let scaled = if pixbuf.width() != 48 || pixbuf.height() != 48 {
                        pixbuf.scale_simple(48, 48, gdk_pixbuf::InterpType::Bilinear)
                    } else {
                        Some(pixbuf)
                    };
                    scaled.map(|pb| gdk::Texture::for_pixbuf(&pb))
                }
                Err(_) => None,
            }
        })
    }
}

#[derive(Debug)]
pub enum BookmarkRowMsg {
    Clicked,
    HoverEnter,
    HoverLeave,
    EditClicked,
    DeleteClicked,
}

#[derive(Debug)]
pub enum BookmarkRowOutput {
    Open(String),
    Edit(i64),
    Delete(i64),
}

#[relm4::factory(pub)]
impl FactoryComponent for BookmarkRow {
    type Init = BookmarkWithTags;
    type Input = BookmarkRowMsg;
    type Output = BookmarkRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        root = gtk::ListBoxRow {
            set_activatable: true,
            set_selectable: false,
            add_css_class: "card",
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_margin_start: 0,
            set_margin_end: 0,

            connect_activate[sender] => move |_| {
                sender.input(BookmarkRowMsg::Clicked);
            },

            add_controller = gtk::EventControllerMotion {
                connect_enter[sender] => move |_, _, _| {
                    sender.input(BookmarkRowMsg::HoverEnter);
                },
                connect_leave[sender] => move |_| {
                    sender.input(BookmarkRowMsg::HoverLeave);
                },
            },

            gtk::Overlay {
                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_all: 12,

                    // Avatar on left with favicon or initials fallback
                    #[name = "avatar"]
                    adw::Avatar {
                        set_text: Some(&self.bookmark.title),
                        set_size: 48,
                        set_show_initials: true,
                    },

                    // Content box (title, URL, tags)
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 4,
                        set_hexpand: true,

                        // Title (top line)
                        gtk::Label {
                            set_label: &truncate_text(&self.bookmark.title, 100),
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.0,
                            add_css_class: "title-4",
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                        },

                        // URL and tags (bottom line)
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,
                            set_homogeneous: false,

                            // URL
                            gtk::Label {
                                set_label: &truncate_text(&self.bookmark.url, 50),
                                set_halign: gtk::Align::Start,
                                set_xalign: 0.0,
                                add_css_class: "dim-label",
                                add_css_class: "caption",
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                            },

                            // Tags badges container
                            #[name = "tags_box"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 4,
                                set_visible: !self.tags.is_empty(),
                                set_halign: gtk::Align::Start,
                            }
                        }
                    }
                },

                add_overlay = &gtk::Box {
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::Start,
                    set_margin_top: 6,
                    set_margin_end: 6,
                    set_spacing: 4,
                    #[watch]
                    set_visible: self.hovered,

                    gtk::Button {
                        set_icon_name: "document-edit-symbolic",
                        add_css_class: "flat",
                        add_css_class: "circular",
                        set_tooltip_text: Some("Edit bookmark"),
                        connect_clicked[sender] => move |_| {
                            sender.input(BookmarkRowMsg::EditClicked);
                        }
                    },

                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        add_css_class: "flat",
                        add_css_class: "circular",
                        set_tooltip_text: Some("Delete bookmark"),
                        connect_clicked[sender] => move |_| {
                            sender.input(BookmarkRowMsg::DeleteClicked);
                        }
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            bookmark: init.bookmark,
            tags: init.tags,
            hovered: false,
            favicon_data: init.favicon_data,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        // Set favicon as custom image on avatar if available
        if let Some(texture) = self.get_favicon_texture() {
            widgets.avatar.set_custom_image(Some(&texture));
            widgets.avatar.set_show_initials(false);
        }

        // Add compact tag badges - show as many as fit, then "+X"
        if !self.tags.is_empty() {
            // We'll show up to 3 tags, then "+X" for remaining
            let max_visible_tags = 3;
            let visible_count = std::cmp::min(max_visible_tags, self.tags.len());

            for (idx, tag) in self.tags.iter().enumerate() {
                if idx < visible_count {
                    // Create a simple label badge for each tag
                    let badge = gtk::Label::builder()
                        .label(&format!("#{}", tag.title))
                        .css_classes(vec!["tag-badge".to_string(), "accent".to_string()])
                        .build();
                    badge.set_margin_start(2);
                    badge.set_margin_end(2);
                    widgets.tags_box.append(&badge);
                }
            }

            // Show "+X" if there are more tags
            if self.tags.len() > visible_count {
                let remaining = self.tags.len() - visible_count;
                let more_label = gtk::Label::builder()
                    .label(&format!("+{}", remaining))
                    .css_classes(vec!["tag-badge".to_string(), "accent".to_string()])
                    .build();
                more_label.set_margin_start(2);
                more_label.set_margin_end(2);
                widgets.tags_box.append(&more_label);
            }
        }

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            BookmarkRowMsg::Clicked => {
                sender
                    .output(BookmarkRowOutput::Open(self.bookmark.url.clone()))
                    .unwrap();
            }
            BookmarkRowMsg::HoverEnter => {
                self.hovered = true;
            }
            BookmarkRowMsg::HoverLeave => {
                self.hovered = false;
            }
            BookmarkRowMsg::EditClicked => {
                if let Some(id) = self.bookmark.id {
                    sender.output(BookmarkRowOutput::Edit(id)).unwrap();
                }
            }
            BookmarkRowMsg::DeleteClicked => {
                if let Some(id) = self.bookmark.id {
                    sender.output(BookmarkRowOutput::Delete(id)).unwrap();
                }
            }
        }
    }
}

/// Truncate text to a maximum length with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}
