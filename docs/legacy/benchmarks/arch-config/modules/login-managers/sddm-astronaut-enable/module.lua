local packages = {
    "sddm",
    "git",
    "qt6-svg",
    "qt6-virtualkeyboard",
    "qt6-multimedia-ffmpeg",
}

return {
    description = "SDDM login manager with Astronaut pixel_sakura theme",
    conflicts = {
        "login-managers/gdm-enable",
        "login-managers/lightdm-enable",
        "login-managers/dms-greeter",
        "login-managers/ly-enable",
        "login-managers/sddm-cat-enable",
        "login-managers/sysc-greet",
    },
    services = {
        enabled = { "sddm" },
    },
    post_install_hook = "scripts/enable-sddm.sh",
    hook_behavior = "once",
    packages = packages,
}
