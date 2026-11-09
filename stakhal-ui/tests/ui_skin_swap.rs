use gtk4::gdk;
use gtk4::prelude::*;
use stakhal_ui::{apply_skin, ui};

#[test]
fn test_skin_swap_resolved_color() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping skin swap test: {err}");
        return;
    }
    let Some(display) = gdk::Display::default() else {
        eprintln!("GDK default display not available, skipping skin swap test");
        return;
    };

    let provider = gtk4::CssProvider::new();
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
    assert_eq!(ui::tokens::palette(), ui::tokens::Palette::DEFAULT);
    let win = gtk4::Window::new();
    let color_default = win.color();

    apply_skin(&provider, ui::tokens::Palette::STAGE);
    assert_eq!(ui::tokens::palette(), ui::tokens::Palette::STAGE);
    let color_stage = win.color();

    assert_ne!(
        color_default, color_stage,
        "Widget resolved color must change after applying Stage skin"
    );

    gtk4::style_context_remove_provider_for_display(&display, &provider);
    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
}
