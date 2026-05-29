import type { DownloadFormat, DownloadTaskEvent } from './api'

export type ProgressRow = DownloadTaskEvent & {
  percentage: number
  indicator: string
  /** 失敗時的詳細原因（與進度分開顯示） */
  failureDetail?: string
}

export function formatDownloadBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${bytes} B`
}

export function buildProgressRow(
  ev: DownloadTaskEvent,
  downloadFormat: DownloadFormat,
  downloadRetryCount: number,
): ProgressRow {
  const { state, downloadedImgCount, totalImgCount, downloadedBytes, totalBytes } = ev
  let percentage = NaN
  if (totalBytes > 0) percentage = (downloadedBytes / totalBytes) * 100
  else if (totalImgCount > 0) percentage = (downloadedImgCount / totalImgCount) * 100

  const isServer2 = downloadFormat === 'Server2Zip'
  const isJpegPack = downloadFormat === 'JpegZipPack'
  let indicator = ''
  if (state === 'Pending') indicator = '排隊中'
  else if (state === 'Downloading') {
    if (isServer2) indicator = '下載中 (Server 2)'
    else if (isJpegPack) indicator = totalImgCount > 0 && downloadedImgCount >= totalImgCount ? '打包 ZIP…' : '下載圖片…'
    else indicator = '下載中'
  } else if (state === 'Paused') indicator = '已暫停'
  else if (state === 'Cancelled') indicator = '已取消'
  else if (state === 'Completed') indicator = '下載完成'
  else if (state === 'Failed') {
    indicator = `下載失敗 (已嘗試 ${downloadRetryCount + 1} 次)`
  }
  if (totalBytes > 0) {
    indicator += ` ${formatDownloadBytes(downloadedBytes)}/${formatDownloadBytes(totalBytes)}`
  } else if (totalImgCount > 0) {
    indicator += ` ${downloadedImgCount}/${totalImgCount}`
  }
  const failureDetail =
    state === 'Failed' && ev.lastError?.trim() ? ev.lastError.trim() : undefined
  return { ...ev, percentage, indicator, failureDetail }
}
