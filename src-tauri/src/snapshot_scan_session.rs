use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::Context;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::{
    snapshot_catalog::{self, SnapshotMeta},
    snapshot_large::{self, LARGE_SNAPSHOT_THRESHOLD},
    snapshot_storage,
    types::ComicInSearch,
    utils,
};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

struct SnapshotScanSession {
    meta: SnapshotMeta,
    comics: HashMap<i64, ComicInSearch>,
    label: String,
    cate_id: i64,
    scan_target_kind: String,
    source_file_path: Option<String>,
    /// 大檔更新：僅載入舊 ID，漫畫本體留在磁碟串流合併
    existing_ids: Option<HashSet<i64>>,
    /// 更新掃描開始時快照內已有 ID（僅此集合計入「重複」停止條件，與 PC knownComicIds 相同）
    baseline_known_ids: HashSet<i64>,
    /// 更新前快照 `comics` 陣列長度（與 PC existing.comics.length 相同）
    original_file_comic_count: i64,
    source_cache_path: Option<PathBuf>,
}

static SESSIONS: Mutex<Option<HashMap<String, SnapshotScanSession>>> = Mutex::new(None);

fn sessions_mut() -> parking_lot::MutexGuard<'static, Option<HashMap<String, SnapshotScanSession>>> {
    let mut guard = SESSIONS.lock();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn remove_session(id: &str) -> Option<SnapshotScanSession> {
    sessions_mut().as_mut()?.remove(id)
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotScanBeginResult {
    pub session_id: String,
    pub existing_count: i64,
    pub meta: SnapshotMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotScanMergeResult {
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotScanUpdatePageResult {
    pub duplicate_hits: i64,
    pub added_count: i64,
    pub total_count: i64,
    pub should_stop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotScanFinalizeInput {
    pub session_id: String,
    pub category_directory: String,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub previous_snapshot_path: Option<String>,
    pub total_pages: i64,
    pub scan_completion_percent: i64,
    pub scan_completed_pages: i64,
    pub scan_completed_page_ranges: Vec<PageRange>,
}

pub use crate::snapshot_catalog::SnapshotPageRange as PageRange;

pub fn begin_session(
    app: &AppHandle,
    file_path: Option<&str>,
    cate_id: i64,
    label: &str,
    total_pages: i64,
    scan_target_kind: &str,
    existing_load_mode: &str,
) -> anyhow::Result<SnapshotScanBeginResult> {
    let is_albums = scan_target_kind == "albums";
    let mut comics: HashMap<i64, ComicInSearch> = HashMap::new();
    let mut meta = SnapshotMeta {
        id: uuid::Uuid::new_v4().to_string(),
        saved_at: now_iso_timestamp(),
        total_count: 0,
        total_pages,
        scan_completion_percent: 0,
        scan_direction: Some("tailToHead".to_string()),
        scan_completed_pages: 0,
        scan_completed_page_ranges: Vec::new(),
        scan_target_kind: Some(scan_target_kind.to_string()),
        scan_target_cate_id: if is_albums || cate_id < 0 {
            None
        } else {
            Some(cate_id)
        },
        scan_target_label: Some(label.to_string()),
    };

    let mut existing_ids: Option<HashSet<i64>> = None;
    let mut source_cache_path: Option<PathBuf> = None;
    let mut original_file_comic_count = 0_i64;
    let source_file_path = file_path
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string);

    if let Some(path) = source_file_path.as_deref() {
        let (cache_path, size) = snapshot_large::prepare_snapshot_cache(app, path)?;
        let ids_only = existing_load_mode == "idsOnly";

        if ids_only {
            let summary = snapshot_storage::read_meta_summary_from_path(&cache_path)?;
            meta = snapshot_meta_from_summary(&summary, total_pages);
            source_cache_path = Some(cache_path.clone());

            match snapshot_catalog::load_snapshot_from_local_path(&cache_path) {
                Ok((loaded_meta, loaded_comics)) => {
                    let file_count = loaded_comics.len() as i64;
                    original_file_comic_count = if loaded_meta.total_count > 0 {
                        loaded_meta.total_count
                    } else {
                        file_count
                    };
                    if summary.total_count > 0 {
                        original_file_comic_count = original_file_comic_count.max(summary.total_count);
                    }
                    meta.total_count = file_count;
                    let ids: HashSet<i64> = loaded_comics.iter().map(|c| c.id()).collect();
                    existing_ids = Some(ids);
                }
                Err(err) => {
                    tracing::warn!(error = %err, "完整載入快照失敗，改以串流解析漫畫項目");
                    let (ids, object_count) =
                        snapshot_large::stream_parsed_comic_baseline(&cache_path)
                            .with_context(|| format!("讀取快照漫畫 ID 失敗（約 {size} 位元組）"))?;
                    original_file_comic_count = if summary.total_count > 0 {
                        summary.total_count
                    } else {
                        object_count as i64
                    };
                    meta.total_count = ids.len() as i64;
                    existing_ids = Some(ids);
                }
            }
        } else {
            if size >= LARGE_SNAPSHOT_THRESHOLD {
                let summary = snapshot_storage::read_meta_summary_from_path(&cache_path)?;
                let incomplete = summary.total_pages > 0
                    && summary.scan_completed_pages > 0
                    && summary.scan_completed_pages < summary.total_pages;
                snapshot_large::remove_cache_if_temp(&cache_path, Some(path));
                let mb = size as f64 / 1024.0 / 1024.0;
                if incomplete {
                    anyhow::bail!(
                        "快照約 {mb:.0} MB，手機無法接續未完成的掃描（已掃 {}/{} 頁）。請在 PC 版接續後再更新。",
                        summary.scan_completed_pages,
                        summary.total_pages
                    );
                }
                anyhow::bail!(
                    "快照約 {mb:.0} MB，手機無法接續掃描。請在 PC 版接續或完成後再以「更新」掃描。"
                );
            }
            let (loaded_meta, loaded_comics) =
                snapshot_catalog::load_snapshot_from_local_path(&cache_path)?;
            meta = loaded_meta;
            for comic in loaded_comics {
                comics.insert(comic.id(), comic);
            }
            snapshot_large::remove_cache_if_temp(&cache_path, Some(path));
        }
    }

    let session_id = format!("snap-{}", NEXT_SESSION.fetch_add(1, Ordering::Relaxed));
    let baseline_known_ids: HashSet<i64> = existing_ids
        .as_ref()
        .cloned()
        .unwrap_or_else(|| comics.keys().copied().collect());
    if original_file_comic_count <= 0 {
        original_file_comic_count = baseline_known_ids.len() as i64;
    }
    let existing_count = original_file_comic_count;
    let session = SnapshotScanSession {
        meta: meta.clone(),
        comics,
        label: label.to_string(),
        cate_id,
        scan_target_kind: scan_target_kind.to_string(),
        source_file_path,
        existing_ids,
        baseline_known_ids,
        original_file_comic_count,
        source_cache_path,
    };
    sessions_mut()
        .as_mut()
        .context("快照掃描工作階段未初始化")?
        .insert(session_id.clone(), session);

    Ok(SnapshotScanBeginResult {
        session_id,
        existing_count,
        meta,
    })
}

pub fn merge_batch(session_id: &str, batch: Vec<ComicInSearch>) -> anyhow::Result<SnapshotScanMergeResult> {
    let mut guard = sessions_mut();
    let map = guard.as_mut().context("快照掃描工作階段未初始化")?;
    let session = map
        .get_mut(session_id)
        .with_context(|| format!("找不到快照掃描工作階段：{session_id}"))?;
    for comic in batch {
        session.comics.insert(comic.id(), comic);
    }
    let total_count = session.original_file_comic_count + session.comics.len() as i64;
    Ok(SnapshotScanMergeResult { total_count })
}

pub fn apply_update_page(
    session_id: &str,
    comics: Vec<ComicInSearch>,
    cumulative_duplicate_hits: i64,
    duplicate_stop_threshold: i64,
) -> anyhow::Result<SnapshotScanUpdatePageResult> {
    let mut guard = sessions_mut();
    let map = guard.as_mut().context("快照掃描工作階段未初始化")?;
    let session = map
        .get_mut(session_id)
        .with_context(|| format!("找不到快照掃描工作階段：{session_id}"))?;

    let mut duplicate_hits = cumulative_duplicate_hits;
    let mut added_count = 0_i64;
    for comic in comics {
        let id = comic.id();
        if session.baseline_known_ids.contains(&id) {
            duplicate_hits += 1;
            continue;
        }
        if session.comics.contains_key(&id) {
            continue;
        }
        session.comics.insert(id, comic);
        added_count += 1;
    }

    let total_count = session.original_file_comic_count + session.comics.len() as i64;

    Ok(SnapshotScanUpdatePageResult {
        duplicate_hits,
        added_count,
        total_count,
        should_stop: duplicate_hits > duplicate_stop_threshold,
    })
}

pub fn dispose_session(session_id: &str) {
    if let Some(map) = sessions_mut().as_mut() {
        if let Some(session) = map.remove(session_id) {
            if let Some(cache) = session.source_cache_path {
                snapshot_large::remove_cache_if_temp(
                    &cache,
                    session.source_file_path.as_deref(),
                );
            }
        }
    }
}

pub fn finalize_session(app: &AppHandle, input: SnapshotScanFinalizeInput) -> anyhow::Result<String> {
    let session = remove_session(&input.session_id)
        .with_context(|| format!("找不到快照掃描工作階段：{}", input.session_id))?;

    let mut meta = session.meta;
    meta.saved_at = now_iso_timestamp();
    meta.total_pages = input.total_pages;
    meta.scan_completion_percent = input.scan_completion_percent;
    meta.scan_completed_pages = input.scan_completed_pages;
    meta.scan_completed_page_ranges = input.scan_completed_page_ranges;
    meta.scan_target_kind = Some(session.scan_target_kind.clone());
    meta.scan_target_cate_id = if session.scan_target_kind == "albums" || session.cate_id < 0 {
        None
    } else {
        Some(session.cate_id)
    };
    meta.scan_target_label = Some(session.label.clone());

    let app_data_dir = utils::app_data_dir(app)?;
    std::fs::create_dir_all(&app_data_dir)?;
    let temp_path = app_data_dir.join(format!(
        "gm-snapshot-export-{}.json",
        input.session_id.replace('/', "_")
    ));

    let exported_at = now_iso_timestamp();
    if let Some(source_cache) = session.source_cache_path.as_ref() {
        let new_comics: Vec<ComicInSearch> = session.comics.into_values().collect();
        snapshot_large::write_merged_snapshot_export(
            source_cache,
            &mut meta,
            &new_comics,
            session.original_file_comic_count,
            &temp_path,
            &exported_at,
        )?;
        snapshot_large::remove_cache_if_temp(
            source_cache,
            session.source_file_path.as_deref(),
        );
    } else {
        let mut comics: Vec<ComicInSearch> = session.comics.into_values().collect();
        snapshot_large::sort_comics_create_date_desc(&mut comics);
        meta.total_count = comics.len() as i64;
        write_export_temp_file(&temp_path, &meta, &comics, &exported_at)?;
    }

    let label_for_name = meta
        .scan_target_label
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(session.label.as_str());
    let file_name = input
        .file_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| snapshot_storage::export_snapshot_file_name(label_for_name));
    let previous_path = input
        .previous_snapshot_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or(session.source_file_path.as_deref());

    let result = snapshot_storage::publish_snapshot_file(
        app,
        &input.category_directory,
        &file_name,
        &temp_path,
        session.cate_id,
        previous_path,
    );

    let _ = std::fs::remove_file(&temp_path);
    result
}

fn snapshot_meta_from_summary(
    summary: &snapshot_storage::MetaSummary,
    total_pages: i64,
) -> SnapshotMeta {
    SnapshotMeta {
        id: if summary.meta_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            summary.meta_id.clone()
        },
        saved_at: summary.saved_at.clone(),
        total_count: summary.total_count,
        total_pages: if summary.total_pages > 0 {
            summary.total_pages
        } else {
            total_pages
        },
        scan_completion_percent: summary.scan_completion_percent,
        scan_direction: Some("tailToHead".to_string()),
        scan_completed_pages: summary.scan_completed_pages,
        scan_completed_page_ranges: Vec::new(),
        scan_target_kind: summary
            .scan_target_kind
            .clone()
            .or(Some("category".to_string())),
        scan_target_cate_id: summary.cate_id,
        scan_target_label: Some(summary.label.clone()),
    }
}

fn write_export_temp_file(
    path: &PathBuf,
    meta: &SnapshotMeta,
    comics: &[ComicInSearch],
    exported_at: &str,
) -> anyhow::Result<()> {
    crate::snapshot_export::write_global_snapshot_export(path, meta, comics, exported_at)
}

/// 與前端 `Date.toISOString()` 相容的 UTC 時間字串。
fn now_iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let days = total_secs / 86_400;
    let time_of_day = total_secs % 86_400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z"
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
