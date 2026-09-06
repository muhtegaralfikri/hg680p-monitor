use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};

static INDEX_HTML: &str = include_str!("../web/index.html");

#[derive(Clone, Copy, Default)]
struct CpuSample {
    idle: u64,
    total: u64,
}

#[derive(Default)]
struct AppState {
    previous_cpu: CpuSample,
}

#[derive(Default)]
struct MemInfo {
    total: u64,
    available: u64,
    swap_total: u64,
    swap_free: u64,
}

#[derive(Clone, Default)]
struct DiskInfo {
    path: String,
    filesystem: String,
    used: u64,
    total: u64,
    percent: f64,
}

struct ServiceInfo {
    name: String,
    active: bool,
}

struct ContainerInfo {
    name: String,
    image: String,
    status: String,
}

#[derive(Default)]
struct LoadInfo {
    one: f64,
    five: f64,
    fifteen: f64,
}

struct ProcessInfo {
    pid: String,
    name: String,
    cpu: String,
    memory: String,
    rss: u64,
    command: String,
}

fn main() {
    let bind = env::var("MONITOR_BIND").unwrap_or_else(|_| "127.0.0.1:8099".to_string());
    let listener = TcpListener::bind(&bind).unwrap_or_else(|err| {
        panic!("failed to bind {}: {}", bind, err);
    });

    let state = Arc::new(Mutex::new(AppState::default()));
    println!("hg680p-monitor listening on http://{}", bind);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                handle_connection(stream, state);
            }
            Err(err) => eprintln!("connection error: {}", err),
        }
    }
}

fn handle_connection(mut stream: TcpStream, state: Arc<Mutex<AppState>>) {
    let mut buffer = [0; 2048];
    let bytes = match stream.read(&mut buffer) {
        Ok(size) => size,
        Err(_) => return,
    };

    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let path = parse_path(&request);

    match path.as_str() {
        "/" | "/index.html" => respond(&mut stream, 200, "text/html; charset=utf-8", INDEX_HTML),
        "/api/status" => {
            let body = build_status_json(state);
            respond(&mut stream, 200, "application/json; charset=utf-8", &body);
        }
        "/health" => respond(&mut stream, 200, "text/plain; charset=utf-8", "ok\n"),
        _ => respond(&mut stream, 404, "text/plain; charset=utf-8", "not found\n"),
    }
}

fn parse_path(request: &str) -> String {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string()
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "OK",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        status,
        reason,
        content_type,
        body.as_bytes().len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
}

