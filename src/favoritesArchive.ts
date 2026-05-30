import type { ComicInSearch } from './api'
import type { SearchTabBookmark } from './searchTabBookmarkTypes'
import { isSearchTabBookmarkExport } from './searchTabBookmarksStorage'

export const FAVORITE_COMICS_FORMAT = 'gentleman-manager.favorite-comics.v1' as const
export const FAVORITE_TABS_FORMAT = 'gentleman-manager.favorite-tabs.v1' as const

export type FavoriteComicsExportFile = {
  format: typeof FAVORITE_COMICS_FORMAT
  exportedAt: string
  comics: ComicInSearch[]
}

export type FavoriteTabsExportFile = {
  format: typeof FAVORITE_TABS_FORMAT
  exportedAt: string
  bookmarks: SearchTabBookmark[]
}

function archiveTimestamp() {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}_${pad(d.getMonth() + 1)}_${pad(d.getDate())}_${pad(d.getHours())}_${pad(d.getMinutes())}_${pad(d.getSeconds())}`
}

export function favoriteComicsArchiveFileName() {
  return `收藏漫畫存檔_${archiveTimestamp()}.gm-snapshot.json`
}

export function favoriteTabsArchiveFileName() {
  return `收藏分頁存檔_${archiveTimestamp()}.gm-snapshot.json`
}

function isComicInSearch(value: unknown): value is ComicInSearch {
  if (typeof value !== 'object' || value === null) return false
  const item = value as ComicInSearch
  return (
    typeof item.id === 'number' &&
    typeof item.title === 'string' &&
    typeof item.cover === 'string'
  )
}

function isSearchTabBookmark(value: unknown): value is SearchTabBookmark {
  if (typeof value !== 'object' || value === null) return false
  const item = value as SearchTabBookmark
  return (
    typeof item.id === 'string' &&
    typeof item.sourceTabId === 'string' &&
    typeof item.savedAt === 'string' &&
    typeof item.title === 'string' &&
    typeof item.tabState === 'object' &&
    item.tabState !== null
  )
}

export function buildFavoriteComicsExportFile(comics: ComicInSearch[]): FavoriteComicsExportFile {
  return {
    format: FAVORITE_COMICS_FORMAT,
    exportedAt: new Date().toISOString(),
    comics: [...comics],
  }
}

export function buildFavoriteTabsExportFile(bookmarks: SearchTabBookmark[]): FavoriteTabsExportFile {
  return {
    format: FAVORITE_TABS_FORMAT,
    exportedAt: new Date().toISOString(),
    bookmarks: [...bookmarks],
  }
}

export function serializeFavoriteArchive(exportFile: FavoriteComicsExportFile | FavoriteTabsExportFile) {
  return `${JSON.stringify(exportFile, null, 2)}\n`
}

export function parseFavoriteComicsImport(raw: string): ComicInSearch[] {
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    throw new Error('收藏漫畫存檔格式錯誤，無法載入')
  }
  if (typeof parsed !== 'object' || parsed === null) {
    throw new Error('這不是有效的收藏漫畫存檔')
  }
  const file = parsed as FavoriteComicsExportFile
  if (file.format !== FAVORITE_COMICS_FORMAT || !Array.isArray(file.comics)) {
    throw new Error('這不是有效的收藏漫畫存檔')
  }
  return file.comics.filter(isComicInSearch)
}

export function parseFavoriteTabsImport(raw: string): SearchTabBookmark[] {
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    throw new Error('收藏分頁存檔格式錯誤，無法載入')
  }
  if (typeof parsed !== 'object' || parsed === null) {
    throw new Error('這不是有效的收藏分頁存檔')
  }
  const file = parsed as FavoriteTabsExportFile
  if (file.format === FAVORITE_TABS_FORMAT && Array.isArray(file.bookmarks)) {
    return file.bookmarks.filter(isSearchTabBookmark)
  }
  if (isSearchTabBookmarkExport(parsed)) {
    return [parsed.bookmark]
  }
  throw new Error('這不是有效的收藏分頁存檔')
}
