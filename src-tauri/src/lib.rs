mod android_commands;
mod commands;
mod config;
mod download_manager;
mod download_task_store;
mod errors;
mod events;
mod extensions;
#[cfg(target_os = "android")]
mod folder_picker;
#[cfg(target_os = "android")]
mod lan_discovery;
mod korean_series_folder;
mod korean_txt_catalog;
mod local_reader;
mod logger;
mod mobile_settings;
mod pc_remote_discovery;
mod remote_pc_file_op;
mod remote_pc_transfer;
mod remote_pc_upload;
mod snapshot_catalog;
mod snapshot_export;
mod snapshot_large;
mod snapshot_scan_session;
mod snapshot_storage;
mod types;
mod utils;
mod wnacg_client;
mod zip_download;

use anyhow::Context;
use config::Config;
use download_manager::DownloadManager;
use events::{DownloadSleepingEvent, DownloadSpeedEvent, DownloadTaskEvent, LogEvent};
use remote_pc_transfer::RemoteTransferProgressEvent;
use mobile_settings::MobileSettings;
use parking_lot::RwLock;
use tauri::{Manager, Wry};
use wnacg_client::WnacgClient;

use crate::{
    android_commands::*,
    commands::*,
    events::{DownloadShelfEvent, SearchScanProgressEvent},
};

fn generate_context() -> tauri::Context<Wry> {
    tauri::generate_context!()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<Wry>::new()
        .commands(tauri_specta::collect_commands![
            get_mobile_settings,
            save_mobile_settings,
            pick_category_directory,
            pick_download_directory,
            pick_korean_txt_file,
            pick_import_archive_file,
            read_import_archive_file,
            pick_local_reader_zip,
            pick_local_reader_folder,
            list_snapshot_category_headers,
            list_snapshot_resume_candidates,
            write_category_snapshot_file,
            snapshot_scan_begin,
            snapshot_scan_merge_batch,
            snapshot_scan_apply_update_page,
            snapshot_scan_finalize,
            snapshot_scan_dispose,
            search_snapshot_file,
            cancel_scoped_search_scan,
            advance_scoped_search_scan,
            get_config,
            save_config,
            login,
            get_user_profile,
            search_by_keyword,
            search_by_tag,
            browse_by_category,
            browse_ranking,
            browse_albums_list,
            browse_home,
            get_comic,
            get_comic_tags,
            create_download_task,
            create_download_task_by_id,
            create_download_task_placeholder,
            prepare_korean_series_folder,
            list_similar_korean_series_folders,
            pause_download_task,
            resume_download_task,
            cancel_download_task,
            remove_download_task_record,
            get_download_task_snapshots,
            get_downloaded_comics,
            get_cover_data,
            get_reader_image,
            list_local_reader_sources,
            prepare_local_reader_zip,
            load_local_reader_pages,
            get_local_reader_image,
            close_local_reader_zip_session,
            read_snapshot_export_file,
            read_korean_txt_catalog,
            scan_lan_remote_pcs,
            enter_remote_wifi_mode,
            leave_remote_wifi_mode,
            test_remote_pc_connection,
            list_remote_pc_directory,
            pick_remote_transfer_destination,
            transfer_remote_pc_files,
            cancel_remote_pc_transfer,
            pick_remote_upload_file,
            pick_remote_upload_folder,
            plan_remote_pc_upload,
            upload_remote_pc_files,
            remote_pc_file_op,
        ])
        .events(tauri_specta::collect_events![
            LogEvent,
            DownloadTaskEvent,
            DownloadSpeedEvent,
            DownloadSleepingEvent,
            DownloadShelfEvent,
            SearchScanProgressEvent,
            RemoteTransferProgressEvent,
        ]);

    let mut tauri_builder = tauri::Builder::default();
    #[cfg(target_os = "android")]
    {
        tauri_builder = tauri_builder
            .plugin(crate::folder_picker::init())
            .plugin(crate::lan_discovery::init());
    }
    tauri_builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let app_data_dir =
                crate::utils::app_data_dir(app.handle()).context("獲取 app_data_dir 失敗")?;
            std::fs::create_dir_all(&app_data_dir).context("創建 app_data_dir 失敗")?;

            let mut config = Config::new(app.handle())?;
            if let Ok(mobile) = MobileSettings::load(app.handle()) {
                if let Some(dir) = mobile.download_directory.as_ref() {
                    config.download_dir = std::path::PathBuf::from(dir);
                }
            }
            app.manage(RwLock::new(config));

            let wnacg_client = WnacgClient::new(app.handle().clone());
            app.manage(wnacg_client);

            let download_manager = DownloadManager::new(app.handle());
            app.manage(download_manager);

            let _ = logger::init(app.handle());

            Ok(())
        })
        .run(generate_context())
        .expect("error while running tauri application");
}
