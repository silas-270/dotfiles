//! Bluetooth API module (bluetoothctl / rfkill)

pub fn is_bluetooth_active() -> bool {
    // 1. Try bluetoothctl show
    if let Ok(output) = std::process::Command::new("bluetoothctl")
        .arg("show")
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                return stdout.contains("Powered: yes");
            }
        }
    }

    // 2. Fallback to rfkill
    if let Ok(output) = std::process::Command::new("rfkill")
        .args(&["list", "bluetooth"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if stdout.contains("Soft blocked: no") && stdout.contains("Hard blocked: no") {
                    return true;
                }
                if stdout.contains("Soft blocked: yes") || stdout.contains("Hard blocked: yes") {
                    return false;
                }
            }
        }
    }

    false
}

pub fn set_bluetooth_enabled(active: bool) {
    std::thread::spawn(move || {
        if active {
            let _ = std::process::Command::new("rfkill")
                .args(&["unblock", "bluetooth"])
                .status();
            let _ = std::process::Command::new("bluetoothctl")
                .args(&["power", "on"])
                .status();
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

