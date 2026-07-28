//! Display brightness API module (brightnessctl)

pub fn get_brightness() -> f64 {
    if let Ok(output) = std::process::Command::new("brightnessctl")
        .arg("-m")
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            let parts: Vec<&str> = stdout.trim().split(',').collect();
            if parts.len() >= 4 {
                let pct_str = parts[3].trim_end_matches('%');
                if let Ok(pct) = pct_str.parse::<f64>() {
                    return pct / 100.0;
                }
            }
        }
    }
    1.0 // default fallback
}

pub fn set_brightness(value: f64) {
    std::thread::spawn(move || {
        let pct = (value * 100.0).round() as i32;
        let _ = std::process::Command::new("brightnessctl")
            .args(&["set", &format!("{}%", pct)])
            .status();
    });
}
