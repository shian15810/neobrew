#!/bin/sh

set -euvx

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"

cd -- "$SCRIPT_DIR/.."

sudo chown -- "$(id -un):$(id -gn)" "$SCCACHE_DIR"

brew analytics off
brew bundle

mise trust
mise install
mise upgrade

rustup self update
rustup update

rustup --version
cargo --version
rustc --version

rustup +nightly --version
cargo +nightly --version
rustc +nightly --version
