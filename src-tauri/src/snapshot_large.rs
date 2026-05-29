use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::Context;
use regex::Regex;
use tauri::AppHandle;

use crate::{
    snapshot_catalog::{self, SnapshotMeta},
    snapshot_export,
    types::ComicInSearch,
};

/// 接續掃描若快照超過此大小則拒絕（需完整載入漫畫本體）。更新掃描一律走 ID 掃描路徑。
pub const LARGE_SNAPSHOT_THRESHOLD: u64 = 32 * 1024 * 1024;

const COMICS_MARKER: &[u8] = b"\"comics\":";

fn comic_created_at_sort_key_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"創建於(\d{4}-\d{2}-\d{2})(?:\s+(\d{2}):(\d{2}):(\d{2}))?")
            .expect("comic created-at regex")
    })
}

fn comic_created_at_sort_key(additional_info: &str) -> String {
    let Some(caps) = comic_created_at_sort_key_re().captures(additional_info) else {
        return String::new();
    };
    let date = caps.get(1).map(|m| m.as_str()).unwrap_or("0000-00-00");
    let hour = caps.get(2).map(|m| m.as_str()).unwrap_or("00");
    let minute = caps.get(3).map(|m| m.as_str()).unwrap_or("00");
    let second = caps.get(4).map(|m| m.as_str()).unwrap_or("00");
    format!("{date} {hour}:{minute}:{second}")
}

/// 與 PC `sortSearchComics(..., 'createDateDesc')` 一致。
pub fn sort_comics_create_date_desc(comics: &mut [ComicInSearch]) {
    comics.sort_by(|a, b| {
        let ka = comic_created_at_sort_key(a.additional_info());
        let kb = comic_created_at_sort_key(b.additional_info());
        kb.cmp(&ka).then_with(|| b.id().cmp(&a.id()))
    });
}

fn comic_id_head_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#""id"\s*:\s*(-?\d+)"#).expect("comic id regex"))
}

/// 將 SAF URI 快取到本機，或回傳本機路徑與檔案大小。
pub fn prepare_snapshot_cache(app: &AppHandle, path: &str) -> anyhow::Result<(PathBuf, u64)> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("快照路徑為空");
    }

    #[cfg(target_os = "android")]
    if trimmed.starts_with("content://") {
        let picker = crate::folder_picker::folder_picker(app)
            .map_err(|e| anyhow::anyhow!(e.err_message))?;
        let cache = picker
            .cache_document_to_file(trimmed)
            .map_err(|e| anyhow::anyhow!(e.err_message))?;
        let cache_path = PathBuf::from(&cache);
        let size = std::fs::metadata(&cache_path)
            .with_context(|| format!("讀取快照快取大小失敗：{}", cache_path.display()))?
            .len();
        return Ok((cache_path, size));
    }

    let local = PathBuf::from(trimmed);
    let size = std::fs::metadata(&local)
        .with_context(|| format!("讀取快照檔大小失敗：{}", local.display()))?
        .len();
    Ok((local, size))
}

