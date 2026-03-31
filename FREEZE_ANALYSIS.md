# Edit Dialog Freeze Analysis - Marca

## Executive Summary
The edit dialog freezes due to **improper usage of `ComponentSender` in GTK signal handlers** within dynamically-created widgets in `rebuild_tags()`. The issue stems from multiple closures being created with cloned `ComponentSender` instances that may not properly propagate events through relm4's message queue.

---

## Issue #1: Signal Handlers in `rebuild_tags()` - CRITICAL

### Location: `src/components/bookmark_edit_dialog.rs`, lines 312-362

The `rebuild_tags()` function creates widgets **outside the view macro** with manual GTK signal handlers:

```rust
// Line 312-316: close_btn.connect_clicked
let tag_clone = tag.clone();
let sender_clone = sender.clone();
close_btn.connect_clicked(move |_| {
    sender_clone.input(BookmarkEditMsg::RemoveTag(tag_clone.clone()));
});
```

**Problems:**
1. **Manual sender cloning** for dynamically-created widgets
2. **No relm4 integration** - these aren't using the `[sender]` syntax from the view macro
3. **Potential message queue blocking** - direct `.input()` calls may not yield control back to GTK main loop
4. **Multiple signal handlers created per rebuild** - every time tags are modified, new closures are created with new sender clones

### Similar Issues at Lines 344-359:

```rust
// Line 344-349: completion.connect_match_selected
let sender_clone = sender.clone();
completion.connect_match_selected(move |_, model, iter| {
    let tag = model.get::<String>(iter, 0);
    sender_clone.input(BookmarkEditMsg::AddTag(tag));
    glib::Propagation::Stop
});

// Line 351-354: entry.connect_changed
let sender_clone = sender.clone();
entry.connect_changed(move |entry| {
    sender_clone.input(BookmarkEditMsg::TagEntryChanged(entry.text().to_string()));
});

// Line 356-359: entry.connect_activate
let sender_clone = sender.clone();
entry.connect_activate(move |_| {
    sender_clone.input(BookmarkEditMsg::TagEntryActivate);
});
```

---

## Issue #2: NoteView Buffer Handler - Lines 207-210

```rust
let sender_clone = sender.clone();
buffer.connect_changed(move |_buffer| {
    sender_clone.input(BookmarkEditMsg::NoteChanged);
});
```

**Problem:** While this is in `init()`, it still uses manual cloning instead of relm4's macro integration.

**Impact:** Every keystroke in the note field sends a message, and if the event loop is blocked by the tag entry handlers, this queue up and freeze the UI.

---

## Issue #3: View Macro Handlers vs Manual Handlers Mismatch

### Working Pattern (from `app.rs` lines 100-102):
```rust
adw::EntryRow {
    set_title: "Title",
    #[watch]
    set_text: &model.title,
    connect_changed[sender] => move |entry| {
        sender.input(BookmarkEditMsg::TitleChanged(entry.text().to_string()));
    }
}
```

This uses relm4's macro integration with `[sender]` syntax.

### Broken Pattern (in `rebuild_tags()`):
```rust
let sender_clone = sender.clone();
entry.connect_changed(move |entry| {
    sender_clone.input(BookmarkEditMsg::TagEntryChanged(entry.text().to_string()));
});
```

This bypasses relm4's macro system entirely.

---

## Why This Causes a Freeze

1. **GTK Main Loop Blocking**: When entry text changes in the tag field:
   - Signal handler fires
   - Calls `sender_clone.input()` directly
   - If relm4's event queue is processing updates from other handlers, this may not yield

2. **Cascading Updates**: Changing a tag triggers:
   - `TagEntryChanged` message
   - Model update in `update()`
   - `rebuild_tags()` call
   - Creates NEW widgets with NEW closures
   - All old closures still exist (memory leak + handler spam)

3. **Completion Handler Blocking**: Line 348 returns `glib::Propagation::Stop`:
   ```rust
   completion.connect_match_selected(move |_, model, iter| {
       let tag = model.get::<String>(iter, 0);
       sender_clone.input(BookmarkEditMsg::AddTag(tag));
       glib::Propagation::Stop  // <-- Blocks propagation
   });
   ```
   This stops event propagation, potentially preventing GTK from updating its internal state.

---

## Best Practice in relm4

### For View Macro Widgets (lines 100-115):
✅ **CORRECT** - Use relm4's built-in macro support:
```rust
connect_changed[sender] => move |entry| {
    sender.input(BookmarkEditMsg::TitleChanged(entry.text().to_string()));
}
```

### For Dynamically-Created Widgets:
Since you can't use the view macro for dynamically-created widgets, use this pattern:

```rust
use relm4::Sender;  // or ComponentSender<BookmarkEditDialog>

fn rebuild_tags(
    container: &gtk::FlowBox,
    tags: &[String],
    all_tags: &[Tag],
    sender: &ComponentSender<BookmarkEditDialog>,
) {
    // Clear existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    // ... tag creation ...

    // KEY: Use sender.input_sender() for GTK signal handlers
    for tag in tags {
        let pill_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        // ... setup ...

        let close_btn = gtk::Button::new();
        
        // Use input_sender() to get a thread-safe sender
        let sender = sender.input_sender();
        let tag_clone = tag.clone();
        
        close_btn.connect_clicked(move |_| {
            let _ = sender.send(BookmarkEditMsg::RemoveTag(tag_clone.clone()));
        });
        
        // ... rest of setup ...
    }
}
```

### Key Differences:
1. Use `sender.input_sender()` instead of `sender.clone()` for GTK signals
2. `input_sender()` returns a `glib::Sender` that's GTK-thread-safe
3. Use `.send()` instead of `.input()` (returns Result)
4. Avoid Propagation::Stop unless absolutely necessary

---

## Root Cause Summary

| Issue | Location | Severity | Fix |
|-------|----------|----------|-----|
| Manual sender cloning in GTK handlers | Lines 312-316, 344-359 | CRITICAL | Use `sender.input_sender()` |
| Widgets created outside view macro | `rebuild_tags()` | HIGH | Redesign to use view macro or proper sender handling |
| Buffer handler not using relm4 integration | Lines 207-210 | MEDIUM | Use `input_sender()` |
| Event propagation stopped | Line 348 | MEDIUM | Return `Propagation::Continue` or let default behavior |
| Multiple handler creation | `rebuild_tags()` called repeatedly | HIGH | Store widgets or prevent rebuilds |

---

## Recommended Fix Strategy

1. **Priority 1**: Replace all manual `sender.clone()` with `sender.input_sender()`
2. **Priority 2**: Change `.input()` calls to `.send()` (handles thread safety)
3. **Priority 3**: Consider if `rebuild_tags()` is called too frequently
4. **Priority 4**: Add `.ok()` or `?` to `.send()` calls to handle errors gracefully
5. **Priority 5**: Review if completion handler needs to stop propagation

