#!/bin/sh

set -eu

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

cd -- "${SCRIPT_DIR}/../../.."

DEFAULT_TOOLCHAIN="$(
    taplo get --file-path="rust-toolchain.toml" --output-format="json" \
        | jq --raw-output '.toolchain.channel'
)"

rustup default "$DEFAULT_TOOLCHAIN"
