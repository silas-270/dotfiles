use control_center::api;

fn main() {
    println!("=== TESTING CONTROL CENTER APIS ON THIS ARCH MACHINE ===");

    let brightness = api::brightness::get_brightness();
    println!("[1] Brightness: {:.2}%", brightness * 100.0);

    let blue_light = api::compositor::is_blue_light_active();
    println!("[2] Blue Light Filter Active: {}", blue_light);

    let bt_active = api::bluetooth::is_bluetooth_active();
    println!("[3] Bluetooth Active: {}", bt_active);

    let wifi_active = api::network::is_wifi_active();
    let wifi_status = api::network::get_wifi_status();
    let wifi_ssid = api::network::get_wifi_ssid();
    println!("[4] Wi-Fi Active: {}, Status: '{}', SSID: '{}'", wifi_active, wifi_status, wifi_ssid);

    let (vol, muted) = api::audio::get_volume();
    println!("[5] Audio Volume: {:.2}%, Muted: {}", vol * 100.0, muted);

    let media_state = api::media::get_media_state();
    println!("[6] Media State: Status = {:?}, Title = '{}', Artist = '{}'", media_state.status, media_state.metadata.title, media_state.metadata.artist);

    println!("=== ALL API TESTS EXECUTED SUCCESSFULLY ===");
}
