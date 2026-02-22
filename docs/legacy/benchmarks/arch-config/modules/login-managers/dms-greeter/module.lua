local packages = {
    "greetd",
    "greetd-dms-greeter-git",
}

return {
    description = "DankGreeter (greetd-based display manager with DMS theming)",
    conflicts = {
        "login-managers/gdm-enable",
        "login-managers/sddm-enable",
        "login-managers/lightdm-enable",
        "login-managers/sysc-greet",
    },
    post_install_hook = "scripts/enable-dms-greeter.sh",
    hook_behavior = "once",
    packages = packages,
}
