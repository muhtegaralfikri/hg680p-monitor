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

## Website Monitor

Website dipantau otomatis dari:

- Nginx aktif di `/etc/nginx/sites-enabled`
- Port Docker yang dipublish ke host

Override manual tetap bisa lewat env:

```bash
MONITOR_WEBSITES="Nama=http://127.0.0.1:8080/,App=http://127.0.0.1:3000/"
```

## Build Lokal

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

Update manual tanpa build di server:

```bash
bash deploy/pull-update.sh
```

Server tidak perlu `cargo`/`rustc`. Semua build berat dikerjakan GitHub Actions.

## Catatan Nginx

File `deploy/nginx-hg680p-monitor.conf` membuka dashboard di port `8099` dan meneruskan request ke backend Rust lokal di `127.0.0.1:18099`.
