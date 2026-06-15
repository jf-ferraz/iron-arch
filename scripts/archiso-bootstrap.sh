#!/usr/bin/env bash
#
# Bootstrap Iron from the Arch ISO before installing the target system.
#
# Typical usage from the Arch live terminal:
#   curl -fsSL https://raw.githubusercontent.com/laraj/iron/main/scripts/archiso-bootstrap.sh | bash
#
# Safer reviewed usage:
#   curl -fsSLO https://raw.githubusercontent.com/laraj/iron/main/scripts/archiso-bootstrap.sh
#   less archiso-bootstrap.sh
#   bash archiso-bootstrap.sh --host desktop --target /mnt

set -Eeuo pipefail

REPO_URL="${IRON_REPO_URL:-https://github.com/laraj/iron.git}"
BRANCH="${IRON_BRANCH:-main}"
WORKDIR="${IRON_WORKDIR:-/tmp/iron-bootstrap}"
HOST_ID="${IRON_HOST:-desktop}"
TARGET_MOUNT="${IRON_TARGET:-/mnt}"
RUN_PLAN=false
ASSUME_YES=false
LOG_FILE="${IRON_BOOTSTRAP_LOG:-/tmp/iron-bootstrap.log}"

trap 'printf "[iron-bootstrap] ERROR line %s: %s\n" "$LINENO" "$BASH_COMMAND" | tee -a "$LOG_FILE" >&2' ERR

usage() {
    cat <<'EOF'
Bootstrap Iron from an Arch ISO.

Options:
  --repo URL          Git repository URL
  --branch NAME      Git branch/ref to checkout
  --workdir DIR      Working directory, default /tmp/iron-bootstrap
  --host ID          Iron host ID, default desktop
  --target DIR       Target mountpoint, default /mnt
  --run              Execute the generated install plan after review prompt
  --yes              Non-interactive confirmations for this bootstrap script
  -h, --help         Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo)
            REPO_URL="$2"
            shift 2
            ;;
        --branch)
            BRANCH="$2"
            shift 2
            ;;
        --workdir)
            WORKDIR="$2"
            shift 2
            ;;
        --host)
            HOST_ID="$2"
            shift 2
            ;;
        --target)
            TARGET_MOUNT="$2"
            shift 2
            ;;
        --run)
            RUN_PLAN=true
            shift
            ;;
        --yes)
            ASSUME_YES=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

log() {
    printf '[iron-bootstrap] %s\n' "$*" | tee -a "$LOG_FILE"
}

die() {
    printf '[iron-bootstrap] ERROR: %s\n' "$*" | tee -a "$LOG_FILE" >&2
    exit 1
}

confirm() {
    local prompt="$1"
    if [[ "$ASSUME_YES" == "true" ]]; then
        log "Auto-confirmed: $prompt"
        return 0
    fi
    read -r -p "$prompt [y/N] " response
    [[ "$response" =~ ^[Yy]$ ]]
}

require_archiso() {
    [[ -r /etc/arch-release ]] || die "This bootstrap expects an Arch environment."
    command -v pacman >/dev/null 2>&1 || die "pacman not found."
    [[ $EUID -eq 0 ]] || die "Run this from the Arch ISO root shell."
}

validate_inputs() {
    [[ "$TARGET_MOUNT" == /* ]] || die "--target must be an absolute path."
    [[ "$TARGET_MOUNT" != "/" ]] || die "--target cannot be /."
    [[ -n "$HOST_ID" ]] || die "--host cannot be empty."
    [[ -n "$REPO_URL" ]] || die "--repo cannot be empty."
    [[ -n "$BRANCH" ]] || die "--branch cannot be empty."
}

check_network() {
    log "Checking network connectivity"
    ping -c 1 archlinux.org >/dev/null 2>&1 || die "No network connectivity to archlinux.org."
}

install_dependencies() {
    local packages=(git rust base-devel ca-certificates)
    local missing=()

    for package in "${packages[@]}"; do
        pacman -Q "$package" >/dev/null 2>&1 || missing+=("$package")
    done

    if ((${#missing[@]} == 0)); then
        log "Bootstrap dependencies already installed"
    else
        log "Installing bootstrap dependencies: ${missing[*]}"
        pacman -Sy --needed --noconfirm "${missing[@]}"
    fi

    command -v cargo >/dev/null 2>&1 || die "cargo is still unavailable after installing bootstrap dependencies."
    command -v git >/dev/null 2>&1 || die "git is still unavailable after installing bootstrap dependencies."
}

fetch_iron() {
    mkdir -p "$WORKDIR"

    if [[ -d "$WORKDIR/iron/.git" ]]; then
        log "Updating existing Iron checkout at $WORKDIR/iron"
        git -C "$WORKDIR/iron" fetch --depth 1 origin "$BRANCH"
        git -C "$WORKDIR/iron" checkout FETCH_HEAD
    else
        log "Cloning Iron from $REPO_URL#$BRANCH"
        rm -rf "$WORKDIR/iron"
        git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$WORKDIR/iron"
    fi
}

build_iron() {
    log "Building Iron CLI"
    cargo build --release -p iron-cli --manifest-path "$WORKDIR/iron/Cargo.toml"
}

emit_plan() {
    local iron_bin="$WORKDIR/iron/target/release/iron"
    local plan_path="$WORKDIR/iron-install-plan.sh"

    [[ -x "$iron_bin" ]] || die "Iron binary was not built at $iron_bin."

    log "Generating install plan for host '$HOST_ID'"
    "$iron_bin" \
        --root "$WORKDIR/iron" \
        install plan \
        --host "$HOST_ID" \
        --target "$TARGET_MOUNT" \
        --emit-script > "$plan_path"

    chmod +x "$plan_path"
    log "Generated: $plan_path"
    log "Iron binary: $iron_bin"
    log "Review it with: less $plan_path"
    log "Run it with:    IRON_BIN=$iron_bin bash $plan_path"

    if [[ "$RUN_PLAN" == "true" ]]; then
        confirm "Execute generated install plan now?" || die "Install plan execution cancelled."
        IRON_BIN="$iron_bin" bash "$plan_path"
    fi
}

main() {
    require_archiso
    validate_inputs
    check_network
    install_dependencies
    fetch_iron
    build_iron
    emit_plan
}

main "$@"
