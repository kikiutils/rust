#!/bin/bash

set -e
if ! git diff-index --quiet HEAD --; then
    echo 'Error: There are uncommitted changes in your working directory.'
    echo 'Please commit or discard the changes before proceeding.'
    exit 1
fi

cargo test
pnpx @kikiutils/changelogen --bump
new_version=$(node -p "require('./package.json').version")
cargo set-version "$new_version"
git checkout -- ./CHANGELOG.md ./package.json
git add ./Cargo.toml
pnpx @kikiutils/changelogen --push --release
cargo publish
