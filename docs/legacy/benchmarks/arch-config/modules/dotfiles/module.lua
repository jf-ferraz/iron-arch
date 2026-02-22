local packages = {
    "qt5ct",
    "qt6ct",
    "gtk3",
    "gtk4",
    "catppuccin-gtk-theme-mocha",
    "tela-icon-theme-purple-git",
    "bibata-cursor-theme-bin",
    "matugen-bin",
    "qt6-wayland",
    "qt5-wayland",
    "kvantum-theme-catppuccin-git",
}

return {
    description = "Dotfiles for my WM themes",
    dotfiles_sync = true,
    packages = packages,
}
