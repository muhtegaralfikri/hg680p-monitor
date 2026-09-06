#!/usr/bin/env bash
set -euo pipefail

APP_NAME="hg680p-monitor"
REPO="${HG680P_MONITOR_REPO:?Set HG680P_MONITOR_REPO, example: muhtegaralfikri/hg680p-monitor}"
TOKEN="${HG680P_MONITOR_GITHUB_TOKEN:-}"
ASSET_NAME="${HG680P_MONITOR_ASSET_NAME:-hg680p-monitor}"
DOWNLOAD_BASE="https://github.com/${REPO}/releases/latest/download"
INSTALL_DIR="/opt/${APP_NAME}"
BIN_PATH="${INSTALL_DIR}/${APP_NAME}"
SERVICE="${APP_NAME}"
STATE_DIR="/var/lib/${APP_NAME}"
VERSION_FILE="${STATE_DIR}/deployed-sha256.txt"
WORK_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "${WORK_DIR}"
}
trap cleanup EXIT

header_args=(-fsSL)
if [ -n "${TOKEN}" ]; then
  header_args+=(-H "Authorization: Bearer ${TOKEN}")
fi

mkdir -p "${INSTALL_DIR}" "${STATE_DIR}"

asset_url="${DOWNLOAD_BASE}/${ASSET_NAME}"
sha_url="${DOWNLOAD_BASE}/${ASSET_NAME}.sha256"

curl "${header_args[@]}" -L "${asset_url}" -o "${WORK_DIR}/${ASSET_NAME}"
curl "${header_args[@]}" -L "${sha_url}" -o "${WORK_DIR}/${ASSET_NAME}.sha256"

cd "${WORK_DIR}"
expected_sha="$(awk '{print $1}' "${ASSET_NAME}.sha256" | head -n1)"
if [ -f "${VERSION_FILE}" ] && [ "$(cat "${VERSION_FILE}")" = "${expected_sha}" ]; then
  echo "${APP_NAME} already up to date: ${expected_sha}"
  exit 0
fi

printf '%s  %s\n' "${expected_sha}" "${ASSET_NAME}" > "${ASSET_NAME}.sha256.local"
sha256sum -c "${ASSET_NAME}.sha256.local"
chmod +x "${ASSET_NAME}"

systemctl stop "${SERVICE}"
install -m 755 "${ASSET_NAME}" "${BIN_PATH}"
systemctl start "${SERVICE}"
systemctl is-active "${SERVICE}"
curl -fsSI "http://127.0.0.1:8099/" >/dev/null

printf '%s' "${expected_sha}" > "${VERSION_FILE}"
echo "${APP_NAME} deployed from latest release: ${expected_sha}"
