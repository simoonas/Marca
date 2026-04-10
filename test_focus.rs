use gtk::prelude::*;
fn test() {
    let entry = gtk::Entry::new();
    entry.connect_activate(|entry| {
        if let Some(root) = entry.root() {
            if let Ok(window) = root.downcast::<gtk::Window>() {
                window.set_focus(None::<&gtk::Widget>);
            }
        }
    });
}
