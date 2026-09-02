//! Bluetooth mutators (bluetoothctl / rfkill).
//!
//! Polling lives in `shell_common::api::connectivity`; what stays here is the
//! panel-only half -- powering the adapter and opening the rofi menu.

use shell_common::api::connectivity::is_bluetooth_powered;

pub fn set_bluetooth_enabled(active: bool) {
    std::thread::spawn(move || {
        if active {
            // Order matters: while the adapter is rfkill soft-blocked,
            // `power on` fails with org.bluez.Error.Blocked.
            let _ = std::process::Command::new("rfkill")
                .args(&["unblock", "bluetooth"])
                .status();
            for _ in 0..10 {
                let _ = std::process::Command::new("bluetoothctl")
                    .args(&["power", "on"])
                    .status();
                if is_bluetooth_powered() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        } else {
            let _ = std::process::Command::new("bluetoothctl")
                .args(&["power", "off"])
                .status();
            let _ = std::process::Command::new("rfkill")
                .args(&["block", "bluetooth"])
                .status();
        }
    });
}

pub fn open_bluetooth_menu() {
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg("~/.config/hypr/scripts/rofi-bluetooth-menu.sh &")
        .status();
}
