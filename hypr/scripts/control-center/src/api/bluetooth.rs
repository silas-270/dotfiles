//! Bluetooth API module (bluetoothctl / rfkill)

fn bluetoothctl(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("bluetoothctl").args(args).output().ok()?;
    // bluetoothctl exits 0 even for failed operations, so callers must inspect
    // the output rather than trusting the status.
    String::from_utf8(output.stdout).ok()
}

pub fn is_bluetooth_active() -> bool {
    if let Some(stdout) = bluetoothctl(&["show"]) {
        if stdout.contains("Controller ") {
            return stdout.contains("Powered: yes");
        }
    }

    if let Ok(output) = std::process::Command::new("rfkill")
        .args(&["list", "bluetooth"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if stdout.contains("Soft blocked: no") && stdout.contains("Hard blocked: no") {
                    return true;
                }
            }
        }
    }
    false
}

pub fn is_bluetooth_enabled() -> bool {
    is_bluetooth_active()
}

/// The name of the currently connected device and its battery percentage where
/// the device reports one. `None` when the adapter is on but nothing is
/// connected - the panel must show "Disconnected" in that case rather than
/// assuming that a powered adapter means a connected device.
pub fn get_connected_device() -> Option<(String, Option<u8>)> {
    let out = bluetoothctl(&["devices", "Connected"])?;

    // "Device 80:C3:BA:A2:EC:5D MOMENTUM TW 4"
    let (mac, name) = out.lines().find_map(|line| {
        let rest = line.strip_prefix("Device ")?;
        let (mac, name) = rest.split_once(' ')?;
        Some((mac.to_string(), name.trim().to_string()))
    })?;

    let battery = bluetoothctl(&["info", &mac]).and_then(|info| {
        // "\tBattery Percentage: 0x64 (100)"
        info.lines()
            .find_map(|l| l.trim().strip_prefix("Battery Percentage:"))
            .and_then(|v| v.split_once('('))
            .and_then(|(_, r)| r.split_once(')'))
            .and_then(|(n, _)| n.trim().parse::<u8>().ok())
    });

    let label = if name.is_empty() { mac } else { name };
    Some((label, battery))
}

/// Status line for the connections widget: the device name (with battery when
/// reported), or an empty string when nothing is connected.
pub fn get_bluetooth_status() -> String {
    match get_connected_device() {
        Some((name, Some(batt))) => format!("{} {}%", name, batt),
        Some((name, None)) => name,
        None => String::new(),
    }
}

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
                if is_bluetooth_active() {
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
