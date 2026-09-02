//! Cached Wi-Fi / Bluetooth state for the bar modules.
//!
//! The polling itself lives in `shell_common::api::connectivity`; this is the
//! bar's delivery mechanism for it. `nmcli` and `bluetoothctl` each cost tens to
//! hundreds of milliseconds, so they must never be called from the render path
//! (which runs on a 16ms loop) nor from the calloop timer (which would stall the
//! event loop once a second). A detached thread refreshes the snapshot; the
//! modules read it behind a short-lived lock.

use shell_common::api::connectivity::{poll_bt, poll_net, BtState, NetState};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// How often the snapshot is refreshed.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

static NET: Mutex<Option<NetState>> = Mutex::new(None);
static BT: Mutex<Option<BtState>> = Mutex::new(None);
static STARTED: OnceLock<()> = OnceLock::new();

/// Starts the refresh thread on first use; safe to call from the render path.
fn ensure_started() {
    STARTED.get_or_init(|| {
        std::thread::spawn(|| loop {
            let n = poll_net();
            if let Ok(mut g) = NET.lock() {
                *g = Some(n);
            }
            let b = poll_bt();
            if let Ok(mut g) = BT.lock() {
                *g = Some(b);
            }
            std::thread::sleep(POLL_INTERVAL);
        });
    });
}

/// The most recent network snapshot, or defaults before the first poll lands.
pub fn net_state() -> NetState {
    ensure_started();
    NET.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
}

/// The most recent bluetooth snapshot, or defaults before the first poll lands.
pub fn bt_state() -> BtState {
    ensure_started();
    BT.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
}
