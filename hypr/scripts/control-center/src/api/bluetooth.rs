//! Bluetooth API module (bluetoothctl / rfkill)

pub fn is_bluetooth_active() -> bool {
    if let Ok(output) = std::process::Command::new("bluetoothctl")
        .arg("show")
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            return stdout.contains("Powered: yes");
        }
    }
    false
}

pub fn set_bluetooth_enabled(active: bool) {
    std::thread::spawn(move || {
        if active {
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg("rfkill unblock bluetooth && sleep 0.5 && bluetoothctl power on")
                .status();
        } else {
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg("bluetoothctl power off && rfkill block bluetooth")
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
