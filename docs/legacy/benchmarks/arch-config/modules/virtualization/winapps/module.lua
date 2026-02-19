-- WinApps - Run Windows applications seamlessly on Linux
-- Requires Docker/Podman and KVM virtualization support

local packages = {}
local services = { enabled = {}, disabled = {} }

-- System requirements check
local memory_mb = dcli.system.memory_total_mb()
local cpu_cores = dcli.system.cpu_cores()
local cpu_vendor = dcli.hardware.cpu_vendor()

-- Only warn if system doesn't meet minimum requirements
if memory_mb < 8000 then
    dcli.log.warn("WinApps recommends 8GB+ total RAM for optimal performance")
end

if cpu_vendor ~= "intel" and cpu_vendor ~= "amd" then
    dcli.log.warn("Could not detect CPU vendor - verify KVM support manually")
end

-- Core packages (always needed)
table.insert(packages, "curl")
table.insert(packages, "git")
table.insert(packages, "iproute2")
table.insert(packages, "libnotify")
table.insert(packages, "openbsd-netcat")

-- Optional: dialog (for WinApps installer TUI)
if dcli.package.is_available("dialog") then
    table.insert(packages, "dialog")
end

-- FreeRDP v3 (critical requirement)
table.insert(packages, "freerdp")

-- Docker backend
if not dcli.package.is_installed("docker") then
    table.insert(packages, "docker")
    table.insert(packages, "docker-compose")
    table.insert(services.enabled, "docker.service")
else
    -- Still enable service if not already
    if not dcli.service.is_enabled("docker.service") then
        table.insert(services.enabled, "docker.service")
    end
end

-- System packages for KVM/networking
if not dcli.package.is_installed("iptables-nft") and not dcli.package.is_installed("iptables") then
    table.insert(packages, "iptables-nft")
end

-- Calculate recommended VM resources for hook
local recommended_ram_gb = math.floor(memory_mb / 1024 * 0.4) -- 40% of system RAM
if recommended_ram_gb < 4 then
    recommended_ram_gb = 4 -- Minimum for Windows 10
elseif recommended_ram_gb > 16 then
    recommended_ram_gb = 16 -- Reasonable maximum
end

local recommended_cores = math.floor(cpu_cores / 2)
if recommended_cores < 2 then
    recommended_cores = 2 -- Minimum for Windows
elseif recommended_cores > 8 then
    recommended_cores = 8 -- Reasonable maximum
end

return {
    description = "WinApps - Seamlessly run Windows applications on Linux via Docker/RDP",
    packages = packages,
    services = services,
    post_install_hook = "scripts/setup-winapps.sh",
    hook_behavior = "ask",
    
    -- Store recommendations as metadata (accessible in hook via env vars)
    metadata = {
        recommended_ram_gb = recommended_ram_gb,
        recommended_cores = recommended_cores,
        total_ram_gb = math.floor(memory_mb / 1024),
        total_cores = cpu_cores,
    },
}
