#!/bin/bash

if ! cargo +nightly fmt --version >/dev/null 2>&1; then
    rustup toolchain install nightly --component rustfmt --profile minimal
fi

cargo +nightly fmt --all -- "$@"
