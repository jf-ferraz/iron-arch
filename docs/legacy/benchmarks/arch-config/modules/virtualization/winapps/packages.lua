-- Core WinApps dependencies
-- These are managed by module.lua, this file is for reference

return {
    description = "WinApps core dependencies (reference only - managed by module.lua)",

    packages = {
        -- RDP client (v3+ required)
        "freerdp",

        -- Core utilities
        "curl",
        "dialog",
        "git",
        "iproute2",
        "libnotify",
        "openbsd-netcat",
        "yad",

        -- Container runtime
        "docker",
        "docker-compose",

        -- Networking
        "iptables-nft",
    },
}
