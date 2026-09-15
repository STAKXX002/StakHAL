use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ProcessEvent {
    Started(String),
    Line(String),
    Finished(bool, Option<i32>),
    FailedToStart(String),
}

/// Run an external command in `cwd`, streaming output lines over an mpsc channel.
/// Returns the receiver immediately without blocking the caller.
pub fn spawn_streaming_process(
    cmd: String,
    args: Vec<String>,
    cwd: PathBuf,
) -> std::sync::mpsc::Receiver<ProcessEvent> {
    let (tx, rx) = channel();

    std::thread::spawn(move || {
        let full_cmd_str = format!("{} {}", cmd, args.join(" "));
        tx.send(ProcessEvent::Started(full_cmd_str)).ok();

        let mut child = match Command::new(&cmd)
            .args(&args)
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to execute '{}': {}", cmd, e);
                tx.send(ProcessEvent::FailedToStart(err_msg)).ok();
                tx.send(ProcessEvent::Finished(false, None)).ok();
                return;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let t1 = if let Some(out) = stdout {
            let tx1 = tx.clone();
            Some(std::thread::spawn(move || {
                let reader = BufReader::new(out);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        tx1.send(ProcessEvent::Line(l)).ok();
                    }
                }
            }))
        } else {
            None
        };

        let t2 = if let Some(err) = stderr {
            let tx2 = tx.clone();
            Some(std::thread::spawn(move || {
                let reader = BufReader::new(err);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        tx2.send(ProcessEvent::Line(l)).ok();
                    }
                }
            }))
        } else {
            None
        };

        if let Some(h) = t1 {
            h.join().ok();
        }
        if let Some(h) = t2 {
            h.join().ok();
        }

        match child.wait() {
            Ok(status) => {
                tx.send(ProcessEvent::Finished(status.success(), status.code())).ok();
            }
            Err(e) => {
                tx.send(ProcessEvent::Line(format!("Process wait error: {}", e))).ok();
                tx.send(ProcessEvent::Finished(false, None)).ok();
            }
        }
    });

    rx
}

/// Helper to get the optimal `-j` flag based on available system parallelism.
pub fn get_make_jobs_flag() -> String {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    format!("-j{}", cpus)
}

/// Check whether an executable exists on PATH.
pub fn is_executable_on_path(exe: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe);
            if candidate.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = candidate.metadata() {
                        if meta.permissions().mode() & 0o111 != 0 {
                            return true;
                        }
                    }
                }
                #[cfg(not(unix))]
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_executable_on_path() {
        assert!(is_executable_on_path("make") || is_executable_on_path("sh"));
        assert!(!is_executable_on_path("non_existent_binary_12345xyz"));
    }

    #[test]
    fn test_spawn_streaming_process_echo() {
        let temp = std::env::temp_dir();
        let rx = spawn_streaming_process("echo".to_string(), vec!["Hello from runner".to_string()], temp);

        let mut lines = Vec::new();
        let mut finished = false;
        while let Ok(evt) = rx.recv() {
            match evt {
                ProcessEvent::Line(l) => lines.push(l),
                ProcessEvent::Finished(success, code) => {
                    assert!(success);
                    assert_eq!(code, Some(0));
                    finished = true;
                }
                _ => {}
            }
        }
        assert!(finished);
        assert!(lines.iter().any(|l| l.contains("Hello from runner")));
    }
}
