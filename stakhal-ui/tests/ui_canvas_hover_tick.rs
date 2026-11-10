use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_ui::state::AppState;
use stakhal_ui::ui::nucleo_pinout::ensure_pinout_hover_tick;
use stakhal_ui::ui::state_diagram::ensure_diagram_animation_tick;
use stakhal_ui::ui::tokens;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

struct PaletteResetGuard(tokens::Palette);
impl Drop for PaletteResetGuard {
    fn drop(&mut self) {
        tokens::set_palette(self.0);
    }
}

fn pump_until(mut cond: impl FnMut() -> bool, max_wait: Duration) {
    let start = Instant::now();
    while start.elapsed() < max_wait {
        while glib::MainContext::default().iteration(false) {}
        if cond() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    while glib::MainContext::default().iteration(false) {}
}

fn pump_for(duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        while glib::MainContext::default().iteration(false) {}
        std::thread::sleep(Duration::from_millis(10));
    }
    while glib::MainContext::default().iteration(false) {}
}

#[test]
fn test_canvas_hover_tick_settles_and_stops_cleanly_on_live_display() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping canvas hover tick test: {err}");
        return;
    }
    if gdk::Display::default().is_none() {
        eprintln!("GDK default display not available, skipping canvas hover tick test");
        return;
    }
    let _ = adw::init();

    let _guard = PaletteResetGuard(tokens::palette());
    tokens::set_palette(tokens::Palette::STAGE);
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    let state = Rc::new(RefCell::new(AppState::default()));
    let diagram_draws = Rc::new(Cell::new(0u32));
    let pinout_draws = Rc::new(Cell::new(0u32));

    let diagram_area = gtk4::DrawingArea::builder()
        .content_width(240)
        .content_height(160)
        .build();
    let dd_count = Rc::clone(&diagram_draws);
    diagram_area.set_draw_func(move |_, _, _, _| {
        dd_count.set(dd_count.get() + 1);
    });

    let pinout_area = gtk4::DrawingArea::builder()
        .content_width(240)
        .content_height(160)
        .build();
    let pd_count = Rc::clone(&pinout_draws);
    pinout_area.set_draw_func(move |_, _, _, _| {
        pd_count.set(pd_count.get() + 1);
    });

    let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    root.append(&diagram_area);
    root.append(&pinout_area);

    let win = adw::ApplicationWindow::builder()
        .default_width(520)
        .default_height(200)
        .content(&root)
        .build();
    win.present();

    pump_until(
        || diagram_area.is_mapped() && pinout_area.is_mapped(),
        Duration::from_secs(2),
    );
    pump_for(Duration::from_millis(80));

    // At idle before hover: zero tick callbacks active and draw count remains static
    assert!(state.borrow().with_canvas_state(|c| !c.diagram_tick_active));
    assert!(state.borrow().with_canvas_state(|c| !c.pinout_tick_active));
    let base_dd = diagram_draws.get();
    let base_pd = pinout_draws.get();
    pump_for(Duration::from_millis(120));
    assert_eq!(diagram_draws.get(), base_dd, "Idle diagram must not redraw");
    assert_eq!(pinout_draws.get(), base_pd, "Idle pinout must not redraw");

    // Start hover on both canvases (if animations are enabled on this session)
    if tokens::motion::is_animations_enabled() {
        let now = Instant::now();
        state.borrow().with_canvas_state_mut(|c| {
            c.set_diagram_hovered_node(Some("GOING".to_string()), true, now);
            c.set_pinout_hovered_pin(Some(("CN7".to_string(), 1)), true, now);
        });
        ensure_diagram_animation_tick(&diagram_area, &state);
        ensure_pinout_hover_tick(&pinout_area, &state);
        assert!(state.borrow().with_canvas_state(|c| c.diagram_tick_active));
        assert!(state.borrow().with_canvas_state(|c| c.pinout_tick_active));

        pump_until(
            || {
                state
                    .borrow()
                    .with_canvas_state(|c| !c.diagram_tick_active && !c.pinout_tick_active)
            },
            Duration::from_secs(2),
        );

        assert!(
            state
                .borrow()
                .with_canvas_state(|c| !c.diagram_tick_active && !c.pinout_tick_active),
            "Both canvas hover ticks must break once amounts reach target within epsilon"
        );
        assert_eq!(
            state
                .borrow()
                .with_canvas_state(|c| c.node_hover_amounts.get("GOING").copied()),
            Some(1.0)
        );
        assert_eq!(
            state
                .borrow()
                .with_canvas_state(|c| c.pin_hover_amounts.get(&("CN7".to_string(), 1)).copied()),
            Some(1.0)
        );
        assert!(diagram_draws.get() > base_dd + 1);
        assert!(pinout_draws.get() > base_pd + 1);

        // Settled while still hovered: zero extra draws or frame clock spinning
        let settled_dd = diagram_draws.get();
        let settled_pd = pinout_draws.get();
        pump_for(Duration::from_millis(150));
        assert_eq!(
            diagram_draws.get(),
            settled_dd,
            "Settled hovered diagram must not redraw at idle"
        );
        assert_eq!(
            pinout_draws.get(),
            settled_pd,
            "Settled hovered pinout must not redraw at idle"
        );

        // Leave: ease back to 0, prune entries, and break cleanly
        let leave_now = Instant::now();
        state.borrow().with_canvas_state_mut(|c| {
            c.set_diagram_hovered_node(None, true, leave_now);
            c.set_pinout_hovered_pin(None, true, leave_now);
        });
        ensure_diagram_animation_tick(&diagram_area, &state);
        ensure_pinout_hover_tick(&pinout_area, &state);
        assert!(state.borrow().with_canvas_state(|c| c.diagram_tick_active));
        assert!(state.borrow().with_canvas_state(|c| c.pinout_tick_active));

        pump_until(
            || {
                state
                    .borrow()
                    .with_canvas_state(|c| !c.diagram_tick_active && !c.pinout_tick_active)
            },
            Duration::from_secs(2),
        );
        assert!(state.borrow().with_canvas_state(|c| {
            !c.diagram_tick_active
                && !c.pinout_tick_active
                && c.node_hover_amounts.is_empty()
                && c.pin_hover_amounts.is_empty()
        }));

        // Reduced motion never activates hover easing ticks
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_enable_animations(false);
        }
        state.borrow().with_canvas_state_mut(|c| {
            c.set_diagram_hovered_node(Some("GOING".to_string()), false, Instant::now());
            c.set_pinout_hovered_pin(Some(("CN7".to_string(), 1)), false, Instant::now());
        });
        ensure_diagram_animation_tick(&diagram_area, &state);
        ensure_pinout_hover_tick(&pinout_area, &state);
        assert!(state
            .borrow()
            .with_canvas_state(|c| !c.diagram_tick_active && !c.pinout_tick_active));
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_enable_animations(true);
        }
    }

    win.close();
}
