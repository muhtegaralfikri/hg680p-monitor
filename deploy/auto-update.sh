#!/usr/bin/env bash
set -euo pipefail

APP_NAME="hg680p-monitor"
REPO="${HG680P_MONITOR_REPO:?Set HG680P_MONITOR_REPO, example: muhtegaralfikri/hg680p-monitor}"
TOKEN="${HG680P_MONITOR_GITHUB_TOKEN:-}"
ASSET_NAME="${HG680P_MONITOR_ASSET_NAME:-hg680p-monitor}"
API_BASE="https://api.github.com/repos/${REPO}/releases/latest"
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

release_json="${WORK_DIR}/release.json"
curl "${header_args[@]}" "${API_BASE}" -o "${release_json}"

tag="$(grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' "${release_json}" | head -n1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"

if [ -z "${tag}" ]; then
  echo "Cannot read latest release metadata from ${REPO}" >&2
  exit 1
fi

asset_url="$(grep -A 20 "\"name\"[[:space:]]*:[[:space:]]*\"${ASSET_NAME}\"" "${release_json}" | grep -m1 '"browser_download_url"' | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"
sha_url="$(grep -A 20 "\"name\"[[:space:]]*:[[:space:]]*\"${ASSET_NAME}.sha256\"" "${release_json}" | grep -m1 '"browser_download_url"' | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"

if [ -z "${asset_url}" ] || [ -z "${sha_url}" ]; then
  echo "Release asset not found. Expected ${ASSET_NAME} and ${ASSET_NAME}.sha256" >&2
  exit 1
fi

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
echo "${APP_NAME} deployed from ${tag}: ${expected_sha}"
