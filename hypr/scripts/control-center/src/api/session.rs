//! Session control API module (Hyprlock, Systemd Suspend/Reboot/Poweroff)

use std::process::Command;

pub fn lock() {
    std::thread::spawn(|| {
        let _ = Command::new("hyprlock").spawn();
    });
}

pub fn suspend() {
    std::thread::spawn(|| {
        let _ = Command::new("systemctl").arg("suspend").spawn();
    });
}

pub fn reboot() {
    std::thread::spawn(|| {
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });
}

pub fn poweroff() {
    std::thread::spawn(|| {
        let _ = Command::new("systemctl").arg("poweroff").spawn();
    });
}
