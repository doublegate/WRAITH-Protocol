#!/usr/bin/env bash
# WRAITH Protocol - Isolated Benchmark Runner
# Ensures consistent, reliable benchmark results by:
# - Setting CPU governor to performance
# - Disabling turbo boost
# - Pinning to specific cores
# - Running benchmarks sequentially
#
# Usage: ./scripts/bench-isolated.sh [crate-name] [-- extra-args]
# Examples:
#   ./scripts/bench-isolated.sh              # Run all benchmarks
#   ./scripts/bench-isolated.sh wraith-core  # Run wraith-core only
#   ./scripts/bench-isolated.sh wraith-files -- --save-baseline v2.3.2

set -euo pipefail

# When running under sudo, inherit the invoking user's Rust toolchain
if [[ $EUID -eq 0 ]] && [[ -n "${SUDO_USER:-}" ]]; then
    SUDO_HOME="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
    export HOME="$SUDO_HOME"
    export PATH="$SUDO_HOME/.cargo/bin:$PATH"
    export RUSTUP_HOME="${RUSTUP_HOME:-$SUDO_HOME/.rustup}"
    export CARGO_HOME="${CARGO_HOME:-$SUDO_HOME/.cargo}"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$PROJECT_ROOT/benchmarks"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
VERSION="$(grep -A1 '\[workspace\.package\]' "$PROJECT_ROOT/Cargo.toml" | grep 'version' | sed 's/.*"\(.*\)".*/\1/')"
RESULT_DIR="$RESULTS_DIR/v${VERSION}/${TIMESTAMP}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Parse arguments
TARGET_CRATE="${1:-all}"
shift || true
EXTRA_ARGS="${*:-}"

# Benchmark crates in execution order
BENCH_CRATES=(
    "wraith-core"
    "wraith-crypto"
    "wraith-files"
    "wraith-obfuscation"
)

log() { echo -e "${CYAN}[bench]${NC} $*"; }
warn() { echo -e "${YELLOW}[warn]${NC} $*"; }
ok() { echo -e "${GREEN}[ok]${NC} $*"; }
err() { echo -e "${RED}[error]${NC} $*"; }

# System info capture
capture_system_info() {
    local info_file="$1"
    {
        echo "# System Information"
        echo "Date: $(date -Iseconds)"
        echo "Kernel: $(uname -r)"
        echo "CPU: $(lscpu | grep 'Model name' | sed 's/.*:\s*//')"
        echo "Cores: $(nproc)"
        echo "Architecture: $(uname -m)"
        echo "CPU MHz: $(lscpu | grep 'CPU MHz' | head -1 | sed 's/.*:\s*//')"
        echo "CPU Max MHz: $(lscpu | grep 'CPU max MHz' | sed 's/.*:\s*//' 2>/dev/null || echo 'N/A')"
        echo "L1d Cache: $(lscpu | grep 'L1d' | sed 's/.*:\s*//')"
        echo "L1i Cache: $(lscpu | grep 'L1i' | sed 's/.*:\s*//')"
        echo "L2 Cache: $(lscpu | grep 'L2' | sed 's/.*:\s*//')"
        echo "L3 Cache: $(lscpu | grep 'L3' | sed 's/.*:\s*//')"
        echo "Memory: $(free -h | awk '/Mem:/{print $2}')"
        echo "Governor: $(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo 'N/A')"
        echo "Turbo: $(cat /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || cat /sys/devices/system/cpu/cpufreq/boost 2>/dev/null || echo 'N/A')"
        echo "Rust: $(rustc --version)"
        echo "Cargo: $(cargo --version)"
        echo "WRAITH Version: v${VERSION}"
        echo ""
    } > "$info_file"
}

