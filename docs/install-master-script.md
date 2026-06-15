# Iron Arch Install Flow

This flow is designed for the Arch live ISO before installing the target system.
It separates planning from execution so destructive operations remain reviewable.

## Bootstrap Iron in the live ISO

Reviewed flow:

```bash
curl -fsSLO https://raw.githubusercontent.com/laraj/iron/main/scripts/archiso-bootstrap.sh
less archiso-bootstrap.sh
bash archiso-bootstrap.sh --host desktop --target /mnt
```

One-shot flow:

```bash
curl -fsSL https://raw.githubusercontent.com/laraj/iron/main/scripts/archiso-bootstrap.sh | bash
```

The bootstrap script will:

1. verify it is running on Arch as root;
2. verify network access;
3. install bootstrap dependencies;
4. clone and build Iron;
5. build the integrated Iron install wizard;
6. generate `/tmp/iron-bootstrap/iron-install-plan.sh` as a fallback.

## Run the integrated wizard

Preferred flow:

```bash
/tmp/iron-bootstrap/iron/target/release/iron \
  --root /tmp/iron-bootstrap/iron \
  install wizard \
  --host desktop \
  --target /mnt
```

The wizard provides:

- phase navigation with status markers;
- destructive step visibility before execution;
- dry-run review with `d`;
- typed `INSTALL` confirmation before real execution with `r`;
- execution logs in the TUI.

## Review and run the fallback generated plan

```bash
less /tmp/iron-bootstrap/iron-install-plan.sh
IRON_BIN=/tmp/iron-bootstrap/iron/target/release/iron \
IRON_CONFIG_SRC=/tmp/iron-bootstrap/iron \
bash /tmp/iron-bootstrap/iron-install-plan.sh --list-phases

IRON_BIN=/tmp/iron-bootstrap/iron/target/release/iron \
IRON_CONFIG_SRC=/tmp/iron-bootstrap/iron \
bash /tmp/iron-bootstrap/iron-install-plan.sh --dry-run

IRON_BIN=/tmp/iron-bootstrap/iron/target/release/iron \
IRON_CONFIG_SRC=/tmp/iron-bootstrap/iron \
bash /tmp/iron-bootstrap/iron-install-plan.sh --menu
```

The fallback generated plan:

- logs to `/tmp/iron-install.log`;
- uses `set -Eeuo pipefail`;
- traps failed commands with line numbers;
- asks before manual and destructive steps;
- opens an interactive menu by default;
- supports `--menu`, `--run`, `--dry-run`, `--list-phases`, `--only PHASE`, and `--from PHASE`;
- tracks progress in `/tmp/iron-install-state`;
- installs the built Iron binary into the target before running `arch-chroot ... iron`;
- copies the Iron configuration into `/opt/iron-config` inside the target.

## Generate from an existing checkout

```bash
cargo run -p iron-cli -- \
  --root /path/to/iron \
  install wizard \
  --host desktop \
  --target /mnt
```

Fallback script generation:

```bash
cargo run -p iron-cli -- \
  --root /path/to/iron \
  install plan \
  --host desktop \
  --target /mnt \
  --emit-script > /tmp/iron-install-plan.sh
```

## Non-interactive mode

Only use this after reviewing the generated script:

```bash
ASSUME_YES=true \
IRON_BIN=/path/to/iron \
IRON_CONFIG_SRC=/path/to/iron-config \
bash /tmp/iron-install-plan.sh
```

## Phase controls

```bash
bash /tmp/iron-bootstrap/iron-install-plan.sh --list-phases
bash /tmp/iron-bootstrap/iron-install-plan.sh --menu
bash /tmp/iron-bootstrap/iron-install-plan.sh --only preflight
bash /tmp/iron-bootstrap/iron-install-plan.sh --from system-config
bash /tmp/iron-bootstrap/iron-install-plan.sh --run
```
