#!/bin/bash

# Get current layout
current_layout=$(hyprctl getoption general:layout | grep -oP 'str: \K\w+')

# Toggle between scrolling and dwindle
if [ "$current_layout" = "dwindle" ]; then
    hyprctl keyword general:layout scrolling
else
    hyprctl keyword general:layout dwindle
fi
