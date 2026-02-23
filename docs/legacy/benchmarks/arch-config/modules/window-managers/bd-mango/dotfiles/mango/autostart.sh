#!/bin/bash


set +e

# obs
dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP=wlroots >/dev/null 2>&1

# xdg desktop portal for wlroots
systemctl --user start xdg-desktop-portal-wlr.service >/dev/null 2>&1 &

# xwayland dpi scale
echo "Xft.dpi: 96" | xrdb -merge #dpi缩放
# xrdb merge ~/.Xresources >/dev/null 2>&1

# noctalia shell
QT_WAYLAND_FORCE_DPI=96 qs -c noctalia-shell >/dev/null 2>&1 &

# ime input
fcitx5 --replace -d >/dev/null 2>&1 &

# keep clipboard content
wl-clip-persist --clipboard regular --reconnect-tries 0 >/dev/null 2>&1 &

# clipboard content manager
wl-paste --type text --watch cliphist store >/dev/null 2>&1 &

# inhibit by audio
sway-audio-idle-inhibit >/dev/null 2>&1 &
