#!/bin/sh
set -eu

repository="eoshipnyagov/Skadi-Language"
version="latest"
install_dir=""
manifest_path=""
archive_path=""
checksum_path=""
modify_path=1
system_install=0

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: install.sh [options]

Options:
  --version <version>         Release version, for example 1.2.0-rc.1
  --install-dir <path>        Override installation directory
  --manifest-path <path>      Override installation manifest path
  --repository <owner/repo>   GitHub repository
  --archive <path>            Install from a local release archive
  --checksum-file <path>      Checksum manifest for a local archive
  --no-modify-path            Do not update a shell profile
  --system                    Install into /usr/local/bin
  --help                      Show this help
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --version)
            [ "$#" -ge 2 ] || die "--version requires a value"
            version=$2
            shift 2
            ;;
        --install-dir)
            [ "$#" -ge 2 ] || die "--install-dir requires a value"
            install_dir=$2
            shift 2
            ;;
        --manifest-path)
            [ "$#" -ge 2 ] || die "--manifest-path requires a value"
            manifest_path=$2
            shift 2
            ;;
        --repository)
            [ "$#" -ge 2 ] || die "--repository requires a value"
            repository=$2
            shift 2
            ;;
        --archive)
            [ "$#" -ge 2 ] || die "--archive requires a value"
            archive_path=$2
            shift 2
            ;;
        --checksum-file)
            [ "$#" -ge 2 ] || die "--checksum-file requires a value"
            checksum_path=$2
            shift 2
            ;;
        --no-modify-path)
            modify_path=0
            shift
            ;;
        --system)
            system_install=1
            shift
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

if [ "$system_install" -eq 1 ]; then
    [ -z "$install_dir" ] || die "--system and --install-dir cannot be combined"
    install_dir="/usr/local/bin"
elif [ -z "$install_dir" ]; then
    install_dir="$HOME/.local/bin"
fi

if [ -z "$manifest_path" ]; then
    state_home=${XDG_STATE_HOME:-"$HOME/.local/state"}
    manifest_path="$state_home/skadi/install-manifest"
fi

case "$install_dir$manifest_path" in
    *'
'*) die "installation paths cannot contain newlines" ;;
esac

previous_path_added=0
previous_profile_file=""
if [ -f "$manifest_path" ]; then
    previous_install_dir=$(sed -n 's/^install_dir=//p' "$manifest_path" | head -n 1)
    previous_path_added=$(sed -n 's/^path_added=//p' "$manifest_path" | head -n 1)
    previous_profile_file=$(sed -n 's/^profile_file=//p' "$manifest_path" | head -n 1)
    if [ -n "$previous_install_dir" ] && [ "$previous_install_dir" != "$install_dir" ]; then
        die "Skadi is already managed at '$previous_install_dir'; uninstall it before changing --install-dir"
    fi
fi

os=$(uname -s)
arch=$(uname -m)
case "$os:$arch" in
    Linux:x86_64|Linux:amd64)
        target="x86_64-unknown-linux-musl"
        ;;
    Linux:aarch64|Linux:arm64)
        target="aarch64-unknown-linux-musl"
        ;;
    Darwin:x86_64)
        target="x86_64-apple-darwin"
        ;;
    Darwin:arm64|Darwin:aarch64)
        target="aarch64-apple-darwin"
        ;;
    *)
        die "unsupported platform '$os/$arch'"
        ;;
esac

fetch() {
    url=$1
    destination=$2
    if command -v curl >/dev/null 2>&1; then
        curl -fL --retry 3 --proto '=https' --tlsv1.2 \
            -A "skadi-cli-installer" -o "$destination" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$destination" "$url"
    else
        die "curl or wget is required to download Skadi"
    fi
}

latest_tag() {
    temp_json=$1
    fetch "https://api.github.com/repos/$repository/releases/latest" "$temp_json"
    sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$temp_json" |
        head -n 1
}

normalize_version() {
    raw=$1
    case "$raw" in
        v*) clean=${raw#v} ;;
        *) clean=$raw ;;
    esac
    printf '%s\n' "$clean" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$' ||
        die "invalid Skadi release version '$raw'"
    printf '%s\n' "$clean"
}

sha256_file() {
    file=$1
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        die "sha256sum or shasum is required"
    fi
}

verify_checksum() {
    file=$1
    sums=$2
    name=$(basename "$file")
    expected=$(awk -v name="$name" '$2 == name || $2 == "*" name { print $1; exit }' "$sums")
    [ -n "$expected" ] || die "checksum manifest does not contain '$name'"
    actual=$(sha256_file "$file")
    [ "$actual" = "$expected" ] ||
        die "checksum mismatch for '$name': expected $expected, got $actual"
}

temp_root=$(mktemp -d "${TMPDIR:-/tmp}/skadi-install.XXXXXX")
trap 'rm -rf "$temp_root"' EXIT HUP INT TERM

if [ "$version" = "latest" ]; then
    tag=$(latest_tag "$temp_root/latest.json")
    [ -n "$tag" ] || die "unable to resolve the latest GitHub release"
else
    case "$version" in
        v*) tag=$version ;;
        *) tag="v$version" ;;
    esac
