use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::append_log_text;
use crate::state::{AppState, AppWidgets};
use crate::toolchain;
use crate::{update_build_status, StatusKind};

pub fn setup_build_flash_handlers(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    // Connect Clear Console Button
    let state_clear = Rc::clone(state);
    let log_view_clear = widgets.build_log_view.clone();
    let status_clear = widgets.lbl_build_status.clone();
    let area_flash_clear = widgets.area_flash_verify.clone();
    widgets.btn_clear_log.connect_clicked(move |_| {
        log_view_clear.buffer().set_text("");
        update_build_status(&status_clear, "IDLE", StatusKind::Idle);
        {
            let st = state_clear.borrow();
            let mut bt = st.build_trace.borrow_mut();
            bt.flash_verify_start = None;
        }
        area_flash_clear.queue_draw();
    });

    // Connect Build Button (compile only, no flash)
    let state_b = Rc::clone(state);
    let widgets_b = Rc::clone(widgets);
    widgets.btn_build.connect_clicked(move |_| {
        execute_build_pipeline(false, &state_b, &widgets_b);
    });

    // Connect Build & Flash Button
    let state_bf = Rc::clone(state);
    let widgets_bf = Rc::clone(widgets);
    widgets.btn_build_flash.connect_clicked(move |_| {
        execute_build_pipeline(true, &state_bf, &widgets_bf);
    });

    // Connect Enable Build Traceability Button
    let state_trace = Rc::clone(state);
    let widgets_trace = Rc::clone(widgets);
    widgets.btn_enable_traceability.connect_clicked(move |_| {
        let (project_dir, main_c_path) = {
            let st = state_trace.borrow();
            let proj = st.project.borrow();
            match (&proj.project_dir, &proj.discovered_main_c) {
                (Some(d), Some(m)) => (d.clone(), m.clone()),
                _ => return,
            }
        };

        if !toolchain::traceability::is_git_repository(&project_dir) {
            widgets_trace
                .toast_overlay
                .add_toast(adw::Toast::new("Cannot enable traceability: project is not inside a Git repository"));
            return;
        }

        if toolchain::traceability::is_traceability_enabled_in_source(&main_c_path) {
            widgets_trace
                .toast_overlay
                .add_toast(adw::Toast::new("Build traceability is already enabled in main.c"));
            return;
        }

        let diff_preview = toolchain::traceability::generate_traceability_diff_preview(&main_c_path);

        let dialog = adw::MessageDialog::builder()
            .heading("Enable Build Traceability")
            .body("StakHAL can insert the build-info header include and boot banner printf into CubeMX user code regions in main.c.\n\nReview the exact changes below before applying:")
            .transient_for(&widgets_trace.window)
            .build();

        let preview_view = gtk4::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(12)
            .right_margin(12)
            .build();
        preview_view.buffer().set_text(&diff_preview);

        let preview_scrolled = gtk4::ScrolledWindow::builder()
            .min_content_height(140)
            .max_content_height(240)
            .child(&preview_view)
            .css_classes(vec!["card".to_string()])
            .build();

        dialog.set_extra_child(Some(&preview_scrolled));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("confirm", "Enable & Insert");
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);

        let state_dlg = Rc::clone(&state_trace);
        let widgets_dlg = Rc::clone(&widgets_trace);
        let dir_clone = project_dir.clone();
        let main_c_clone = main_c_path.clone();

        dialog.connect_response(None, move |_, resp| {
            if resp == "confirm" {
                match toolchain::traceability::insert_traceability_into_source(&main_c_clone, &dir_clone) {
                    Ok(()) => {
                        append_log_text(
                            &widgets_dlg.build_log_view,
                            &format!("[TRACEABILITY] Successfully inserted build info into {}", main_c_clone.display()),
                        );
                        widgets_dlg.toast_overlay.add_toast(
                            adw::Toast::new("Build traceability enabled in main.c")
                        );
                        update_traceability_ui(&state_dlg, &widgets_dlg);
                    }
                    Err(err) => {
                        append_log_text(
                            &widgets_dlg.build_log_view,
                            &format!("[TRACEABILITY ERROR] Failed to insert build info: {}", err),
                        );
                        widgets_dlg.toast_overlay.add_toast(
                            adw::Toast::new(&format!("Error: {}", err))
                        );
                    }
                }
            }
        });

        dialog.present();
    });
}

