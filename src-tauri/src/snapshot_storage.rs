use std::path::{Path, PathBuf};

use anyhow::Context;
use regex::Regex;
use tauri::AppHandle;

use crate::utils::filename_filter;

fn is_snapshot_timestamp(value: &str) -> bool {
    if value.len() != 19 {
        return false;
    }
    value.chars().enumerate().all(|(index, ch)| match index {
        4 | 7 | 10 | 13 | 16 => ch == '_',
        _ => ch.is_ascii_digit(),
    })
}

pub fn website_snapshot_file_prefix(file_name: &str) -> &str {
    let stem = file_name
        .strip_suffix(".gm-snapshot.json")
        .unwrap_or(file_name);
    if stem.len() > 20 {
        let split_at = stem.len() - 20;
        let (prefix, timestamp_with_space) = stem.split_at(split_at);
        if let Some(timestamp) = timestamp_with_space.strip_prefix(' ') {
            if is_snapshot_timestamp(timestamp) {
                return prefix;
            }
        }
    }
    stem
}


fn delete_previous_snapshot_file(previous_snapshot_path: Option<&str>) -> anyhow::Result<()> {
    let Some(prev) = previous_snapshot_path.filter(|p| !p.trim().is_empty()) else {
        return Ok(());
    };
    if prev.starts_with("content://") {
        return Ok(());
    }
    let path = PathBuf::from(prev.trim());
    if !path.is_file() {
        return Ok(());
    }
    std::fs::remove_file(&path)
        .with_context(|| format!("刪除舊快照`{}`失敗", path.display()))?;
    Ok(())
}

/// 與前端 `safeSnapshotFileName` 相同：`{label} YYYY_MM_DD_HH_MM_SS.gm-snapshot.json`
pub fn export_snapshot_file_name(scan_target_label: &str) -> String {
    let base = format!("{}快照", scan_target_label.trim());
    let safe = filename_filter(&base);
    format!(
        "{} {}.gm-snapshot.json",
        if safe.is_empty() { "snapshot".to_string() } else { safe },
        snapshot_export_timestamp()
    )
}

fn snapshot_export_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = dur.as_secs();
    let time_of_day = total_secs % 86_400;
    let days = total_secs / 86_400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}_{month:02}_{day:02}_{hours:02}_{minutes:02}_{seconds:02}"
    )
}

fn civil_from_days(days_since_epoch: u64) -> (u64, u64, u64) {
    let z = days_since_epoch + 719_468;
    let era = z / 1_460_969;
    let doe = z - era * 1_460_969;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mp_i = mp as i64;
    let m_i = mp_i + if mp_i < 10 { 3 } else { -9 };
    let year = y + if m_i <= 2 { 1 } else { 0 };
    let month = m_i.max(1) as u64;
    (year, month, d)
}

pub fn write_snapshot_to_directory(
    snapshot_dir: &Path,
    file_name: &str,
    content: &str,
) -> anyhow::Result<String> {
    let file_name = filename_filter(file_name);
    if file_name.is_empty() {
        anyhow::bail!("快照檔名不可為空");
    }
    std::fs::create_dir_all(snapshot_dir)
        .with_context(|| format!("建立快照目錄`{}`失敗", snapshot_dir.display()))?;
    let path = snapshot_dir.join(&file_name);
    std::fs::write(&path, content)
        .with_context(|| format!("寫入快照檔案`{}`失敗", path.display()))?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg(target_os = "android")]
pub fn write_snapshot_to_tree(
    app: &AppHandle,
    tree_uri: &str,
    file_name: &str,
    content: &str,
) -> anyhow::Result<String> {
    let picker = crate::folder_picker::folder_picker(app)
        .map_err(|e| anyhow::anyhow!(e.err_message))?;
    picker
        .write_snapshot_to_tree(tree_uri, file_name, content)
        .map_err(|e| anyhow::anyhow!(e.err_message))
}

