---@diagnostic disable: undefined-global
local hostname = dcli.system.hostname()

local dotfiles = {}

if hostname == "don-desktop" then
    table.insert(dotfiles, {
        source = "dotfiles/niri/outputs-desktop.kdl",
        target = "~/.config/niri/outputs.kdl",
    })
elseif hostname == "don-flow" then
    table.insert(dotfiles, {
        source = "dotfiles/niri/outputs-flow.kdl",
        target = "~/.config/niri/outputs.kdl",
    })
else
    dcli.log.warn("niri-outputs: no outputs.kdl defined for host " .. hostname)
end

return {
    description = "Host-specific Niri outputs",
    dotfiles_sync = false,
    dotfiles = dotfiles,
}
