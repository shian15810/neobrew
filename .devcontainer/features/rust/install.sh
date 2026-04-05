#!/bin/sh

set -euvx

export LC_ALL='C'

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

VERSION="${VERSION:-"latest"}"
PROFILE="${PROFILE:-"minimal"}"
TARGETS="${TARGETS:-""}"
COMPONENTS="${COMPONENTS:-"rust-analyzer,rust-src,rustfmt,clippy"}"

RUSTUP_PERMIT_COPY_RENAME="${RUSTUP_PERMIT_COPY_RENAME:?}"

RUSTUP_HOME="${RUSTUP_HOME:?}"
CARGO_HOME="${CARGO_HOME:?}"
RUST_VERSION="${RUST_VERSION:?}"

# The 'install.sh' entrypoint script is always executed as the root user.
#
# These following environment variables are passed in by the dev container CLI.
# These may be useful in instances where the context of the final
# remoteUser or containerUser is useful.
# For more details, see https://containers.dev/implementors/features#user-env-var

_REMOTE_USER="${_REMOTE_USER:?}"
_REMOTE_USER_HOME="${_REMOTE_USER_HOME:?}"

_CONTAINER_USER="${_CONTAINER_USER:?}"
_CONTAINER_USER_HOME="${_CONTAINER_USER_HOME:?}"

echo "The effective dev container remoteUser is '$_REMOTE_USER'"
echo "The effective dev container remoteUser's home directory is '$_REMOTE_USER_HOME'"

echo "The effective dev container containerUser is '$_CONTAINER_USER'"
echo "The effective dev container containerUser's home directory is '$_CONTAINER_USER_HOME'"

if [ "$VERSION" = "none" ]; then
    set -- "$RUST_VERSION"
else
    set -f
    OLDIFS="$IFS"

    IFS=','
    # shellcheck disable=SC2086
    set -- $VERSION

    set +f
    IFS="$OLDIFS"

    TOOLCHAINS=""

    for TOOLCHAIN in "$@"; do
        if [ "$TOOLCHAIN" = "none" ]; then
            exit 64
        elif [ "$TOOLCHAIN" = "latest" ] || [ "$TOOLCHAIN" = "lts" ]; then
            TOOLCHAIN="stable"
        elif [ "$TOOLCHAIN" = "current" ]; then
            CURRENT_VERSION="$(
                git ls-remote --tags --refs --sort="-version:refname" \
                    "https://github.com/rust-lang/rust.git" \
                    "[0-9]*.[0-9]*.[0-9]*"
            )"
            CURRENT_VERSION="$(
                printf '%s' "$CURRENT_VERSION" \
                    | awk -F'/' '/[0-9]+\.[0-9]+\.[0-9]+$/ { print $NF; exit }'
            )"

            if [ -z "$CURRENT_VERSION" ]; then
                exit 69
            fi

            TOOLCHAIN="$CURRENT_VERSION"
        fi

        case " ${TOOLCHAINS} " in
            *" ${TOOLCHAIN} "*) ;;
            *) TOOLCHAINS="${TOOLCHAINS} ${TOOLCHAIN}" ;;
        esac
    done

    set -f

    # shellcheck disable=SC2086
    set -- $TOOLCHAINS

    set +f
fi

set -- -- "$@"

if [ -n "$TARGETS" ]; then
    set -- --target="$TARGETS" "$@"
fi

if [ -n "$COMPONENTS" ]; then
    set -- --component="$COMPONENTS" "$@"
fi

set -- --profile="$PROFILE" "$@"

sudo --user="$_REMOTE_USER" \
    RUSTUP_PERMIT_COPY_RENAME="$RUSTUP_PERMIT_COPY_RENAME" \
    RUSTUP_HOME="$RUSTUP_HOME" \
    CARGO_HOME="$CARGO_HOME" \
    RUST_VERSION="$RUST_VERSION" \
    --login exec \
    /bin/sh -euvx -- "${SCRIPT_DIR}/install-feature.sh" "$@"
