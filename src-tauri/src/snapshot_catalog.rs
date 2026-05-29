use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotCategoryHeader {
    pub cate_id: Option<i64>,
    pub label: String,
    pub file_path: String,
    pub total_count: i64,
    pub saved_at: String,
    pub modified_ms: Option<i64>,
}


#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMeta {
    pub id: String,
    pub saved_at: String,
    pub total_count: i64,
    pub total_pages: i64,
    pub scan_completion_percent: i64,
    #[serde(default)]
    pub scan_direction: Option<String>,
    #[serde(default)]
    pub scan_completed_pages: i64,
    #[serde(default)]
    pub scan_completed_page_ranges: Vec<SnapshotPageRange>,
    #[serde(default)]
    pub scan_target_kind: Option<String>,
    #[serde(default)]
    pub scan_target_cate_id: Option<i64>,
    #[serde(default)]
    pub scan_target_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPageRange {
    pub start: i64,
    pub end: i64,
}

fn file_modified_ms(path: &Path) -> i64 {
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn collect_snapshot_files(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).context("讀取分類目錄失敗")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.eq_ignore_ascii_case("old") || name.eq_ignore_ascii_case("OLD") {
                continue;
            }
            collect_snapshot_files(&path, out)?;
            continue;
        }
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains("gm-snapshot") || n.ends_with(".json"))
                .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(target_os = "android")]
fn android_headers_from_tree_uri(
    app: &AppHandle,
    tree_uri: &str,
) -> anyhow::Result<Vec<SnapshotCategoryHeader>> {
    let picker = crate::folder_picker::folder_picker(app).map_err(|e| anyhow::anyhow!(e.err_message))?;
    let files = picker
        .list_snapshot_files(tree_uri)
        .map_err(|e| anyhow::anyhow!(e.err_message))?;

    let mut best: HashMap<String, (i64, SnapshotCategoryHeader)> = HashMap::new();
    for f in files {
        let key = f
            .cate_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| f.label.clone());
        let mtime = f.modified_ms.unwrap_or(0);
        let header = SnapshotCategoryHeader {
            cate_id: f.cate_id,
            label: f.label.clone(),
            file_path: f.file_path,
            total_count: f.total_count,
            saved_at: f.saved_at,
            modified_ms: f.modified_ms,
        };
        match best.get(&key) {
            Some((prev_mtime, _)) if *prev_mtime >= mtime => {}
            _ => {
                best.insert(key, (mtime, header));
            }
        }
    }

    let mut headers: Vec<_> = best.into_values().map(|(_, h)| h).collect();
    headers.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(headers)
}

/// 僅掃描檔頭 meta，不載入 comics。
pub fn list_category_headers(
    app: &AppHandle,
    category_dir: &str,
) -> anyhow::Result<Vec<SnapshotCategoryHeader>> {
    #[cfg(target_os = "android")]
    if category_dir.starts_with("content://") {
        return android_headers_from_tree_uri(app, category_dir);
    }

    let root = PathBuf::from(category_dir);
    if !root.is_dir() {
        anyhow::bail!("分類目錄不存在");
    }

    let mut files = Vec::new();
    collect_snapshot_files(&root, &mut files)?;

    let mut best: HashMap<String, (i64, SnapshotCategoryHeader)> = HashMap::new();

    for path in files {
        let summary = match crate::snapshot_storage::read_meta_summary_from_path(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let key = summary
            .cate_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| summary.label.clone());
        let mtime = file_modified_ms(&path);
        let header = SnapshotCategoryHeader {
            cate_id: summary.cate_id,
            label: summary.label.clone(),
            file_path: path.to_string_lossy().to_string(),
            total_count: summary.total_count,
            saved_at: summary.saved_at,
            modified_ms: Some(mtime),
        };
        match best.get(&key) {
            Some((prev_mtime, _)) if *prev_mtime >= mtime => {}
            _ => {
                best.insert(key, (mtime, header));
            }
        }
    }

    let mut headers: Vec<SnapshotCategoryHeader> = best.into_values().map(|(_, h)| h).collect();
    headers.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(headers)
}

pub fn load_snapshot_from_local_path(
    path: &Path,
) -> anyhow::Result<(SnapshotMeta, Vec<crate::types::ComicInSearch>)> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("讀取快照檔失敗：{}", path.display()))?;
    parse_snapshot_bytes(&bytes)
}

fn parse_snapshot_bytes(bytes: &[u8]) -> anyhow::Result<(SnapshotMeta, Vec<crate::types::ComicInSearch>)> {
    let file = crate::snapshot_export::parse_snapshot_export_bytes(bytes)?;
    Ok((file.snapshot.meta, file.snapshot.comics))
}

