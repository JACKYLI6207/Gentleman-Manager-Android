import type { ComicInSearch } from './api'
import { comicsForBookmarkStorage } from './searchTabBookmarkTypes'
import type { GridLayout, SearchSortOrder } from './searchUtils'

export type MobileBrowseKind =
  | 'home'
  | 'albums'
  | 'category'
  | 'ranking'
  | 'keyword'
  | 'snapshot'
  | 'none'

/** 關鍵詞搜索範圍：全站為 null；有 filePath 時僅搜該快照檔 */
export type MobileSearchScope = {
  label: string
  cateId: number | null
  filePath: string
}

export type MobileSearchTab = {
  id: string
  title: string
  keyword: string
  searchScope: MobileSearchScope | null
  browseKind: MobileBrowseKind
  activeTopCategory: string
  categoryBrowseCateId: number | null
  rankingCateId: number | null
  snapshotPath: string | null
  snapshotLabel: string | null
  allComics: ComicInSearch[]
  currentPage: number
  totalPages: number
  totalCount: number
  sortOrder: SearchSortOrder
  pageSize: number
  gridLayout: GridLayout
  /** 列表比對結果（漫畫 id → 說明） */
  catalogAnalysisEntries?: [number, string][]
  /** 搜索結果列表捲動位置（切換分頁／底欄時還原） */
  scrollTop?: number
  /** 官網列表合併段起始頁（切換分頁時還原，避免切頁誤用他分頁快取） */
  serverChunkBase?: number
  /** 官網/API 已載入至第幾頁 */
  serverPage?: number
  /** 總筆數是否已依尾頁下修 */
  totalCountRefined?: boolean
}

const TABS_KEY = 'gm-android.searchResultTabs.v1'
const FAV_KEY = 'gm-android.favoriteSearchTabIds.v1'
const TAB_TITLE_MAX = 28

function truncateTitle(text: string): string {
  const t = text.trim()
  return t.length <= TAB_TITLE_MAX ? t : `${t.slice(0, TAB_TITLE_MAX)}…`
}

export function formatMobileSearchTabTitle(input: {
  browseKind: MobileBrowseKind
  keyword: string
  searchScope: { label: string } | null
  activeTopCategory: string
  snapshotLabel: string | null
}): string {
  const { browseKind, keyword, searchScope, activeTopCategory, snapshotLabel } = input
  const kw = keyword.trim()
  if (browseKind === 'snapshot' && snapshotLabel) {
    if (kw) return truncateTitle(`${snapshotLabel} · ${kw}`)
    return truncateTitle(`${snapshotLabel}快照`)
  }
  if (browseKind === 'keyword' && kw) {
    if (searchScope) return truncateTitle(`${searchScope.label} · ${kw}`)
    return truncateTitle(kw)
  }
  if (activeTopCategory) return truncateTitle(activeTopCategory)
  return '搜尋結果'
}

function migrateTabBrowseKind(tab: MobileSearchTab): MobileSearchTab {
  const root = tab.activeTopCategory.split(' / ')[0] ?? ''
  if (root === '更新' && tab.browseKind !== 'albums') {
    return { ...tab, browseKind: 'albums', categoryBrowseCateId: null }
  }
  if (root === '首頁' && tab.browseKind !== 'home') {
    return { ...tab, browseKind: 'home', categoryBrowseCateId: null }
  }
  if (root === '排行' && tab.browseKind !== 'ranking' && !tab.keyword.trim()) {
    return { ...tab, browseKind: 'ranking' }
  }
  return tab
}

export function loadSearchTabs(): { tabs: MobileSearchTab[]; activeId: string | null } {
  try {
    const raw = localStorage.getItem(TABS_KEY)
    if (!raw) return { tabs: [], activeId: null }
    const parsed = JSON.parse(raw) as { tabs?: MobileSearchTab[]; activeId?: string | null }
    const tabs = Array.isArray(parsed.tabs) ? parsed.tabs.map(migrateTabBrowseKind) : []
    return { tabs, activeId: parsed.activeId ?? null }
  } catch {
    return { tabs: [], activeId: null }
  }
}

/** 寫入 localStorage 時縮小體積：快照分頁只留路徑，切回時再從檔案載入 */
export function tabToPersistedStorage(tab: MobileSearchTab): MobileSearchTab {
  if (tab.browseKind === 'snapshot' && tab.snapshotPath) {
    return {
      ...tab,
      allComics: [],
    }
  }
  return {
    ...tab,
    allComics: comicsForBookmarkStorage(tab.allComics),
  }
}

export function saveSearchTabs(tabs: MobileSearchTab[], activeId: string | null): boolean {
  try {
    const persisted = tabs.map(tabToPersistedStorage)
    localStorage.setItem(TABS_KEY, JSON.stringify({ tabs: persisted, activeId }))
    return true
  } catch (e) {
    const isQuota =
      (e instanceof DOMException && e.name === 'QuotaExceededError') ||
      String(e).includes('QuotaExceededError')
    if (!isQuota) {
      throw e
    }
    try {
      const minimal = tabs.map((tab) => ({
        ...tabToPersistedStorage(tab),
        allComics: [],
        catalogAnalysisEntries: [],
      }))
      localStorage.setItem(TABS_KEY, JSON.stringify({ tabs: minimal, activeId }))
      return false
    } catch {
      return false
    }
  }
}

export function loadFavoriteTabIds(): Set<string> {
  try {
    const raw = localStorage.getItem(FAV_KEY)
    if (!raw) return new Set()
    const ids = JSON.parse(raw) as string[]
    return new Set(Array.isArray(ids) ? ids : [])
  } catch {
    return new Set()
  }
}

export function saveFavoriteTabIds(ids: Set<string>) {
  localStorage.setItem(FAV_KEY, JSON.stringify([...ids]))
}
