use crate::db::models::{Tag, UNTAGGED_TAG_ID};
use gtk::prelude::*;
use relm4::factory::{DynamicIndex, FactoryComponent};
use relm4::prelude::*;

#[derive(Debug, Clone)]
pub struct TagRow {
    tag: Tag,
    is_pinned: bool,
}

#[derive(Debug)]
pub enum TagRowMsg {
    Clicked,
}

#[derive(Debug)]
pub enum TagRowOutput {
    Toggle(i64), // tag_id
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

                gtk::Label {
                    #[watch]
                    set_label: &self.tag.title,
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                    #[watch]
                    set_css_classes: if self.tag.id == Some(UNTAGGED_TAG_ID) {
                        &["untagged-tag-label"]
                    } else {
                        &[]
                    },
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            tag: init.0,
            is_pinned: init.1,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            TagRowMsg::Clicked => {
                if let Some(tag_id) = self.tag.id {
                    let _ = sender.output(TagRowOutput::Toggle(tag_id));
                }
            }
        }
    }
}