pub fn execute_build_pipeline(
    flash_after_build: bool,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let (dir, build_sys) = {
        let st = state.borrow();
        let mut bt = st.build_trace.borrow_mut();
        if bt.build_in_progress {
            return;
        }

        let dir = match &st.project.borrow().project_dir {
            Some(d) => d.clone(),
            None => return,
        };
        let build_sys = match &bt.detected_build_system {
            Some(bs) => bs.clone(),
            None => return,
        };
        bt.build_in_progress = true;
        bt.flash_verify_start = None;
        (dir, build_sys)
    };

    widgets.area_flash_verify.queue_draw();

    widgets.btn_build.set_sensitive(false);
    widgets.btn_build_flash.set_sensitive(false);
    widgets.btn_enable_traceability.set_sensitive(false);
    update_build_status(&widgets.lbl_build_status, "BUILDING...", StatusKind::Active);

    let build_cmd_res = toolchain::builder::get_build_command(&build_sys, &dir);
    let (cmd, args, exec_dir) = match build_cmd_res {
        Ok(tuple) => tuple,
        Err(err) => {
            append_log_text(&widgets.build_log_view, &format!("[ERROR] {}", err));
            update_build_status(&widgets.lbl_build_status, "BUILD FAILED", StatusKind::Error);
            let has_build_system = {
                let st = state.borrow();
                let mut bt = st.build_trace.borrow_mut();
                bt.build_in_progress = false;
                bt.has_build_system
            };
            widgets.btn_build.set_sensitive(has_build_system);
            widgets.btn_build_flash.set_sensitive(has_build_system);
            update_traceability_ui(state, widgets);
            return;
        }
    };

    // Generate build-info header before invoking compiler
    match toolchain::traceability::generate_build_info_header(&dir) {
        Ok(hash) => {
            append_log_text(
                &widgets.build_log_view,
                &format!("[TRACE] Generated Core/Inc/stakhal_build_info.h (STAKHAL_BUILD_HASH: \"{}\")", hash),
            );
        }
        Err(err) => {
            append_log_text(
                &widgets.build_log_view,
                &format!("[TRACE WARNING] Failed to generate build info header: {}", err),
            );
        }
    }

    append_log_text(&widgets.build_log_view, "============================================================");
    append_log_text(&widgets.build_log_view, &format!("[BUILD] [{}] Running `{} {}` in {}", build_sys.display_name(), cmd, args.join(" "), exec_dir.display()));
    append_log_text(&widgets.build_log_view, "============================================================");

    let rx = toolchain::runner::spawn_streaming_process(
        cmd.clone(),
        args,
        exec_dir,
    );

    let state_timer = Rc::clone(state);
    let widgets_timer = Rc::clone(widgets);
    let dir_timer = dir.clone();
    let build_sys_timer = build_sys.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(25), move || {
        let mut finished = None;
        while let Ok(evt) = rx.try_recv() {
            match evt {
                toolchain::runner::ProcessEvent::Line(line) => {
                    append_log_text(&widgets_timer.build_log_view, &line);
                }
                toolchain::runner::ProcessEvent::Finished(success, code) => {
                    finished = Some((success, code));
                }
                toolchain::runner::ProcessEvent::FailedToStart(err) => {
                    append_log_text(&widgets_timer.build_log_view, &format!("[ERROR] {}", err));
                }
                _ => {}
            }
        }

        if let Some((success, code)) = finished {
            if success {
                append_log_text(&widgets_timer.build_log_view, &format!("\n[BUILD SUCCESS] `{}` finished successfully.", cmd));
                let res = toolchain::builder::resolve_artifact_for_build_system(&dir_timer, &build_sys_timer);
                match res {
                    toolchain::makefile::ArtifactResolution::Exact(bin_path) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ARTIFACT] Resolved output binary: {}", bin_path.display()));
                        if flash_after_build {
                            update_build_status(&widgets_timer.lbl_build_status, "PROBING...", StatusKind::Active);
                            run_probe_detection_and_flash(bin_path, &state_timer, &widgets_timer, dir_timer.clone());
                        } else {
                            update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                            widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build succeeded"));
                            let has_build_system = {
                                let st = state_timer.borrow();
                                let mut bt = st.build_trace.borrow_mut();
                                bt.build_in_progress = false;
                                bt.has_build_system
                            };
                            widgets_timer.btn_build.set_sensitive(has_build_system);
                            widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                            update_traceability_ui(&state_timer, &widgets_timer);
                        }
                    }
                    toolchain::makefile::ArtifactResolution::MultipleCandidates(candidates) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ARTIFACT] Found {} candidate .bin files in build directory:", candidates.len()));
                        for c in &candidates {
                            append_log_text(&widgets_timer.build_log_view, &format!("  - {}", c.display()));
                        }
                        if flash_after_build {
                            update_build_status(&widgets_timer.lbl_build_status, "SELECT ARTIFACT", StatusKind::Active);

                            let dialog = adw::MessageDialog::builder()
                                .heading("Multiple Build Artifacts Found")
                                .body("Please select which binary to flash:")
                                .transient_for(&widgets_timer.window)
                                .build();
                            for (idx, c) in candidates.iter().enumerate() {
                                let name = c.file_name().unwrap_or_default().to_string_lossy();
                                dialog.add_response(&idx.to_string(), &name);
                            }
                            dialog.add_response("cancel", "Cancel");
                            let state_dlg = Rc::clone(&state_timer);
                            let widgets_dlg = Rc::clone(&widgets_timer);
                            let cand_clone = candidates.clone();
                            let dir_clone = dir_timer.clone();
                            dialog.connect_response(None, move |_, resp| {
                                if resp != "cancel" {
                                    if let Ok(idx) = resp.parse::<usize>() {
                                        if let Some(chosen) = cand_clone.get(idx) {
                                            append_log_text(&widgets_dlg.build_log_view, &format!("[ARTIFACT] Selected candidate: {}", chosen.display()));
                                            run_probe_detection_and_flash(chosen.clone(), &state_dlg, &widgets_dlg, dir_clone.clone());
                                            return;
                                        }
                                    }
                                }
                                append_log_text(&widgets_dlg.build_log_view, "[ARTIFACT] Operation cancelled by user.");
                                update_build_status(&widgets_dlg.lbl_build_status, "CANCELLED", StatusKind::Idle);
                                let has_build_system = {
                                    let st = state_dlg.borrow();
                                    let mut bt = st.build_trace.borrow_mut();
                                    bt.build_in_progress = false;
                                    bt.has_build_system
                                };
                                widgets_dlg.btn_build.set_sensitive(has_build_system);
                                widgets_dlg.btn_build_flash.set_sensitive(has_build_system);
                                update_traceability_ui(&state_dlg, &widgets_dlg);
                            });
                            dialog.present();
                        } else {
                            update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                            widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build succeeded"));
                            let has_build_system = {
                                let st = state_timer.borrow();
                                let mut bt = st.build_trace.borrow_mut();
                                bt.build_in_progress = false;
                                bt.has_build_system
                            };
                            widgets_timer.btn_build.set_sensitive(has_build_system);
                            widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                            update_traceability_ui(&state_timer, &widgets_timer);
                        }
                    }
                    toolchain::makefile::ArtifactResolution::NoneFound(expected) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ERROR] Build succeeded but target .bin was not found. Expected: {}", expected.display()));
                        update_build_status(&widgets_timer.lbl_build_status, "ARTIFACT MISSING", StatusKind::Error);
                        let has_build_system = {
                            let st = state_timer.borrow();
                            let mut bt = st.build_trace.borrow_mut();
                            bt.build_in_progress = false;
                            bt.has_build_system
                        };
                        widgets_timer.btn_build.set_sensitive(has_build_system);
                        widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                        update_traceability_ui(&state_timer, &widgets_timer);
                    }
                }
            } else {
                let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                let action_type = if flash_after_build { "Flashing halted." } else { "Build failed." };
                append_log_text(&widgets_timer.build_log_view, &format!("\n[BUILD FAILED] {} exited with error code {}. {}", cmd, code_str, action_type));
                update_build_status(&widgets_timer.lbl_build_status, "BUILD FAILED", StatusKind::Error);
                let has_build_system = {
                    let st = state_timer.borrow();
                    let mut bt = st.build_trace.borrow_mut();
                    bt.build_in_progress = false;
                    bt.has_build_system
                };
                widgets_timer.btn_build.set_sensitive(has_build_system);
                widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                update_traceability_ui(&state_timer, &widgets_timer);
            }

            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });
}

