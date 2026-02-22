local packages = {
    "cosmic-session",
}

return {
    description = "Main Module to source other yaml files for comsmic-xero",
    post_install_hook = "scripts/install-cosmic-xero.sh",
    hook_behavior = "once",
    packages = packages,
}
