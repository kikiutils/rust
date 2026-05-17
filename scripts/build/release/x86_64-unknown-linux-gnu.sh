#!/usr/bin/env bash

set -euo pipefail

sep=$'\x1f'
flags=(
    -C link-arg=-fuse-ld=mold

    # Optional size/link optimization for ELF linkers that support identical code
    # folding. Keep disabled by default because --icf=all can merge functions with
    # identical machine code and therefore change function pointer identity.
    # -C link-arg=-Wl,--icf=all
)

encoded=""
for flag in "${flags[@]}"; do
    if [[ -n "${encoded}" ]]; then
        encoded+="$sep"
    fi

    encoded+="$flag"
done

exec env CARGO_ENCODED_RUSTFLAGS="${encoded}" \
    cargo b -r --target x86_64-unknown-linux-gnu "$@"
