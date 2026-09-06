# HG680P Monitor

Dashboard monitoring server ringan untuk STB/HG680P.

## Tujuan

- Backend Rust kecil tanpa dependency eksternal.
- Frontend HTML/CSS/JS native tanpa build step.
- Tidak memakai database, Docker, Node, atau framework.
- Cocok dipasang di server kecil untuk melihat CPU, RAM, swap, disk, suhu, service, dan container.

## Endpoint

- `/` dashboard
- `/api/status` data JSON
- `/health` health check

## Build

```bash
cargo build --release
```

Binary ada di:

```bash
target/release/hg680p-monitor
```

## Jalankan Lokal

```bash
MONITOR_BIND=127.0.0.1:8099 ./target/release/hg680p-monitor
```

Di browser:

```text
http://127.0.0.1:8099
```

## Deploy Manual ke Server

```bash
mkdir -p /opt/hg680p-monitor
cp target/release/hg680p-monitor /opt/hg680p-monitor/
cp deploy/hg680p-monitor.service /etc/systemd/system/
cp deploy/nginx-hg680p-monitor.conf /etc/nginx/sites-available/hg680p-monitor.conf
ln -s /etc/nginx/sites-available/hg680p-monitor.conf /etc/nginx/sites-enabled/hg680p-monitor.conf
nginx -t
systemctl daemon-reload
systemctl enable --now hg680p-monitor
systemctl reload nginx
```

Default service bind ke `127.0.0.1:18099`. Untuk akses lewat LAN, pakai Nginx reverse proxy di port `8099`.

## Auto Deploy dari GitHub

Alur ringan untuk STB:

1. Push ke branch `main`.
2. GitHub Actions build binary Linux ARM64.
3. GitHub membuat release `latest`.
4. Server mengecek release terbaru tiap 5 menit.
5. Jika ada binary baru, server mengganti binary dan restart service.

Setup awal di server:

```bash
cd /srv/src/hg680p-monitor
bash deploy/install-auto-update.sh
nano /etc/hg680p-monitor/update.env
systemctl enable --now hg680p-monitor-update.timer
systemctl start hg680p-monitor-update.service
```

Isi minimal `/etc/hg680p-monitor/update.env`:

```bash
HG680P_MONITOR_REPO=muhtegaralfikri/hg680p-monitor
HG680P_MONITOR_ASSET_NAME=hg680p-monitor
```

Jika repository private, tambahkan token GitHub read-only:

```bash
HG680P_MONITOR_GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Pantau update:

```bash
systemctl status hg680p-monitor-update.timer
journalctl -u hg680p-monitor-update.service -n 80 --no-pager
```

## Catatan Nginx

File `deploy/nginx-hg680p-monitor.conf` membuka dashboard di port `8099` dan meneruskan request ke backend Rust lokal di `127.0.0.1:18099`.
