use gtk4::gdk;
use gtk4::prelude::*;
use stakhal_ui::is_text_input_focused;

#[test]
fn test_is_text_input_focused_rules() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping text-input focus rule test: {err}");
        return;
    }
    if gdk::Display::default().is_none() {
        eprintln!("GDK default display not available, skipping text-input focus rule test");
        return;
    }

    // Entry -> true (implements gtk4::Editable, including internal child walking up to Entry)
    let entry = gtk4::Entry::new();
    assert!(
        is_text_input_focused(Some(entry.upcast_ref())),
        "Entry must be treated as text input"
    );
    if let Some(child) = entry.first_child() {
        assert!(
            is_text_input_focused(Some(&child)),
            "Internal child of Entry must walk up to Editable and return true"
        );
    }

    // Editable TextView -> true
    let editable_tv = gtk4::TextView::builder().editable(true).build();
    assert!(
        is_text_input_focused(Some(editable_tv.upcast_ref())),
        "Editable TextView must be treated as text input"
    );

    // Read-only TextView -> false
    let readonly_tv = gtk4::TextView::builder().editable(false).build();
    assert!(
        !is_text_input_focused(Some(readonly_tv.upcast_ref())),
        "Read-only TextView must not block sequence detection"
    );

    // Button (and child label inside Button) -> false
    let button = gtk4::Button::with_label("Action");
    assert!(
        !is_text_input_focused(Some(button.upcast_ref())),
        "Button must not block sequence detection"
    );
    if let Some(lbl) = button.child() {
        assert!(
            !is_text_input_focused(Some(&lbl)),
            "Child of Button must not block sequence detection"
        );
    }

    // No focused widget -> false
    assert!(
        !is_text_input_focused(None),
        "No focused widget must return false"
    );
}
