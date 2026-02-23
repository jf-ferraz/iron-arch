---@diagnostic disable: undefined-global

local hostname = dcli.system.hostname()
local desktop = (dcli.env.get("XDG_CURRENT_DESKTOP") or dcli.env.get("DESKTOP_SESSION") or ""):lower()

local desktop_map = {
    hypr = "hyprland",
    hyprland = "hyprland",
    ["hyprland-x11"] = "hyprland",
    ["hyprland-wayland"] = "hyprland",
    niri = "niri",
    mango = "mango",
    kde = "kde",
    plasma = "kde",
    plasmawayland = "kde",
    cosmic = "cosmic",
    ["cosmic-xero"] = "cosmic",
}

local host_variants = {
    ["don-desktop"] = "niri-desktop",
    ["don-flow"] = "niri-flow",
    ["don-vm"] = "niri",
}

local variant = host_variants[hostname] or desktop_map[desktop] or "niri"

local dotfiles = {
    {
        source = "dotfiles/noctalia/" .. variant,
        target = "~/.config/noctalia",
    },
}

if host_variants[hostname] == nil and desktop_map[desktop] == nil then
    dcli.log.warn("noctalia-ui: unknown host/desktop, defaulting to niri variant")
end

return {
    description = "Host/desktop-aware Noctalia UI configuration",
    dotfiles_sync = false,
    dotfiles = dotfiles,
}
