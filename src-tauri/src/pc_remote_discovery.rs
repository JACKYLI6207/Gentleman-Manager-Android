use std::time::Duration;

use anyhow::Context;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use specta::Type;
use tokio::net::UdpSocket;

const SERVICE_TYPE: &str = "_gentleman-manager._tcp.local.";
const UDP_DISCOVERY_PORT: u16 = 38765;
const DISCOVER_PACKET: &[u8] = b"GM_REMOTE_V1\nDISCOVER\n";
const SCAN_SECS: u64 = 4;
const CONNECT_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredRemotePc {
    pub name: String,
    /// 區網內可嘗試連線的 IPv4（多網卡時可能有多個）
    pub hosts: Vec<String>,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemotePcConnectionResult {
    pub connected: bool,
    pub message: String,
    pub connected_host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemotePcDirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemotePcBrowseResult {
    pub path: String,
    pub entries: Vec<RemotePcDirEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemotePcFileItem {
    pub relative_path: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
struct ListFilesResponse {
    ok: bool,
    files: Vec<RemotePcFileItem>,
}

#[derive(Debug, Deserialize)]
struct BrowseResponse {
    ok: bool,
    path: String,
    entries: Vec<RemotePcDirEntry>,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    ok: bool,
    /// 2 = 支援 POST 傳 path；舊版 PC 無此欄位視為 1
    #[serde(default = "default_remote_api_v1")]
    remote_api: u32,
}

fn default_remote_api_v1() -> u32 {
    1
}

/// 確認 PC 遠端 API 版本足夠（含 `[`、長路徑等需 v2）。
pub async fn ensure_pc_remote_api_v2(host: &str, port: u16) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .build()
        .context("建立 HTTP 用戶端失敗")?;
    let resp = client
        .get(format!("http://{host}:{port}/api/v1/health"))
        .send()
        .await
        .context("無法連線 PC health")?;
    if !resp.status().is_success() {
        anyhow::bail!("PC health HTTP {}", resp.status());
    }
    let body: HealthResponse = resp.json().await.context("解析 PC health 失敗")?;
    if !body.ok {
        anyhow::bail!("PC 遠端服務回應異常");
    }
    if body.remote_api < 2 {
        anyhow::bail!(
            "PC 遠端服務版本過舊（remote_api={}）。請更新並執行最新版 Gentleman-Manager-v1.3.0.exe，在設定中重新啟動遠端管理後再傳輸",
            body.remote_api
        );
    }
    Ok(())
}

/// 確認 PC 支援手機上傳（remote_api >= 3）。
pub async fn ensure_pc_remote_api_v3(host: &str, port: u16) -> anyhow::Result<()> {
    ensure_pc_remote_api_v2(host, port).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .build()
        .context("建立 HTTP 用戶端失敗")?;
    let resp = client
        .get(format!("http://{host}:{port}/api/v1/health"))
        .send()
        .await
        .context("無法連線 PC health")?;
    let body: HealthResponse = resp.json().await.context("解析 PC health 失敗")?;
    if body.remote_api < 3 {
        anyhow::bail!(
            "PC 遠端服務不支援上傳（remote_api={}）。請更新 PC 版 EXE 並重新啟動遠端管理",
            body.remote_api
        );
    }
    Ok(())
}

pub async fn check_remote_upload_conflicts(
    host: &str,
    port: u16,
    paths: &[String],
) -> anyhow::Result<Vec<String>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .build()
        .context("建立 HTTP 用戶端失敗")?;
    let resp = client
        .post(format!("http://{host}:{port}/api/v1/upload-exists"))
        .json(&json!({ "paths": paths }))
        .send()
        .await
        .context("檢查 PC 檔案衝突失敗")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "檢查 PC 檔案衝突 HTTP {status}{}",
            if body.is_empty() {
                String::new()
            } else {
                format!("：{body}")
            }
        );
    }
    #[derive(Deserialize)]
    struct ExistsResponse {
        conflicts: Vec<String>,
    }
    let body: ExistsResponse = resp.json().await.context("解析衝突清單失敗")?;
    Ok(body.conflicts)
}

