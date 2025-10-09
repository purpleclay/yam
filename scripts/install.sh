#!/bin/sh

# Install script is heavily based on: https://github.com/Masterminds/glide.sh/blob/master/get

: "${USE_SUDO:=true}"
: "${INSTALL_DIR:=/usr/local/bin}"
: "${DEBUG:=false}"
: "${VERIFY:=true}"
: "${YAM_VERSION:=}"

APP_NAME="yam"
HAS_CURL="$(command -v curl >/dev/null 2>&1 && echo true || echo false)"
HAS_WGET="$(command -v wget >/dev/null 2>&1 && echo true || echo false)"

datefmt() { date +'%Y-%m-%dT%H:%M:%S'; }

GRAY='\033[1;30m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

error() {
  echo "${GRAY}$(datefmt) ${RED}ERROR${NC} $*" >&2
  exit 1
}

warn() { echo "${GRAY}$(datefmt) ${YELLOW}WARN${NC} $*" >&2; }

info() { echo "${GRAY}$(datefmt) ${GREEN}INFO${NC} $*"; }

debug() {
  if [ "${DEBUG}" = "true" ]; then
    echo "${GRAY}$(datefmt) ${BLUE}DEBUG${NC} $*"
  fi
}

initTarget() {
  arch="$(uname -m)"
  case "$arch" in
    x86_64) arch="x86_64" ;;
    amd64) arch="x86_64" ;;
    aarch64) arch="aarch64" ;;
    arm64) arch="aarch64" ;; # Keep unknown architectures as-is
  esac

  case "$(uname -s)" in
    "Linux")
      platform="unknown-linux-musl"
      TARGET="$arch-$platform"
      ;;
    "Darwin")
      platform="apple-darwin"
      TARGET="$arch-$platform"
      ;;
  esac
}

checkPrerequisites() {
  debug "Checking supported target..."
  _supported="x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-apple-darwin aarch64-apple-darwin"
  if ! echo "${_supported}" | grep -qw "${TARGET}"; then
    error "No prebuilt binary currently exists for target ${TARGET}."
  fi

  debug "Checking download utility..."
  if [ "${HAS_CURL}" != "true" ] && [ "${HAS_WGET}" != "true" ]; then
    error "Either curl or wget is required to download binary. Please install and try again"
  fi
}

download() {
  url=$1
  output=$2

  if [ "${HAS_CURL}" = "true" ]; then
    curl -sSL -o "${output}" "${url}"
  elif [ "${HAS_WGET}" = "true" ]; then
    wget -q -O "${output}" "${url}"
  fi
}

getLatestRelease() {
  download https://api.github.com/repos/purpleclay/$APP_NAME/releases/latest - | grep "tag_name" | cut -d'"' -f4
}

getTag() {
  info "Checking ${APP_NAME} version to install..."
  if [ -z "${YAM_VERSION}" ]; then
    TAG=$(getLatestRelease)
  else
    if ! echo "${YAM_VERSION}" | grep -qE "^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?(\+[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?$"; then
      error "Invalid version provided. Please provide a valid version: e.g. $(getLatestRelease)"
    fi

    _versions=$(download "https://api.github.com/repos/purpleclay/$APP_NAME/releases" - | grep "tag_name" | cut -d'"' -f4)
    debug "Available versions: $(echo "${_versions}" | tr '\n' ' ')"
    if ! echo "${_versions}" | grep -q "${YAM_VERSION}"; then
      error "Version ${YAM_VERSION} does not exist. Please provide a valid version: e.g. $(getLatestRelease)"
    fi

    debug "Using provided version: ${YAM_VERSION}"
    TAG=${YAM_VERSION}
  fi

  if [ -z "${TAG}" ]; then
    error "Failed to set version to install. Exiting..."
  fi
}

downloadBinary() {
  info "Attempting to download ${APP_NAME} version ${TAG}..."

  PACKAGE_TYPE="tar.gz"
  _archive="${APP_NAME}-${TAG#v}-${TARGET}.${PACKAGE_TYPE}"

  DOWNLOAD_URL="https://github.com/purpleclay/${APP_NAME}/releases/download/${TAG}/${_archive}"
  DOWNLOAD_DIR="$(mktemp -dt ${APP_NAME}-install-XXXXXXX)"
  DOWNLOAD_FILE="${DOWNLOAD_DIR}/${_archive}"

  debug "Downloading ${DOWNLOAD_URL} to ${DOWNLOAD_FILE}"
  download "${DOWNLOAD_URL}" "${DOWNLOAD_FILE}"
}

