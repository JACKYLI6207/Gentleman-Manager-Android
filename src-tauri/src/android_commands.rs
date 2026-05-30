use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

use crate::errors::{CommandError, CommandResult};
use crate::extensions::AppHandleExt;
use crate::mobile_settings::MobileSettings;
use crate::snapshot_catalog::{self, SnapshotCategoryHeader, SnapshotResumeCandidate};
use crate::snapshot_scan_session::{
    self, SnapshotScanBeginResult, SnapshotScanFinalizeInput, SnapshotScanMergeResult,
    SnapshotScanUpdatePageResult,
};
use crate::snapshot_storage;

#[cfg(target_os = "android")]
use crate::folder_picker::folder_picker;

#[cfg(not(target_os = "android"))]
async fn pick_folder_path(app: &AppHandle) -> CommandResult<Option<String>> {
    let (tx, rx) = oneshot::channel();
    app.dialog()
        .file()
        .pick_folder(move |path| {
            let _ = tx.send(path);
        });
    let picked = rx
        .await
        .map_err(|_| CommandError::from("選擇資料夾失敗", anyhow::anyhow!("對話框已關閉")))?;
    Ok(picked.map(|p| p.to_string()))
}

#[cfg(target_os = "android")]
async fn pick_folder_path(app: &AppHandle) -> CommandResult<Option<String>> {
    let picker = folder_picker(app)?;
    Ok(picker.pick_document_tree()?)
}

#[tauri::command]
#[specta::specta]
pub fn get_mobile_settings(app: AppHandle) -> CommandResult<MobileSettings> {
    MobileSettings::load(&app).map_err(|e| CommandError::from("讀取設定失敗", e))
}

#[tauri::command]
#[specta::specta]
pub fn save_mobile_settings(app: AppHandle, settings: MobileSettings) -> CommandResult<()> {
    settings
        .save(&app)
        .map_err(|e| CommandError::from("儲存設定失敗", e))
}

#[tauri::command]
#[specta::specta]
pub async fn pick_category_directory(app: AppHandle) -> CommandResult<Option<String>> {
    let Some(path) = pick_folder_path(&app).await? else {
        return Ok(None);
    };
    let mut settings = MobileSettings::load(&app).map_err(|e| CommandError::from("讀取設定失敗", e))?;
    settings.category_directory = Some(path.clone());
    settings
        .save(&app)
        .map_err(|e| CommandError::from("儲存設定失敗", e))?;
    Ok(Some(path))
}

#[tauri::command]
#[specta::specta]
pub async fn pick_import_archive_file(app: AppHandle) -> CommandResult<Option<String>> {
    #[cfg(target_os = "android")]
    {
        let picker = folder_picker(&app)?;
        return Ok(picker.pick_open_txt()?);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err(CommandError::from(
            "不支援的平台",
            anyhow::anyhow!("僅 Android 可選擇存檔檔案"),
        ))
    }
}

#[tauri::command]
#[specta::specta]
pub fn read_import_archive_file(app: AppHandle, path: String) -> CommandResult<String> {
    #[cfg(target_os = "android")]
    if path.starts_with("content://") {
        let picker = folder_picker(&app)?;
        return picker.read_text(&path);
    }
    std::fs::read_to_string(&path).map_err(|err| {
        CommandError::from(
            "讀取存檔失敗",
            anyhow::anyhow!("讀取存檔`{path}`失敗: {err}"),
        )
    })
}

