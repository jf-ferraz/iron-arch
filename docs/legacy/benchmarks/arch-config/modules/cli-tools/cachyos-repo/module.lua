return {
    description = "Main Module file for cachy repos module",
    conflicts = {},
    post_install_hook = "scripts/install-cachyos-repos.sh",
    packages = {},
    metadata = {
        author = "theblackdon",
        version = "1.0.0",
        category = "system",
    },
}