install() {
  info "Installing ${APP_NAME}..."
  test ! -d "$INSTALL_DIR" && mkdir -p "$INSTALL_DIR"

  _extract_dir="$DOWNLOAD_DIR/${APP_NAME}-${TAG}"
  debug "Extracting ${DOWNLOAD_FILE} to ${_extract_dir}..."
  mkdir -p "$_extract_dir"
  tar xf "$DOWNLOAD_FILE" -C "${_extract_dir}"
  runAsRoot cp "${_extract_dir}/${APP_NAME}" "${INSTALL_DIR}/${APP_NAME}"

  info "Installed ${APP_NAME} to ${INSTALL_DIR}"
}

runAsRoot() {
  if [ "$(id -u)" -ne 0 ] && [ "$USE_SUDO" = "true" ]; then
    sudo "${@}"
  else
    "${@}"
  fi
}

tidy() {
  debug "Performing cleanup..."
  if [ -n "${DOWNLOAD_DIR:-}" ] && [ -d "$DOWNLOAD_DIR" ]; then
    rm -rf "$DOWNLOAD_DIR"
  fi
}

verify() {
  set +e
  debug "Verifying installation..."
  if ! command -v "$APP_NAME" >/dev/null; then
    error "${APP_NAME} not found. Is ${INSTALL_DIR} on your PATH?"
  fi

  # Ensure backwards compatibility with versions predating 0.5.0
  if ! INSTALLED_VERSION="$($APP_NAME version --short 2>/dev/null)"; then
    debug "Falling back to legacy version check..."
    INSTALLED_VERSION="$($APP_NAME -V 2>/dev/null)"
    if ! INSTALLED_VERSION="$($APP_NAME -V 2>/dev/null)"; then
      error "Failed to get version of ${APP_NAME} for verification"
    fi

    # Extract version number from output
    INSTALLED_VERSION="$(echo "$INSTALLED_VERSION" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)"
  fi

  if [ "${INSTALLED_VERSION}" != "${TAG}" ]; then
    error "Found version ${INSTALLED_VERSION} of ${APP_NAME} and not expected installed version of $TAG"
  fi

  info "Installation verified"
  set -e
}

bye() {
  _result=$?
  tidy
  exit $_result
}

help() {
  cat <<EOF
${APP_NAME} installer

Flags:
      --debug              Enable debug mode
  -d, --dir <directory>    Directory where the binary will be installed (default '$INSTALL_DIR')
      --no-sudo            Install without using sudo
      --skip-verify        Skip verification step
  -v, --version <version>  Download and install a specific version (default 'latest')
  -h, --help               Print help for the installer
EOF
}

trap "bye" EXIT
set -e

# Parsing input arguments (if any)
for arg in "$@"; do
  if [ "$arg" = "--help" ] || [ "$arg" = "-h" ]; then
    help
    exit 0
  fi
done

set -u
while [ $# -gt 0 ]; do
  case $1 in
    '--debug')
      DEBUG="true"
      ;;
    '--dir' | -d)
      shift
      if [ $# -eq 0 ]; then
        error "Please provide a valid location for the install directory"
      fi

      if [ ! -d "${1}" ]; then
        error "Directory ${1} does not exist"
      fi

      INSTALL_DIR="${1}"
      ;;
    '--no-sudo')
      USE_SUDO="false"
      ;;
    '--skip-verify')
      VERIFY="false"
      ;;
    '--version' | -v)
      shift
      if [ $# -eq 0 ]; then
        error "Please provide a valid version: e.g. --version $(getLatestRelease)"
      fi
      YAM_VERSION="${1}"
      ;;
    *)
      error \
        "Invalid flag provided '$1'." \
        "Run '$0 --help' to see the available options"
      ;;
  esac
  shift
done
set +u

initTarget
checkPrerequisites
getTag
downloadBinary
install
[ "${VERIFY}" = "true" ] && verify

info "Successfully installed ${APP_NAME} ${TAG} 🎉"