#[tauri::command]
#[specta::specta]
pub async fn pick_korean_txt_file(app: AppHandle) -> CommandResult<Option<String>> {
    #[cfg(target_os = "android")]
    {
        let picker = folder_picker(&app)?;
        let Some(uri) = picker.pick_open_txt()? else {
            return Ok(None);
        };
        {
            let config_state = app.state::<parking_lot::RwLock<crate::config::Config>>();
            let mut config = config_state.write();
            config.korean_txt_catalog_dir = std::path::PathBuf::from(&uri);
            let _ = config.save(&app);
        }
        return Ok(Some(uri));
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err(CommandError::from(
            "不支援的平台",
            anyhow::anyhow!("僅 Android 可選擇 TXT 檔案"),
        ))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn pick_download_directory(app: AppHandle) -> CommandResult<Option<String>> {
    let Some(path) = pick_folder_path(&app).await? else {
        return Ok(None);
    };
    #[cfg(target_os = "android")]
    {
        let picker = folder_picker(&app)?;
        let writable = picker.probe_tree_writable(&path)?;
        if !writable {
            return Err(CommandError::from(
                "下載目錄不可寫",
                anyhow::anyhow!("請改選可寫入目錄（目前目錄僅可讀取）"),
            ));
        }
    }
    let mut settings = MobileSettings::load(&app).map_err(|e| CommandError::from("讀取設定失敗", e))?;
    settings.download_directory = Some(path.clone());
    {
        let config_state = app.state::<parking_lot::RwLock<crate::config::Config>>();
        let mut config = config_state.write();
        config.download_dir = std::path::PathBuf::from(&path);
        let _ = config.save(&app);
    }
    settings
        .save(&app)
        .map_err(|e| CommandError::from("儲存設定失敗", e))?;
    Ok(Some(path))
}

#[tauri::command]
#[specta::specta]
pub fn pick_local_reader_zip(app: AppHandle) -> CommandResult<Option<String>> {
    #[cfg(target_os = "android")]
    {
        let picker = folder_picker(&app)?;
        return picker.pick_open_zip();
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(None)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn pick_local_reader_folder(app: AppHandle) -> CommandResult<Option<String>> {
    #[cfg(target_os = "android")]
    {
        let picker = folder_picker(&app)?;
        return picker.pick_document_tree();
    }
    #[cfg(not(target_os = "android"))]
    {
        Ok(pick_folder_path(&app).await?)
    }
}

#[tauri::command]
#[specta::specta]
pub fn list_snapshot_category_headers(
    app: AppHandle,
    category_directory: String,
) -> CommandResult<Vec<SnapshotCategoryHeader>> {
    snapshot_catalog::list_category_headers(&app, &category_directory)
        .map_err(|e| CommandError::from("讀取分類目錄失敗", e))
}

#[tauri::command]
#[specta::specta]
pub fn list_snapshot_resume_candidates(
    app: AppHandle,
    category_directory: String,
) -> CommandResult<Vec<SnapshotResumeCandidate>> {
    snapshot_catalog::list_resume_candidates(&app, &category_directory)
        .map_err(|e| CommandError::from("列出可更新快照失敗", e))
}

#[tauri::command(async)]
#[specta::specta]
pub async fn snapshot_scan_begin(
    app: AppHandle,
    file_path: Option<String>,
    cate_id: i64,
    label: String,
    total_pages: i64,
    scan_target_kind: String,
    existing_load_mode: String,
) -> CommandResult<SnapshotScanBeginResult> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        snapshot_scan_session::begin_session(
            &app,
            file_path.as_deref(),
            cate_id,
            &label,
            total_pages,
            &scan_target_kind,
            &existing_load_mode,
        )
    })
    .await
    .map_err(|e| CommandError::from("快照掃描初始化失敗", e))?
    .map_err(|e| CommandError::from("快照掃描初始化失敗", e))
}

#[tauri::command(async)]
#[specta::specta]
pub async fn snapshot_scan_merge_batch(
    session_id: String,
    comics: Vec<crate::types::ComicInSearch>,
) -> CommandResult<SnapshotScanMergeResult> {
    tokio::task::spawn_blocking(move || snapshot_scan_session::merge_batch(&session_id, comics))
        .await
        .map_err(|e| CommandError::from("合併快照資料失敗", e))?
        .map_err(|e| CommandError::from("合併快照資料失敗", e))
}

#[tauri::command(async)]
#[specta::specta]
pub async fn snapshot_scan_apply_update_page(
    app: AppHandle,
    session_id: String,
    comics: Vec<crate::types::ComicInSearch>,
    cumulative_duplicate_hits: i64,
) -> CommandResult<SnapshotScanUpdatePageResult> {
    let duplicate_stop_threshold = app
        .get_config()
        .read()
        .snapshot_update_duplicate_stop_limit();
    tokio::task::spawn_blocking(move || {
        snapshot_scan_session::apply_update_page(
            &session_id,
            comics,
            cumulative_duplicate_hits,
            duplicate_stop_threshold,
        )
    })
    .await
    .map_err(|e| CommandError::from("更新快照頁面失敗", e))?
    .map_err(|e| CommandError::from("更新快照頁面失敗", e))
}

#[tauri::command(async)]
#[specta::specta]
pub async fn snapshot_scan_finalize(
    app: AppHandle,
    input: SnapshotScanFinalizeInput,
) -> CommandResult<String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || snapshot_scan_session::finalize_session(&app, input))
        .await
        .map_err(|e| CommandError::from("儲存快照失敗", e))?
        .map_err(|e| CommandError::from("儲存快照失敗", e))
}

#[tauri::command]
#[specta::specta]
pub fn snapshot_scan_dispose(session_id: String) {
    snapshot_scan_session::dispose_session(&session_id);
}

#[tauri::command]
#[specta::specta]
pub fn write_category_snapshot_file(
    app: AppHandle,
    category_directory: String,
    file_name: String,
    content: String,
) -> CommandResult<String> {
    #[cfg(target_os = "android")]
    if category_directory.starts_with("content://") {
        return snapshot_storage::write_snapshot_to_tree(&app, &category_directory, &file_name, &content)
            .map_err(|e| CommandError::from("寫入快照檔失敗", e));
    }

    let dir = std::path::PathBuf::from(&category_directory);
    snapshot_storage::write_snapshot_to_directory(&dir, &file_name, &content)
        .map_err(|e| CommandError::from("寫入快照檔失敗", e))
}

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSearchResult {
    pub meta: snapshot_catalog::SnapshotMeta,
    pub comics: Vec<crate::types::ComicInSearch>,
    pub total: usize,
}

#[tauri::command(async)]
#[specta::specta]
pub async fn search_snapshot_file(
    app: AppHandle,
    file_path: String,
    keyword: String,
) -> CommandResult<SnapshotSearchResult> {
    let keyword = keyword.trim().to_string();
    let app = app.clone();
    let (meta, comics) = tokio::task::spawn_blocking(move || {
        snapshot_catalog::load_snapshot_for_search(&app, &file_path)
    })
    .await
    .map_err(|e| CommandError::from("載入快照失敗", e))?
    .map_err(|e| CommandError::from("載入快照失敗", e))?;
    let filtered = snapshot_catalog::filter_comics(&comics, &keyword);
    let total = filtered.len();
    Ok(SnapshotSearchResult {
        meta,
        comics: filtered,
        total,
    })
}