fi
clean_version=$(normalize_version "$tag")
archive_name="skadi-cli-v$clean_version-$target.tar.gz"

if [ -n "$archive_path" ]; then
    [ -n "$checksum_path" ] || die "--checksum-file is required with --archive"
    archive_path=$(cd "$(dirname "$archive_path")" && pwd)/$(basename "$archive_path")
    checksum_path=$(cd "$(dirname "$checksum_path")" && pwd)/$(basename "$checksum_path")
    [ "$(basename "$archive_path")" = "$archive_name" ] ||
        die "local archive must be named '$archive_name'"
else
    archive_path="$temp_root/$archive_name"
    checksum_path="$temp_root/SHA256SUMS"
    base_url="https://github.com/$repository/releases/download/$tag"
    printf 'Downloading %s\n' "$archive_name"
    fetch "$base_url/$archive_name" "$archive_path"
    fetch "$base_url/SHA256SUMS" "$checksum_path"
fi

verify_checksum "$archive_path" "$checksum_path"

extract_dir="$temp_root/extract"
mkdir -p "$extract_dir"
tar -xzf "$archive_path" -C "$extract_dir"
source_binary=$(find "$extract_dir" -type f -name skadi-cli | head -n 1)
[ -n "$source_binary" ] || die "archive '$archive_name' does not contain skadi-cli"

sudo_cmd=""
if [ "$system_install" -eq 1 ] && [ "$(id -u)" -ne 0 ]; then
    command -v sudo >/dev/null 2>&1 || die "sudo is required for --system"
    sudo_cmd="sudo"
fi

if [ -n "$sudo_cmd" ]; then
    $sudo_cmd mkdir -p "$install_dir"
else
    mkdir -p "$install_dir"
fi

binary_path="$install_dir/skadi-cli"
staged_path="$install_dir/.skadi-cli.installing.$$"
if [ -n "$sudo_cmd" ]; then
    $sudo_cmd cp "$source_binary" "$staged_path"
    $sudo_cmd chmod 755 "$staged_path"
else
    cp "$source_binary" "$staged_path"
    chmod 755 "$staged_path"
fi

staged_version=$("$staged_path" --version 2>&1) ||
    die "downloaded binary failed to execute"
[ "$staged_version" = "skadi-cli $clean_version" ] ||
    die "downloaded binary reported unexpected version '$staged_version'"

if [ -n "$sudo_cmd" ]; then
    $sudo_cmd mv -f "$staged_path" "$binary_path"
else
    mv -f "$staged_path" "$binary_path"
fi

profile_file=$previous_profile_file
path_added=$previous_path_added
if [ "$modify_path" -eq 1 ] && [ "$system_install" -eq 0 ] && [ "$path_added" != "1" ]; then
    case ":${PATH:-}:" in
        *":$install_dir:"*) ;;
        *)
            shell_name=$(basename "${SHELL:-sh}")
            case "$shell_name" in
                zsh) profile_file="$HOME/.zshrc" ;;
                fish) profile_file="$HOME/.config/fish/config.fish" ;;
                bash) profile_file="$HOME/.bashrc" ;;
                *) profile_file="$HOME/.profile" ;;
            esac
            mkdir -p "$(dirname "$profile_file")"
            touch "$profile_file"
            if ! grep -Fq "# >>> skadi-cli >>>" "$profile_file"; then
                {
                    printf '\n# >>> skadi-cli >>>\n'
                    if [ "$shell_name" = "fish" ]; then
                        printf 'fish_add_path "%s"\n' "$install_dir"
                    else
                        escaped_dir=$(printf '%s' "$install_dir" | sed 's/\\/\\\\/g; s/"/\\"/g; s/`/\\`/g; s/\$/\\$/g')
                        printf 'export PATH="%s:$PATH"\n' "$escaped_dir"
                    fi
                    printf '# <<< skadi-cli <<<\n'
                } >>"$profile_file"
                path_added=1
            fi
            ;;
    esac
fi

manifest_dir=$(dirname "$manifest_path")
mkdir -p "$manifest_dir"
manifest_temp="$manifest_path.tmp.$$"
{
    printf 'schema_version=1\n'
    printf 'version=%s\n' "$clean_version"
    printf 'install_dir=%s\n' "$install_dir"
    printf 'binary_path=%s\n' "$binary_path"
    printf 'profile_file=%s\n' "$profile_file"
    printf 'path_added=%s\n' "$path_added"
    printf 'system_install=%s\n' "$system_install"
} >"$manifest_temp"
mv -f "$manifest_temp" "$manifest_path"

"$binary_path" --version
if ! "$binary_path" doctor; then
    printf 'warning: skadi-cli doctor reported an incomplete host toolchain\n' >&2
fi

printf '\nSkadi CLI %s installed to %s\n' "$clean_version" "$binary_path"
if [ "$modify_path" -eq 0 ]; then
    printf "PATH was not modified. Add '%s' to PATH.\n" "$install_dir"
elif [ "$path_added" -eq 1 ]; then
    printf 'Shell profile updated: %s\nRestart the shell before running skadi-cli.\n' "$profile_file"
fi
