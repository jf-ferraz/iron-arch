local packages = {
    "game-devices-udev",
    "sc-controller",
    "xboxdrv",
    "antimicrox",
    "piper",
    "joyutils",
    "xpadneo-dkms",
}

return {
    description = "Game controller drivers and tools for various gaming controllers",
    post_install_hook = "controller-support/scripts/install-controller-udev-rules.sh",
    packages = packages,
}
