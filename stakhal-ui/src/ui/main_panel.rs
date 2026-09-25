use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use crate::state::create_icon_button;
use crate::ui::tokens::motion;

pub struct MainPanelWidgets {
    pub overview_box: gtk4::Box,
    pub btn_browse: gtk4::Button,
    pub btn_load: gtk4::Button,
    pub btn_build: gtk4::Button,
    pub btn_build_flash: gtk4::Button,
    pub btn_enable_traceability: gtk4::Button,
    pub btn_call_graph: gtk4::Button,
    pub btn_nucleo_pinout: gtk4::Button,
    pub lbl_discovered_dir: gtk4::Label,
    pub lbl_ioc_path: gtk4::Label,
    pub lbl_main_c_path: gtk4::Label,
    pub lbl_project_name: gtk4::Label,
    pub lbl_mcu_family: gtk4::Label,
    pub lbl_mcu_name: gtk4::Label,
    pub lbl_build_traceability: gtk4::Label,
    pub area_traceability_verify: gtk4::DrawingArea,
    pub lbl_periph_header: gtk4::Label,
    pub lbl_region_header: gtk4::Label,
    pub list_peripherals: gtk4::ListBox,
    pub list_user_regions: gtk4::ListBox,
    pub build_log_view: gtk4::TextView,
    pub lbl_build_status: gtk4::Label,
    pub area_flash_verify: gtk4::DrawingArea,
    pub btn_clear_log: gtk4::Button,
    pub btn_serial_monitor: gtk4::Button,
}