fn build_status_json(state: Arc<Mutex<AppState>>) -> String {
    let cpu = read_cpu_percent(&state);
    let load = read_loadavg();
    let mem = read_meminfo();
    let disks = read_disks();
    let root_disk = disks.iter().find(|disk| disk.path == "/").cloned().unwrap_or_default();
    let uptime = read_uptime_seconds();
    let temperature = read_temperature_c();
    let services = read_services();
    let containers = read_containers();
    let processes = read_top_processes();

    format!(
        "{{\"hostname\":\"{}\",\"uptime_seconds\":{},\"cpu_percent\":{},\"load\":{{\"one\":{},\"five\":{},\"fifteen\":{}}},\"temperature_c\":{},\"memory\":{{\"total\":{},\"used\":{},\"available\":{},\"percent\":{}}},\"swap\":{{\"total\":{},\"used\":{},\"free\":{},\"percent\":{}}},\"disk\":{{\"path\":\"/\",\"filesystem\":\"{}\",\"total\":{},\"used\":{},\"free\":{},\"percent\":{}}},\"disks\":[{}],\"services\":[{}],\"containers\":[{}],\"processes\":[{}]}}",
        json_escape(&hostname()),
        uptime,
        fmt_f64(cpu),
        fmt_f64(load.one),
        fmt_f64(load.five),
        fmt_f64(load.fifteen),
        optional_f64(temperature),
        mem.total,
        mem.total.saturating_sub(mem.available),
        mem.available,
        percent(mem.total.saturating_sub(mem.available), mem.total),
        mem.swap_total,
        mem.swap_total.saturating_sub(mem.swap_free),
        mem.swap_free,
        percent(mem.swap_total.saturating_sub(mem.swap_free), mem.swap_total),
        json_escape(&root_disk.filesystem),
        root_disk.total,
        root_disk.used,
        root_disk.total.saturating_sub(root_disk.used),
        fmt_f64(root_disk.percent),
        disks
            .iter()
            .map(|disk| format!(
                "{{\"path\":\"{}\",\"filesystem\":\"{}\",\"total\":{},\"used\":{},\"free\":{},\"percent\":{}}}",
                json_escape(&disk.path),
                json_escape(&disk.filesystem),
                disk.total,
                disk.used,
                disk.total.saturating_sub(disk.used),
                fmt_f64(disk.percent)
            ))
            .collect::<Vec<_>>()
            .join(","),
        services
            .iter()
            .map(|s| format!("{{\"name\":\"{}\",\"active\":{}}}", json_escape(&s.name), s.active))
            .collect::<Vec<_>>()
            .join(","),
        containers
            .iter()
            .map(|c| format!(
                "{{\"name\":\"{}\",\"image\":\"{}\",\"status\":\"{}\"}}",
                json_escape(&c.name),
                json_escape(&c.image),
                json_escape(&c.status)
            ))
            .collect::<Vec<_>>()
            .join(",")
        ,
        processes
            .iter()
            .map(|p| format!(
                "{{\"pid\":\"{}\",\"name\":\"{}\",\"cpu\":\"{}\",\"memory\":\"{}\",\"rss\":{},\"command\":\"{}\"}}",
                json_escape(&p.pid),
                json_escape(&p.name),
                json_escape(&p.cpu),
                json_escape(&p.memory),
                p.rss,
                json_escape(&p.command)
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn read_cpu_percent(state: &Arc<Mutex<AppState>>) -> f64 {
    let sample = match read_cpu_sample() {
        Some(sample) => sample,
        None => return 0.0,
    };

    let mut guard = match state.lock() {
        Ok(guard) => guard,
        Err(_) => return 0.0,
    };

    let previous = guard.previous_cpu;
    guard.previous_cpu = sample;

    if previous.total == 0 {
        return 0.0;
    }

    let idle_delta = sample.idle.saturating_sub(previous.idle);
    let total_delta = sample.total.saturating_sub(previous.total);
    if total_delta == 0 {
        0.0
    } else {
        ((total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64) * 100.0
    }
}

fn read_cpu_sample() -> Option<CpuSample> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    let values = line
        .split_whitespace()
        .skip(1)
        .filter_map(|value| value.parse::<u64>().ok())
        .collect::<Vec<_>>();

    if values.len() < 5 {
        return None;
    }

    let idle = values.get(3).copied().unwrap_or(0) + values.get(4).copied().unwrap_or(0);
    let total = values.iter().sum();
    Some(CpuSample { idle, total })
}

fn read_meminfo() -> MemInfo {
    let mut info = MemInfo::default();
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().unwrap_or("").trim_end_matches(':');
        let value = parts.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0) * 1024;

        match key {
            "MemTotal" => info.total = value,
            "MemAvailable" => info.available = value,
            "SwapTotal" => info.swap_total = value,
            "SwapFree" => info.swap_free = value,
            _ => {}
        }
    }

    info
}

fn read_disks() -> Vec<DiskInfo> {
    let output = Command::new("df")
        .args(["-B1", "-T"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_default();

    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            let filesystem = parts.get(0)?.to_string();
            let fs_type = parts.get(1)?.to_string();
            let total = parts.get(2)?.parse::<u64>().ok()?;
            let used = parts.get(3)?.parse::<u64>().ok()?;
            let path = parts.get(6)?.to_string();

            if !["/", "/boot", "/srv", "/var", "/DATA"].contains(&path.as_str()) {
                return None;
            }

            Some(DiskInfo {
                path,
                filesystem: format!("{} ({})", filesystem, fs_type),
                used,
                total,
                percent: percent(used, total),
            })
        })
        .collect()
}

fn read_uptime_seconds() -> u64 {
    fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| content.split_whitespace().next().map(str::to_string))
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value as u64)
        .unwrap_or(0)
}

