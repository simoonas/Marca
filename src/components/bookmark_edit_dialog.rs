use crate::db::models::{Bookmark, Tag};
use adw::prelude::*;
use gtk::glib;
use relm4::prelude::*;

#[derive(Debug)]
pub struct BookmarkEditDialog {
    bookmark_id: Option<i64>,
    tags: Vec<String>,
    all_tags: Vec<String>,
    suggestions: Vec<String>,
    tags_container: gtk::FlowBox,
    tag_entry: gtk::Entry,
    title_entry: adw::EntryRow,
    url_entry: adw::EntryRow,
    note_view: gtk::TextView,
    dialog: adw::Dialog,
    suggestion_popover: gtk::Popover,
    suggestion_list: gtk::ListBox,
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
    TagInputChanged(String),
    SelectSuggestion(String),
    NextSuggestion,
    PrevSuggestion,
    AcceptSuggestion,
    CloseSuggestions,
    Save,
    Cancel,
    DoSaveAfterMetadata {
        title: String,
        url: String,
        note: Option<String>,
        tag_titles: Vec<String>,
        is_edit: bool,
        bookmark_id: Option<i64>,
    },
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

            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sender] => move |_, key, _, state| {
                    if key == gtk::gdk::Key::Return && state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                        let _ = sender.input_sender().send(BookmarkEditMsg::Save);
                        return gtk::glib::Propagation::Stop;
                    }
                    gtk::glib::Propagation::Proceed
                }
            },

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                adw::HeaderBar {
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 6,

                            gtk::Label {
                                set_label: "Cancel",
                            },

                            gtk::Label {
                                set_label: "Esc",
                                add_css_class: "dim-label",
                            }
                        },

                        connect_clicked => BookmarkEditMsg::Cancel,
                    },

                    pack_end = &gtk::Button {
                        add_css_class: "suggested-action",
                        connect_clicked => BookmarkEditMsg::Save,

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 6,

                            gtk::Label {
                                set_label: "Save",
                            },

                            gtk::Label {
                                set_label: "Ctrl+Enter",
                                add_css_class: "dim-label",
                            }
                        }
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
                                    connect_realize => move |row| {
                                        row.grab_focus();
                                    },
                                }
                            },

                            // URL field
                            adw::PreferencesGroup {
                                #[name = "url_entry"]
                                adw::EntryRow {
                                    set_title: "URL",
                                    set_hexpand: true,
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

                                    connect_changed[sender] => move |entry| {
                                        sender.input(BookmarkEditMsg::TagInputChanged(entry.text().to_string()));
                                    },

                                    add_controller = gtk::EventControllerFocus {
                                        connect_enter[sender] => move |_| {
                                            sender.input(BookmarkEditMsg::TagInputChanged("".to_string()));
                                        },
                                        connect_leave[sender] => move |_| {
                                            sender.input(BookmarkEditMsg::CloseSuggestions);
                                        }
                                    },

                                    add_controller = gtk::EventControllerKey {
                                        connect_key_pressed[sender] => move |_, key, _, _| {
                                            match key {
                                                gtk::gdk::Key::Down | gtk::gdk::Key::Tab => {
                                                    sender.input(BookmarkEditMsg::NextSuggestion);
                                                    gtk::glib::Propagation::Stop
                                                }
                                                gtk::gdk::Key::Up => {
                                                    sender.input(BookmarkEditMsg::PrevSuggestion);
                                                    gtk::glib::Propagation::Stop
                                                }
                                                gtk::gdk::Key::Return => {
                                                    sender.input(BookmarkEditMsg::AcceptSuggestion);
                                                    gtk::glib::Propagation::Stop
                                                }
                                                _ => gtk::glib::Propagation::Proceed,
                                            }
                                        }
                                    },
                                },

                                #[name = "suggestion_popover"]
                                gtk::Popover {
                                    set_parent: &tag_entry,
                                    set_autohide: false,
                                    set_can_focus: false,
                                    set_has_arrow: false,
                                    set_halign: gtk::Align::Fill,
                                    set_hexpand: true,
                                    add_css_class: "suggestion-popover",

                                    gtk::ScrolledWindow {
                                        set_propagate_natural_height: true,
                                        set_max_content_height: 200,
                                        set_hscrollbar_policy: gtk::PolicyType::Never,
                                        set_vscrollbar_policy: gtk::PolicyType::Automatic,
                                        set_hexpand: true,

                                        #[name = "suggestion_list"]
                                        gtk::ListBox {
                                            set_selection_mode: gtk::SelectionMode::Single,
                                            set_hexpand: true,
                                            add_css_class: "suggestion-list",
                                            connect_row_activated[sender] => move |_, row| {
                                                if let Some(label) = row.child().and_then(|c| c.downcast::<gtk::Label>().ok()) {
                                                    sender.input(BookmarkEditMsg::SelectSuggestion(label.label().to_string()));
                                                }
                                            }
                                        }
                                    }
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
            all_tags: init.all_tags.iter().map(|t| t.title.clone()).collect(),
            suggestions: vec![],
            tags_container: gtk::FlowBox::new(),
            tag_entry: gtk::Entry::new(),
            title_entry: adw::EntryRow::new(),
            url_entry: adw::EntryRow::new(),
            note_view: gtk::TextView::new(),
            dialog: root.clone(),
            suggestion_popover: gtk::Popover::new(),
            suggestion_list: gtk::ListBox::new(),
        };

        let widgets = view_output!();

        // Set initial values in widgets only if editing
        if let Some(ref bookmark) = init.bookmark {
            widgets.title_entry.set_text(&bookmark.title);
            widgets.url_entry.set_text(&bookmark.url);

            let buffer = widgets.note_view.buffer();
            buffer.set_text(&bookmark.note.clone().unwrap_or_default());
        }

        // Handle Enter key → AddTag
        let input_sender = sender.input_sender().clone();
        widgets.tag_entry.connect_activate(move |_| {
            let _ = input_sender.send(BookmarkEditMsg::AcceptSuggestion);
        });

        // Update model with real widget references
        let model = BookmarkEditDialog {
            tags_container: widgets.tags_container.clone(),
            tag_entry: widgets.tag_entry.clone(),
            title_entry: widgets.title_entry.clone(),
            url_entry: widgets.url_entry.clone(),
            note_view: widgets.note_view.clone(),
            dialog: root.clone(),
            suggestion_popover: widgets.suggestion_popover.clone(),
            suggestion_list: widgets.suggestion_list.clone(),
            ..model
        };

        // Build initial tag pills
        Self::rebuild_tag_pills(&model.tags_container, &tags, &sender);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BookmarkEditMsg::TagInputChanged(input) => {
                let input_lower = input.trim().to_lowercase();

                self.suggestions = self
                    .all_tags
                    .iter()
                    .filter(|t| t.to_lowercase().contains(&input_lower))
                    .filter(|t| !self.tags.contains(t))
                    .cloned()
                    .collect();

                if self.suggestions.is_empty() {
                    self.suggestion_popover.popdown();
                } else {
                    // Rebuild suggestion list
                    while let Some(child) = self.suggestion_list.first_child() {
                        self.suggestion_list.remove(&child);
                    }

                    for suggestion in &self.suggestions {
                        let label = gtk::Label::builder()
                            .label(suggestion)
                            .halign(gtk::Align::Start)
                            .hexpand(true)
                            .xalign(0.0)
                            .ellipsize(gtk::pango::EllipsizeMode::End)
                            .build();
                        self.suggestion_list.append(&label);
                    }

                    self.suggestion_list.select_row(None::<&gtk::ListBoxRow>);
                    self.suggestion_popover.popup();
                }
            }

            BookmarkEditMsg::NextSuggestion => {
                if self.suggestion_popover.is_visible() {
                    if let Some(row) = self.suggestion_list.selected_row() {
                        let index = row.index();
                        if let Some(next_row) = self.suggestion_list.row_at_index(index + 1) {
                            self.suggestion_list.select_row(Some(&next_row));
                        } else {
                            // Wrap to first
                            self.suggestion_list
                                .select_row(self.suggestion_list.row_at_index(0).as_ref());
                        }
                    } else {
                        self.suggestion_list
                            .select_row(self.suggestion_list.row_at_index(0).as_ref());
                    }
                }
            }

            BookmarkEditMsg::PrevSuggestion => {
                if self.suggestion_popover.is_visible() {
                    if let Some(row) = self.suggestion_list.selected_row() {
                        let index = row.index();
                        if index > 0 {
                            if let Some(prev_row) = self.suggestion_list.row_at_index(index - 1) {
                                self.suggestion_list.select_row(Some(&prev_row));
                            }
                        } else {
                            // Wrap to last
                            let mut last_index = 0;
                            while self.suggestion_list.row_at_index(last_index + 1).is_some() {
                                last_index += 1;
                            }
                            self.suggestion_list
                                .select_row(self.suggestion_list.row_at_index(last_index).as_ref());
                        }
                    } else {
                        // Select last
                        let mut last_index = 0;
                        while self.suggestion_list.row_at_index(last_index + 1).is_some() {
                            last_index += 1;
                        }
                        self.suggestion_list
                            .select_row(self.suggestion_list.row_at_index(last_index).as_ref());
                    }
                }
            }

            BookmarkEditMsg::AcceptSuggestion => {
                if self.suggestion_popover.is_visible() {
                    if let Some(row) = self.suggestion_list.selected_row() {
                        if let Some(label) =
                            row.child().and_then(|c| c.downcast::<gtk::Label>().ok())
                        {
                            sender.input(BookmarkEditMsg::SelectSuggestion(
                                label.label().to_string(),
                            ));
                        }
                    } else {
                        // Just activate entry
                        let text = self.tag_entry.text().to_string();
                        if !text.trim().is_empty() {
                            sender.input(BookmarkEditMsg::AddTag(text.trim().to_string()));
                        }
                    }
                } else {
                    // Normal enter
                    let text = self.tag_entry.text().to_string();
                    if !text.trim().is_empty() {
                        sender.input(BookmarkEditMsg::AddTag(text.trim().to_string()));
                    }
                }
            }

            BookmarkEditMsg::CloseSuggestions => {
                self.suggestion_popover.popdown();
            }

            BookmarkEditMsg::SelectSuggestion(tag) => {
                sender.input(BookmarkEditMsg::AddTag(tag));
                self.suggestion_popover.popdown();
            }

            BookmarkEditMsg::AddTag(tag) => {
                if !tag.is_empty() && !self.tags.contains(&tag) {
                    self.tags.push(tag);

                    // Clear the entry field
                    self.tag_entry.set_text("");
                    self.suggestion_popover.popdown();

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
                let mut url = self.url_entry.text().to_string();

                // Validation
                if url.trim().is_empty() {
                    sender
                        .output(BookmarkEditOutput::ValidationError(
                            "URL cannot be empty".to_string(),
                        ))
                        .unwrap();
                    return;
                }

                // Normalize URL: add http:// if no schema present
                let url_trimmed = url.trim();
                let has_schema = regex::Regex::new(r"^[a-zA-Z0-9+.-]*://")
                    .map(|re| re.is_match(url_trimmed))
                    .unwrap_or(false);
                if !has_schema {
                    url = format!("https://{}", url_trimmed);
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

                let tag_titles = self.tags.clone();
                let is_edit = self.bookmark_id.is_some();
                let bookmark_id = self.bookmark_id;

                // Check if we need to fetch metadata (if title OR note is empty)
                if !title.trim().is_empty() && note.is_some() {
                    // Both title and note provided - save immediately (fast path)
                    if is_edit {
                        if let Some(id) = bookmark_id {
                            sender
                                .output(BookmarkEditOutput::SaveEdit {
                                    id,
                                    title: title.trim().to_string(),
                                    url: url.trim().to_string(),
                                    note,
                                    tag_titles,
                                })
                                .unwrap();
                        }
                    } else {
                        sender
                            .output(BookmarkEditOutput::SaveCreate {
                                title: title.trim().to_string(),
                                url: url.trim().to_string(),
                                note,
                                tag_titles,
                            })
                            .unwrap();
                    }
                    return;
                }

                // Title OR note is empty - fetch metadata with 5s timeout
                let input_sender = sender.input_sender().clone();
                let url_clone = url.clone();
                glib::MainContext::default().spawn_local(async move {
                    match crate::fetch_metadata::fetch_quick_metadata(&url_clone).await {
                        Ok(metadata) => {
                            let final_title = if title.trim().is_empty() {
                                if metadata.title.trim().is_empty() {
                                    // Use URL as fallback title
                                    url_clone.clone()
                                } else {
                                    metadata.title
                                }
                            } else {
                                title.trim().to_string()
                            };

                            let final_note = if note.is_none() {
                                metadata.description
                            } else {
                                note
                            };

                            let _ = input_sender.send(BookmarkEditMsg::DoSaveAfterMetadata {
                                title: final_title,
                                url: url.trim().to_string(),
                                note: final_note,
                                tag_titles,
                                is_edit,
                                bookmark_id,
                            });
                        }
                        Err(_e) => {
                            // Timeout or error - use title if provided, otherwise URL
                            let final_title = if title.trim().is_empty() {
                                url_clone.clone()
                            } else {
                                title.trim().to_string()
                            };
                            let _ = input_sender.send(BookmarkEditMsg::DoSaveAfterMetadata {
                                title: final_title,
                                url: url.trim().to_string(),
                                note,
                                tag_titles,
                                is_edit,
                                bookmark_id,
                            });
                        }
                    }
                });
            }

            BookmarkEditMsg::DoSaveAfterMetadata {
                title,
                url,
                note,
                tag_titles,
                is_edit,
                bookmark_id,
            } => {
                // Send appropriate output based on mode
                if is_edit {
                    if let Some(id) = bookmark_id {
                        sender
                            .output(BookmarkEditOutput::SaveEdit {
                                id,
                                title,
                                url,
                                note,
                                tag_titles,
                            })
                            .unwrap();
                    }
                } else {
                    sender
                        .output(BookmarkEditOutput::SaveCreate {
                            title,
                            url,
                            note,
                            tag_titles,
                        })
                        .unwrap();
                }
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
            pill_box.set_css_classes(&["accent", "tag", "pill"]);
            pill_box.set_margin_all(2);
            pill_box.set_hexpand(false);
            pill_box.set_halign(gtk::Align::Start);

            let escaped = gtk::glib::markup_escape_text(tag).to_string();
            let styled = escaped.replace("/", "<span alpha=\"55%\">\u{2009}/\u{2009}</span>");
            let label = gtk::Label::builder()
                .use_markup(true)
                .label(&styled)
                .build();
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
