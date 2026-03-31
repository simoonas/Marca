use crate::db::models::{Bookmark, Tag};
use adw::prelude::*;
use gtk::glib;
use relm4::prelude::*;

#[derive(Debug)]
pub struct BookmarkEditDialog {
    bookmark_id: i64,
    tags: Vec<String>,
    all_tags: Vec<Tag>,
    tags_container: gtk::FlowBox,
    tag_entry: gtk::Entry,
    title_entry: adw::EntryRow,
    url_entry: adw::EntryRow,
    note_view: gtk::TextView,
}

#[derive(Debug, Clone)]
pub struct BookmarkEditInit {
    pub bookmark: Bookmark,
    pub current_tags: Vec<Tag>,
    pub all_tags: Vec<Tag>,
}

#[derive(Debug)]
pub enum BookmarkEditMsg {
    AddTag(String),
    RemoveTag(String),
    Save,
    Cancel,
}

#[derive(Debug)]
pub enum BookmarkEditOutput {
    Save {
        id: i64,
        title: String,
        url: String,
        note: Option<String>,
        tag_titles: Vec<String>,
    },
    ValidationError(String),
}

#[relm4::component(pub)]
impl SimpleComponent for BookmarkEditDialog {
    type Init = BookmarkEditInit;
    type Input = BookmarkEditMsg;
    type Output = BookmarkEditOutput;

