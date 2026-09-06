#!/usr/bin/env bash
set -euo pipefail

APP_NAME="hg680p-monitor"
ENV_DIR="/etc/${APP_NAME}"
ENV_FILE="${ENV_DIR}/update.env"
SCRIPT_PATH="/usr/local/sbin/${APP_NAME}-auto-update"

if [ "${EUID}" -ne 0 ]; then
  echo "Run as root." >&2
  exit 1
fi

mkdir -p "${ENV_DIR}" "/var/lib/${APP_NAME}" "/opt/${APP_NAME}"

install -m 755 "deploy/auto-update.sh" "${SCRIPT_PATH}"
install -m 644 "deploy/hg680p-monitor-update.service" "/etc/systemd/system/hg680p-monitor-update.service"
install -m 644 "deploy/hg680p-monitor-update.timer" "/etc/systemd/system/hg680p-monitor-update.timer"

if [ ! -f "${ENV_FILE}" ]; then
  install -m 600 "deploy/update.env.example" "${ENV_FILE}"
  echo "Created ${ENV_FILE}. Edit HG680P_MONITOR_REPO before enabling the timer."
else
  echo "Keeping existing ${ENV_FILE}."
fi

systemctl daemon-reload
echo "Next:"
echo "  nano ${ENV_FILE}"
echo "  systemctl enable --now hg680p-monitor-update.timer"
echo "  systemctl start hg680p-monitor-update.service"

