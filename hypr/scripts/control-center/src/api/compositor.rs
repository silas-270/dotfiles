//! Compositor API module (Hyprland hyprctl shaders)

pub fn is_blue_light_active() -> bool {
    if let Ok(output) = std::process::Command::new("hyprctl")
        .args(&["getoption", "decoration:screen_shader"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            return stdout.contains("blue-light.frag");
        }
    }
    false
}

pub fn set_blue_light_enabled(active: bool) {
    let shader = if active {
        "/home/silas270/.config/hypr/shaders/blue-light.frag"
    } else {
        "[[EMPTY]]"
    };
    std::thread::spawn(move || {
        let _ = std::process::Command::new("hyprctl")
            .args(&["keyword", "decoration:screen_shader", shader])
            .status();
    });
}
