local packages = {
    "yazi",
}

return {
    description = "Yazi file manager with Catppuccin Mocha theme",
    dotfiles = {
        { source = "yazi-catppuccin/dotfiles/yazi", target = "~/.config/yazi" },
    },
    packages = packages,
}
