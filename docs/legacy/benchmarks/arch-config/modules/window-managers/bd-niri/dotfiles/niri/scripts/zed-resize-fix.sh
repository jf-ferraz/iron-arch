#!/bin/bash

# Launch Zeditor and apply resize fix
zeditor &

# Wait for the window to appear
sleep 0.5

# Trigger resize by adjusting column width to fix responsiveness
niri msg action set-column-width "+1"
sleep 0.05
niri msg action set-column-width "-1"
