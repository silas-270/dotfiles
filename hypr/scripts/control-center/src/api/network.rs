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

pub fn is_wifi_enabled() -> bool {
    is_wifi_active()
}

/// Name of the wireless interface (e.g. `wlp2s0`), if one exists.
fn wifi_device() -> Option<String> {
    let out = nmcli(&["-t", "--escape", "no", "-f", "DEVICE,TYPE", "device"])?;
    out.lines()
        .find_map(|l| l.rsplit_once(':').filter(|(_, ty)| *ty == "wifi").map(|(d, _)| d.to_string()))
}

/// The SSID of the currently associated network, or an empty string when the
/// radio is on but nothing is connected.
///
/// Returning an empty string here is load-bearing: `widgets::connections`
/// renders "Disconnected" for it. The previous implementation fell back to the
/// literal string "Connected", which was then displayed as if it were an SSID.
pub fn get_wifi_ssid() -> String {
    let Some(dev) = wifi_device() else {
        return String::new();
    };

    // GENERAL.STATE is a numeric code (100 == connected), so this does not
    // depend on the user's locale the way matching on "yes"/"ja" did.
    let Some(out) = nmcli(&[
        "-t", "--escape", "no", "-f", "GENERAL.STATE,GENERAL.CONNECTION",
        "device", "show", &dev,
    ]) else {
        return String::new();
    };

    let mut connected = false;
    let mut connection = String::new();
    for line in out.lines() {
        if let Some(v) = line.strip_prefix("GENERAL.STATE:") {
            connected = v.trim().starts_with("100");
        } else if let Some(v) = line.strip_prefix("GENERAL.CONNECTION:") {
            connection = v.trim().to_string();
        }
    }

    if !connected || connection.is_empty() || connection == "--" {
        return String::new();
    }

    // Prefer the actual SSID over the profile name: NetworkManager appends
    // suffixes to duplicate profiles and users rename them.
    if let Some(ssid) = nmcli(&[
        "-t", "--escape", "no", "-g", "802-11-wireless.ssid",
        "connection", "show", &connection,
    ]) {
        let ssid = ssid.trim();
        if !ssid.is_empty() {
            return ssid.to_string();
        }
    }

    connection
}

pub fn get_connected_ssid() -> String {
    get_wifi_ssid()
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