fn apply_traceability_status_to_label(
    label: &gtk4::Label,
    status: &toolchain::traceability::TraceabilityStatus,
) {
    label.remove_css_class("status-ready");
    label.remove_css_class("status-active");
    label.remove_css_class("status-error");
    label.remove_css_class("status-idle");
    label.remove_css_class("dim-label");

    match status {
        toolchain::traceability::TraceabilityStatus::Unknown => {
            label.set_text("BUILD: Unknown");
            label.add_css_class("dim-label");
            label.set_tooltip_text(Some(
                "Firmware build unknown (not flashed with traceability enabled, or haven't reconnected since)",
            ));
        }
        toolchain::traceability::TraceabilityStatus::Dirty { base_hash } => {
            label.set_text(&format!("BUILD: {}-dirty", base_hash));
            label.add_css_class("status-active");
            label.set_tooltip_text(Some(&format!(
                "Board was flashed from an uncommitted working tree near {} - exact source unknown",
                base_hash
            )));
        }
        toolchain::traceability::TraceabilityStatus::MatchesWorkingTree { hash } => {
            label.set_text(&format!("BUILD: {} (Matches working tree)", hash));
            label.add_css_class("status-ready");
            label.set_tooltip_text(Some("Board matches working tree"));
        }
        toolchain::traceability::TraceabilityStatus::BehindWorkingTree { hash, count } => {
            let commit_str = if *count == 1 {
                "1 commit".to_string()
            } else {
                format!("{} commits", count)
            };
            label.set_text(&format!("BUILD: {} ({} behind)", hash, commit_str));
            label.add_css_class("status-active");
            label.set_tooltip_text(Some(&format!(
                "Board is {} behind working tree",
                commit_str
            )));
        }
        toolchain::traceability::TraceabilityStatus::Diverged { hash } => {
            label.set_text(&format!("BUILD: {} (Diverged)", hash));
            label.add_css_class("status-error");
            label.set_tooltip_text(Some(
                "Board's build doesn't match this branch's history",
            ));
        }
    }
}

