local packages = {
    "ly",
}

return {
    description = "Ly TUI login manager",
    conflicts = {
        "login-managers/gdm-enable",
        "login-managers/lightdm-enable",
        "login-managers/sddm-enable",
        "login-managers/dms-greeter",
        "login-managers/sysc-greet",
    },
    post_install_hook = "scripts/enable-ly.sh",
    packages = packages,
}
