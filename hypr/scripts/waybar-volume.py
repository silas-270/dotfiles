#!/usr/bin/env python3
import sys
import subprocess
import time
import json
import threading

last_change_time = 0
current_volume = 0
is_muted = False

def get_volume():
    try:
        out = subprocess.check_output(['wpctl', 'get-volume', '@DEFAULT_AUDIO_SINK@']).decode('utf-8').strip()
        parts = out.split()
        vol = 0
        muted = False
        if len(parts) >= 2:
            try:
                vol = int(float(parts[1]) * 100)
            except ValueError:
                pass
        if '[MUTED]' in out:
            muted = True
        return vol, muted
    except Exception:
        return 0, False

def print_waybar(show_pct):
    global current_volume, is_muted
    if is_muted:
        icon = "󰝟"
        text = f"[ {icon} ]"
    else:
        if current_volume == 0:
            icon = "󰕿"
        elif current_volume < 50:
            icon = "󰖀"
        else:
            icon = "󰕾"
        
        if show_pct:
            text = f"[ {icon} {current_volume}% ]"
        else:
            text = f"[ {icon} ]"
            
    print(json.dumps({"text": text, "tooltip": f"Volume: {current_volume}%"}), flush=True)

def timer_thread():
    global last_change_time
    while True:
        now = time.time()
        if last_change_time > 0 and now - last_change_time >= 3.0:
            last_change_time = 0
            print_waybar(False)
        time.sleep(0.2)

def main():
    global last_change_time, current_volume, is_muted
    current_volume, is_muted = get_volume()
    print_waybar(False)

    t = threading.Thread(target=timer_thread, daemon=True)
    t.start()

    # pactl subscribe works with pipewire-pulse to stream audio events
    p = subprocess.Popen(['pactl', 'subscribe'], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    while True:
        line = p.stdout.readline()
        if not line:
            break
        line = line.decode('utf-8')
        if 'sink' in line or 'server' in line:
            new_vol, new_muted = get_volume()
            # Only trigger UI update if actual volume/mute state changed
            if new_vol != current_volume or new_muted != is_muted:
                current_volume = new_vol
                is_muted = new_muted
                last_change_time = time.time()
                print_waybar(True)

if __name__ == '__main__':
    main()
