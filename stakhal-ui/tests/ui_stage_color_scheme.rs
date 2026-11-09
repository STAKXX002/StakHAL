use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use stakhal_ui::{apply_skin, ui};
use std::rc::Rc;

#[test]
fn test_stage_skin_swap_sets_force_light_color_scheme() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping color scheme test: {err}");
        return;
    }
    let Some(display) = gdk::Display::default() else {
        eprintln!("GDK default display not available, skipping color scheme test");
        return;
    };
    let _ = adw::init();

    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);

    let provider = gtk4::CssProvider::new();
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
    while glib::MainContext::default().iteration(false) {}
    assert!(
        style_manager.is_dark(),
        "Default skin startup must stay in dark color scheme"
    );

    apply_skin(&provider, ui::tokens::Palette::STAGE);
    while glib::MainContext::default().iteration(false) {}
    assert!(
        !style_manager.is_dark(),
        "Stage skin swap must switch Adwaita StyleManager to light scheme (is_dark == false)"
    );
    assert_eq!(
        style_manager.color_scheme(),
        adw::ColorScheme::ForceLight,
        "Stage skin swap must set ColorScheme::ForceLight"
    );

    // Also verify SkinOverlayController triggers ForceLight through the swap callback
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
    assert!(style_manager.is_dark());

    let window = adw::ApplicationWindow::builder().build();
    let skin_overlay = gtk4::Overlay::new();
    let header_overlay = gtk4::Overlay::new();
    window.set_content(Some(&skin_overlay));

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    let provider_swap = provider.clone();
    let ctrl = ui::skin_overlay::SkinOverlayController::setup(
        &window,
        &skin_overlay,
        &header_overlay,
        Rc::new(move || {
            apply_skin(&provider_swap, ui::tokens::Palette::STAGE);
        }),
    );
    ctrl.trigger_unlock();
    while glib::MainContext::default().iteration(false) {}
    assert!(
        !style_manager.is_dark(),
        "SkinOverlayController unlock swap must set Adwaita light scheme (is_dark == false)"
    );

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }
    window.close();
    gtk4::style_context_remove_provider_for_display(&display, &provider);
    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
}
