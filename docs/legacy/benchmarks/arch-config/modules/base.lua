local packages = {
    "base",
    "base-devel",
    "linux-firmware",
    "networkmanager",
    "vim",
    "git",
    "htop",
    "man-db",
    "man-pages",
}

return {
    description = "Base packages for all machines",
    packages = packages,
}
