#!/bin/sh

set -euvx

export LC_ALL='C'

if [ "$(uname)" = "Darwin" ]; then
    echo "$PATH"
    echo "$(ls -al ~/.rustup/bin || true)"
    echo "$(ls -al ~/.cargo/bin || true)"
    echo "$(which -a rustup || true)"
    echo "$(which -a rustc || true)"
    echo "$(which -a cargo || true)"
    echo "$(~/.cargo/bin/rustup --version || true)"
    echo "$(~/.cargo/bin/rustc --version || true)"
    echo "$(~/.cargo/bin/cargo --version || true)"
    echo "$(rustup --version || true)"
    echo "$(rustc --version || true)"
    echo "$(cargo --version || true)"
    echo "$(ls -al /opt/homebrew/bin || true)"

    MDT="$(rustc --print="deployment-target")"
    MDT="$(printf '%s' "$MDT" | cut -d'=' -f2)"

    export MACOSX_DEPLOYMENT_TARGET="$MDT"

    export CARGO_PKG_METADATA_SCCACHE_MACOSX_DEPLOYMENT_TARGET="$MDT"
fi

unset -v LC_ALL