pub fn build_main_panel() -> MainPanelWidgets {
    let btn_browse = gtk4::Button::builder()
        .icon_name("folder-open-symbolic")
        .tooltip_text("Browse Project Folder")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .build();
    btn_browse.set_cursor_from_name(Some("pointer"));

    let lbl_discovered_dir = gtk4::Label::builder()
        .label("No folder selected")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .css_classes(vec!["dim-label".to_string()])
        .build();

    let lbl_ioc_path = gtk4::Label::builder()
        .label("IOC Path: N/A")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string(), "data-mono".to_string()])
        .build();

    let lbl_main_c_path = gtk4::Label::builder()
        .label("Main C Path: N/A")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string(), "data-mono".to_string()])
        .build();

    let btn_load = create_icon_button("Load Project", "system-run-symbolic", true);
    btn_load.set_sensitive(false);

    let btn_build = gtk4::Button::builder()
        .label("Build")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .sensitive(false)
        .tooltip_text("Compile project without flashing")
        .build();
    btn_build.set_cursor_from_name(Some("pointer"));

    let btn_build_flash = gtk4::Button::builder()
        .label("Build & Flash")
        .css_classes(vec!["stakhal-btn".to_string(), "suggested-action".to_string()])
        .sensitive(false)
        .tooltip_text("Build project and flash to STM32 target via ST-Link")
        .build();
    btn_build_flash.set_cursor_from_name(Some("pointer"));

    let btn_enable_traceability = gtk4::Button::builder()
        .label("Enable Build Traceability")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .sensitive(false)
        .tooltip_text("Enable build traceability by inserting version banner into main.c")
        .build();
    btn_enable_traceability.set_cursor_from_name(Some("pointer"));

    let btn_call_graph = gtk4::Button::builder()
        .label("State Machine Graph")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .sensitive(false)
        .tooltip_text("Application State Machine Transition Diagram")
        .build();
    btn_call_graph.set_cursor_from_name(Some("pointer"));

    let btn_nucleo_pinout = gtk4::Button::builder()
        .label("Nucleo Pinout")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .sensitive(false)
        .tooltip_text("Nucleo Pinout visualizer (F446RE only)")
        .build();
    btn_nucleo_pinout.set_cursor_from_name(Some("pointer"));

    let btn_serial_monitor = gtk4::Button::builder()
        .label("Serial Monitor")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .tooltip_text("Open Serial Monitor console")
        .build();
    btn_serial_monitor.set_cursor_from_name(Some("pointer"));

    let toolbar_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();

    let paths_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .build();

    paths_box.append(&lbl_discovered_dir);
    let sub_paths_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(16)
        .build();
    sub_paths_box.append(&lbl_ioc_path);
    sub_paths_box.append(&lbl_main_c_path);
    paths_box.append(&sub_paths_box);

    toolbar_box.append(&btn_browse);
    toolbar_box.append(&paths_box);
    toolbar_box.append(&btn_load);
    toolbar_box.append(&btn_build);
    toolbar_box.append(&btn_build_flash);
    toolbar_box.append(&btn_enable_traceability);
    toolbar_box.append(&btn_call_graph);
    toolbar_box.append(&btn_nucleo_pinout);
    toolbar_box.append(&btn_serial_monitor);

    let lbl_project_name = gtk4::Label::builder()
        .label("NAME: N/A")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["caption".to_string(), "data-mono".to_string()])
        .build();

    let lbl_mcu_family = gtk4::Label::builder()
        .label("FAMILY: N/A")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["caption".to_string(), "data-mono".to_string()])
        .build();

    let lbl_mcu_name = gtk4::Label::builder()
        .label("MCU: N/A")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["caption".to_string(), "data-mono".to_string()])
        .build();

    let lbl_build_traceability = gtk4::Label::builder()
        .label("BUILD: Unknown")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["caption".to_string(), "data-mono".to_string(), "dim-label".to_string()])
        .tooltip_text("Firmware build traceability status")
        .build();

    let area_traceability_verify = crate::ui::verify_stroke::build_verify_stroke_area();

    let box_trace_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();
    box_trace_row.append(&lbl_build_traceability);
    box_trace_row.append(&area_traceability_verify);

    let status_bar_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(24)
        .margin_top(4)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();

    status_bar_box.append(&lbl_project_name);
    status_bar_box.append(&lbl_mcu_family);
    status_bar_box.append(&lbl_mcu_name);
    status_bar_box.append(&box_trace_row);

    let lbl_periph_header = gtk4::Label::builder()
        .label("[ PERIPHERALS ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let lbl_region_header = gtk4::Label::builder()
        .label("[ USER REGIONS ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let list_peripherals = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let list_user_regions = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let col_peripherals = create_column_box(&lbl_periph_header, &list_peripherals);
    let col_regions = create_column_box(&lbl_region_header, &list_user_regions);

    let columns_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .homogeneous(false)
        .spacing(12)
        .vexpand(true)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(16)
        .build();
    columns_box.append(&col_peripherals);
    columns_box.append(&col_regions);

    // Build & Flash Console Panel
    let lbl_console_header = gtk4::Label::builder()
        .label("[ BUILD & FLASH CONSOLE ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let lbl_build_status = gtk4::Label::builder()
        .label("IDLE")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["card".to_string(), "caption".to_string(), "data-mono".to_string(), "status-idle".to_string()])
        .build();

    let btn_clear_log = gtk4::Button::builder()
        .label("Clear")
        .css_classes(vec!["flat".to_string(), "caption".to_string()])
        .tooltip_text("Clear console logs")
        .build();

    let area_flash_verify = crate::ui::verify_stroke::build_verify_stroke_area();

    let console_header_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    console_header_box.append(&lbl_console_header);
    console_header_box.append(&lbl_build_status);
    console_header_box.append(&area_flash_verify);
    let console_spacer = gtk4::Box::builder().hexpand(true).build();
    console_header_box.append(&console_spacer);
    console_header_box.append(&btn_clear_log);

    let build_log_view = gtk4::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();

    let build_scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .min_content_height(260)
        .max_content_height(400)
        .vexpand(false)
        .child(&build_log_view)
        .css_classes(vec!["card".to_string()])
        .build();

    let console_panel_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(16)
        .build();
    console_panel_box.append(&console_header_box);
    console_panel_box.append(&build_scrolled);

    let overview_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    overview_box.append(&toolbar_box);
    overview_box.append(&status_bar_box);
    overview_box.append(&columns_box);
    overview_box.append(&console_panel_box);

    MainPanelWidgets {
        overview_box,
        btn_browse,
        btn_load,
        btn_build,
        btn_build_flash,
        btn_enable_traceability,
        btn_call_graph,
        btn_nucleo_pinout,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_build_traceability,
        area_traceability_verify,
        lbl_periph_header,
        lbl_region_header,
        list_peripherals,
        list_user_regions,
        build_log_view,
        lbl_build_status,
        area_flash_verify,
        btn_clear_log,
        btn_serial_monitor,
    }
}

pub fn create_column_box(header_label: &gtk4::Label, list_box: &gtk4::ListBox) -> gtk4::Box {
    let scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .child(list_box)
        .build();

    let col_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .hexpand(true)
        .vexpand(true)
        .build();

    col_box.append(header_label);
    col_box.append(&scrolled);
    col_box
}

pub fn clear_list_box(list_box: &gtk4::ListBox) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
}

pub fn create_peripheral_row(name: &str, mode: Option<&str>, param_count: usize) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(name)
        .subtitle(mode.unwrap_or("N/A"))
        .css_classes(vec!["data-mono".to_string()])
        .build();

    let badge = gtk4::Label::builder()
        .label(&format!("{} params", param_count))
        .valign(gtk4::Align::Center)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string(), "data-mono".to_string()])
        .build();

    row.add_suffix(&badge);
    row
}