/// 在區網內掃描已開啟遠端管理的 PC（mDNS + UDP 廣播）。
pub async fn scan_lan_remote_pcs() -> anyhow::Result<Vec<DiscoveredRemotePc>> {
    let mdns = tauri::async_runtime::spawn(async { scan_mdns().unwrap_or_default() });
    let udp = tauri::async_runtime::spawn(async { scan_udp_broadcast().await.unwrap_or_default() });

    let mut found = mdns.await.unwrap_or_default();
    for pc in udp.await.unwrap_or_default() {
        merge_discovered(&mut found, pc);
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

fn merge_discovered(found: &mut Vec<DiscoveredRemotePc>, pc: DiscoveredRemotePc) {
    if pc.hosts.is_empty() {
        return;
    }
    if let Some(existing) = found
        .iter_mut()
        .find(|e| e.name == pc.name && e.port == pc.port)
    {
        for host in pc.hosts {
            if !existing.hosts.contains(&host) {
                existing.hosts.push(host);
            }
        }
        existing.hosts.sort();
    } else {
        found.push(pc);
    }
}

fn scan_mdns() -> anyhow::Result<Vec<DiscoveredRemotePc>> {
    let mdns = ServiceDaemon::new().context("建立 mDNS 用戶端失敗")?;
    let receiver = mdns
        .browse(SERVICE_TYPE)
        .context("開始瀏覽區網服務失敗")?;

    let mut found = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(SCAN_SECS);

    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining.min(Duration::from_millis(400))) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                merge_discovered(&mut found, parse_mdns_info(&info));
            }
            Ok(ServiceEvent::ServiceFound(_, _)) => {}
            Ok(_) => {}
            Err(_) => break,
        }
    }

    let _ = mdns.shutdown();
    Ok(found)
}

fn parse_mdns_info(info: &mdns_sd::ServiceInfo) -> DiscoveredRemotePc {
    let name = info.get_fullname().to_string();
    let display_name = name
        .split('.')
        .next()
        .unwrap_or(&name)
        .to_string();
    let mut hosts: Vec<String> = info
        .get_addresses()
        .iter()
        .filter(|ip| ip.is_ipv4())
        .map(|ip| ip.to_string())
        .collect();
    hosts.sort();
    hosts.dedup();
    let port = info.get_port();
    DiscoveredRemotePc {
        name: display_name,
        hosts,
        port,
    }
}

async fn scan_udp_broadcast() -> anyhow::Result<Vec<DiscoveredRemotePc>> {
    let socket = UdpSocket::bind(("0.0.0.0", 0))
        .await
        .context("建立 UDP socket 失敗")?;
    socket
        .set_broadcast(true)
        .context("啟用 UDP broadcast 失敗")?;

    let target = format!("255.255.255.255:{UDP_DISCOVERY_PORT}");
    let _ = socket.send_to(DISCOVER_PACKET, &target).await;

    let mut found = Vec::new();
    let mut buf = [0_u8; 512];
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(
            remaining.min(Duration::from_millis(300)),
            socket.recv_from(&mut buf),
        )
        .await
        {
            Ok(Ok((len, _))) => {
                merge_discovered(&mut found, parse_udp_reply(&buf[..len]));
            }
            _ => break,
        }
    }
    Ok(found)
}

fn parse_udp_reply(data: &[u8]) -> DiscoveredRemotePc {
    let mut name = String::new();
    let mut port = 0_u16;
    let mut hosts = Vec::new();
    if let Ok(text) = std::str::from_utf8(data) {
        let mut lines = text.lines();
        if lines.next() == Some("GM_REMOTE_V1") && lines.next() == Some("OK") {
            name = lines.next().unwrap_or("").to_string();
            if let Some(p) = lines.next() {
                port = p.parse().unwrap_or(0);
            }
            for line in lines {
                let line = line.trim();
                if line.parse::<std::net::Ipv4Addr>().is_ok() {
                    hosts.push(line.to_string());
                }
            }
        }
    }
    hosts.sort();
    hosts.dedup();
    DiscoveredRemotePc { name, hosts, port }
}

/// 依序測試多個 IP，任一成功即視為能連線。
pub async fn test_remote_pc_connection(
    hosts: Vec<String>,
    port: u16,
) -> RemotePcConnectionResult {
    let hosts: Vec<String> = hosts
        .into_iter()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .collect();
    if hosts.is_empty() {
        return RemotePcConnectionResult {
            connected: false,
            message: "沒有可測試的 IP".to_string(),
            connected_host: None,
        };
    }

    let mut last_msg = String::new();
    for host in &hosts {
        let result = probe_health(host, port).await;
        if result.connected {
            return RemotePcConnectionResult {
                connected: true,
                message: if hosts.len() > 1 {
                    format!("能連線（{host}）")
                } else {
                    "能連線".to_string()
                },
                connected_host: Some(host.clone()),
            };
        }
        last_msg = result.message;
    }

    RemotePcConnectionResult {
        connected: false,
        message: format_connection_failure(&hosts, port, &last_msg),
        connected_host: None,
    }
}

