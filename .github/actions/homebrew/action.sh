#!/bin/sh

set -eu

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

cd -- "${SCRIPT_DIR}/../../.."

brew analytics off
