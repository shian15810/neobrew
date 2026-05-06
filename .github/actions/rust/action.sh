#!/bin/sh

set -eu

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

cd -- "${SCRIPT_DIR}/../../.."

DEFAULT_TOOLCHAIN="$(
    taplo get --output-format="json" --file-path="rust-toolchain.toml" \
        | jq --raw-output '.toolchain.channel'
)"

rustup default "$DEFAULT_TOOLCHAIN"
