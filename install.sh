#!/bin/bash
# shellcheck shell=bash

# Install script for fvm (Fluvio Version Manager) from GitHub releases
# Downloads fvm directly from the fluvio repository GitHub releases

set -e
set -o pipefail
set -u

readonly FVM_INSTALL_DIR=${FVM_INSTALL_DIR-"$HOME/.fvm"}
readonly FLUVIO_INSTALL_DIR=${FLUVIO_INSTALL_DIR-"$HOME/.fluvio"}
readonly FLUVIO_ARCH=${FLUVIO_ARCH-}
FLUVIO_VERSION=${FLUVIO_VERSION-}

readonly VERSION=${VERSION-}
readonly FVM_VERSION=${FVM_VERSION-}

# GitHub repository for fluvio releases
readonly GITHUB_REPO=${GITHUB_REPO-"infinyon/fluvio"}

_fluvio_version="${FLUVIO_VERSION:-${VERSION:-}}"

# install fvm from GitHub releases
main() {
    need_cmd curl
    need_cmd unzip

    local _fvmver
    _fvmver=$(get_fvm_version)

    # Detect architecture and ensure it's supported
    get_architecture || return 1
    local _arch="$RETVAL"
    assert_nz "$_arch"

    if [ -z "${FLUVIO_VERSION}" ] && [ "${VERSION}" != "" ]; then
        echo "Warning: VERSION is deprecated in favor of FLUVIO_VERSION"
        export FLUVIO_VERSION=${VERSION}
    fi

    # Normalize the target for GitHub release asset naming
    _target=$(normalize_target "${_arch}")

    echo "Downloading fluvio version manager (fvm) from GitHub releases"
    echo "   Version: ${_fvmver}"
    echo "   Target:  ${_target}"

    _dir="$(mktemp -d 2>/dev/null || ensure mktemp -d -t fluvio-install)"
    _zipfile="${_dir}/fvm.zip"
    _url="https://github.com/${GITHUB_REPO}/releases/download/${_fvmver}/fvm-${_target}.zip"

    downloader "${_url}" "${_zipfile}"
    _status=$?
    if [ $_status -ne 0 ]; then
        err "Failed to download fvm!"
        err "    Error downloading from ${_url}"
        abort_prompt_issue
    fi

    echo "Installing fvm"
    unzip -q -o "${_zipfile}" -d "${_dir}"

    # Find the extracted fvm binary
    local _fvm_binary="${_dir}/fvm"
    if [ ! -f "${_fvm_binary}" ]; then
        err "fvm binary not found after extraction"
        abort_prompt_issue
    fi

    chmod +x "${_fvm_binary}"
    "${_fvm_binary}" self install

    # Check if .fluvio exists, recommend fvm install
    if [ -d "$FLUVIO_INSTALL_DIR" ]; then
        echo "If a version of fluvio is already installed, you can run 'fvm install' or 'fvm switch' to change versions"
    fi

    if [ -n "${_fluvio_version}" ]; then
        echo "Installing fluvio ${_fluvio_version}"
        "$FVM_INSTALL_DIR"/bin/fvm install "${FLUVIO_VERSION}"
    else
        echo "Installing latest fluvio"
        "$FVM_INSTALL_DIR"/bin/fvm install
    fi

    # Cleanup
    rm -rf "${_dir}"

    echo "Install complete!"
    remind_path
}

# Get fvm version to download
# Uses FVM_VERSION env var if set, otherwise fetches the latest release tag
get_fvm_version() {
    if [ -n "${FVM_VERSION}" ]; then
        echo "${FVM_VERSION}"
        return 0
    fi

    # Fetch the latest release tag from GitHub API
    set +e
    local _url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
    local _response
    _response=$(curl --proto '=https' --tlsv1.2 --silent --show-error --fail --location "${_url}" 2>/dev/null)
    local _status=$?
    set -e

    if [ $_status -ne 0 ]; then
        err "Failed to fetch latest release information from GitHub"
        err "    URL: ${_url}"
        err "You can set FVM_VERSION environment variable to specify a version manually"
        abort_prompt_issue
    fi

    # Extract tag_name from JSON response (simple parsing without jq dependency)
    local _tag
    _tag=$(echo "${_response}" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')

    if [ -z "${_tag}" ]; then
        err "Failed to parse release tag from GitHub API response"
        abort_prompt_issue
    fi

    echo "${_tag}"
}

# Prompts the user to add ~/.fvm/bin and ~/.fluvio/bin to their PATH variable
remind_path() {
    say "You'll need to add '~/.fvm/bin' and '~/.fluvio/bin' to your PATH variable"
    say "    You can run the following to set your PATH on shell startup:"

    # shellcheck disable=SC2016,SC2155
    local bash_hint="$(tput bold 2>/dev/null || true)"'echo '\''source "${HOME}/.fvm/env"'\'' >> ~/.bashrc'"$(tput sgr0 2>/dev/null || true)"
    # shellcheck disable=SC2016,SC2155
    local zsh_hint="$(tput bold 2>/dev/null || true)"'echo '\''export PATH="${HOME}/.fvm/bin:${HOME}/.fluvio/bin:${PATH}"'\'' >> ~/.zshrc'"$(tput sgr0 2>/dev/null || true)"
    # shellcheck disable=SC2016,SC2155
    local fish_hint="$(tput bold 2>/dev/null || true)"'fish_add_path "$HOME/.fvm/bin" "$HOME/.fluvio/bin"'"$(tput sgr0 2>/dev/null || true)"

    case "$(basename "${SHELL}")" in
        bash)
            say "      ${bash_hint}"
            ;;
        zsh)
            say "      ${zsh_hint}"
            ;;
        fish)
            say "      ${fish_hint}"
            ;;
        *)
            say "      For bash: ${bash_hint}"
            say "      For zsh : ${zsh_hint}"
            say "      For fish: ${fish_hint}"
            ;;
    esac
}

