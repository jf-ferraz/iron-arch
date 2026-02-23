# System Services Management

dcli now supports declarative management of systemd services through the `services` section in your configuration file.

## Overview

The services feature allows you to:
- Enable and start services automatically on boot
- Disable and stop services
- Track service state across system generations
- Rollback service configurations with config backups
- Manage services declaratively alongside packages

## Configuration

Add a `services` section to your `config.yaml` or host file:

```yaml
services:
  enabled:
    - bluetooth
    - sshd
    - docker

  disabled:
    - cups
    - NetworkManager-wait-online
```

### Services to Enable

Services listed under `services.enabled` will be:
1. **Enabled** for automatic start on boot (if not already enabled)
2. **Started** immediately (if not already running)

### Services to Disable

Services listed under `services.disabled` will be:
1. **Stopped** immediately (if currently running)
2. **Disabled** from starting on boot (if currently enabled)

## Usage

### Bootstrap from Current System

The easiest way to get started is to capture your currently enabled services:

```bash
# Preview services that would be added
dcli merge --services --dry-run

# Add currently enabled services to your config
dcli merge --services
```

This command will:
1. Scan all enabled services on your system
2. Filter out system-critical services (like systemd, dbus, getty)
3. Add remaining services to your host configuration file
4. Sort them alphabetically for easy management

**Example output:**
```
→ Loading configuration...
→ Scanning enabled services...
→ Found 87 enabled services on system
→ 35 manageable services (after filtering system-critical)
→ Found 0 services declared in config (0 enabled, 0 disabled)
→ Found 35 unmanaged services

=== Unmanaged Services ===

These services are currently enabled but not in your dcli config:

  • bluetooth
  • cups
  • docker
  • NetworkManager
  • sshd
  ...

✓ Added 35 services to host configuration
```

### Basic Workflow

1. **Edit your configuration** to add services:
   ```yaml
   services:
     enabled:
       - sshd
     disabled:
       - cups
   ```

2. **Sync your system** to apply changes:
   ```bash
   dcli sync
   ```

3. **Verify changes** (optional):
   ```bash
   systemctl status sshd
   systemctl status cups
   ```

### During Sync

When you run `dcli sync`, services are synchronized:

```
Syncing services...
  ✓ Enabled sshd
  ✓ Started sshd
  ✓ Stopped cups
  ✓ Disabled cups

Services enabled: 1
Services disabled: 1
```

## Service State Tracking

dcli tracks service state in `~/.config/arch-config/state/services-state.yaml`:

```yaml
last_updated: "2023-12-16T10:30:00Z"
enabled_services:
  - sshd
  - bluetooth
disabled_services:
  - cups
```

This file is:
- **Automatically created** on first sync with services
- **Updated** after each successful sync
- **Included in config backups** automatically
- **Restored** with config restores

## Backup and Rollback

### Automatic Backup

Services state is automatically backed up with configuration backups:

```bash
# Manual backup
dcli save-config

# Backup is created automatically before sync (if config_backups.enabled: true)
dcli sync
```

### Restore Services

To restore services to a previous state:

```bash
# Restore a config backup (includes services state)
dcli restore-config

# Apply the restored configuration
dcli sync
```

## Service Name Format

Service names can be specified with or without the `.service` suffix:

```yaml
services:
  enabled:
    - sshd              # Simple name
    - sshd.service      # Full name (equivalent)
    - bluetooth
    - docker
    - getty@tty1        # Service templates
```

## Validation and Error Handling

dcli validates service names and handles errors gracefully:

### Service Name Validation

- **Allowed characters**: alphanumeric, dash (`-`), underscore (`_`), dot (`.`), at sign (`@`)
- **Prevents**: command injection and invalid service names
- **Example errors**:
  ```
  ✗ Invalid service name 'service; rm -rf /': service names can only contain...
  ```

### Service Existence Check

Before operating on a service, dcli checks if it exists:

```
Warning: Service custom-service does not exist on system, skipping
```

### Conflict Detection

If a service appears in both `enabled` and `disabled`:

```
Warning: Service sshd is in both enabled and disabled lists, skipping
```

### Permission Errors

Service operations require root privileges:

```bash
# dcli sync will prompt for sudo when needed
sudo dcli sync
```

