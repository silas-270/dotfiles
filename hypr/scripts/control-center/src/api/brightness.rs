//! Display brightness API module (brightnessctl / sysfs / light)

use std::fs;

pub fn get_brightness() -> f64 {
    // 1. Try brightnessctl
    if let Ok(output) = std::process::Command::new("brightnessctl")
        .arg("-m")
        .output()
    {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = stdout.trim().split(',').collect();
                if parts.len() >= 4 {
                    let pct_str = parts[3].trim_end_matches('%');
                    if let Ok(pct) = pct_str.parse::<f64>() {
                        return (pct / 100.0).clamp(0.0, 1.0);
                    }
                }
            }
        }
    }

    // 2. Direct sysfs fallback: /sys/class/backlight/*
    if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
        for entry in entries.flatten() {
            let path = entry.path();
            let cur_file = path.join("brightness");
            let max_file = path.join("max_brightness");

            if let (Ok(cur_str), Ok(max_str)) = (fs::read_to_string(&cur_file), fs::read_to_string(&max_file)) {
                if let (Ok(cur), Ok(max)) = (cur_str.trim().parse::<f64>(), max_str.trim().parse::<f64>()) {
                    if max > 0.0 {
                        return (cur / max).clamp(0.0, 1.0);
                    }
                }
            }
        }
    }

    // 3. Try light -G
    if let Ok(output) = std::process::Command::new("light").arg("-G").output() {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Ok(pct) = stdout.trim().parse::<f64>() {
                    return (pct / 100.0).clamp(0.0, 1.0);
                }
            }
        }
    }

    1.0 // default fallback
}

pub fn set_brightness(value: f64) {
    std::thread::spawn(move || {
        let value = value.clamp(0.0, 1.0);
        let pct = (value * 100.0).round() as i32;

        // 1. Try brightnessctl
        let res = std::process::Command::new("brightnessctl")
            .args(&["set", &format!("{}%", pct)])
            .status();

        if res.is_ok() && res.unwrap().success() {
            return;
        }

        // 2. Try light
        let light_res = std::process::Command::new("light")
            .args(&["-S", &format!("{}", pct)])
            .status();

        if light_res.is_ok() && light_res.unwrap().success() {
            return;
        }

        // 3. Try DBus systemd-logind via busctl
        if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
            for entry in entries.flatten() {
                let dev_name = entry.file_name().to_string_lossy().to_string();
                let max_file = entry.path().join("max_brightness");
                if let Ok(max_str) = fs::read_to_string(&max_file) {
                    if let Ok(max) = max_str.trim().parse::<f64>() {
                        let target_val = (value * max).round() as u64;
                        let bus_res = std::process::Command::new("busctl")
                            .args(&[
                                "call",
                                "org.freedesktop.login1",
                                "/org/freedesktop/login1/session/auto",
                                "org.freedesktop.login1.Session",
                                "SetBrightness",
                                "ssu",
                                "backlight",
                                &dev_name,
                                &target_val.to_string(),
                            ])
                            .status();

                        if bus_res.is_ok() && bus_res.unwrap().success() {
                            return;
                        }

                        // 4. Fallback to direct file write if permissions allow
                        let cur_file = entry.path().join("brightness");
                        let _ = fs::write(cur_file, target_val.to_string());
                    }
                }
            }
        }
    });
}