# Uses curl to download the contents of a URL to a file.
#
# @param $1: The URL of the file to download
# @param $2: The filename of where to download
downloader() {
    local _status
    local _url="$1"; shift
    local _file="$1"; shift

    # Allow trap of error
    set +e
    # Use curl for downloads
    _err=$(curl --proto '=https' --tlsv1.2 --silent --compressed --show-error --fail --location "${_url}" --output "${_file}" 2>&1)
    _status=$?
    set -e

    # If there is anything on stderr, print it
    if [ -n "$_err" ]; then
        echo "$_err" >&2
    fi
    return $_status
}

# Ensure that this target is supported and matches the
# naming convention of known platform releases in GitHub
#
# @param $1: The target triple of this architecture
# @return: The normalized target name for GitHub release assets
normalize_target() {
    local _target="$1"; shift

    # Match against all supported targets and normalize to GitHub asset naming
    case $_target in
        x86_64-unknown-linux-gnu)
            echo "x86_64-unknown-linux-musl"
            return 0
            ;;
        aarch64-unknown-linux-gnu)
            echo "aarch64-unknown-linux-musl"
            return 0
            ;;
        armv7-unknown-linux-gnueabihf)
            echo "armv7-unknown-linux-gnueabihf"
            return 0
            ;;
        arm-unknown-linux-gnueabihf)
            echo "arm-unknown-linux-gnueabihf"
            return 0
            ;;
    esac

    echo "${_target}"
    return 0
}

get_bitness() {
    need_cmd head
    # Architecture detection without dependencies beyond coreutils.
    # ELF files start out "\x7fELF", and the following byte is
    #   0x01 for 32-bit and
    #   0x02 for 64-bit.
    # The printf builtin on some shells like dash only supports octal
    # escape sequences, so we use those.
    local _current_exe_head
    _current_exe_head=$(head -c 5 /proc/self/exe)
    if [ "$_current_exe_head" = "$(printf '\177ELF\001')" ]; then
        echo 32
    elif [ "$_current_exe_head" = "$(printf '\177ELF\002')" ]; then
        echo 64
    else
        err "unknown platform bitness"
    fi
}

get_endianness() {
    local cputype=$1
    local suffix_eb=$2
    local suffix_el=$3

    # Detect endianness without od/hexdump, like get_bitness() does.
    need_cmd head
    need_cmd tail

    local _current_exe_endianness
    _current_exe_endianness="$(head -c 6 /proc/self/exe | tail -c 1)"
    if [ "$_current_exe_endianness" = "$(printf '\001')" ]; then
        echo "${cputype}${suffix_el}"
    elif [ "$_current_exe_endianness" = "$(printf '\002')" ]; then
        echo "${cputype}${suffix_eb}"
    else
        err "unknown platform endianness"
    fi
}