# Attempt to set performance mode (requires root)
setup_performance() {
    if [[ $EUID -eq 0 ]]; then
        log "Setting CPU governor to performance..."
        for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
            echo performance > "$cpu" 2>/dev/null || true
        done

        # Disable turbo boost (Intel)
        if [[ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]]; then
            echo 1 > /sys/devices/system/cpu/intel_pstate/no_turbo
            log "Disabled Intel turbo boost"
        fi
        # Disable boost (AMD)
        if [[ -f /sys/devices/system/cpu/cpufreq/boost ]]; then
            echo 0 > /sys/devices/system/cpu/cpufreq/boost
            log "Disabled AMD boost"
        fi
        PERF_SETUP=true
    else
        warn "Not running as root - skipping CPU governor/turbo settings"
        warn "For best results: sudo $0 $TARGET_CRATE $EXTRA_ARGS"
        PERF_SETUP=false
    fi
}

# Restore system settings
restore_system() {
    if [[ "${PERF_SETUP:-false}" == "true" ]]; then
        log "Restoring CPU settings..."
        for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
            echo schedutil > "$cpu" 2>/dev/null || true
        done
        if [[ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]]; then
            echo 0 > /sys/devices/system/cpu/intel_pstate/no_turbo
        fi
        if [[ -f /sys/devices/system/cpu/cpufreq/boost ]]; then
            echo 1 > /sys/devices/system/cpu/cpufreq/boost
        fi
        ok "System settings restored"
    fi

    # Fix ownership of target/criterion and results dirs when run as sudo
    if [[ $EUID -eq 0 ]] && [[ -n "${SUDO_USER:-}" ]]; then
        log "Fixing file ownership for ${SUDO_USER}..."
        chown -R "$SUDO_USER:$SUDO_USER" "$PROJECT_ROOT/target/criterion" 2>/dev/null || true
        chown -R "$SUDO_USER:$SUDO_USER" "$RESULTS_DIR" 2>/dev/null || true
    fi
}

trap restore_system EXIT

# Run benchmarks for a single crate
run_crate_bench() {
    local crate="$1"
    local output_file="$RESULT_DIR/${crate}.txt"

    log "Benchmarking ${crate}..."

    # Use taskset to pin to cores 2-3 (avoiding core 0 interrupt handling)
    local taskset_cmd=""
    if command -v taskset &>/dev/null; then
        local ncores
        ncores=$(nproc)
        if [[ $ncores -ge 4 ]]; then
            taskset_cmd="taskset -c 2,3"
        elif [[ $ncores -ge 2 ]]; then
            taskset_cmd="taskset -c 1"
        fi
    fi

    local nice_cmd=""
    if [[ $EUID -eq 0 ]]; then
        nice_cmd="nice -n -20"
    fi

    # Run with isolation
    local start_time
    start_time=$(date +%s)

    if [[ -n "$taskset_cmd" ]] || [[ -n "$nice_cmd" ]]; then
        $nice_cmd $taskset_cmd cargo bench -p "$crate" $EXTRA_ARGS 2>&1 | tee "$output_file"
    else
        cargo bench -p "$crate" $EXTRA_ARGS 2>&1 | tee "$output_file"
    fi

    local end_time
    end_time=$(date +%s)
    local elapsed=$((end_time - start_time))

    ok "${crate} completed in ${elapsed}s"
    echo "# Duration: ${elapsed}s" >> "$output_file"
}

# Main
main() {
    log "WRAITH Protocol Isolated Benchmark Runner"
    log "Version: v${VERSION}"
    log "Target: ${TARGET_CRATE}"
    log ""

    # Create results directory
    mkdir -p "$RESULT_DIR"

    # System setup
    setup_performance

    # Capture system info
    capture_system_info "$RESULT_DIR/system-info.txt"
    ok "System info captured"

    # Build release first (shared compilation)
    log "Building release targets..."
    cargo bench --workspace --no-run 2>&1 | tail -5
    ok "Build complete"

    # Run benchmarks sequentially
    if [[ "$TARGET_CRATE" == "all" ]]; then
        for crate in "${BENCH_CRATES[@]}"; do
            run_crate_bench "$crate"
            echo ""
        done
    else
        run_crate_bench "$TARGET_CRATE"
    fi

    # Summary
    log ""
    log "Results saved to: $RESULT_DIR/"
    ls -la "$RESULT_DIR/"
    ok "All benchmarks complete"
}

main
