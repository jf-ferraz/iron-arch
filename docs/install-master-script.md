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
5. generate `/tmp/iron-bootstrap/iron-install-plan.sh`.

## Review and run the generated plan

```bash
less /tmp/iron-bootstrap/iron-install-plan.sh
IRON_BIN=/tmp/iron-bootstrap/iron/target/release/iron bash /tmp/iron-bootstrap/iron-install-plan.sh
```

The generated plan:

- logs to `/tmp/iron-install.log`;
- uses `set -Eeuo pipefail`;
- traps failed commands with line numbers;
- asks before manual and destructive steps;
- installs the built Iron binary into the target before running `arch-chroot ... iron`.

## Generate from an existing checkout

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
ASSUME_YES=true IRON_BIN=/path/to/iron bash /tmp/iron-install-plan.sh
```
