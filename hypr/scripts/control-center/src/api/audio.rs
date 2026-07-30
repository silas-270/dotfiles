//! Audio API module (WirePlumber / PulseAudio / ALSA)

pub fn get_volume() -> (f64, bool) {
    if let Ok(output) = std::process::Command::new("wpctl")
        .args(&["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let is_muted = stdout.contains("[MUTED]");
                let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(vol) = parts[1].parse::<f64>() {
                        return (vol.clamp(0.0, 1.0), is_muted);
                    }
                }
            }
        }
    }

    if let Ok(output) = std::process::Command::new("pactl")
        .args(&["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Some(pos) = stdout.find('%') {
                    let start = stdout[..pos].rfind(|c: char| c.is_ascii_whitespace()).map_or(0, |i| i + 1);
                    if let Ok(pct) = stdout[start..pos].trim().parse::<f64>() {
                        let is_muted = if let Ok(mute_out) = std::process::Command::new("pactl")
                            .args(&["get-sink-mute", "@DEFAULT_SINK@"])
                            .output()
                        {
                            let mute_str = String::from_utf8_lossy(&mute_out.stdout);
                            mute_str.contains("yes") || mute_str.contains("ja") || mute_str.contains("true")
                        } else {
                            false
                        };
                        return ((pct / 100.0).clamp(0.0, 1.0), is_muted);
                    }
                }
            }
        }
    }

    if let Ok(output) = std::process::Command::new("amixer")
        .args(&["sget", "Master"])
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let is_muted = stdout.contains("[off]");
                if let Some(pos) = stdout.find('%') {
                    let start = stdout[..pos].rfind('[').map_or(0, |i| i + 1);
                    if let Ok(pct) = stdout[start..pos].trim().parse::<f64>() {
                        return ((pct / 100.0).clamp(0.0, 1.0), is_muted);
                    }
                }
            }
        }
    }

    (0.7, false)
}

pub fn set_volume(value: f64) {
    std::thread::spawn(move || {
        let pct = (value * 100.0).round() as i32;
        let wp_res = std::process::Command::new("wpctl")
            .args(&["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &format!("{}%", pct)])
            .status();

        if wp_res.is_err() || !wp_res.unwrap().success() {
            let pactl_res = std::process::Command::new("pactl")
                .args(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", pct)])
                .status();

            if pactl_res.is_err() || !pactl_res.unwrap().success() {
                let _ = std::process::Command::new("amixer")
                    .args(&["sset", "Master", &format!("{}%", pct)])
                    .status();
            }
        }
    });
}

pub fn set_mute(muted: bool) {
    std::thread::spawn(move || {
        let val = if muted { "1" } else { "0" };
        let wp_res = std::process::Command::new("wpctl")
            .args(&["set-mute", "@DEFAULT_AUDIO_SINK@", val])
            .status();

        if wp_res.is_err() || !wp_res.unwrap().success() {
            let p_val = if muted { "1" } else { "0" };
            let pactl_res = std::process::Command::new("pactl")
                .args(&["set-sink-mute", "@DEFAULT_SINK@", p_val])
                .status();

            if pactl_res.is_err() || !pactl_res.unwrap().success() {
                let amix_val = if muted { "mute" } else { "unmute" };
                let _ = std::process::Command::new("amixer")
                    .args(&["sset", "Master", amix_val])
                    .status();
            }
        }
    });
}

