local packages = {
    { name = "org.freedesktop.Platform.GL.default",          type = "flatpak" },
    { name = "org.freedesktop.Platform.GL.default",          type = "flatpak" },
    { name = "org.freedesktop.Platform.GL.nvidia-590-48-01", type = "flatpak" },
    { name = "org.freedesktop.Platform.openh264",            type = "flatpak" },
    { name = "org.kde.Platform",                             type = "flatpak" },
}

return {
    description = "Flatpak module for Arch Linux",
    packages = packages,
}
