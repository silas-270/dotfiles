//! Network / Wi-Fi API module (NetworkManager / nmcli)

fn nmcli(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("nmcli").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

pub fn is_wifi_active() -> bool {
    nmcli(&["radio", "wifi"]).map_or(false, |s| s.trim() == "enabled")
}

/// Which link is actually carrying traffic. Ethernet wins over Wi-Fi when both
/// are up, matching how the kernel routes.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum NetKind {
    #[default]
    None,
    Ethernet,
    Wifi,
}

#[derive(Clone, Default, Debug)]
pub struct NetInfo {
    pub kind: NetKind,
    /// Connection name for ethernet, SSID for Wi-Fi.
    pub name: String,
    /// Wi-Fi signal strength; meaningless for ethernet.
    pub signal: u8,
    /// Whether the Wi-Fi radio is on, independent of what carries traffic.
    pub wifi_enabled: bool,
    /// An ethernet interface exists (the cable may still be unplugged).
    pub eth_present: bool,
}

/// The connection the machine is actually using, preferring a plugged-in cable
/// over Wi-Fi. Both the panel and the bar key their icon and label off this.
pub fn get_net_info() -> NetInfo {
    let wifi_enabled = is_wifi_active();

    // TYPE uses stable identifiers; STATE would be localised.
    let eth_present = nmcli(&["-t", "--escape", "no", "-f", "TYPE", "device"])
        .map_or(false, |o| o.lines().any(|l| l.trim() == "ethernet"));

    let mut eth: Option<String> = None;
    let mut wifi: Option<String> = None;
    if let Some(out) = nmcli(&[
        "-t", "--escape", "no", "-f", "NAME,TYPE", "connection", "show", "--active",
    ]) {
        for line in out.lines() {
            let Some((name, ty)) = line.rsplit_once(':') else { continue };
            if name.is_empty() {
                continue;
            }
            match ty {
                "802-3-ethernet" => { eth.get_or_insert_with(|| name.to_string()); }
                "802-11-wireless" => { wifi.get_or_insert_with(|| name.to_string()); }
                _ => {}
            }
        }
    }

    if let Some(name) = eth {
        return NetInfo { kind: NetKind::Ethernet, name, signal: 0, wifi_enabled, eth_present };
    }

    if wifi.is_some() {
        let mut ssid = String::new();
        let mut signal = 0u8;
        if let Some(out) = nmcli(&[
            "-t", "--escape", "no", "-f", "IN-USE,SSID,SIGNAL",
            "device", "wifi", "list", "--rescan", "no",
        ]) {
            for line in out.lines() {
                let mut parts = line.splitn(3, ':');
                let (inuse, sd, sig) = (parts.next(), parts.next(), parts.next());
                if inuse == Some("*") {
                    ssid = sd.unwrap_or("").to_string();
                    signal = sig.unwrap_or("0").trim().parse().unwrap_or(0);
                    break;
                }
            }
        }
        let name = if ssid.is_empty() { wifi.unwrap_or_default() } else { ssid };
        if !name.is_empty() {
            return NetInfo { kind: NetKind::Wifi, name, signal, wifi_enabled, eth_present };
        }
    }

    NetInfo { kind: NetKind::None, name: String::new(), signal: 0, wifi_enabled, eth_present }
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
