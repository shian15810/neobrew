#!/bin/sh

set -euvx

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"

PACKAGE="${PACKAGE:-""}"
VERSION="${VERSION:-"latest"}"
INSTALLATION_FLAGS="${INSTALLATION_FLAGS:-""}"

HOMEBREW_NO_ANALYTICS="${HOMEBREW_NO_ANALYTICS:?}"
HOMEBREW_NO_ENV_HINTS="${HOMEBREW_NO_ENV_HINTS:?}"

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

if [ -z "$PACKAGE" ]; then
    exit 0
fi

set -f
OLDIFS="$IFS"

IFS=','
# shellcheck disable=SC2086
set -- $PACKAGE

set +f
IFS="$OLDIFS"

if [ "$#" -eq 1 ] && [ "$VERSION" != "latest" ]; then
    case "$1" in
        *@*) ;;
        *) set -- "$1@$VERSION" ;;
    esac
fi

set -- -- "$@"

if [ -n "$INSTALLATION_FLAGS" ]; then
    while IFS='' read -r INSTALLATION_FLAG; do
        set -- "$INSTALLATION_FLAG" "$@"
    done <<- EOF
		$(printf '%s' "$INSTALLATION_FLAGS" | xargs -n1)
	EOF
fi

sudo --user="$_REMOTE_USER" \
    HOMEBREW_NO_ANALYTICS="$HOMEBREW_NO_ANALYTICS" \
    HOMEBREW_NO_ENV_HINTS="$HOMEBREW_NO_ENV_HINTS" \
    --login exec \
    /bin/sh -- "$SCRIPT_DIR/install-feature.sh" "$@"
