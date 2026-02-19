local packages = {
    "zen-browser-bin",
}

return {
    description = "Catppuccin Mocha theme for Zen Browser",
    post_install_hook = "scripts/install-zen-catppuccin.sh",
    hook_behavior = "once",
    packages = packages,
}
