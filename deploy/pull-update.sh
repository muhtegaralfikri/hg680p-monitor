#!/usr/bin/env bash
set -euo pipefail

APP_NAME="hg680p-monitor"

systemctl start "${APP_NAME}-update.service"
journalctl -u "${APP_NAME}-update.service" -n 40 --no-pager
