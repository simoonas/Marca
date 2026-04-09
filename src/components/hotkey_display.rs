use adw::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct HotkeyAction {
    pub id: usize, // Action ID to identify which button was clicked
    pub label: String,
    pub accelerator: String,
}

pub struct HotkeyDisplay {
    actions: Vec<HotkeyAction>,
    root: gtk::Box,
}

#[derive(Debug)]
pub enum HotkeyDisplayMsg {
    UpdateActions(Vec<HotkeyAction>),
    ActionClicked(usize), // id of the clicked action
}

#[derive(Debug, Clone, Copy)]
pub enum HotkeyDisplayOutput {
    ActionTriggered(usize), // id of the triggered action
}

#[relm4::component(pub)]
impl SimpleComponent for HotkeyDisplay {
    type Init = ();
    type Input = HotkeyDisplayMsg;
    type Output = HotkeyDisplayOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 6,
            set_homogeneous: false,
            set_margin_start: 6,
            set_margin_end: 6,
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let initial_actions = vec![HotkeyAction {
            id: 0,
            label: "Search bookmarks".to_string(),
            accelerator: "<Ctrl>l".to_string(),
        }];

        let mut model = HotkeyDisplay {
            actions: initial_actions.clone(),
            root: root.clone(),
        };

        let widgets = view_output!();

        // Initial population
        let sender_clone = _sender.clone();
        Self::populate_actions(&root, &model.actions, move |id| {
            sender_clone.input(HotkeyDisplayMsg::ActionClicked(id));
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            HotkeyDisplayMsg::UpdateActions(actions) => {
                self.actions = actions;
                let sender_clone = sender.clone();
                Self::populate_actions(&self.root, &self.actions, move |id| {
                    sender_clone.input(HotkeyDisplayMsg::ActionClicked(id));
                });
            }
            HotkeyDisplayMsg::ActionClicked(id) => {
                let _ = sender.output(HotkeyDisplayOutput::ActionTriggered(id));
            }
        }
    }
}

impl HotkeyDisplay {
    fn populate_actions<F>(root: &gtk::Box, actions: &[HotkeyAction], on_click: F)
    where
        F: Fn(usize) + 'static,
    {
        let on_click = Rc::new(on_click);

        // Clear existing children
        while let Some(child) = root.first_child() {
            child.unparent();
        }

        // Add new action buttons
        for action in actions {
            let button = gtk::Button::new();
            button.add_css_class("flat");

            let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            container.set_homogeneous(false);

            let label = gtk::Label::new(Some(&action.label));
            label.add_css_class("caption-heading");
            label.set_halign(gtk::Align::Center);
            label.set_margin_end(3);
            container.append(&label);

            let shortcut_label = adw::ShortcutLabel::new(&action.accelerator);
            shortcut_label.add_css_class("hotkey-shortcut");
            container.append(&shortcut_label);

            button.set_child(Some(&container));

            let action_id = action.id;
            let on_click_clone = on_click.clone();
            button.connect_clicked(move |_| {
                on_click_clone(action_id);
            });

            root.append(&button);
        }
    }
}