### Partial Failures

If some service operations fail, dcli continues with others and reports errors:

```
Syncing services...
  ✓ Enabled sshd
  ✓ Started sshd
  ✗ Failed to enable invalid-service: Service does not exist
  ✓ Disabled cups

Warning: 1 service operations failed
```

## Filtered System Services

When using `dcli merge --services`, the following system-critical services are automatically filtered out and will NOT be added to your configuration:

**Core System Services:**
- systemd-* services (journald, logind, udevd, resolved, timesyncd, networkd, etc.)
- dbus, dbus-broker
- kmod-static-nodes

**Security Services:**
- polkit
- rtkit-daemon

**Display Managers:**
- gdm, sddm, lightdm
- getty@tty1-6
- display-manager

**System Targets:**
- multi-user.target, graphical.target, basic.target, sysinit.target

These services are essential for system operation and should not be managed declaratively. The filter list ensures you don't accidentally capture critical services.

## Important Considerations

### Critical Services

Be careful when disabling critical services on remote systems:

**⚠️ WARNING**: Disabling these services on remote systems may cause loss of connectivity:
- `NetworkManager`
- `sshd`
- `systemd-networkd`

### Service Dependencies

dcli does not automatically handle service dependencies. If you disable a service that other services depend on, those services may fail.

### Masked Services

dcli will skip masked services and report them as errors. To unmask a service:

```bash
sudo systemctl unmask <service-name>
```

### User vs System Services

Currently, dcli only manages **system services**. User services (`systemctl --user`) are not supported yet.

## Examples

### Basic Example

Enable SSH and Bluetooth, disable printing:

```yaml
services:
  enabled:
    - sshd
    - bluetooth
  disabled:
    - cups
```

### Server Configuration

Typical server services:

```yaml
services:
  enabled:
    - sshd
    - docker
    - fail2ban
  disabled:
    - bluetooth
    - cups
    - NetworkManager-wait-online
```

### Desktop Configuration

Typical desktop services:

```yaml
services:
  enabled:
    - bluetooth
    - NetworkManager
    - cups
  disabled:
    - sshd  # Disable SSH on desktop
```

### Container Host

Docker/Podman setup:

```yaml
services:
  enabled:
    - docker
    - containerd
  disabled:
    - bluetooth
    - cups
```

## Integration with Modules

Services can also be managed per-module. In a module's `packages.yaml` or directory module, you can specify services:

**Note**: Per-module services support is planned for a future release. Currently, services must be declared in the main config or host files.

## Troubleshooting

### Services not changing

1. Check if you have root privileges:
   ```bash
   sudo dcli sync
   ```

2. Verify service exists:
   ```bash
   systemctl list-unit-files | grep <service-name>
   ```

3. Check service status manually:
   ```bash
   systemctl status <service-name>
   ```

### Service fails to start

If a service is enabled but fails to start, check the logs:

```bash
journalctl -u <service-name> -n 50
```

### State file corruption

If the services state file is corrupted, you can delete it:

```bash
rm ~/.config/arch-config/state/services-state.yaml
```

It will be recreated on the next `dcli sync`.

## Technical Details

### State File Location

`~/.config/arch-config/state/services-state.yaml`

### Sync Order

During `dcli sync`, operations happen in this order:

1. Pre-flight validation
2. Package sync (install/remove)
3. Dotfiles sync
4. **Services sync** ← New
5. Post-install hooks
6. State file update

### Service Operations

For each enabled service:
1. Validate service name
2. Check if service exists
3. Enable if not already enabled (`systemctl enable`)
4. Start if not already active (`systemctl start`)

For each disabled service:
1. Validate service name
2. Check if service exists
3. Stop if currently active (`systemctl stop`)
4. Disable if currently enabled (`systemctl disable`)

## Future Enhancements

Planned features for future releases:

- [ ] Per-module service declarations
- [ ] User service support (`systemctl --user`)
- [ ] Service dependency checking
- [ ] Service timer management
- [ ] Service status in `dcli status` output
- [ ] Dry-run preview for service changes
- [ ] Service change notifications
- [ ] Service rollback without full config restore

## Contributing

Found a bug or have a feature request? Please open an issue on the dcli GitLab repository.

## License

This feature is part of dcli and follows the same license.
