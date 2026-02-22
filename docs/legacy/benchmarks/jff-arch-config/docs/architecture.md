# Architecture

## Core goals
- Declarative system management.
- Safe-by-default operations.
- Reproducible config deployment.
- Observable execution via structured logs.

## Repository model
- `app/manifests/`: policy and registries (`hosts.toml`, `services.toml`, `update-policy.toml`).
- `modules/`: domain modules (`configs`, `services`, `updates`, `backups`, etc.).
- `hosts/`: host overlays (single host: `desktop`).
- `scripts/`: shell execution adapters and operational tooling.
- `rust/`: CLI and TUI application layer.
- `app/state/`: runtime state (`logs`, `locks`, `run`) not intended as source-of-truth.

## Control Flow Diagram
```text
manifests + modules
        |
        v
   app-cli validate
        |
        v
     app-cli plan
        |
        v
     app-cli apply
        |
        v
 scripts/systemctl/stow
        |
        v
 app/state/logs/operations.jsonl
```

## Runtime Surfaces
```text
            +----------------+
            |    app-tui     |
            | (interactive)  |
            +--------+-------+
                     |
                     v
+---------+    +-----+------+    +----------------+
| scripts |<-->|   app-cli  |<-->|  core-domain   |
| shell   |    | orchestration|   | parse/validate |
+----+----+    +------+-----+    +----------------+
     |                |
     v                v
 systemctl/stow    manifests/modules
```

## Execution model
1. **Validation layer**
- `app-cli validate` and `app-cli doctor` ensure manifests and module contracts are coherent.

2. **Planning layer**
- `app-cli plan` builds an ordered operation list from `module.toml` contracts.

3. **Apply layer**
- `app-cli apply <module>` executes module operations (supports dry-run behaviors).

## Logging and observability
All CLI and major scripts write JSONL audit entries to:
- `app/state/logs/operations.jsonl`

Typical fields:
- `timestamp_unix_ms`
- `module`
- `operation`
- `args`
- `duration_ms`
- `result`
- `exit_code`

## Service model
`app/manifests/services.toml` defines services by `type`:
- `systemd-user`
- `systemd-system`
- `session-autostart`
- `health-check`

Systemd user units are maintained in:
- `modules/services/systemd-user/units`

Unit synchronization:
- `app-cli service sync-user-units`

## Host model
Current strategy is single-host desktop:
- `app/manifests/hosts.toml` default host: `desktop`
- active host state: `app/state/run/active-host`

## Safety defaults
- Most workflows support dry-run first.
- Update script requires explicit `--apply` for real changes.
- Apply-like TUI actions use confirmation prompts.
