use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;
use serialport::{available_ports, SerialPortType};

pub const COMMON_BAUD_RATES: &[u32] = &[
    9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortInfo {
    pub port_name: String,
    pub display_name: String,
    pub is_usb: bool,
}

#[derive(Debug, Clone)]
pub enum SerialRxEvent {
    Connected {
        port_name: String,
        baud_rate: u32,
    },
    Data(String),
    Disconnected,
    Error(String),
}

pub enum SerialTxCommand {
    Send(String),
    Disconnect,
}

pub struct ActiveSerialSession {
    #[allow(dead_code)]
    pub port_name: String,
    #[allow(dead_code)]
    pub baud_rate: u32,
    pub tx_cmd: Sender<SerialTxCommand>,
}

/// Enumerate available serial ports on the host system.
/// USB serial ports (e.g. ST-Link VCP /dev/ttyACM*, /dev/ttyUSB*) are sorted first.
pub fn enumerate_serial_ports() -> Vec<SerialPortInfo> {
    let mut result = Vec::new();

    if let Ok(ports) = available_ports() {
        for p in ports {
            let port_name = p.port_name.clone();
            let mut is_usb = false;
            let display_name = match p.port_type {
                SerialPortType::UsbPort(info) => {
                    is_usb = true;
                    let mut parts = Vec::new();
                    if let Some(prod) = info.product {
                        parts.push(prod);
                    } else if let Some(mfg) = info.manufacturer {
                        parts.push(mfg);
                    }
                    if parts.is_empty() {
                        format!("{} (USB Serial)", port_name)
                    } else {
                        format!("{} ({})", port_name, parts.join(" - "))
                    }
                }
                SerialPortType::PciPort => format!("{} (PCI)", port_name),
                SerialPortType::BluetoothPort => format!("{} (Bluetooth)", port_name),
                SerialPortType::Unknown => port_name.clone(),
            };

            result.push(SerialPortInfo {
                port_name,
                display_name,
                is_usb,
            });
        }
    }

    // Sort so USB/ACM ports appear first
    result.sort_by(|a, b| {
        let a_score = if a.is_usb || a.port_name.contains("ACM") || a.port_name.contains("USB") { 0 } else { 1 };
        let b_score = if b.is_usb || b.port_name.contains("ACM") || b.port_name.contains("USB") { 0 } else { 1 };
        a_score.cmp(&b_score).then_with(|| a.port_name.cmp(&b.port_name))
    });

    result
}

/// Connects to a serial port in a background worker thread.
/// Returns the session handle (for sending commands) and an event receiver (for receiving incoming data/status).
pub fn spawn_serial_connection(
    port_name: String,
    baud_rate: u32,
) -> Result<(ActiveSerialSession, Receiver<SerialRxEvent>), String> {
    let (event_tx, event_rx) = channel();
    let (cmd_tx, cmd_rx) = channel();

    let port_res = serialport::new(&port_name, baud_rate)
        .timeout(Duration::from_millis(50))
        .open();

    let mut port = match port_res {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to open port {}: {}", port_name, e)),
    };

    let session = ActiveSerialSession {
        port_name: port_name.clone(),
        baud_rate,
        tx_cmd: cmd_tx,
    };

    thread::spawn(move || {
        event_tx
            .send(SerialRxEvent::Connected {
                port_name: port_name.clone(),
                baud_rate,
            })
            .ok();

        let mut buf = [0u8; 1024];
        let mut running = true;

        while running {
            // Process outgoing commands from UI
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    SerialTxCommand::Send(text) => {
                        if let Err(e) = port.write_all(text.as_bytes()) {
                            event_tx.send(SerialRxEvent::Error(format!("Tx error: {}", e))).ok();
                        } else {
                            let _ = port.flush();
                        }
                    }
                    SerialTxCommand::Disconnect => {
                        running = false;
                        break;
                    }
                }
            }

            if !running {
                break;
            }

            // Read incoming bytes
            match port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    event_tx.send(SerialRxEvent::Data(chunk)).ok();
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // Timeout is expected, continue loop
                }
                Err(e) => {
                    event_tx.send(SerialRxEvent::Error(format!("Rx error: {}", e))).ok();
                    break;
                }
            }
        }

        event_tx.send(SerialRxEvent::Disconnected).ok();
    });

    Ok((session, event_rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_baud_rates_contain_standard_values() {
        assert!(COMMON_BAUD_RATES.contains(&9600));
        assert!(COMMON_BAUD_RATES.contains(&115200));
        assert!(COMMON_BAUD_RATES.contains(&921600));
    }

    #[test]
    fn test_enumerate_ports_executes_without_panic() {
        let ports = enumerate_serial_ports();
        // Just verify it returns a vector and doesn't crash
        println!("Enumerated {} serial ports", ports.len());
    }
}