pub fn create_region_row(
    tag: &str,
    byte_start: usize,
    byte_end: usize,
    line_start: usize,
    line_end: usize,
    is_implicit: bool,
) -> adw::ActionRow {
    let details = format!("L{}-L{} (bytes {}..{})", line_start, line_end, byte_start, byte_end);
    let row = adw::ActionRow::builder()
        .title(tag)
        .subtitle(&details)
        .css_classes(vec!["data-mono".to_string()])
        .build();

    if is_implicit {
        let badge = gtk4::Label::builder()
            .label("implicit")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["implicit-badge".to_string(), "caption".to_string(), "data-mono".to_string()])
            .build();
        row.add_suffix(&badge);
    }

    row
}

pub const STAGGER_BASE_STEP_MS: u64 = 18;
pub const STAGGER_MAX_CASCADE_MS: u64 = 300;

/// Calculate delay in milliseconds for staggered row entry.
/// Rows stagger by ~18ms per row, scaling down if total cascade duration exceeds 300ms.
pub fn calculate_stagger_delay_ms(index: usize, total_count: usize) -> u64 {
    if total_count <= 1 || index == 0 {
        return 0;
    }
    let natural_last = (total_count - 1) as u64 * STAGGER_BASE_STEP_MS;
    if natural_last <= STAGGER_MAX_CASCADE_MS {
        (index as u64) * STAGGER_BASE_STEP_MS
    } else {
        let step = STAGGER_MAX_CASCADE_MS as f64 / (total_count - 1) as f64;
        let delay = ((index as f64) * step).round() as u64;
        delay.min(STAGGER_MAX_CASCADE_MS)
    }
}

/// Appends a row widget to a ListBox with a staggered opacity reveal animation.
/// Each row appears ~15-20ms after the previous (scaled down if total cascade exceeds 300ms)
/// using ease-out cubic opacity fade over DURATION_SHORT_MS (180ms).
/// When reduced-motion is enabled, collapses to instant (0ms) opacity 1.0.
pub fn append_staggered_row<W: IsA<gtk4::Widget> + Clone + 'static>(
    list_box: &gtk4::ListBox,
    row: &W,
    index: usize,
    total_count: usize,
) {
    list_box.append(row);
    row.add_css_class("stagger-reveal-row");

    if !motion::is_animations_enabled() {
        row.set_opacity(1.0);
        return;
    }

    let delay_ms = calculate_stagger_delay_ms(index, total_count);
    let start = std::time::Instant::now();
    let delay = std::time::Duration::from_millis(delay_ms);
    let duration = std::time::Duration::from_millis(motion::DURATION_SHORT_MS);

    row.set_opacity(0.0);
    row.add_tick_callback(move |widget, _| {
        let elapsed = start.elapsed();
        if elapsed < delay {
            widget.set_opacity(0.0);
            return glib::ControlFlow::Continue;
        }
        let anim_elapsed = elapsed - delay;
        if anim_elapsed >= duration {
            widget.set_opacity(1.0);
            widget.queue_draw();
            return glib::ControlFlow::Break;
        }
        let progress = anim_elapsed.as_secs_f64() / duration.as_secs_f64();
        let alpha = motion::ease_out_cubic(progress);
        widget.set_opacity(alpha);
        widget.queue_draw();
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_stagger_delay_bounds_and_capping() {
        // Zero or single row
        assert_eq!(calculate_stagger_delay_ms(0, 0), 0);
        assert_eq!(calculate_stagger_delay_ms(0, 1), 0);

        // Small list (5 rows): step is 18ms, max cascade = 4 * 18 = 72ms <= 300ms
        assert_eq!(calculate_stagger_delay_ms(0, 5), 0);
        assert_eq!(calculate_stagger_delay_ms(1, 5), 18);
        assert_eq!(calculate_stagger_delay_ms(2, 5), 36);
        assert_eq!(calculate_stagger_delay_ms(3, 5), 54);
        assert_eq!(calculate_stagger_delay_ms(4, 5), 72);

        // Moderate list (17 rows): 16 * 18 = 288ms <= 300ms
        assert_eq!(calculate_stagger_delay_ms(0, 17), 0);
        assert_eq!(calculate_stagger_delay_ms(16, 17), 288);

        // Large list (31 rows): 30 * 18 = 540ms > 300ms, scaled down to 300 / 30 = 10ms per row
        assert_eq!(calculate_stagger_delay_ms(0, 31), 0);
        assert_eq!(calculate_stagger_delay_ms(1, 31), 10);
        assert_eq!(calculate_stagger_delay_ms(15, 31), 150);
        assert_eq!(calculate_stagger_delay_ms(30, 31), 300);

        // Very large list (100 rows): last row capped at 300ms
        assert_eq!(calculate_stagger_delay_ms(0, 100), 0);
        assert_eq!(calculate_stagger_delay_ms(99, 100), 300);
        for i in 0..100 {
            assert!(calculate_stagger_delay_ms(i, 100) <= 300);
        }
    }
}

