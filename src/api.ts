import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export type DownloadFormat = 'JpegZipPack' | 'Server2Zip'
export type DownloadTaskState =
  | 'Pending'
  | 'Downloading'
  | 'Paused'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'

export type DownloadTaskEvent = {
  state: DownloadTaskState
  comic: Comic
  downloadedImgCount: number
  totalImgCount: number
  downloadPath?: string | null
  zipServer?: 'Server1' | 'Server2' | null
  downloadedBytes: number
  totalBytes: number
  seriesParentDir?: string | null
  lastError?: string | null
}

export type Config = {
  cookie: string
  downloadDir: string
  enableFileLogger: boolean
  downloadFormat: DownloadFormat
  proxyMode: 'System' | 'NoProxy' | 'Custom'
  proxyHost: string
  proxyPort: number
  comicConcurrency: number
  comicDownloadIntervalSec: number
  imgConcurrency: number
  imgDownloadIntervalSec: number
  downloadShelfIntervalMs: number
  batchDownloadIntervalMs: number
  useOriginalFilename: boolean
  apiDomainMode: 'Default' | 'Custom'
  customApiDomain: string
  downloadRetryCount: number
  downloadFailureRestSec: number
  koreanTxtCatalogDir: string
  koreanTxtDuplicateCheckEnabled: boolean
  /** 快照更新掃描：舊 ID 重複超過此數後才可能提早停止（預設 20） */
  snapshotUpdateDuplicateStopCount: number
}

export type ImgInImgList = {
  caption: string
  url: string
}

export type Comic = {
  id: number
  title: string
  cover: string
  category: string
  imageCount: number
  tags: Tag[]
  intro: string
  isDownloaded?: boolean | null
  imgList: ImgInImgList[]
}

export type DownloadTaskSeed = {
  id: number
  title: string
  cover?: string | null
  category?: string | null
  imageCount?: number | null
}

export type LocalReaderPage = {
  caption: string
  pageId: string
}

export type LocalReaderPages = {
  title: string
  pages: LocalReaderPage[]
}

export type LocalReaderSource = {
  path: string
  label: string
  kind: 'zip' | 'folder'
}

export type SearchScanProgressEvent = {
  current: number
  total: number
  matchedCount: number
  scanKind: string
  finished: boolean
  paused: boolean
  retryInSecs: number | null
  pausedReason: string | null
  cancelled: boolean
}

