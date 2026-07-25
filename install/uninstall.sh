#!/bin/sh
set -eu

manifest_path=""

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: uninstall.sh [--manifest-path <path>]
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --manifest-path)
            [ "$#" -ge 2 ] || die "--manifest-path requires a value"
            manifest_path=$2
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            die "unknown option '$1'"
            ;;
    esac
done

[ -n "${HOME:-}" ] || die "HOME is not set"
if [ -z "$manifest_path" ]; then
    state_home=${XDG_STATE_HOME:-"$HOME/.local/state"}
    manifest_path="$state_home/skadi/install-manifest"
fi

if [ ! -f "$manifest_path" ]; then
    printf 'Skadi installation manifest not found: %s\n' "$manifest_path"
    exit 0
fi

manifest_value() {
    key=$1
    sed -n "s/^$key=//p" "$manifest_path" | head -n 1
}

install_dir=$(manifest_value install_dir)
binary_path=$(manifest_value binary_path)
profile_file=$(manifest_value profile_file)
path_added=$(manifest_value path_added)
system_install=$(manifest_value system_install)

sudo_cmd=""
if [ "$system_install" = "1" ] && [ "$(id -u)" -ne 0 ]; then
    command -v sudo >/dev/null 2>&1 || die "sudo is required to remove system installation"
    sudo_cmd="sudo"
fi

if [ -n "$binary_path" ] && [ -e "$binary_path" ]; then
    if [ -n "$sudo_cmd" ]; then
        $sudo_cmd rm -f "$binary_path"
    else
        rm -f "$binary_path"
    fi
fi

if [ "$path_added" = "1" ] && [ -n "$profile_file" ] && [ -f "$profile_file" ]; then
    profile_temp="$profile_file.skadi-remove.$$"
    awk '
        $0 == "# >>> skadi-cli >>>" { skipping = 1; next }
        $0 == "# <<< skadi-cli <<<" { skipping = 0; next }
        !skipping { print }
    ' "$profile_file" >"$profile_temp"
    cat "$profile_temp" >"$profile_file"
    rm -f "$profile_temp"
fi

rm -f "$manifest_path"
rmdir "$(dirname "$manifest_path")" 2>/dev/null || true
if [ "$system_install" != "1" ] && [ -n "$install_dir" ]; then
    rmdir "$install_dir" 2>/dev/null || true
fi

printf 'Skadi CLI uninstalled. User projects and configuration were preserved.\n'
