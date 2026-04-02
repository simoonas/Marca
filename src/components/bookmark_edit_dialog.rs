use crate::db::models::{Bookmark, Tag};
use adw::prelude::*;
use gtk::glib;
use relm4::prelude::*;

#[derive(Debug)]
pub struct BookmarkEditDialog {
    bookmark_id: Option<i64>,
    tags: Vec<String>,
    tags_container: gtk::FlowBox,
    tag_entry: gtk::Entry,
    title_entry: adw::EntryRow,
    url_entry: adw::EntryRow,
    note_view: gtk::TextView,
    dialog: adw::Dialog,
}

#[derive(Debug, Clone)]
pub struct BookmarkEditInit {
    pub bookmark: Option<Bookmark>,
    pub current_tags: Vec<Tag>,
    pub all_tags: Vec<Tag>,
}

#[derive(Debug)]
pub enum BookmarkEditMsg {
    AddTag(String),
    RemoveTag(String),
    Save,
    Cancel,
    FetchMetadata,
    MetadataFetched {
        title: String,
        description: Option<String>,
        had_error: bool,
    },
    MetadataFetchError(String),
}

#[derive(Debug)]
pub enum BookmarkEditOutput {
    SaveEdit {
        id: i64,
        title: String,
        url: String,
        note: Option<String>,
        tag_titles: Vec<String>,
    },
    SaveCreate {
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
            #[watch]
            set_title: if model.bookmark_id.is_some() { "Edit Bookmark" } else { "Create Bookmark" },
            set_content_width: 500,
            set_content_height: 450,

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                adw::HeaderBar {
                    set_show_end_title_buttons: false,
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
                            set_spacing: 15,

                            // Title field
                            adw::PreferencesGroup {
                                #[name = "title_entry"]
                                adw::EntryRow {
                                    set_title: "Title",
                                }
                            },

                            // URL field with fetch button
                            adw::PreferencesGroup {
                                #[name = "url_entry"]
                                adw::EntryRow {
                                    set_title: "URL",
                                    set_hexpand: true,

                                    add_suffix = &gtk::Button {
                                        set_icon_name: "view-refresh-symbolic",
                                        set_tooltip_text: Some("Fetch title and description from URL"),
                                        add_css_class: "flat",
                                        set_valign: gtk::Align::Center,
                                        connect_clicked => BookmarkEditMsg::FetchMetadata,
                                    }
                                }
                            },

                            // Note field
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
                            },

                            // Tags field
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                // Entry field (fixed at top)
                                #[name = "tag_entry"]
                                gtk::Entry {
                                    set_placeholder_text: Some("Add tag..."),
                                    set_hexpand: true,
                                },

                                // Pills container (wraps below)
                                #[name = "tags_container"]
                                gtk::FlowBox {
                                    set_selection_mode: gtk::SelectionMode::None,
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

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (bookmark_id, tags) = if let Some(ref bm) = init.bookmark {
            (
                Some(bm.id.unwrap()),
                init.current_tags.iter().map(|t| t.title.clone()).collect(),
            )
        } else {
            (None, vec![])
        };

        // Create temporary model for view_output!()
        let model = BookmarkEditDialog {
            bookmark_id,
            tags: tags.clone(),
            tags_container: gtk::FlowBox::new(),
            tag_entry: gtk::Entry::new(),
            title_entry: adw::EntryRow::new(),
            url_entry: adw::EntryRow::new(),
            note_view: gtk::TextView::new(),
            dialog: root.clone(),
        };

        let widgets = view_output!();

        // Set initial values in widgets only if editing
        if let Some(ref bookmark) = init.bookmark {
            widgets.title_entry.set_text(&bookmark.title);
            widgets.url_entry.set_text(&bookmark.url);

            let buffer = widgets.note_view.buffer();
            buffer.set_text(&bookmark.note.clone().unwrap_or_default());
        }

        // Setup autocomplete on tag entry
        let completion = gtk::EntryCompletion::new();
        let list_store = gtk::ListStore::new(&[glib::Type::STRING]);

        for tag in &init.all_tags {
            list_store.set(&list_store.append(), &[(0, &tag.title)]);
        }

        completion.set_model(Some(&list_store));
        completion.set_text_column(0);
        completion.set_inline_completion(true);
        completion.set_popup_completion(true);
        widgets.tag_entry.set_completion(Some(&completion));

        // Handle autocomplete selection → AddTag
        let input_sender = sender.input_sender().clone();
        completion.connect_match_selected(move |_, model, iter| {
            let tag = model.get::<String>(iter, 0);
            let _ = input_sender.send(BookmarkEditMsg::AddTag(tag));
            glib::Propagation::Stop
        });

        // Handle Enter key → AddTag
        let input_sender = sender.input_sender().clone();
        widgets.tag_entry.connect_activate(move |entry| {
            let text = entry.text().to_string();
            if !text.trim().is_empty() {
                let _ = input_sender.send(BookmarkEditMsg::AddTag(text.trim().to_string()));
            }
        });

        // Update model with real widget references
        let model = BookmarkEditDialog {
            tags_container: widgets.tags_container.clone(),
            tag_entry: widgets.tag_entry.clone(),
            title_entry: widgets.title_entry.clone(),
            url_entry: widgets.url_entry.clone(),
            note_view: widgets.note_view.clone(),
            dialog: root.clone(),
            ..model
        };

        // Build initial tag pills
        Self::rebuild_tag_pills(&model.tags_container, &tags, &sender);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BookmarkEditMsg::AddTag(tag) => {
                if !tag.is_empty() && !self.tags.contains(&tag) {
                    self.tags.push(tag);

                    // Clear the entry field
                    self.tag_entry.set_text("");

                    // Rebuild tag pills
                    Self::rebuild_tag_pills(&self.tags_container, &self.tags, &sender);
                }
            }

            BookmarkEditMsg::RemoveTag(tag) => {
                self.tags.retain(|t| t != &tag);

                // Rebuild tag pills
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

                // Send appropriate output based on mode
                if let Some(id) = self.bookmark_id {
                    sender
                        .output(BookmarkEditOutput::SaveEdit {
                            id,
                            title: title.trim().to_string(),
                            url: url.trim().to_string(),
                            note,
                            tag_titles: self.tags.clone(),
                        })
                        .unwrap();
                } else {
                    sender
                        .output(BookmarkEditOutput::SaveCreate {
                            title: title.trim().to_string(),
                            url: url.trim().to_string(),
                            note,
                            tag_titles: self.tags.clone(),
                        })
                        .unwrap();
                }
            }

            BookmarkEditMsg::FetchMetadata => {
                let url = self.url_entry.text().to_string();

                if url.trim().is_empty() {
                    sender
                        .output(BookmarkEditOutput::ValidationError(
                            "Enter a URL first".to_string(),
                        ))
                        .unwrap();
                    return;
                }

                // Spawn async task to fetch metadata
                let input_sender = sender.input_sender().clone();
                glib::MainContext::default().spawn_local(async move {
                    match crate::fetch_metadata::fetch_url_metadata(&url).await {
                        Ok((title, description, had_error)) => {
                            let _ = input_sender
                                .send(BookmarkEditMsg::MetadataFetched { title, description, had_error });
                        }
                        Err(e) => {
                            let _ = input_sender
                                .send(BookmarkEditMsg::MetadataFetchError(e.to_string()));
                        }
                    }
                });
            }

            BookmarkEditMsg::MetadataFetched { title, description, had_error } => {
                // Populate title if empty (don't override user input)
                if self.title_entry.text().is_empty() {
                    self.title_entry.set_text(&title);
                }

                // Populate note with description if empty
                if let Some(desc) = description {
                    let buffer = self.note_view.buffer();
                    let current = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                    if current.is_empty() {
                        buffer.set_text(&desc);
                    }
                }
                
                // Show error toast if we had to use fallbacks
                if had_error {
                    sender
                        .output(BookmarkEditOutput::ValidationError(
                            "Could not find page title or description".to_string(),
                        ))
                        .unwrap();
                }
            }

            BookmarkEditMsg::MetadataFetchError(msg) => {
                sender
                    .output(BookmarkEditOutput::ValidationError(format!(
                        "Could not fetch page: {}",
                        msg
                    )))
                    .unwrap();
            }

            BookmarkEditMsg::Cancel => {
                // Close dialog without saving
                self.dialog.close();
            }
        }
    }
}

