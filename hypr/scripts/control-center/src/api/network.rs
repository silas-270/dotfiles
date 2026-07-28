//! Network / Wi-Fi API module (NetworkManager / nmcli)

pub fn is_wifi_active() -> bool {
    if let Ok(output) = std::process::Command::new("nmcli")
        .args(&["radio", "wifi"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            return stdout.trim() == "enabled";
        }
    }
    false
}

pub fn get_wifi_ssid() -> String {
    if let Ok(output) = std::process::Command::new("nmcli")
        .args(&["-t", "-f", "active,ssid", "dev", "wifi"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            for line in stdout.lines() {
                if line.starts_with("ja:") {
                    return line["ja:".len()..].to_string();
                }
                if line.starts_with("yes:") {
                    return line["yes:".len()..].to_string();
                }
            }
        }
    }
    "Connected".to_string()
}

pub fn get_wifi_status() -> String {
    if is_wifi_active() {
        let ssid = get_wifi_ssid();
        if ssid == "Connected" || ssid.is_empty() {
            "On".to_string()
        } else {
            ssid
        }
    } else {
        "Off".to_string()
    }
}

pub fn set_wifi_enabled(active: bool) {
    std::thread::spawn(move || {
        let status = if active { "on" } else { "off" };
        let _ = std::process::Command::new("nmcli")
            .args(&["radio", "wifi", status])
            .status();
    });
}

pub fn open_wifi_menu() {
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg("~/.config/hypr/scripts/rofi-wifi-menu.sh &")
        .status();
}