/** 監聽分類／關鍵詞掃描進度（與 PC 版相同事件） */
export async function listenSearchScanProgress(
  handler: (payload: SearchScanProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<SearchScanProgressEvent>('search-scan-progress-event', (ev) => {
    handler(ev.payload)
  })
}

export type MobileSettings = {
  categoryDirectory?: string | null
  downloadDirectory?: string | null
}

export type SnapshotCategoryHeader = {
  cateId?: number | null
  label: string
  filePath: string
  totalCount: number
  savedAt: string
  modifiedMs?: number | null
}

export type SnapshotResumeCandidate = {
  cateId?: number | null
  label: string
  scanTargetKind?: string | null
  filePath: string
  metaId: string
  savedAt: string
  modifiedMs?: number | null
  totalCount: number
  totalPages: number
  scanCompletionPercent: number
  scanCompletedPages: number
}

export type Tag = {
  name: string
  url: string
}


export type ComicInSearch = {
  id: number
  title: string
  titleHtml: string
  cover: string
  additionalInfo: string
  isDownloaded: boolean
  listCateId?: number | null
}

export type SearchResult = {
  comics: ComicInSearch[]
  currentPage: number
  totalPage: number
  totalCount: number
  isSearchByTag: boolean
}

export type SnapshotSearchResult = {
  meta: Record<string, unknown>
  comics: ComicInSearch[]
  total: number
}

export function getMobileSettings() {
  return invoke<MobileSettings>('get_mobile_settings')
}

export function pickCategoryDirectory(persist = true) {
  return invoke<string | null>('pick_category_directory', { persist })
}

export function pickDownloadDirectory() {
  return invoke<string | null>('pick_download_directory')
}

export function pickKoreanTxtFile() {
  return invoke<string | null>('pick_korean_txt_file')
}

export function pickImportArchiveFile() {
  return invoke<string | null>('pick_import_archive_file')
}

export function readImportArchiveFile(path: string) {
  return invoke<string>('read_import_archive_file', { path })
}

export function listSnapshotCategoryHeaders(categoryDirectory: string) {
  return invoke<SnapshotCategoryHeader[]>('list_snapshot_category_headers', { categoryDirectory })
}

export function listSnapshotResumeCandidates(categoryDirectory: string) {
  return invoke<SnapshotResumeCandidate[]>('list_snapshot_resume_candidates', { categoryDirectory })
}

export function writeCategorySnapshotFile(
  categoryDirectory: string,
  fileName: string,
  content: string,
) {
  return invoke<string>('write_category_snapshot_file', { categoryDirectory, fileName, content })
}

export type SnapshotScanBeginResult = {
  sessionId: string
  existingCount: number
  meta: Record<string, unknown>
}

/** 更新既有快照：僅讀 ID（大檔安全）；接續掃描：完整載入漫畫本體 */
export type SnapshotExistingLoadMode = 'idsOnly' | 'full'

export function snapshotScanBegin(
  filePath: string | null,
  cateId: number,
  label: string,
  totalPages: number,
  scanTargetKind: 'category' | 'albums',
  existingLoadMode: SnapshotExistingLoadMode,
) {
  return invoke<SnapshotScanBeginResult>('snapshot_scan_begin', {
    filePath,
    cateId,
    label,
    totalPages,
    scanTargetKind,
    existingLoadMode,
  })
}

export function snapshotScanMergeBatch(sessionId: string, comics: ComicInSearch[]) {
  return invoke<{ totalCount: number }>('snapshot_scan_merge_batch', { sessionId, comics })
}

export function snapshotScanApplyUpdatePage(
  sessionId: string,
  comics: ComicInSearch[],
  cumulativeDuplicateHits: number,
) {
  return invoke<{
    duplicateHits: number
    addedCount: number
    totalCount: number
    shouldStop: boolean
  }>('snapshot_scan_apply_update_page', { sessionId, comics, cumulativeDuplicateHits })
}

export function snapshotScanFinalize(input: {
  sessionId: string
  categoryDirectory: string
  fileName?: string
  previousSnapshotPath?: string | null
  totalPages: number
  scanCompletionPercent: number
  scanCompletedPages: number
  scanCompletedPageRanges: { start: number; end: number }[]
}) {
  return invoke<string>('snapshot_scan_finalize', { input })
}

export function snapshotScanDispose(sessionId: string) {
  return invoke<null>('snapshot_scan_dispose', { sessionId })
}

export function searchSnapshotFile(filePath: string, keyword: string) {
  return invoke<SnapshotSearchResult>('search_snapshot_file', { filePath, keyword })
}

export function searchByKeyword(keyword: string, pageNum: number, cateId?: number | null) {
  return invoke<SearchResult>('search_by_keyword', { keyword, pageNum, cateId: cateId ?? null })
}

export function searchByTag(tagName: string, pageNum: number, cateId?: number | null) {
  return invoke<SearchResult>('search_by_tag', { tagName, pageNum, cateId: cateId ?? null })
}

export function browseHome(pageNum: number) {
  return invoke<SearchResult>('browse_home', { pageNum })
}

export function browseAlbumsList(pageNum: number) {
  return invoke<SearchResult>('browse_albums_list', { pageNum })
}

export function browseByCategory(cateId: number, pageNum: number) {
  return invoke<SearchResult>('browse_by_category', { cateId, pageNum })
}

export function browseRanking(period: string, cateId: number | null, pageNum: number) {
  return invoke<SearchResult>('browse_ranking', { period, cateId, pageNum })
}

export function getComic(id: number) {
  return invoke<Comic>('get_comic', { id })
}

export async function createDownloadTask(seed: DownloadTaskSeed, seriesParentDir?: string | null) {
  return invoke<null>('create_download_task_placeholder', {
    comicId: seed.id,
    title: seed.title,
    cover: seed.cover ?? null,
    category: seed.category ?? null,
    imageCount: seed.imageCount ?? null,
    seriesParentDir: seriesParentDir ?? null,
  })
}

export function getConfig() {
  return invoke<Config>('get_config')
}

export function saveConfig(config: Config) {
  return invoke<null>('save_config', { config })
}

export function getDownloadTaskSnapshots() {
  return invoke<DownloadTaskEvent[]>('get_download_task_snapshots')
}

export function pauseDownloadTask(comicId: number) {
  return invoke<null>('pause_download_task', { comicId })
}

export function resumeDownloadTask(comicId: number) {
  return invoke<null>('resume_download_task', { comicId })
}

export function cancelDownloadTask(comicId: number) {
  return invoke<null>('cancel_download_task', { comicId })
}

export function removeDownloadTaskRecord(comicId: number) {
  return invoke<null>('remove_download_task_record', { comicId })
}

export function listSimilarKoreanSeriesFolders(
  seriesLabel: string,
  episodeStart: number,
  episodeEnd: number,
) {
  return invoke<string[]>('list_similar_korean_series_folders', {
    seriesLabel,
    episodeStart,
    episodeEnd,
  })
}

export function prepareKoreanSeriesFolder(
  seriesLabel: string,
  episodeStart: number,
  episodeEnd: number,
  existingFolderName?: string | null,
) {
  return invoke<string>('prepare_korean_series_folder', {
    seriesLabel,
    episodeStart,
    episodeEnd,
    existingFolderName: existingFolderName ?? null,
  })
}

export function getReaderImage(comicId: number, imgUrl: string) {
  return invoke<number[]>('get_reader_image', { comicId, imgUrl })
}

export function listLocalReaderSources(folderPath: string) {
  return invoke<LocalReaderSource[]>('list_local_reader_sources', { folderPath })
}

export function prepareLocalReaderZip(sourceUri: string) {
  return invoke<string>('prepare_local_reader_zip', { sourceUri })
}

export function loadLocalReaderPages(sourcePath: string, sourceKind?: 'zip' | 'folder') {
  return invoke<LocalReaderPages>('load_local_reader_pages', {
    sourcePath,
    sourceKind: sourceKind ?? null,
  })
}

export function pickLocalReaderZip() {
  return invoke<string | null>('pick_local_reader_zip')
}

export function pickLocalReaderFolder() {
  return invoke<string | null>('pick_local_reader_folder')
}

export function getLocalReaderImage(pageId: string) {
  return invoke<number[]>('get_local_reader_image', { pageId })
}

export function closeLocalReaderZipSession() {
  return invoke<null>('close_local_reader_zip_session')
}

export function readKoreanTxtCatalog(catalogDir: string) {
  return invoke<string[]>('read_korean_txt_catalog', { catalogDir })
}

export async function listenDownloadTaskEvent(
  handler: (payload: DownloadTaskEvent) => void,
): Promise<UnlistenFn> {
  return listen<DownloadTaskEvent>('download-task-event', (ev) => handler(ev.payload))
}

export type DiscoveredRemotePc = {
  name: string
  hosts: string[]
  port: number
}

export type RemotePcConnectionResult = {
  connected: boolean
  message: string
  connectedHost: string | null
}

export type RemotePcListItem = DiscoveredRemotePc & {
  connected: boolean | null
  message: string
  connectedHost: string | null
}

export type RemotePcDirEntry = {
  name: string
  isDir: boolean
  size: number | null
}

export type RemotePcBrowseResult = {
  path: string
  entries: RemotePcDirEntry[]
}

export type RemotePcScanResult = {
  pcs: DiscoveredRemotePc[]
  log: string
}

export function scanLanRemotePcs() {
  return invoke<RemotePcScanResult>('scan_lan_remote_pcs')
}

export function enterRemoteWifiMode() {
  return invoke<string>('enter_remote_wifi_mode')
}

export function leaveRemoteWifiMode() {
  return invoke<void>('leave_remote_wifi_mode')
}

export function testRemotePcConnection(hosts: string[], port: number) {
  return invoke<RemotePcConnectionResult>('test_remote_pc_connection', { hosts, port })
}

export function listRemotePcDirectory(host: string, port: number, path: string) {
  return invoke<RemotePcBrowseResult>('list_remote_pc_directory', { host, port, path })
}

export type RemotePcFileItem = {
  relativePath: string
  size: number
}

export type RemoteTransferFailedItem = {
  path: string
  error: string
}

export type RemoteTransferProgressEvent = {
  phase: string
  fileIndex: number
  fileCount: number
  bytesDone: number
  bytesTotal: number
  speedBps: number
  message: string
  finished: boolean
  error: string | null
  detailLog?: string | null
  succeededPaths?: string[] | null
  failedItems?: RemoteTransferFailedItem[] | null
}

export function pickRemoteTransferDestination() {
  return invoke<string | null>('pick_remote_transfer_destination')
}

export type RemotePcTransferSelection = {
  path: string
  anchorPath: string
}

export function transferRemotePcFiles(
  host: string,
  port: number,
  selections: RemotePcTransferSelection[],
  destTreeUri: string,
) {
  return invoke<null>('transfer_remote_pc_files', { host, port, selections, destTreeUri })
}

export type RemoteUploadPlanItem = {
  sourceUri: string
  destRelativePath: string
  size: number
}

export type RemoteUploadPlan = {
  files: RemoteUploadPlanItem[]
  conflicts: string[]
}

export type RemoteUploadConflictPolicy = 'overwrite' | 'keep_both'

export function pickRemoteUploadFile() {
  return invoke<string | null>('pick_remote_upload_file')
}

export function pickRemoteUploadFolder() {
  return invoke<string | null>('pick_remote_upload_folder')
}

export function planRemotePcUpload(
  host: string,
  port: number,
  pcDestDir: string,
  sourceUri: string,
  kind: 'file' | 'folder',
) {
  return invoke<RemoteUploadPlan>('plan_remote_pc_upload', {
    host,
    port,
    pcDestDir,
    sourceUri,
    kind,
  })
}

export function uploadRemotePcFiles(
  host: string,
  port: number,
  files: RemoteUploadPlanItem[],
  onConflict: RemoteUploadConflictPolicy,
) {
  return invoke<null>('upload_remote_pc_files', { host, port, files, onConflict })
}

export async function listenRemoteTransferProgress(
  handler: (payload: RemoteTransferProgressEvent) => void,
): Promise<import('@tauri-apps/api/event').UnlistenFn> {
  return listen<RemoteTransferProgressEvent>('remote-transfer-progress-event', (ev) =>
    handler(ev.payload),
  )
}