fn read_temperature_c() -> Option<f64> {
    for path in [
        "/etc/armbianmonitor/datasources/soctemp",
        "/sys/devices/virtual/thermal/thermal_zone0/temp",
    ] {
        if let Some(value) = read_temperature_file(path) {
            return Some(value);
        }
    }

    let entries = fs::read_dir("/sys/class/thermal").ok()?;
    for entry in entries.flatten() {
        if let Some(path) = entry.path().join("temp").to_str() {
            if let Some(value) = read_temperature_file(path) {
                return Some(value);
            }
        }
    }
    None
}

fn read_temperature_file(path: &str) -> Option<f64> {
    let raw = fs::read_to_string(path).ok()?;
    let value = raw.trim().parse::<f64>().ok()?;
    if value > 1000.0 {
        Some(value / 1000.0)
    } else {
        Some(value)
    }
}

fn read_loadavg() -> LoadInfo {
    let content = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let parts = content.split_whitespace().collect::<Vec<_>>();

    LoadInfo {
        one: parts.get(0).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
        five: parts.get(1).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
        fifteen: parts.get(2).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
    }
}

fn read_services() -> Vec<ServiceInfo> {
    let default = "nginx,php8.3-fpm,mariadb,cloudflared,docker";
    let names = env::var("MONITOR_SERVICES").unwrap_or_else(|_| default.to_string());

    names
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            let active = Command::new("systemctl")
                .args(["is-active", "--quiet", name])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);

            ServiceInfo {
                name: name.to_string(),
                active,
            }
        })
        .collect()
}

fn read_containers() -> Vec<ContainerInfo> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}|{{.Image}}|{{.Status}}"])
        .output();

    let stdout = match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout).unwrap_or_default(),
        _ => return Vec::new(),
    };

    stdout
        .lines()
        .filter_map(|line| {
            let parts = line.split('|').collect::<Vec<_>>();
            Some(ContainerInfo {
                name: parts.get(0)?.to_string(),
                image: parts.get(1)?.to_string(),
                status: parts.get(2)?.to_string(),
            })
        })
        .collect()
}

fn read_top_processes() -> Vec<ProcessInfo> {
    let output = Command::new("ps")
        .args(["-eo", "pid,comm,%cpu,%mem,rss,args", "--sort=-rss"])
        .output();

    let stdout = match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout).unwrap_or_default(),
        _ => return Vec::new(),
    };

    stdout
        .lines()
        .skip(1)
        .take(10)
        .filter_map(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            Some(ProcessInfo {
                pid: parts.get(0)?.to_string(),
                name: parts.get(1)?.to_string(),
                cpu: parts.get(2)?.to_string(),
                memory: parts.get(3)?.to_string(),
                rss: parts.get(4)?.parse::<u64>().ok()? * 1024,
                command: sanitize_command(&parts.get(5..).unwrap_or(&[]).join(" ")),
            })
        })
        .collect()
}

fn sanitize_command(command: &str) -> String {
    let mut parts = command.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < parts.len() {
        if parts[index] == "--token" && index + 1 < parts.len() {
            parts[index + 1] = "[redacted]";
            index += 2;
            continue;
        }
        if parts[index].starts_with("--token=") {
            parts[index] = "--token=[redacted]";
        }
        index += 1;
    }

    let sanitized = parts.join(" ");
    let max = 96;
    if sanitized.chars().count() > max {
        format!("{}...", sanitized.chars().take(max).collect::<String>())
    } else {
        sanitized
    }
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|_| "server".to_string())
}

fn percent(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        ((used as f64 / total as f64) * 1000.0).round() / 10.0
    }
}

fn fmt_f64(value: f64) -> String {
    format!("{:.1}", value)
}

fn optional_f64(value: Option<f64>) -> String {
    value.map(fmt_f64).unwrap_or_else(|| "null".to_string())
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
