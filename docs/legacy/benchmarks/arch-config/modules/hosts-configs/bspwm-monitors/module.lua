---@diagnostic disable: undefined-global
local hostname = dcli.system.hostname()

local dotfiles = {}

if hostname == "don-desktop" then
    table.insert(dotfiles, {
        source = "dotfiles/don-desktop.conf",
        target = "~/.config/bspwm/monitor-config.conf",
    })
    dcli.log.info("bspwm-monitors: Using don-desktop monitor configuration (DP-2 + DP-1)")
elseif hostname == "don-flow" then
    table.insert(dotfiles, {
        source = "dotfiles/don-flow.conf",
        target = "~/.config/bspwm/monitor-config.conf",
    })
    dcli.log.info("bspwm-monitors: Using don-flow monitor configuration (auto-detect)")
else
    dcli.log.warn("bspwm-monitors: No specific config for host '" .. hostname .. "', using auto-detect mode")
    table.insert(dotfiles, {
        source = "dotfiles/default.conf",
        target = "~/.config/bspwm/monitor-config.conf",
    })
end

return {
    description = "Host-specific BSPWM monitor configurations",
    dotfiles_sync = false,
    dotfiles = dotfiles,
}
