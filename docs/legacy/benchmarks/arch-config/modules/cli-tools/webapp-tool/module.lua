local packages = {
    "gum",
    "curl",
    "gtk3",
    "chromium",
}

return {
    description = "Web app installer tools for creating desktop entries from URLs",
    author = "theblackdon",
    version = "1.0.0",
    post_install_hook = "scripts/install-webapp-tools.sh",
    hook_behavior = "skip",
    packages = packages,
}
