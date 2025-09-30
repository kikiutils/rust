#!/bin/bash

set -euo pipefail

SCRIPTS_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
cd "${SCRIPTS_DIR}"

if ! git diff-index --quiet HEAD --; then
    echo 'Error: There are uncommitted changes in your working directory'
    echo 'Please commit or discard the changes before proceeding'
    exit 1
fi

cargo format
cargo lint
cargo t --all-features
cargo b -r --all-features

pnpx @kikiutils/changelogen@latest --bump --hideAuthorEmail
new_version=$(node -p "require('./package.json').version")
cargo set-version "$new_version"
git checkout -- ./CHANGELOG.md ./package.json
git add ./Cargo.toml
pnpx @kikiutils/changelogen@latest --hideAuthorEmail --push --release
cargo publish
