#!/usr/bin/env bash
# waybar-network.sh - Real-time network status daemon for Waybar

# WiFi connecting frames (Nerd Font wifi icons from low to high strength)
frames=("󰤯" "󰤟" "󰤢" "󰤥" "󰖩")
frame_index=0

while true; do
    # Get status of network devices
    devs=$(nmcli -t -f TYPE,STATE,CONNECTION device 2>/dev/null || echo "")
    
    # Prioritize ethernet first
    eth_line=$(echo "$devs" | grep '^ethernet:' | head -n1)
    wifi_line=$(echo "$devs" | grep '^wifi:' | head -n1)
    
    text=""
    class=""
    tooltip=""
    
    if [[ -n "$eth_line" ]]; then
        IFS=':' read -r type state connection <<< "$eth_line"
        if [[ "$state" == "connected" ]]; then
            text="󰈀 $connection"
            class="ethernet"
            ip=$(ip route get 1.1.1.1 2>/dev/null | awk '{print $7; exit}')
            tooltip="Ethernet verbunden\nIP: ${ip:-unbekannt}"
        fi
    fi
    
    # Fallback to wifi if ethernet is not connected
    if [[ -z "$text" && -n "$wifi_line" ]]; then
        IFS=':' read -r type state connection <<< "$wifi_line"
        
        # Check if the device is in a connecting state
        if [[ "$state" =~ ^connecting ]]; then
            icon="${frames[$frame_index]}"
            frame_index=$(( (frame_index + 1) % 5 ))
            text="$icon Verbinde..."
            class="connecting"
            tooltip="Verbindung wird aufgebaut zu: $connection"
        elif [[ "$state" == "connected" ]]; then
            text="󰖩 $connection"
            class="connected"
            ip=$(ip route get 1.1.1.1 2>/dev/null | awk '{print $7; exit}')
            tooltip="WLAN: $connection\nIP: ${ip:-unbekannt}"
        else
            text="󰖪 offline"
            class="disconnected"
            tooltip="WLAN nicht verbunden"
        fi
    fi
    
    # Default fallback
    if [[ -z "$text" ]]; then
        text="󰖪 offline"
        class="disconnected"
        tooltip="Keine Netzwerkgeräte gefunden"
    fi
    
    # Output json format for waybar
    echo "{\"text\": \"$text\", \"class\": \"$class\", \"tooltip\": \"$tooltip\"}"
    
    sleep 0.5
done
