//! Network / Wi-Fi API module (NetworkManager / nmcli)

pub fn is_wifi_active() -> bool {
    if let Ok(output) = std::process::Command::new("nmcli")
        .args(&["radio", "wifi"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                return stdout.trim() == "enabled";
            }
        }
    }
    false
}

pub fn is_wifi_enabled() -> bool {
    is_wifi_active()
}

pub fn get_wifi_ssid() -> String {
    if let Ok(output) = std::process::Command::new("nmcli")
        .args(&["-t", "-f", "active,ssid", "dev", "wifi"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    let line = line.trim();
                    if line.starts_with("ja:") {
                        let ssid = line["ja:".len()..].trim();
                        if !ssid.is_empty() { return ssid.to_string(); }
                    }
                    if line.starts_with("yes:") {
                        let ssid = line["yes:".len()..].trim();
                        if !ssid.is_empty() { return ssid.to_string(); }
                    }
                }
            }
        }
    }

    if let Ok(output) = std::process::Command::new("nmcli")
        .args(&["-t", "-f", "TYPE,NAME", "connection", "show", "--active"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    if line.starts_with("802-11-wireless:") {
                        return line["802-11-wireless:".len()..].to_string();
                    }
                }
            }
        }
    }

    "Connected".to_string()
}

pub fn get_connected_ssid() -> String {
    get_wifi_ssid()
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

pub fn open_network_menu() {
    open_wifi_menu();
}
