//! Cached Wi-Fi / Bluetooth state for the bar modules.
//!
//! `nmcli` and `bluetoothctl` each cost tens to hundreds of milliseconds, so
//! they must never be called from the render path (which runs on a 16ms loop)
//! nor from the calloop timer (which would stall the event loop once a second).
//! A detached thread refreshes the snapshot; the modules read it behind a
//! short-lived lock.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Which link is actually carrying traffic. Ethernet wins over Wi-Fi when both
/// are up, matching how the kernel routes.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum NetKind {
    #[default]
    None,
    Ethernet,
    Wifi,
}

#[derive(Clone, Default)]
pub struct NetState {
    pub kind: NetKind,
    /// Connection name for ethernet, SSID for Wi-Fi.
    pub name: String,
    /// Wi-Fi signal strength; meaningless for ethernet.
    pub signal: u8,
    /// Whether the Wi-Fi radio is on, independent of what is carrying traffic.
    pub wifi_enabled: bool,
    /// An ethernet interface exists but has no carrier (cable unplugged).
    pub eth_present: bool,
}

impl NetState {
    pub fn connected(&self) -> bool {
        self.kind != NetKind::None && !self.name.is_empty()
    }
}

#[derive(Clone, Default)]
pub struct BtState {
    pub powered: bool,
    pub device: String,
    pub battery: Option<u8>,
}

impl BtState {
    pub fn connected(&self) -> bool {
        self.powered && !self.device.is_empty()
    }
}

static NET: Mutex<Option<NetState>> = Mutex::new(None);
static BT: Mutex<Option<BtState>> = Mutex::new(None);
static STARTED: OnceLock<()> = OnceLock::new();

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).output().ok()?;
    String::from_utf8(out.stdout).ok()
}

fn poll_net() -> NetState {
    let wifi_enabled = run("nmcli", &["radio", "wifi"]).map_or(false, |s| s.trim() == "enabled");

    // An ethernet interface exists at all (cable may or may not be plugged in).
    // The TYPE column uses stable identifiers, unlike STATE which is localised.
    let eth_present = run("nmcli", &["-t", "--escape", "no", "-f", "TYPE", "device"])
        .map_or(false, |o| o.lines().any(|l| l.trim() == "ethernet"));

    // Active connections carry stable type ids: 802-3-ethernet / 802-11-wireless.
    let mut eth: Option<String> = None;
    let mut wifi: Option<String> = None;
    if let Some(out) = run(
        "nmcli",
        &["-t", "--escape", "no", "-f", "NAME,TYPE", "connection", "show", "--active"],
    ) {
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
        if let Some(out) = run(
            "nmcli",
            &["-t", "--escape", "no", "-f", "IN-USE,SSID,SIGNAL", "device", "wifi", "list", "--rescan", "no"],
        ) {
            for line in out.lines() {
                let mut parts = line.splitn(3, ':');
                let (inuse, s, sig) = (parts.next(), parts.next(), parts.next());
                if inuse == Some("*") {
                    ssid = s.unwrap_or("").to_string();
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

fn poll_bt() -> BtState {
    let show = run("bluetoothctl", &["show"]).unwrap_or_default();
    let powered = show.contains("Controller ") && show.contains("Powered: yes");
    if !powered {
        return BtState { powered: false, ..Default::default() };
    }

    let Some(out) = run("bluetoothctl", &["devices", "Connected"]) else {
        return BtState { powered, ..Default::default() };
    };

    let Some((mac, name)) = out.lines().find_map(|line| {
        let rest = line.strip_prefix("Device ")?;
        let (mac, name) = rest.split_once(' ')?;
        Some((mac.to_string(), name.trim().to_string()))
    }) else {
        return BtState { powered, ..Default::default() };
    };

    let battery = run("bluetoothctl", &["info", &mac]).and_then(|info| {
        info.lines()
            .find_map(|l| l.trim().strip_prefix("Battery Percentage:"))
            .and_then(|v| v.split_once('('))
            .and_then(|(_, r)| r.split_once(')'))
            .and_then(|(n, _)| n.trim().parse::<u8>().ok())
    });

    BtState { powered, device: if name.is_empty() { mac } else { name }, battery }
}

/// Starts the refresh thread on first use; safe to call from the render path.
fn ensure_started() {
    STARTED.get_or_init(|| {
        std::thread::spawn(|| loop {
            let n = poll_net();
            *NET.lock().unwrap() = Some(n);
            let b = poll_bt();
            *BT.lock().unwrap() = Some(b);
            std::thread::sleep(Duration::from_secs(3));
        });
    });
}

pub fn net_state() -> NetState {
    ensure_started();
    NET.lock().unwrap().clone().unwrap_or_default()
}

pub fn bt_state() -> BtState {
    ensure_started();
    BT.lock().unwrap().clone().unwrap_or_default()
}