# Cross-platform architecture detection, borrowed from rustup-init.sh
get_architecture() {
    local _ostype _cputype _bitness _arch _clibtype
    _ostype="$(uname -s)"
    _cputype="$(uname -m)"
    _clibtype="gnu"

    if [ -n "${FLUVIO_ARCH}" ]; then
        RETVAL="${FLUVIO_ARCH}"
        return 0
    fi

    if [ "$_ostype" = Linux ]; then
        if [ "$(uname -o 2>/dev/null)" = Android ]; then
            _ostype=Android
        fi
        if ldd --version 2>&1 | grep -q 'musl'; then
            _clibtype="musl"
        fi
    fi

    if [ "$_ostype" = Darwin ] && [ "$_cputype" = i386 ]; then
        # Darwin `uname -m` lies
        if sysctl hw.optional.x86_64 2>/dev/null | grep -q ': 1'; then
            _cputype=x86_64
        fi
    fi

    if [ "$_ostype" = SunOS ]; then
        # Both Solaris and illumos presently announce as "SunOS" in "uname -s"
        # so use "uname -o" to disambiguate.
        if [ "$(/usr/bin/uname -o 2>/dev/null)" = illumos ]; then
            _ostype=illumos
        fi

        # illumos systems have multi-arch userlands
        if [ "$_cputype" = i86pc ]; then
            _cputype="$(isainfo -n)"
        fi
    fi

    case "$_ostype" in

        Android)
            _ostype=linux-android
            ;;

        Linux)
            _ostype=unknown-linux-$_clibtype
            _bitness=$(get_bitness)
            ;;

        FreeBSD)
            _ostype=unknown-freebsd
            ;;

        NetBSD)
            _ostype=unknown-netbsd
            ;;

        DragonFly)
            _ostype=unknown-dragonfly
            ;;

        Darwin)
            _ostype=apple-darwin
            ;;

        illumos)
            _ostype=unknown-illumos
            ;;

        MINGW* | MSYS* | CYGWIN*)
            _ostype=pc-windows-gnu
            ;;

        *)
            err "unrecognized OS type: $_ostype"
            ;;

    esac

    case "$_cputype" in

        i386 | i486 | i686 | i786 | x86)
            _cputype=i686
            ;;

        xscale | arm)
            _cputype=arm
            if [ "$_ostype" = "linux-android" ]; then
                _ostype=linux-androideabi
            fi
            ;;

        armv6l)
            _cputype=arm
            if [ "$_ostype" = "linux-android" ]; then
                _ostype=linux-androideabi
            else
                _ostype="${_ostype}eabihf"
            fi
            ;;

        armv7l | armv8l)
            _cputype=armv7
            if [ "$_ostype" = "linux-android" ]; then
                _ostype=linux-androideabi
            else
                _ostype="${_ostype}eabihf"
            fi
            ;;

        aarch64 | arm64)
            _cputype=aarch64
            ;;

        x86_64 | x86-64 | x64 | amd64)
            _cputype=x86_64
            ;;

        mips)
            _cputype=$(get_endianness mips '' el)
            ;;

        mips64)
            if [ "$_bitness" -eq 64 ]; then
                # only n64 ABI is supported for now
                _ostype="${_ostype}abi64"
                _cputype=$(get_endianness mips64 '' el)
            fi
            ;;

        ppc)
            _cputype=powerpc
            ;;

        ppc64)
            _cputype=powerpc64
            ;;

        ppc64le)
            _cputype=powerpc64le
            ;;

        s390x)
            _cputype=s390x
            ;;

        riscv64)
            _cputype=riscv64gc
            ;;

        *)
            err "unknown CPU type: $_cputype"
            ;;

    esac

    # Detect 64-bit linux with 32-bit userland
    if [ "${_ostype}" = unknown-linux-gnu ] && [ "${_bitness}" -eq 32 ]; then
        case $_cputype in
            x86_64)
                _cputype=i686
                ;;
            mips64)
                _cputype=$(get_endianness mips '' el)
                ;;
            powerpc64)
                _cputype=powerpc
                ;;
            aarch64)
                _cputype=armv7
                if [ "$_ostype" = "linux-android" ]; then
                    _ostype=linux-androideabi
                else
                    _ostype="${_ostype}eabihf"
                fi
                ;;
            riscv64gc)
                err "riscv64 with 32-bit userland unsupported"
                ;;
        esac
    fi

    # Detect armv7 but without the CPU features Rust needs in that build,
    # and fall back to arm.
    if [ "$_ostype" = "unknown-linux-gnueabihf" ] && [ "$_cputype" = armv7 ]; then
        if ensure grep '^Features' /proc/cpuinfo | grep -q -v neon; then
            # At least one processor does not have NEON.
            _cputype=arm
        fi
    fi

    _arch="${_cputype}-${_ostype}"

    RETVAL="$_arch"
}

need_cmd() {
    if ! check_cmd "$1"; then
        err "need '$1' (command not found)"
        exit 1
    fi
}

check_cmd() {
    command -v "$1" > /dev/null 2>&1
}

# Run a command that should never fail. If the command fails execution
# will immediately terminate with an error showing the failing command.
ensure() {
    if ! "$@"; then
        err "command failed: $*"
        exit 1
    fi
}

assert_nz() {
    if [ -z "$1" ]; then
        err "assert_nz $2"
        exit 1
    fi
}

say() {
    printf 'fluvio: %s\n' "$1"
}

err() {
    printf 'fluvio: %s\n' "$1" >&2
}

# Exit immediately, prompting the user to file an issue on GH
abort_prompt_issue() {
    err ""
    err "If you believe this is a bug (or just need help),"
    err "please feel free to file an issue on GitHub"
    err "    https://github.com/${GITHUB_REPO}/issues/new"
    exit 1
}

main "$@"