pub fn load_snapshot_for_search(
    app: &AppHandle,
    path: &str,
) -> anyhow::Result<(SnapshotMeta, Vec<crate::types::ComicInSearch>)> {
    #[cfg(target_os = "android")]
    if path.starts_with("content://") {
        let picker =
            crate::folder_picker::folder_picker(app).map_err(|e| anyhow::anyhow!(e.err_message))?;
        let cache_path = picker
            .cache_document_to_file(path)
            .map_err(|e| anyhow::anyhow!(e.err_message))?;
        let result = load_snapshot_from_local_path(Path::new(&cache_path));
        let _ = std::fs::remove_file(&cache_path);
        return result;
    }

    load_snapshot_from_local_path(Path::new(path))
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotResumeCandidate {
    pub cate_id: Option<i64>,
    pub label: String,
    pub scan_target_kind: Option<String>,
    pub file_path: String,
    pub meta_id: String,
    pub saved_at: String,
    pub modified_ms: Option<i64>,
    pub total_count: i64,
    pub total_pages: i64,
    pub scan_completion_percent: i64,
    pub scan_completed_pages: i64,
}

fn resume_candidate_key(cate_id: Option<i64>, label: &str, scan_target_kind: Option<&str>) -> String {
    if scan_target_kind == Some("albums") {
        return format!("albums:{label}");
    }
    cate_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| label.to_string())
}

fn header_to_candidate(
    cate_id: Option<i64>,
    label: String,
    file_path: String,
    summary: crate::snapshot_storage::MetaSummary,
) -> SnapshotResumeCandidate {
    SnapshotResumeCandidate {
        cate_id: cate_id.or(summary.cate_id),
        label: if summary.label.is_empty() {
            label
        } else {
            summary.label
        },
        scan_target_kind: summary.scan_target_kind.clone(),
        file_path,
        meta_id: summary.meta_id,
        saved_at: summary.saved_at,
        modified_ms: None,
        total_count: summary.total_count,
        total_pages: summary.total_pages,
        scan_completion_percent: summary.scan_completion_percent,
        scan_completed_pages: summary.scan_completed_pages,
    }
}

/// 列出可接續／更新的分類快照（每個分類只保留最新一筆）。
pub fn list_resume_candidates(
    app: &AppHandle,
    category_dir: &str,
) -> anyhow::Result<Vec<SnapshotResumeCandidate>> {
    #[cfg(target_os = "android")]
    if category_dir.starts_with("content://") {
        let picker =
            crate::folder_picker::folder_picker(app).map_err(|e| anyhow::anyhow!(e.err_message))?;
        let files = picker
            .list_snapshot_files(category_dir)
            .map_err(|e| anyhow::anyhow!(e.err_message))?;
        let mut best: HashMap<String, (i64, SnapshotResumeCandidate)> = HashMap::new();
        for f in files {
            let summary = crate::snapshot_storage::MetaSummary {
                meta_id: f.meta_id.clone(),
                saved_at: f.saved_at.clone(),
                total_count: f.total_count,
                total_pages: f.total_pages,
                scan_completion_percent: f.scan_completion_percent,
                scan_completed_pages: f.scan_completed_pages,
                cate_id: f.cate_id,
                label: f.label.clone(),
                scan_target_kind: f.scan_target_kind.clone(),
            };
            let key = resume_candidate_key(f.cate_id, f.label.as_str(), f.scan_target_kind.as_deref());
            let mtime = f.modified_ms.unwrap_or(0);
            let mut candidate = header_to_candidate(f.cate_id, f.label, f.file_path, summary);
            candidate.modified_ms = f.modified_ms;
            match best.get(&key) {
                Some((prev_mtime, _)) if *prev_mtime >= mtime => {}
                _ => {
                    best.insert(key, (mtime, candidate));
                }
            }
        }
        let mut out: Vec<_> = best.into_values().map(|(_, c)| c).collect();
        out.sort_by(|a, b| a.label.cmp(&b.label));
        return Ok(out);
    }

    let root = PathBuf::from(category_dir);
    if !root.is_dir() {
        anyhow::bail!("分類目錄不存在");
    }
    let mut files = Vec::new();
    collect_snapshot_files(&root, &mut files)?;
    let mut best: HashMap<String, (i64, SnapshotResumeCandidate)> = HashMap::new();
    for path in files {
        let summary = match crate::snapshot_storage::read_meta_summary_from_path(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let key = resume_candidate_key(
            summary.cate_id,
            summary.label.as_str(),
            summary.scan_target_kind.as_deref(),
        );
        let mtime = file_modified_ms(&path);
        let label = summary.label.clone();
        let mut candidate = header_to_candidate(
            summary.cate_id,
            label,
            path.to_string_lossy().to_string(),
            summary,
        );
        candidate.modified_ms = Some(mtime);
        match best.get(&key) {
            Some((prev_mtime, _)) if *prev_mtime >= mtime => {}
            _ => {
                best.insert(key, (mtime, candidate));
            }
        }
    }
    let mut out: Vec<_> = best.into_values().map(|(_, c)| c).collect();
    out.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(out)
}

pub fn filter_comics(
    comics: &[crate::types::ComicInSearch],
    keyword: &str,
) -> Vec<crate::types::ComicInSearch> {
    let kw = keyword.trim().to_lowercase();
    if kw.is_empty() {
        return comics.to_vec();
    }
    comics
        .iter()
        .filter(|c| {
            c.title().to_lowercase().contains(&kw)
                || c.id().to_string().contains(&kw)
                || c.title_html().to_lowercase().contains(&kw)
        })
        .cloned()
        .collect()
}