pub fn update_traceability_ui(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (project_dir, main_c_path, build_in_progress, captured_hash) = {
        let st = state.borrow();
        let proj = st.project.borrow();
        let bt = st.build_trace.borrow();
        (
            proj.project_dir.clone(),
            proj.discovered_main_c.clone(),
            bt.build_in_progress,
            bt.captured_build_hash.clone(),
        )
    };

    let status = match (&project_dir, &captured_hash) {
        (Some(dir), Some(hash)) => {
            toolchain::traceability::compare_build_hash_to_head(dir, hash)
        }
        _ => toolchain::traceability::TraceabilityStatus::Unknown,
    };
    apply_traceability_status_to_label(&widgets.lbl_build_traceability, &status);

    let is_matched = matches!(
        status,
        toolchain::traceability::TraceabilityStatus::MatchesWorkingTree { .. }
    );
    {
        let st = state.borrow();
        let mut bt = st.build_trace.borrow_mut();
        if is_matched {
            if !bt.traceability_was_matched {
                bt.traceability_was_matched = true;
                bt.traceability_verify_start = Some(std::time::Instant::now());
                widgets.area_traceability_verify.queue_draw();
            }
        } else if bt.traceability_was_matched {
            bt.traceability_was_matched = false;
            bt.traceability_verify_start = None;
            widgets.area_traceability_verify.queue_draw();
        }
    }

    match (project_dir, main_c_path) {
        (Some(dir), Some(main_c)) => {
            let is_git = toolchain::traceability::is_git_repository(&dir);
            let is_enabled = toolchain::traceability::is_traceability_enabled_in_source(&main_c);

            state.borrow().build_trace.borrow_mut().is_traceability_enabled = is_enabled;

            if !is_git {
                widgets.btn_enable_traceability.set_sensitive(false);
                widgets
                    .btn_enable_traceability
                    .set_tooltip_text(Some("Build traceability requires a Git repository"));
            } else if is_enabled {
                widgets.btn_enable_traceability.set_sensitive(false);
                widgets
                    .btn_enable_traceability
                    .set_tooltip_text(Some("Build traceability already enabled in main.c"));
            } else if build_in_progress {
                widgets.btn_enable_traceability.set_sensitive(false);
                widgets
                    .btn_enable_traceability
                    .set_tooltip_text(Some("Build in progress..."));
            } else {
                widgets.btn_enable_traceability.set_sensitive(true);
                widgets.btn_enable_traceability.set_tooltip_text(Some(
                    "Enable build traceability by inserting version banner into main.c",
                ));
            }
        }
        _ => {
            widgets.btn_enable_traceability.set_sensitive(false);
            widgets
                .btn_enable_traceability
                .set_tooltip_text(Some("Load a project to enable build traceability"));
        }
    }
}

