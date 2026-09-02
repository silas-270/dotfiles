//! Wi-Fi and Bluetooth state, shared by both surfaces.
//!
//! `nmcli` and `bluetoothctl` cost tens of milliseconds per invocation, so
//! neither surface calls these from its render path. The two do it differently
//! and both are correct for their shape: rustbar refreshes a snapshot on a
//! detached thread and reads it behind a short-lived lock (see
//! `rustbar::api::connectivity`), while control-center polls from its sync
//! thread and pushes results into the event loop over a channel. What is shared
//! is the polling itself — the commands, the parsing, and the precedence rules.

/// Runs `nmcli` and returns stdout, or `None` if it fails.
///
/// nmcli reports real failures through its exit status, so it is checked.
fn nmcli(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("nmcli").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Runs `bluetoothctl` and returns stdout.
///
/// Unlike nmcli, bluetoothctl exits 0 even for failed operations, so the status
/// is deliberately not checked — callers must inspect the output instead.
fn bluetoothctl(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("bluetoothctl").args(args).output().ok()?;
    String::from_utf8(output.stdout).ok()
}

// ── Network ─────────────────────────────────────────────────────────────────

/// Which link is actually carrying traffic. Ethernet wins over Wi-Fi when both
/// are up, matching how the kernel routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetKind {
    #[default]
    None,
    Ethernet,
    Wifi,
}

#[derive(Debug, Clone, Default)]
pub struct NetState {
    pub kind: NetKind,
    /// Connection name for ethernet, SSID for Wi-Fi.
    pub name: String,
    /// Wi-Fi signal strength; meaningless for ethernet.
    pub signal: u8,
    /// Whether the Wi-Fi radio is on, independent of what is carrying traffic.
    pub wifi_enabled: bool,
    /// An ethernet interface exists (the cable may still be unplugged).
    pub eth_present: bool,
}

impl NetState {
    pub fn connected(&self) -> bool {
        self.kind != NetKind::None && !self.name.is_empty()
    }
}

/// Whether the Wi-Fi radio is switched on, independent of any connection.
pub fn is_wifi_enabled() -> bool {
    nmcli(&["radio", "wifi"]).map_or(false, |s| s.trim() == "enabled")
}

/// The connection the machine is actually using, preferring a plugged-in cable
/// over Wi-Fi. Both the panel and the bar key their icon and label off this.
pub fn poll_net() -> NetState {
    let wifi_enabled = is_wifi_enabled();

    // TYPE uses stable identifiers; STATE would be localised.
    let eth_present = nmcli(&["-t", "--escape", "no", "-f", "TYPE", "device"])
        .map_or(false, |o| o.lines().any(|l| l.trim() == "ethernet"));

    // Active connections carry stable type ids: 802-3-ethernet / 802-11-wireless.
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

    // Ethernet takes precedence: if the cable is in, that is the live route.
    if let Some(name) = eth {
        return NetState { kind: NetKind::Ethernet, name, signal: 0, wifi_enabled, eth_present };
    }

    if wifi.is_some() {
        // Prefer the real SSID and signal from the IN-USE row over the profile name.
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
            return NetState { kind: NetKind::Wifi, name, signal, wifi_enabled, eth_present };
        }
    }

    NetState { kind: NetKind::None, name: String::new(), signal: 0, wifi_enabled, eth_present }
}

// ── Bluetooth ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct BtState {
    pub powered: bool,
    pub device: String,
    pub battery: Option<u8>,
}

impl BtState {
    pub fn connected(&self) -> bool {
        self.powered && !self.device.is_empty()
    }

    /// Status line for the connections widget: the device name (with battery
    /// when the device reports one), or an empty string when nothing is
    /// connected. A powered adapter alone does not mean a connected device.
    pub fn status_label(&self) -> String {
        match (self.device.as_str(), self.battery) {
            ("", _) => String::new(),
            (name, Some(batt)) => format!("{} {}%", name, batt),
            (name, None) => name.to_string(),
        }
    }
}

/// Whether the adapter is powered.
///
/// Falls back to `rfkill` when `bluetoothctl` reports no controller at all —
/// which happens while bluetoothd is still starting, on a DBus hiccup, and in
/// the window right after an rfkill unblock. Without the fallback the adapter
/// reads as off in exactly those moments.
pub fn is_bluetooth_powered() -> bool {
    if let Some(stdout) = bluetoothctl(&["show"]) {
        if stdout.contains("Controller ") {
            return stdout.contains("Powered: yes");
        }
    }

    if let Ok(output) = std::process::Command::new("rfkill").args(["list", "bluetooth"]).output() {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if stdout.contains("Soft blocked: no") && stdout.contains("Hard blocked: no") {
                    return true;
                }
            }
        }
    }
    false
}

/// Adapter power state plus the connected device and its battery, if any.
pub fn poll_bt() -> BtState {
    let powered = is_bluetooth_powered();
    if !powered {
        return BtState::default();
    }

    let Some(out) = bluetoothctl(&["devices", "Connected"]) else {
        return BtState { powered, ..Default::default() };
    };

    // "Device 80:C3:BA:A2:EC:5D MOMENTUM TW 4"
    let Some((mac, name)) = out.lines().find_map(|line| {
        let rest = line.strip_prefix("Device ")?;
        let (mac, name) = rest.split_once(' ')?;
        Some((mac.to_string(), name.trim().to_string()))
    }) else {
        return BtState { powered, ..Default::default() };
    };

    let battery = bluetoothctl(&["info", &mac]).and_then(|info| {
        // "\tBattery Percentage: 0x64 (100)"
        info.lines()
            .find_map(|l| l.trim().strip_prefix("Battery Percentage:"))
            .and_then(|v| v.split_once('('))
            .and_then(|(_, r)| r.split_once(')'))
            .and_then(|(n, _)| n.trim().parse::<u8>().ok())
    });

    BtState { powered, device: if name.is_empty() { mac } else { name }, battery }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_label_formats_device_and_battery() {
        let s = BtState { powered: true, device: "MOMENTUM TW 4".into(), battery: Some(100) };
        assert_eq!(s.status_label(), "MOMENTUM TW 4 100%");
    }

    #[test]
    fn status_label_omits_missing_battery() {
        let s = BtState { powered: true, device: "Keyboard".into(), battery: None };
        assert_eq!(s.status_label(), "Keyboard");
    }

    /// A powered adapter with nothing paired must not read as connected.
    #[test]
    fn powered_but_disconnected_is_not_connected() {
        let s = BtState { powered: true, device: String::new(), battery: None };
        assert!(!s.connected());
        assert_eq!(s.status_label(), "");
    }

    #[test]
    fn net_connected_requires_a_named_link() {
        assert!(!NetState::default().connected());
        let named = NetState { kind: NetKind::Wifi, name: "home".into(), ..Default::default() };
        assert!(named.connected());
        let unnamed = NetState { kind: NetKind::Wifi, ..Default::default() };
        assert!(!unnamed.connected());
    }
}
