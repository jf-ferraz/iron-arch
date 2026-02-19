#!/bin/bash
# Toggle remote streaming mode - isolates HDMI-A-1 for Steam Deck streaming
# Changes HDMI-A-1 to 1280x800 and adds spacing between monitors to prevent cursor wandering
# Can be used as Sunshine prep/detached commands or bound to a keybind

STATE_FILE="${XDG_RUNTIME_DIR:-/tmp}/hypr-remote-mode-state"
PID_FILE="${XDG_RUNTIME_DIR:-/tmp}/hypr-remote-mode-pids"

# Normal positions (from monitors.conf)
NORMAL_DP1="3440x1440@180,3231x1388,1"
NORMAL_DP2="2560x1440@59.95,671x1284,1"
NORMAL_DP3="1920x1080@60,8591x1120,1"
NORMAL_HDMI="1920x1080@60,6671x1531,1"

# Remote streaming positions
# HDMI-A-1 stays in position but switches to Steam Deck resolution
# Other monitors move far away to prevent accidental cursor movement
REMOTE_HDMI="1920x1080@60,6671x1531,1"
REMOTE_DP1="3440x1440@180,-15000x1388,1"  # Move far left
REMOTE_DP2="2560x1440@59.95,25000x1284,1"  # Move far right
REMOTE_DP3="1920x1080@60,35000x1120,1"     # Move even further right

if [ -f "$STATE_FILE" ]; then
    # Remote mode is ON, switch to NORMAL
    echo "Switching to NORMAL mode - restoring monitors"

    # Close warning terminals
    if [ -f "$PID_FILE" ]; then
        while read pid; do
            kill "$pid" 2>/dev/null
        done < "$PID_FILE"
        rm "$PID_FILE"
    fi

    hyprctl keyword monitor "DP-1,$NORMAL_DP1"
    hyprctl keyword monitor "DP-2,$NORMAL_DP2"
    hyprctl keyword monitor "DP-3,$NORMAL_DP3"
    hyprctl keyword monitor "HDMI-A-1,$NORMAL_HDMI"

    # Restore DP-3 transform (vertical orientation)
    hyprctl keyword monitor "DP-3,transform,1"

    rm "$STATE_FILE"
    notify-send "Remote Mode OFF" "Monitors restored to normal positions" -i video-display 2>/dev/null || notify-send "Remote Mode OFF" "Monitors restored to normal positions"
else
    # Remote mode is OFF, switch to REMOTE
    echo "Switching to REMOTE mode - isolating HDMI-A-1 for streaming"
    hyprctl keyword monitor "HDMI-A-1,$REMOTE_HDMI"
    hyprctl keyword monitor "DP-1,$REMOTE_DP1"
    hyprctl keyword monitor "DP-2,$REMOTE_DP2"
    hyprctl keyword monitor "DP-3,$REMOTE_DP3"

    # Keep DP-3 in portrait orientation during remote mode
    hyprctl keyword monitor "DP-3,transform,1"

    # Launch warning terminals on all monitors and save their PIDs
    rm -f "$PID_FILE"

    # DP-1 terminal
    kitty --class kitty-floating -o font_size=32 -e bash -c 'echo -e "\n\n\n\n\n  DO NOT TOUCH THE PC\n\n  DON IS REMOTED INTO THIS COMPUTER\n\n\n"; read' &
    PID1=$!
    echo $PID1 >> "$PID_FILE"
    sleep 0.2
    hyprctl dispatch movewindow mon:DP-1

    # DP-2 terminal
    kitty --class kitty-floating -o font_size=32 -e bash -c 'echo -e "\n\n\n\n\n  DO NOT TOUCH THE PC\n\n  DON IS REMOTED INTO THIS COMPUTER\n\n\n"; read' &
    PID2=$!
    echo $PID2 >> "$PID_FILE"
    sleep 0.2
    hyprctl dispatch movewindow mon:DP-2

    # Move cursor back to HDMI-A-1 (streaming monitor)
    sleep 0.3
    hyprctl dispatch focusmonitor HDMI-A-1

    touch "$STATE_FILE"
    notify-send "Remote Mode ON" "HDMI-A-1 isolated at 1920x1080 (Steam Deck)" -i video-display 2>/dev/null || notify-send "Remote Mode ON" "HDMI-A-1 isolated at 1920x1080 (Steam Deck)"
fi