pub fn run_probe_detection_and_flash(
    artifact: PathBuf,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
    project_dir: PathBuf,
) {
    append_log_text(&widgets.build_log_view, "\n============================================================");
    append_log_text(&widgets.build_log_view, "[PROBE] Scanning for connected ST-Link programmers (`st-info --probe`)...");
    append_log_text(&widgets.build_log_view, "============================================================");

    match toolchain::probe::detect_stlink_probes() {
        Ok(probes) => {
            if probes.len() == 1 {
                let p = &probes[0];
                append_log_text(&widgets.build_log_view, &format!("[PROBE] Detected single target: {}", p.display_label()));
                run_flash_stage(artifact, Some(p.serial.clone()), state, widgets, project_dir);
            } else {
                append_log_text(&widgets.build_log_view, &format!("[PROBE] Detected {} ST-Link programmers:", probes.len()));
                for p in &probes {
                    append_log_text(&widgets.build_log_view, &format!("  - {}", p.display_label()));
                }
                update_build_status(&widgets.lbl_build_status, "SELECT PROBE", StatusKind::Active);

                let dialog = adw::MessageDialog::builder()
                    .heading("Multiple ST-Link Probes Detected")
                    .body("Please select which ST-Link probe to flash:")
                    .transient_for(&widgets.window)
                    .build();

                for (idx, p) in probes.iter().enumerate() {
                    dialog.add_response(&idx.to_string(), &p.display_label());
                }
                dialog.add_response("cancel", "Cancel");

                let state_dlg = Rc::clone(state);
                let widgets_dlg = Rc::clone(widgets);
                let probes_clone = probes.clone();
                let artifact_clone = artifact.clone();
                let dir_clone = project_dir.clone();

                dialog.connect_response(None, move |_, resp| {
                    if resp != "cancel" {
                        if let Ok(idx) = resp.parse::<usize>() {
                            if let Some(chosen) = probes_clone.get(idx) {
                                append_log_text(&widgets_dlg.build_log_view, &format!("[PROBE] Selected target: {}", chosen.display_label()));
                                run_flash_stage(artifact_clone.clone(), Some(chosen.serial.clone()), &state_dlg, &widgets_dlg, dir_clone.clone());
                                return;
                            }
                        }
                    }
                    append_log_text(&widgets_dlg.build_log_view, "[PROBE] Flashing cancelled by user.");
                    update_build_status(&widgets_dlg.lbl_build_status, "CANCELLED", StatusKind::Idle);
                    let has_build_system = {
                        let st = state_dlg.borrow();
                        let mut bt = st.build_trace.borrow_mut();
                        bt.build_in_progress = false;
                        bt.has_build_system
                    };
                    widgets_dlg.btn_build.set_sensitive(has_build_system);
                    widgets_dlg.btn_build_flash.set_sensitive(has_build_system);
                    update_traceability_ui(&state_dlg, &widgets_dlg);
                });
                dialog.present();
            }
        }
        Err(err) => {
            append_log_text(&widgets.build_log_view, &format!("[ERROR] {}", err));
            let (txt, kind) = match err {
                toolchain::probe::ProbeError::ToolNotFound => ("ST-INFO MISSING", StatusKind::Error),
                toolchain::probe::ProbeError::ZeroProbesFound => ("NO PROBE", StatusKind::Active),
                toolchain::probe::ProbeError::ExecutionFailed(_) => ("PROBE ERROR", StatusKind::Error),
            };
            update_build_status(&widgets.lbl_build_status, txt, kind);
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("[ERROR] {}", err)));
            let has_build_system = {
                let st = state.borrow();
                let mut bt = st.build_trace.borrow_mut();
                bt.build_in_progress = false;
                bt.has_build_system
            };
            widgets.btn_build.set_sensitive(has_build_system);
            widgets.btn_build_flash.set_sensitive(has_build_system);
            update_traceability_ui(state, widgets);
        }
    }
}