fn read_snapshot_file(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path).with_context(|| format!("開啟快照`{}`失敗", path.display()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .with_context(|| format!("讀取快照`{}`失敗", path.display()))?;
    Ok(data)
}

/// 回傳 `comics` 陣列開頭 `[` 的位元組索引。
fn find_comics_array_bracket(data: &[u8]) -> anyhow::Result<usize> {
    let pos = data
        .windows(COMICS_MARKER.len())
        .position(|w| w == COMICS_MARKER)
        .context("快照 JSON 找不到 comics 欄位")?;
    let mut i = pos + COMICS_MARKER.len();
    while i < data.len() && data[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= data.len() || data[i] != b'[' {
        anyhow::bail!("快照 JSON 中 comics 後方不是陣列");
    }
    Ok(i)
}

/// 掃描陣列內每個 `{...}` 頂層物件的位元組範圍（不依賴 serde 陣列串流）。
fn collect_comic_object_ranges(data: &[u8], array_bracket: usize) -> anyhow::Result<Vec<(usize, usize)>> {
    let mut ranges = Vec::new();
    let mut i = array_bracket + 1;
    let len = data.len();
    while i < len {
        while i < len && (data[i].is_ascii_whitespace() || data[i] == b',') {
            i += 1;
        }
        if i >= len {
            break;
        }
        if data[i] == b']' {
            break;
        }
        if data[i] != b'{' {
            anyhow::bail!(
                "comics 陣列格式異常（偏移 {i}，字元 0x{:02x}）",
                data[i]
            );
        }
        let start = i;
        let end = find_json_object_end(data, i)
            .with_context(|| format!("無法解析 comics 物件結尾（起始偏移 {start}）"))?;
        ranges.push((start, end));
        i = end;
    }
    Ok(ranges)
}

fn find_json_object_end(data: &[u8], start: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start;
    while i < data.len() {
        let c = data[i];
        if in_string {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else {
            match c {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + 1);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

fn comic_id_from_object_bytes(slice: &[u8]) -> Option<i64> {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(slice) {
        if let Some(id) = value.get("id").and_then(|v| v.as_i64()).filter(|&id| id > 0) {
            return Some(id);
        }
    }
    let head_len = slice.len().min(512);
    let head = String::from_utf8_lossy(&slice[..head_len]);
    comic_id_head_re()
        .captures(&head)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .filter(|&id| id > 0)
}

/// 1. 快取檔讀入記憶體 → 2. 括號深度掃描 comics 物件範圍 → 3. 僅從物件開頭讀 `"id":數字`。
pub fn stream_comic_ids(path: &Path) -> anyhow::Result<HashSet<i64>> {
    let data = read_snapshot_file(path)?;
    stream_comic_ids_from_bytes(&data)
}

pub fn stream_parsed_comic_baseline(path: &Path) -> anyhow::Result<(HashSet<i64>, usize)> {
    let data = read_snapshot_file(path)?;
    stream_parsed_comic_baseline_from_bytes(&data)
}

pub fn stream_comic_ids_from_bytes(data: &[u8]) -> anyhow::Result<HashSet<i64>> {
    let (ids, _) = stream_parsed_comic_baseline_from_bytes(data)?;
    Ok(ids)
}

/// 僅從可成功反序列化的漫畫物件建立 baseline（避免 regex 誤判 ID 導致提早停止、漏掃新本）。
pub fn stream_parsed_comic_baseline_from_bytes(
    data: &[u8],
) -> anyhow::Result<(HashSet<i64>, usize)> {
    let bracket = find_comics_array_bracket(data)?;
    let ranges = collect_comic_object_ranges(data, bracket)?;
    let object_count = ranges.len();
    let mut ids = HashSet::with_capacity(object_count);
    for (start, end) in ranges {
        match serde_json::from_slice::<ComicInSearch>(&data[start..end]) {
            Ok(comic) if comic.id() > 0 => {
                ids.insert(comic.id());
            }
            Ok(_) => {}
            Err(err) => {
                tracing::trace!(error = %err, start, "快照漫畫項目略過（無法解析）");
            }
        }
    }
    if ids.is_empty() && object_count > 0 {
        anyhow::bail!("comics 陣列內未解析到任何漫畫 ID");
    }
    Ok((ids, object_count))
}

/// 與 PC `mergeComicsInto` 相同：保留全部舊項目，僅 append 新 ID。
fn merge_comics_like_pc(mut existing: Vec<ComicInSearch>, batch: &[ComicInSearch]) -> Vec<ComicInSearch> {
    let mut seen: HashSet<i64> = existing.iter().map(|c| c.id()).collect();
    for comic in batch {
        if comic.id() <= 0 || seen.contains(&comic.id()) {
            continue;
        }
        seen.insert(comic.id());
        existing.push(comic.clone());
    }
    existing
}

/// 與 PC `upsertGlobalSnapshot` 相同：完整讀取舊快照陣列後 append 新漫畫。
pub fn write_merged_snapshot_export(
    source_path: &Path,
    meta: &mut SnapshotMeta,
    new_comics: &[ComicInSearch],
    original_file_comic_count: i64,
    out_path: &Path,
    exported_at: &str,
) -> anyhow::Result<()> {
    let (_, old_comics) = snapshot_catalog::load_snapshot_from_local_path(source_path)
        .with_context(|| format!("讀取舊快照失敗：{}", source_path.display()))?;

    let old_len = old_comics.len();
    if original_file_comic_count > 0 && old_len as i64 != original_file_comic_count {
        tracing::warn!(
            old_len,
            original_file_comic_count,
            "快照 meta.totalCount 與 comics 陣列長度不一致，以陣列長度為準"
        );
    }

    let seen_old: HashSet<i64> = old_comics.iter().map(|c| c.id()).collect();
    let unique_new = new_comics
        .iter()
        .filter(|c| c.id() > 0 && !seen_old.contains(&c.id()))
        .count();
    let expected = old_len + unique_new;
    let mut merged = merge_comics_like_pc(old_comics, new_comics);
    if merged.len() != expected {
        anyhow::bail!(
            "合併後 {} 本，與預期 {} 本不符（原 {} + 新增 {} 個唯一 ID）",
            merged.len(),
            expected,
            old_len,
            unique_new
        );
    }

    sort_comics_create_date_desc(&mut merged);
    meta.total_count = merged.len() as i64;
    snapshot_export::write_global_snapshot_export(out_path, meta, &merged, exported_at)
}

pub fn remove_cache_if_temp(path: &Path, source_uri: Option<&str>) {
    #[cfg(target_os = "android")]
    if let Some(uri) = source_uri {
        if uri.starts_with("content://") {
            let _ = std::fs::remove_file(path);
        }
    }
}