/// 將本機暫存檔發布至分類目錄（Android SAF 或本機路徑），避免巨大字串經 IPC。
pub fn publish_snapshot_file(
    app: &AppHandle,
    category_directory: &str,
    file_name: &str,
    source_path: &std::path::Path,
    cate_id: i64,
    previous_snapshot_path: Option<&str>,
) -> anyhow::Result<String> {
    let file_name = filename_filter(file_name);
    if file_name.is_empty() {
        anyhow::bail!("快照檔名不可為空");
    }
    if !source_path.is_file() {
        anyhow::bail!("快照暫存檔不存在：{}", source_path.display());
    }

    #[cfg(target_os = "android")]
    if category_directory.starts_with("content://") {
        let picker = crate::folder_picker::folder_picker(app)
            .map_err(|e| anyhow::anyhow!(e.err_message))?;
        return picker
            .publish_snapshot_file(
                category_directory,
                &file_name,
                &source_path.to_string_lossy(),
                cate_id,
                previous_snapshot_path,
            )
            .map_err(|e| anyhow::anyhow!(e.err_message));
    }

    let dir = std::path::PathBuf::from(category_directory);
    let dest = dir.join(&file_name);
    std::fs::copy(source_path, &dest).with_context(|| {
        format!(
            "複製快照至`{}`失敗",
            dest.display()
        )
    })?;
    let _ = cate_id;
    delete_previous_snapshot_file(previous_snapshot_path)?;
    Ok(dest.to_string_lossy().to_string())
}

pub fn read_meta_summary_from_text(text: &str, fallback_label: &str) -> MetaSummary {
    let read_i64 = |key: &str| -> Option<i64> {
        let re = Regex::new(&format!(r#""{key}"\s*:\s*(-?\d+)"#)).ok()?;
        re.captures(text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    };
    let read_str = |key: &str| -> Option<String> {
        let re = Regex::new(&format!(r#""{key}"\s*:\s*"((?:\\.|[^"\\])*)""#)).ok()?;
        re.captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().replace("\\\"", "\"").replace("\\\\", "\\"))
    };

    let cate_id = read_i64("scanTargetCateId");
    let label = read_str("scanTargetLabel")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_label.to_string());

    let scan_target_kind = read_str("scanTargetKind").filter(|s| !s.is_empty());

    MetaSummary {
        meta_id: read_str("id").unwrap_or_default(),
        saved_at: read_str("savedAt").unwrap_or_default(),
        total_count: read_i64("totalCount").unwrap_or(0),
        total_pages: read_i64("totalPages").unwrap_or(0),
        scan_completion_percent: read_i64("scanCompletionPercent").unwrap_or(0),
        scan_completed_pages: read_i64("scanCompletedPages").unwrap_or(0),
        cate_id,
        label,
        scan_target_kind,
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetaSummary {
    pub meta_id: String,
    pub saved_at: String,
    pub total_count: i64,
    pub total_pages: i64,
    pub scan_completion_percent: i64,
    pub scan_completed_pages: i64,
    pub cate_id: Option<i64>,
    pub label: String,
    pub scan_target_kind: Option<String>,
}

pub fn is_favorite_archive_snapshot_file_name(name: &str) -> bool {
    name.starts_with("收藏漫畫存檔_") || name.starts_with("收藏分頁存檔_")
}

pub fn is_favorite_archive_snapshot_text(text: &str) -> bool {
    text.contains("gentleman-manager.favorite-comics.v1")
        || text.contains("gentleman-manager.favorite-tabs.v1")
}

pub fn read_meta_summary_from_path(path: &Path) -> anyhow::Result<MetaSummary> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).context("開啟快照檔失敗")?;
    let mut buf = vec![0u8; 32_768];
    let n = file.read(&mut buf).context("讀取快照檔失敗")?;
    let text = String::from_utf8_lossy(&buf[..n]);
    let fallback = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未命名分類");
    Ok(read_meta_summary_from_text(&text, fallback))
}