pub fn run_flash_stage(
    artifact: PathBuf,
    probe_serial: Option<String>,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
    project_dir: PathBuf,
) {
    if !toolchain::runner::is_executable_on_path("st-flash") {
        append_log_text(&widgets.build_log_view, "[ERROR] `st-flash` executable not found on PATH. Please install stlink-tools (e.g. `sudo apt install stlink-tools`).");
        update_build_status(&widgets.lbl_build_status, "ST-FLASH MISSING", StatusKind::Error);
        widgets.toast_overlay.add_toast(adw::Toast::new("[ERROR] `st-flash` not found on PATH"));
        let has_build_system = {
            let st = state.borrow();
            let mut bt = st.build_trace.borrow_mut();
            bt.build_in_progress = false;
            bt.has_build_system
        };
        widgets.btn_build.set_sensitive(has_build_system);
        widgets.btn_build_flash.set_sensitive(has_build_system);
        update_traceability_ui(state, widgets);
        return;
    }

    // Phase 5: Disconnect any active serial session prior to flashing to avoid USB port contention
    let was_serial_connected = state.borrow().serial.borrow().is_serial_connected;
    if was_serial_connected {
        let session = {
            let st = state.borrow();
            let mut ser = st.serial.borrow_mut();
            ser.is_serial_connected = false;
            ser.serial_session.take()
        };
        if let Some(session) = session {
            session
                .tx_cmd
                .send(crate::toolchain::serial::SerialTxCommand::Disconnect)
                .ok();
        }
        widgets.btn_connect_serial.set_label("Connect");
        widgets.btn_connect_serial.remove_css_class("destructive-action");
        widgets.btn_connect_serial.add_css_class("suggested-action");
        update_build_status(&widgets.lbl_serial_status, "DISCONNECTED", StatusKind::Idle);
        widgets.combo_port.set_sensitive(true);
        widgets.combo_baud.set_sensitive(true);
        widgets.btn_refresh_ports.set_sensitive(true);
    }

    let (cmd, args) = toolchain::flasher::build_flash_command(probe_serial.as_deref(), &artifact);

    update_build_status(&widgets.lbl_build_status, "FLASHING...", StatusKind::Active);
    append_log_text(&widgets.build_log_view, "\n============================================================");
    append_log_text(&widgets.build_log_view, &format!("[FLASH] Running `{} {}`", cmd, args.join(" ")));
    append_log_text(&widgets.build_log_view, "============================================================");

    let rx = toolchain::runner::spawn_streaming_process(cmd, args, project_dir);

    let state_timer = Rc::clone(state);
    let widgets_timer = Rc::clone(widgets);

    glib::timeout_add_local(std::time::Duration::from_millis(25), move || {
        let mut finished = None;
        while let Ok(evt) = rx.try_recv() {
            match evt {
                toolchain::runner::ProcessEvent::Line(line) => {
                    append_log_text(&widgets_timer.build_log_view, &line);
                }
                toolchain::runner::ProcessEvent::Finished(success, code) => {
                    finished = Some((success, code));
                }
                toolchain::runner::ProcessEvent::FailedToStart(err) => {
                    append_log_text(&widgets_timer.build_log_view, &format!("[ERROR] {}", err));
                }
                _ => {}
            }
        }

        if let Some((success, code)) = finished {
            let has_build_system = {
                let st = state_timer.borrow();
                let mut bt = st.build_trace.borrow_mut();
                bt.build_in_progress = false;
                bt.has_build_system
            };
            widgets_timer.btn_build.set_sensitive(has_build_system);
            widgets_timer.btn_build_flash.set_sensitive(has_build_system);
            update_traceability_ui(&state_timer, &widgets_timer);

            if success {
                append_log_text(&widgets_timer.build_log_view, "\n[FLASH SUCCESS] Firmware written to 0x08000000 and target MCU reset successfully!");
                update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                {
                    let st = state_timer.borrow();
                    let mut bt = st.build_trace.borrow_mut();
                    bt.flash_verify_start = Some(std::time::Instant::now());
                }
                widgets_timer.area_flash_verify.queue_draw();
                widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build and Flash Succeeded!"));

                // Phase 5: Auto-switch to Serial Monitor tab and auto-reconnect
                crate::navigate_stack(&widgets_timer.stack, "serial_monitor", gtk4::StackTransitionType::SlideLeft);
                crate::auto_reconnect_serial_after_flash(&state_timer, &widgets_timer);
            } else {
                {
                    let st = state_timer.borrow();
                    let mut bt = st.build_trace.borrow_mut();
                    bt.flash_verify_start = None;
                }
                widgets_timer.area_flash_verify.queue_draw();
                let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                append_log_text(&widgets_timer.build_log_view, &format!("\n[FLASH FAILED] st-flash exited with code {}.", code_str));
                update_build_status(&widgets_timer.lbl_build_status, "FLASH FAILED", StatusKind::Error);
                widgets_timer.toast_overlay.add_toast(adw::Toast::new("[ERROR] Flash failed (see console output)"));
            }

            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });
}
