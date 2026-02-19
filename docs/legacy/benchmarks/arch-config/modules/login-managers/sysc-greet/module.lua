local packages = {
    "sysc-greet-hyprland",
}

return {
    description = "Sysc-greet login manager with Hyprland support",
    conflicts = {
        "login-managers/gdm-enable",
        "login-managers/lightdm-enable",
        "login-managers/sddm-enable",
    },
    post_install_hook = "scripts/install-sysc-greet.sh",
    packages = packages,
}
