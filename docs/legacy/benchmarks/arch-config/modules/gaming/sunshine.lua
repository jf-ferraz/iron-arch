---@diagnostic disable: undefined-global -- dcli globals are provided by dcli runtime
local packages = {
    "sunshine",
    "tailscale",
}

-- Get current hostname for metadata
local hostname = dcli.system.hostname()

return {
    description = "Sunshine remote gaming with Tailscale VPN for secure streaming",

    -- Declaratively enable tailscale service (replaces systemctl enable)
    services = {
        enabled = { "tailscaled" },
        disabled = {},
    },

    -- Post-install hook for tailscale up and IP capture
    post_install_hook = "scripts/setup-sunshine-tailscale.sh",

    metadata = {
        hostname = hostname,
        -- Static IP reference (hook will output current IP)
        tailscale_ip = "100.106.165.111",
    },

    packages = packages,
}
