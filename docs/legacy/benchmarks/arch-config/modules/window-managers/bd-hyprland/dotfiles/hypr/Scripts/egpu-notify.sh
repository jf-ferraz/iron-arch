#!/bin/bash
# Check if AMD eGPU is connected and notify user
# System defaults to Intel iGPU - use egpu-run to launch apps with eGPU

sleep 2  # Wait for system to settle

if lspci -n | grep -q "0b:00.0.*1002:"; then
    notify-send "eGPU Detected" "AMD Radeon RX 7600M XT available\nUsing Intel iGPU by default\nUse 'egpu-run <app>' to run with eGPU" -i video-display
else
    notify-send "eGPU Not Detected" "Using Intel integrated graphics" -i video-display
fi
