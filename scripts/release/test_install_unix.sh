#!/bin/sh
set -eu

archive_path=""
checksum_path=""
expected_version=""
compiler=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --archive)
            archive_path=$2
            shift 2
            ;;
        --checksum-file)
            checksum_path=$2
            shift 2
            ;;
        --version)
            expected_version=$2
            shift 2
            ;;
        --cc)
            compiler=$2
            shift 2
            ;;
        *)
            printf 'error: unknown option %s\n' "$1" >&2
            exit 1
            ;;
    esac
done

[ -n "$archive_path" ] || { printf 'error: --archive is required\n' >&2; exit 1; }
[ -n "$checksum_path" ] || { printf 'error: --checksum-file is required\n' >&2; exit 1; }
[ -n "$expected_version" ] || { printf 'error: --version is required\n' >&2; exit 1; }

repo_root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
archive_path=$(CDPATH= cd -- "$(dirname "$archive_path")" && pwd)/$(basename "$archive_path")
checksum_path=$(CDPATH= cd -- "$(dirname "$checksum_path")" && pwd)/$(basename "$checksum_path")
temp_root=$(mktemp -d "${TMPDIR:-/tmp}/skadi-dist-smoke.XXXXXX")
trap 'rm -rf "$temp_root"' EXIT HUP INT TERM

install_dir="$temp_root/bin"
manifest_path="$temp_root/state/install-manifest"
workspace="$temp_root/workspace"
test_home="$temp_root/home"
original_path=$PATH
mkdir -p "$workspace" "$test_home"
HOME=$test_home
XDG_STATE_HOME="$test_home/.local/state"
SHELL=/bin/sh
export HOME XDG_STATE_HOME SHELL

bad_checksums="$temp_root/BAD_SHA256SUMS"
printf '%064d  %s\n' 0 "$(basename "$archive_path")" >"$bad_checksums"
if "$repo_root/install/install.sh" \
    --version "$expected_version" \
    --archive "$archive_path" \
    --checksum-file "$bad_checksums" \
    --install-dir "$install_dir" \
    --manifest-path "$manifest_path" \
    --no-modify-path; then
    printf 'error: installer accepted an invalid checksum\n' >&2
    exit 1
fi

"$repo_root/install/install.sh" \
    --version "$expected_version" \
    --archive "$archive_path" \
    --checksum-file "$checksum_path" \
    --install-dir "$install_dir" \
    --manifest-path "$manifest_path" \
    --no-modify-path

# Reinstalling the same release exercises the atomic upgrade path.
"$repo_root/install/install.sh" \
    --version "$expected_version" \
    --archive "$archive_path" \
    --checksum-file "$checksum_path" \
    --install-dir "$install_dir" \
    --manifest-path "$manifest_path" \
    --no-modify-path

binary="$install_dir/skadi-cli"
PATH="$install_dir:$original_path"
export PATH
[ "$(skadi-cli --version)" = "skadi-cli $expected_version" ]

cd "$workspace"
if [ -n "$compiler" ]; then
    printf 'output("installed quick-run ok")\n' > quick_smoke.skd
    skadi-cli quick-run quick_smoke.skd --cc "$compiler"
fi
skadi-cli new distribution_smoke
cd distribution_smoke
skadi-cli check
skadi-cli format
skadi-cli format --check
skadi-cli doctor
skadi-cli target list
if [ -n "$compiler" ]; then
    skadi-cli build --target host --cc "$compiler"
    skadi-cli run --target host --cc "$compiler"
fi

"$repo_root/install/uninstall.sh" --manifest-path "$manifest_path"
[ ! -e "$binary" ] || { printf 'error: binary remains after uninstall\n' >&2; exit 1; }
[ ! -e "$manifest_path" ] || { printf 'error: manifest remains after uninstall\n' >&2; exit 1; }

# Exercise installer-owned shell profile state across an upgrade.
path_home="$temp_root/path-home"
mkdir -p "$path_home"
HOME=$path_home
XDG_STATE_HOME="$path_home/.local/state"
SHELL=/bin/sh
export HOME XDG_STATE_HOME SHELL

"$repo_root/install/install.sh" \
    --version "$expected_version" \
    --archive "$archive_path" \
    --checksum-file "$checksum_path"
"$repo_root/install/install.sh" \
    --version "$expected_version" \
    --archive "$archive_path" \
    --checksum-file "$checksum_path"

profile="$path_home/.profile"
[ "$(grep -c '^# >>> skadi-cli >>>$' "$profile")" -eq 1 ] ||
    { printf 'error: installer did not preserve one PATH block\n' >&2; exit 1; }

"$repo_root/install/uninstall.sh"
[ ! -e "$path_home/.local/bin/skadi-cli" ] ||
    { printf 'error: default binary remains after uninstall\n' >&2; exit 1; }
if grep -q '^# >>> skadi-cli >>>$' "$profile"; then
    printf 'error: installer-owned PATH block remains after uninstall\n' >&2
    exit 1
fi
