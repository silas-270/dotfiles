//! Compositor API module (Hyprland hyprctl shaders or Sway wlsunset)

pub fn is_blue_light_active() -> bool {
    let is_sway = std::env::var("SWAYSOCK").is_ok();
    if is_sway {
        std::process::Command::new("pgrep")
            .arg("wlsunset")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        if let Ok(output) = std::process::Command::new("hyprctl")
            .args(&["getoption", "decoration:screen_shader"])
            .output()
        {
            if output.status.success() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    return stdout.contains("blue-light.frag");
                }
            }
        }
        false
    }
}

pub fn set_blue_light_enabled(active: bool) {
    let is_sway = std::env::var("SWAYSOCK").is_ok();
    if is_sway {
        let _ = std::process::Command::new("killall").arg("wlsunset").status();
        if active {
            let _ = std::process::Command::new("wlsunset")
                .args(&["-t", "4500"])
                .spawn();
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/silas270".to_string());
        let shader = if active {
            format!("{}/.config/hypr/shaders/blue-light.frag", home)
        } else {
            "[[EMPTY]]".to_string()
        };
        let eval_str = format!("hl.config({{ decoration = {{ screen_shader = '{}' }} }})", shader);
        let _ = std::process::Command::new("hyprctl")
            .args(&["eval", &eval_str])
            .status();
    }
}

pub fn toggle_blue_light() -> bool {
    let new_state = !is_blue_light_active();
    set_blue_light_enabled(new_state);
    new_state
}
