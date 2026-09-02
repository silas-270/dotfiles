//! Network / Wi-Fi mutators (NetworkManager / nmcli).
//!
//! Polling lives in `shell_common::api::connectivity`; what stays here is the
//! panel-only half -- toggling the radio and opening the rofi menu. The bar has
//! no equivalent: it delegates radio changes to the menu script entirely.

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
