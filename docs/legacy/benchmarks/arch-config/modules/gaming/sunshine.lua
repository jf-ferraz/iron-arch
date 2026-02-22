local packages = {
    "sunshine",
    "tailscale",
}

return {
    description = "Sunshine remote gaming with Tailscale VPN for secure streaming",
    post_install_hook = "scripts/setup-sunshine-tailscale.sh",
    metadata = {
        tailscale_ip = "100.106.165.111",
    },
    packages = packages,
}
