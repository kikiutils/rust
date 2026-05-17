#!/usr/bin/env bash

set -euo pipefail

if ! command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
    if command -v musl-gcc >/dev/null 2>&1; then
        export CC_x86_64_unknown_linux_musl="${CC_x86_64_unknown_linux_musl:-musl-gcc}"
    else
        echo "missing x86_64-linux-musl-gcc/musl-gcc; install musl-tools with your package manager" >&2
        exit 1
    fi
fi

sep=$'\x1f'
flags=(
    # Optional size/link optimization for ELF linkers that support identical code
    # folding. Keep disabled by default because --icf=all can merge functions with
    # identical machine code and therefore change function pointer identity.
    # -C link-arg=-Wl,--icf=all
)

if ((${#flags[@]} == 0)); then
    exec cargo b -r --target x86_64-unknown-linux-musl "$@"
fi

encoded=""
for flag in "${flags[@]}"; do
    if [[ -n "${encoded}" ]]; then
        encoded+="$sep"
    fi

    encoded+="$flag"
done

exec env CARGO_ENCODED_RUSTFLAGS="${encoded}" \
    cargo b -r --target x86_64-unknown-linux-musl "$@"
