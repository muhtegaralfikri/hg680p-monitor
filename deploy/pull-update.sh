#!/usr/bin/env bash
set -euo pipefail

APP_NAME="hg680p-monitor"
SRC_DIR="/srv/src/${APP_NAME}"
BIN_PATH="/opt/${APP_NAME}/${APP_NAME}"
SERVICE="${APP_NAME}"
HEALTH_URL="http://127.0.0.1:8099/"

cd "${SRC_DIR}"
git pull --ff-only
cargo build --release

systemctl stop "${SERVICE}"
install -m 755 "target/release/${APP_NAME}" "${BIN_PATH}"
systemctl start "${SERVICE}"

systemctl is-active "${SERVICE}"
curl -I "${HEALTH_URL}"

