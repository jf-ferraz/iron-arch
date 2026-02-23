-- Auto-detect hostname for automatic host configuration
-- Falls back to "default" if hostname doesn't match a known host config
local hostname = dcli.system.hostname()
dcli.log.info("Auto-detected host: " .. hostname)

-- Check if host-specific config exists by trying to read it
local host_config_path = "hosts/" .. hostname .. ".lua"
local host_exists = dcli.file.is_file(dcli.env.config_dir() .. "/arch-config/" .. host_config_path)

local effective_host = hostname
if not host_exists then
    dcli.log.warn("No host config found for '" .. hostname .. "', using default configuration")
    dcli.log.warn("Create hosts/" .. hostname .. ".lua to customize for this system")
    effective_host = "default"
end

return {
    host = effective_host,
}
