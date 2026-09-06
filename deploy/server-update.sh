#!/usr/bin/env bash
set -euo pipefail

APP_NAME="hg680p-monitor"
SRC_DIR="/srv/src/${APP_NAME}"
ARCHIVE="/srv/src/${APP_NAME}-update.tar.gz"
BIN_DIR="/opt/${APP_NAME}"
BIN_PATH="${BIN_DIR}/${APP_NAME}"
SERVICE="${APP_NAME}"
HEALTH_URL="http://127.0.0.1:8099/"

cd /srv/src

if [ ! -f "${ARCHIVE}" ]; then
  echo "Archive not found: ${ARCHIVE}" >&2
  exit 1
fi

rm -rf "${SRC_DIR}"
tar -xzf "${ARCHIVE}"
cd "${SRC_DIR}"

cargo build --release

systemctl stop "${SERVICE}"
install -m 755 "target/release/${APP_NAME}" "${BIN_PATH}"
systemctl start "${SERVICE}"

systemctl is-active "${SERVICE}"
curl -I "${HEALTH_URL}"