impl BookmarkEditDialog {
    fn rebuild_tag_pills(
        container: &gtk::FlowBox,
        tags: &[String],
        sender: &ComponentSender<BookmarkEditDialog>,
    ) {
        // Clear all pills (NOT the entry, which is in a separate Box container)
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }

        // Recreate pills
        for tag in tags {
            let pill_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            pill_box.add_css_class("pill");
            pill_box.add_css_class("accent");
            pill_box.set_margin_all(2);
            pill_box.set_hexpand(false);
            pill_box.set_halign(gtk::Align::Start);

            let label = gtk::Label::new(Some(tag));
            label.set_margin_start(8);
            label.set_margin_end(2);
            label.set_margin_top(4);
            label.set_margin_bottom(4);
            label.set_hexpand(false);

            let close_btn = gtk::Button::new();
            close_btn.set_icon_name("window-close-symbolic");
            close_btn.add_css_class("flat");
            close_btn.add_css_class("circular");
            close_btn.set_margin_end(2);
            close_btn.set_valign(gtk::Align::Center);
            close_btn.set_hexpand(false);

            let tag_clone = tag.clone();
            let input_sender = sender.input_sender().clone();
            close_btn.connect_clicked(move |_| {
                let _ = input_sender.send(BookmarkEditMsg::RemoveTag(tag_clone.clone()));
            });

            pill_box.append(&label);
            pill_box.append(&close_btn);
            container.insert(&pill_box, -1);
        }
    }
}