/// 列出 PC 遠端管理資料夾內容（`path` 為相對於 PC 設定根目錄的路徑，空字串為根）。
pub async fn list_remote_pc_directory(
    host: &str,
    port: u16,
    path: &str,
) -> anyhow::Result<RemotePcBrowseResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .build()
        .context("建立 HTTP 用戶端失敗")?;
    let resp = client
        .post(format!("http://{host}:{port}/api/v1/browse"))
        .json(&json!({ "path": path }))
        .send()
        .await
        .context("無法連線 PC")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {status}{}", if body.is_empty() { String::new() } else { format!("：{body}") });
    }
    let text = resp.text().await.context("讀取目錄列表失敗")?;
    let body: BrowseResponse = serde_json::from_str(&text).context("解析目錄列表失敗")?;
    if !body.ok {
        anyhow::bail!("PC 回應異常");
    }
    Ok(RemotePcBrowseResult {
        path: body.path,
        entries: body.entries,
    })
}

/// 列出 PC 遠端路徑下所有檔案（資料夾則遞迴；檔案則單筆）。
pub async fn list_remote_pc_files(
    host: &str,
    port: u16,
    path: &str,
) -> anyhow::Result<Vec<RemotePcFileItem>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("建立 HTTP 用戶端失敗")?;
    let resp = client
        .post(format!("http://{host}:{port}/api/v1/list-files"))
        .json(&json!({ "path": path }))
        .send()
        .await
        .context("無法連線 PC")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {status}{}", if body.is_empty() { String::new() } else { format!("：{body}") });
    }
    let text = resp.text().await.context("讀取檔案列表失敗")?;
    let body: ListFilesResponse = serde_json::from_str(&text).context("解析檔案列表失敗")?;
    if !body.ok {
        anyhow::bail!("PC 回應異常");
    }
    Ok(body.files)
}

fn format_connection_failure(hosts: &[String], port: u16, detail: &str) -> String {
    let ips = hosts.join("、");
    if detail.contains("connection refused") || detail.contains("積極拒絕") {
        return format!(
            "無法連線 {ips}:{port}（PC 未開啟 HTTP 服務或 Windows 防火牆阻擋）。請確認 PC 設定顯示「執行中」，並在防火牆允許 Gentleman Manager 私人網路。"
        );
    }
    if detail.contains("timed out") || detail.contains("逾時") {
        return format!(
            "連線逾時（{ips}:{port}）。請確認手機與 PC 同一 Wi‑Fi 網段，且路由器未開啟「AP 隔離／訪客網路隔離」。"
        );
    }
    format!("不能連線（已試 {ips}）：{detail}")
}

async fn probe_health(host: &str, port: u16) -> RemotePcConnectionResult {
    let url = format!("http://{host}:{port}/api/v1/health");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            return RemotePcConnectionResult {
                connected: false,
                message: format!("建立 HTTP 用戶端失敗：{err}"),
                connected_host: None,
            };
        }
    };

    match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                return RemotePcConnectionResult {
                    connected: false,
                    message: format!("HTTP {}", resp.status()),
                    connected_host: None,
                };
            }
            match resp.text().await {
                Ok(text) => match serde_json::from_str::<HealthResponse>(&text) {
                    Ok(body) if body.ok => {
                        let message = if body.remote_api >= 2 {
                            "能連線".to_string()
                        } else {
                            "能連線（PC 程式過舊，長檔名或 [ ] 可能失敗，請更新 EXE）"
                                .to_string()
                        };
                        RemotePcConnectionResult {
                            connected: true,
                            message,
                            connected_host: Some(host.to_string()),
                        }
                    }
                    Ok(_) => RemotePcConnectionResult {
                        connected: false,
                        message: "回應異常".to_string(),
                        connected_host: None,
                    },
                    Err(err) => RemotePcConnectionResult {
                        connected: false,
                        message: format!("解析回應失敗：{err}"),
                        connected_host: None,
                    },
                },
                Err(err) => RemotePcConnectionResult {
                    connected: false,
                    message: format!("讀取回應失敗：{err}"),
                    connected_host: None,
                },
            }
        }
        Err(err) => RemotePcConnectionResult {
            connected: false,
            message: err.to_string(),
            connected_host: None,
        },
    }
}
