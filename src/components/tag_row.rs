use crate::db::models::{Tag, UNTAGGED_TAG_ID};
use gtk::prelude::*;
use relm4::factory::{DynamicIndex, FactoryComponent};
use relm4::prelude::*;

#[derive(Debug, Clone)]
pub struct TagRow {
    pub tag: Tag,
    is_pinned: bool,
    pub is_editing: bool,
}

impl TagRow {
    pub fn display_title(&self) -> String {
        if self.tag.id == Some(UNTAGGED_TAG_ID) {
            self.tag.title.clone()
        } else {
            format!("#{}", self.tag.title)
        }
    }
}

#[derive(Debug)]
pub enum TagRowMsg {
    Clicked,
    StartEdit,
    SubmitEdit(String),
    CancelEdit,
}

#[derive(Debug)]
pub enum TagRowOutput {
    Toggle(i64),         // tag_id
    Rename(i64, String), // tag_id, new_title
    Delete(i64),         // tag_id
}

#[relm4::factory(pub)]
impl FactoryComponent for TagRow {
    type Init = (Tag, bool); // (tag, is_pinned)
    type Input = TagRowMsg;
    type Output = TagRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        root = gtk::ListBoxRow {
            set_activatable: true,

            #[watch]
            #[block_signal(activate_handler)]
            set_css_classes: if self.tag.id == Some(UNTAGGED_TAG_ID) {
                &["untagged-tag"]
            } else if self.is_pinned {
                &["accent-bg-color"]
            } else {
                &[]
            },

            connect_activate[sender] => move |_| {
                sender.input(TagRowMsg::Clicked);
            } @activate_handler,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_all: 8,

                #[name = "label"]
                gtk::Label {
                    #[watch]
                    set_label: &self.display_title(),
                    #[watch]
                    set_visible: !self.is_editing,
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                    #[watch]
                    set_css_classes: if self.tag.id == Some(UNTAGGED_TAG_ID) {
                        &["untagged-tag-label"]
                    } else {
                        &[]
                    },
                },

                #[name = "entry"]
                gtk::Entry {
                    #[watch]
                    set_text: &self.display_title(),
                    #[watch]
                    set_visible: self.is_editing,
                    set_hexpand: true,

                    connect_map => move |entry| {
                        entry.grab_focus();
                        // Put cursor at the end
                        entry.set_position(-1);
                    },

                    connect_activate[sender] => move |entry| {
                        let _ = sender.input_sender().send(TagRowMsg::SubmitEdit(entry.text().to_string()));
                    },

                    add_controller = gtk::EventControllerFocus {
                        connect_leave[sender, entry] => move |_| {
                            let _ = sender.input_sender().send(TagRowMsg::SubmitEdit(entry.text().to_string()));
                        }
                    },

                    add_controller = gtk::EventControllerKey {
                        connect_key_pressed[sender] => move |_, key, _, _| {
                            if key == gtk::gdk::Key::Escape {
                                let _ = sender.input_sender().send(TagRowMsg::CancelEdit);
                                return gtk::glib::Propagation::Stop;
                            }
                            gtk::glib::Propagation::Proceed
                        }
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            tag: init.0,
            is_pinned: init.1,
            is_editing: false,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            TagRowMsg::Clicked => {
                if !self.is_editing {
                    if let Some(tag_id) = self.tag.id {
                        let _ = sender.output(TagRowOutput::Toggle(tag_id));
                    }
                }
            }
            TagRowMsg::StartEdit => {
                if self.tag.id != Some(UNTAGGED_TAG_ID) {
                    self.is_editing = true;
                }
            }
            TagRowMsg::SubmitEdit(new_title) => {
                if self.is_editing {
                    self.is_editing = false;
                    let title = new_title.trim().trim_start_matches('#').to_string();
                    if !title.is_empty() && title != self.tag.title {
                        if let Some(tag_id) = self.tag.id {
                            let _ = sender.output(TagRowOutput::Rename(tag_id, title));
                        }
                    }
                }
            }
            TagRowMsg::CancelEdit => {
                self.is_editing = false;
            }
        }
    }
}
