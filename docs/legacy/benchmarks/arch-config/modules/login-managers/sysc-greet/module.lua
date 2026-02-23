local packages = {
    "sysc-greet",
}

return {
    description = "Sysc-greet login manager with Niri support",
    conflicts = {
        "login-managers/gdm-enable",
        "login-managers/lightdm-enable",
        "login-managers/sddm-enable",
    },
    post_install_hook = "scripts/install-sysc-greet.sh",
    hook_behavior = "once",
    packages = packages,
}
