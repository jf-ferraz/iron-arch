-- Flutter Development Module
-- Installs Flutter SDK with Linux desktop, Android, and Web support
-- Reference: https://docs.flutter.dev/platform-integration/linux/setup

local packages = {
    -- Core development tools (from Flutter Linux requirements)
    "clang",
    "cmake",
    "ninja",
    "gtk3",
    "libstdc++5",
    
    -- Required by Flutter SDK
    "curl",
    "git",
    "unzip",
    "xz",
    "zip",
    
    -- Additional tools for Flutter development
    "file",
    "which",
    
    -- IDE and debugging
    "android-studio",
    "chromium",
    
    -- Java (required for Android Studio/Flutter)
    "jdk17-openjdk",
}

return {
    description = "Flutter SDK with Linux, Android, and Web development support",
    packages = packages,
    conflicts = {},
    post_install_hook = "scripts/install-flutter.sh",
    hook_behavior = "once",
    metadata = {
        author = "arch-config",
        version = "1.0.0",
        category = "development",
        flutter_version = "stable",
    },
}