    view! {
        #[root]
        adw::Dialog {
            set_title: "Edit Bookmark",
            set_content_width: 500,
            set_content_height: 450,

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                adw::HeaderBar {
                    pack_start = &gtk::Button {
                        set_label: "Cancel",
                        connect_clicked => BookmarkEditMsg::Cancel,
                    },

                    pack_end = &gtk::Button {
                        set_label: "Save",
                        add_css_class: "suggested-action",
                        connect_clicked => BookmarkEditMsg::Save,
                    }
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vscrollbar_policy: gtk::PolicyType::Automatic,

                    adw::Clamp {
                        set_maximum_size: 500,
                        set_margin_all: 24,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 18,

                            // Title field
                            adw::PreferencesGroup {
                                #[name = "title_entry"]
                                adw::EntryRow {
                                    set_title: "Title",
                                }
                            },

                            // URL field
                            adw::PreferencesGroup {
                                #[name = "url_entry"]
                                adw::EntryRow {
                                    set_title: "URL",
                                }
                            },

                            // Note field
                            adw::PreferencesGroup {
                                set_title: "Note",

                                gtk::Frame {
                                    set_margin_top: 6,
                                    set_margin_bottom: 6,
                                    set_margin_start: 12,
                                    set_margin_end: 12,

                                    #[name = "note_view"]
                                    gtk::TextView {
                                        set_wrap_mode: gtk::WrapMode::Word,
                                        set_accepts_tab: false,
                                        set_top_margin: 8,
                                        set_bottom_margin: 8,
                                        set_left_margin: 8,
                                        set_right_margin: 8,
                                        set_height_request: 100,
                                        add_css_class: "card",
                                    }
                                }
                            },

                            // Tags field
                            adw::PreferencesGroup {
                                set_title: "Tags",

                                gtk::Frame {
                                    set_margin_top: 6,
                                    set_margin_bottom: 6,
                                    set_margin_start: 12,
                                    set_margin_end: 12,
                                    add_css_class: "card",

                                    #[name = "tags_container"]
                                    gtk::FlowBox {
                                        set_selection_mode: gtk::SelectionMode::None,
                                        set_margin_all: 8,
                                        set_column_spacing: 6,
                                        set_row_spacing: 6,
                                        set_min_children_per_line: 1,
                                        set_max_children_per_line: 10,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let bookmark_id = init.bookmark.id.unwrap();
        let tags: Vec<String> = init.current_tags.iter().map(|t| t.title.clone()).collect();

        // Create temporary model for view_output!()
        let model = BookmarkEditDialog {
            bookmark_id,
            tags: tags.clone(),
            all_tags: init.all_tags.clone(),
            tags_container: gtk::FlowBox::new(),
            tag_entry: gtk::Entry::new(),
            title_entry: adw::EntryRow::new(),
            url_entry: adw::EntryRow::new(),
            note_view: gtk::TextView::new(),
        };

        let widgets = view_output!();

        // Set initial values in widgets (one-time only)
        widgets.title_entry.set_text(&init.bookmark.title);
        widgets.url_entry.set_text(&init.bookmark.url);

        let buffer = widgets.note_view.buffer();
        buffer.set_text(&init.bookmark.note.clone().unwrap_or_default());

        // Create persistent tag entry with autocomplete
        let tag_entry = Self::create_tag_entry(&init.all_tags, &sender);

        // Update model with real widget references
        let model = BookmarkEditDialog {
            tags_container: widgets.tags_container.clone(),
            tag_entry: tag_entry.clone(),
            title_entry: widgets.title_entry.clone(),
            url_entry: widgets.url_entry.clone(),
            note_view: widgets.note_view.clone(),
            ..model
        };

        // Build initial tag pills
        Self::rebuild_tag_pills(&model.tags_container, &tags, &sender);

        // Add persistent entry to container (at the end)
        model.tags_container.insert(&tag_entry, -1);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BookmarkEditMsg::AddTag(tag) => {
                if !tag.is_empty() && !self.tags.contains(&tag) {
                    self.tags.push(tag);

                    // Clear the entry field
                    self.tag_entry.set_text("");

                    // Rebuild only the tag pills (not the entry)
                    Self::rebuild_tag_pills(&self.tags_container, &self.tags, &sender);
                }
            }

            BookmarkEditMsg::RemoveTag(tag) => {
                self.tags.retain(|t| t != &tag);

                // Rebuild only the tag pills
                Self::rebuild_tag_pills(&self.tags_container, &self.tags, &sender);
            }

            BookmarkEditMsg::Save => {
                // Read all values from widgets
                let title = self.title_entry.text().to_string();
                let url = self.url_entry.text().to_string();

                // Validation
                if title.trim().is_empty() {
                    sender
                        .output(BookmarkEditOutput::ValidationError(
                            "Title cannot be empty".to_string(),
                        ))
                        .unwrap();
                    return;
                }

                if url.trim().is_empty() {
                    sender
                        .output(BookmarkEditOutput::ValidationError(
                            "URL cannot be empty".to_string(),
                        ))
                        .unwrap();
                    return;
                }

                // Read note from TextView buffer
                let buffer = self.note_view.buffer();
                let start = buffer.start_iter();
                let end = buffer.end_iter();
                let note_text = buffer.text(&start, &end, false).to_string();
                let note = if note_text.trim().is_empty() {
                    None
                } else {
                    Some(note_text)
                };

                // Send output to parent
                sender
                    .output(BookmarkEditOutput::Save {
                        id: self.bookmark_id,
                        title: title.trim().to_string(),
                        url: url.trim().to_string(),
                        note,
                        tag_titles: self.tags.clone(),
                    })
                    .unwrap();
            }

            BookmarkEditMsg::Cancel => {
                // Dialog closes without saving
                // Parent handles cleanup
            }
        }
    }
}

impl BookmarkEditDialog {
    fn create_tag_entry(
        all_tags: &[Tag],
        sender: &ComponentSender<BookmarkEditDialog>,
    ) -> gtk::Entry {
        let entry = gtk::Entry::new();
        entry.set_placeholder_text(Some("Add tag..."));
        entry.set_hexpand(true);

        // Setup autocompletion
        let completion = gtk::EntryCompletion::new();
        let list_store = gtk::ListStore::new(&[glib::Type::STRING]);

        for tag in all_tags {
            list_store.set(&list_store.append(), &[(0, &tag.title)]);
        }

        completion.set_model(Some(&list_store));
        completion.set_text_column(0);
        completion.set_inline_completion(true);
        completion.set_popup_completion(true);
        entry.set_completion(Some(&completion));

        // Handle autocomplete selection → AddTag
        let input_sender = sender.input_sender().clone();
        completion.connect_match_selected(move |_, model, iter| {
            let tag = model.get::<String>(iter, 0);
            let _ = input_sender.send(BookmarkEditMsg::AddTag(tag));
            glib::Propagation::Stop
        });

        // Handle Enter key → AddTag
        let input_sender = sender.input_sender().clone();
        entry.connect_activate(move |entry| {
            let text = entry.text().to_string();
            if !text.trim().is_empty() {
                let _ = input_sender.send(BookmarkEditMsg::AddTag(text.trim().to_string()));
            }
        });

        entry
    }

    fn rebuild_tag_pills(
        container: &gtk::FlowBox,
        tags: &[String],
        sender: &ComponentSender<BookmarkEditDialog>,
    ) {
        // Remove only tag pill widgets (gtk::Box), NOT the entry field (gtk::Entry)
        let mut children_to_remove = Vec::new();
        let mut child = container.first_child();

        while let Some(widget) = child {
            // Tag pills are gtk::Box, entry is gtk::Entry
            if widget.is::<gtk::Box>() {
                children_to_remove.push(widget.clone());
            }
            child = widget.next_sibling();
        }

        // Remove old pill widgets
        for widget in children_to_remove {
            container.remove(&widget);
        }

        // Find index of entry field (insert pills before it)
        let entry_index = Self::find_entry_index(container);

        // Re-create pill widgets
        for (i, tag) in tags.iter().enumerate() {
            let pill_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            pill_box.add_css_class("pill");
            pill_box.add_css_class("accent");
            pill_box.set_margin_all(2);

            let label = gtk::Label::new(Some(tag));
            label.set_margin_start(8);
            label.set_margin_end(2);
            label.set_margin_top(4);
            label.set_margin_bottom(4);

            let close_btn = gtk::Button::new();
            close_btn.set_icon_name("window-close-symbolic");
            close_btn.add_css_class("flat");
            close_btn.add_css_class("circular");
            close_btn.set_margin_end(2);
            close_btn.set_valign(gtk::Align::Center);

            let tag_clone = tag.clone();
            let input_sender = sender.input_sender().clone();
            close_btn.connect_clicked(move |_| {
                let _ = input_sender.send(BookmarkEditMsg::RemoveTag(tag_clone.clone()));
            });

            pill_box.append(&label);
            pill_box.append(&close_btn);

            // Insert before entry field (entry stays at end)
            container.insert(&pill_box, entry_index + i as i32);
        }
    }

    fn find_entry_index(container: &gtk::FlowBox) -> i32 {
        let mut index = 0;
        let mut child = container.first_child();

        while let Some(widget) = child {
            if widget.is::<gtk::Entry>() {
                return index;
            }
            index += 1;
            child = widget.next_sibling();
        }

        -1 // Append if entry not found (shouldn't happen)
    }
}
