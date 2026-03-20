#!/usr/bin/env bash
# Yaya installer — Post-quantum sovereign mesh VPN
# Usage: curl -fsSL yaya.sh | bash
#
# This script downloads and installs the Yaya binary.
# License: MIT

set -euo pipefail

YAYA_VERSION="${YAYA_VERSION:-0.1.0}"
YAYA_BASE_URL="${YAYA_BASE_URL:-https://releases.yaya.sh}"
YAYA_INSTALL_DIR="${YAYA_INSTALL_DIR:-/usr/local/bin}"

# minisign public key for verifying binary signatures
YAYA_PUBKEY="RWTxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

info() { echo -e "${GREEN}==>${NC} ${BOLD}$1${NC}"; }
warn() { echo -e "${YELLOW}warning:${NC} $1"; }
error() { echo -e "${RED}error:${NC} $1" >&2; exit 1; }

# --- Platform detection ---

detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        *)       error "Unsupported OS: $(uname -s). Yaya supports Linux and macOS." ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "amd64" ;;
        aarch64|arm64)  echo "arm64" ;;
        armv7l|armhf)   echo "armv7" ;;
        *)              error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# --- Downloader ---

has_cmd() { command -v "$1" >/dev/null 2>&1; }

download() {
    local url="$1" dest="$2"
    if has_cmd curl; then
        curl -fsSL --proto '=https' --tlsv1.2 -o "$dest" "$url"
    elif has_cmd wget; then
        wget -q -O "$dest" "$url"
    else
        error "Neither curl nor wget found. Install one and try again."
    fi
}

# --- Signature verification ---

verify_signature() {
    local file="$1" sigfile="$2"

    if has_cmd minisign; then
        info "Verifying signature with minisign..."
        if minisign -Vm "$file" -x "$sigfile" -P "$YAYA_PUBKEY" 2>/dev/null; then
            info "Signature verified."
            return 0
        else
            error "Signature verification failed! Binary may be tampered with."
        fi
    fi

    # Fallback: verify SHA256 checksum
    local checksums_url="${YAYA_BASE_URL}/v${YAYA_VERSION}/SHA256SUMS"
    local checksums_file
    checksums_file="$(mktemp)"

    if download "$checksums_url" "$checksums_file" 2>/dev/null; then
        local expected actual basename
        basename="$(basename "$file")"
        expected="$(grep "$basename" "$checksums_file" | awk '{print $1}')"
        if [ -n "$expected" ]; then
            if has_cmd sha256sum; then
                actual="$(sha256sum "$file" | awk '{print $1}')"
            elif has_cmd shasum; then
                actual="$(shasum -a 256 "$file" | awk '{print $1}')"
            else
                warn "Cannot verify: no sha256sum or shasum found."
                rm -f "$checksums_file"
                return 0
            fi

            if [ "$expected" = "$actual" ]; then
                info "SHA256 checksum verified."
                rm -f "$checksums_file"
                return 0
            else
                rm -f "$checksums_file"
                error "Checksum mismatch! Expected: $expected Got: $actual"
            fi
        fi
    fi

    rm -f "$checksums_file" 2>/dev/null
    warn "Could not verify signature. Install minisign for better security."
    warn "  https://jedisct1.github.io/minisign/"
}

# --- Privilege handling ---

ensure_sudo() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif has_cmd sudo; then
        sudo "$@"
    elif has_cmd doas; then
        doas "$@"
    else
        error "Root privileges required. Run with sudo or as root."
    fi
}

# --- Dependencies ---

check_dependencies() {
    if ! has_cmd wg; then
        warn "wireguard-tools not found. WireGuard is required for Yaya."
        case "$(detect_os)" in
            linux)
                warn "Install with: sudo apt install wireguard-tools"
                warn "          or: sudo dnf install wireguard-tools"
                warn "          or: sudo pacman -S wireguard-tools"
                ;;
            darwin)
                warn "Install with: brew install wireguard-tools"
                ;;
        esac
    fi
}

# --- Main ---

main() {
    local os arch archive sigfile tmpdir

    echo ""
    echo -e "${BOLD}  Yaya${NC} — Post-quantum sovereign mesh VPN"
    echo -e "  ${GREEN}\"You can't surveil what you can't access.\"${NC}"
    echo ""

    os="$(detect_os)"
    arch="$(detect_arch)"
    info "Detected platform: ${os}/${arch}"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    archive="yaya-${os}-${arch}.tar.gz"
    sigfile="${archive}.minisig"

    # Download binary
    info "Downloading Yaya v${YAYA_VERSION}..."
    download "${YAYA_BASE_URL}/v${YAYA_VERSION}/${archive}" "${tmpdir}/${archive}"

    # Download signature
    download "${YAYA_BASE_URL}/v${YAYA_VERSION}/${sigfile}" "${tmpdir}/${sigfile}" 2>/dev/null || true

    # Verify
    if [ -f "${tmpdir}/${sigfile}" ]; then
        verify_signature "${tmpdir}/${archive}" "${tmpdir}/${sigfile}"
    else
        warn "No signature file available. Skipping verification."
    fi

    # Extract
    info "Extracting..."
    tar -xzf "${tmpdir}/${archive}" -C "${tmpdir}"

    # Install
    info "Installing to ${YAYA_INSTALL_DIR}/yaya..."
    ensure_sudo install -m 755 "${tmpdir}/yaya" "${YAYA_INSTALL_DIR}/yaya"

    # Check dependencies
    check_dependencies

    # Initialize
    info "Generating identity..."
    "${YAYA_INSTALL_DIR}/yaya" init 2>/dev/null || true

    echo ""
    info "Yaya v${YAYA_VERSION} installed successfully!"
    echo ""
    echo "  Next steps:"
    echo "    yaya peer add --invite     # Generate invite for a peer"
    echo "    yaya peer add <key@host>   # Add a peer directly"
    echo "    yaya status                # Check mesh status"
    echo ""
    echo "  Documentation: https://docs.yaya.sh"
    echo ""
}

main "$@"
