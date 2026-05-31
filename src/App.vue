<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { SITE_CATEGORIES, type SiteCategoryItem } from './categories'
import MobileComicCard from './components/MobileComicCard.vue'
import MobileComicDetail from './components/MobileComicDetail.vue'
import MobileComicRead from './components/MobileComicRead.vue'
import MobileDownloadPanel from './components/MobileDownloadPanel.vue'
import MobileFavoritesPanel from './components/MobileFavoritesPanel.vue'
import MobileLocalRead from './components/MobileLocalRead.vue'
import MobileKoreanDownloadDialog from './components/MobileKoreanDownloadDialog.vue'
import MobileSettingsPanel from './components/MobileSettingsPanel.vue'
import MobileSnapshotResumeDialog from './components/MobileSnapshotResumeDialog.vue'
import SearchResultTabBar from './components/SearchResultTabBar.vue'
import { formatComicCreatedLabel } from './comicMeta'
import { loadFavoriteComics, saveFavoriteComics } from './favoritesStorage'
import {
  buildFavoriteComicsExportFile,
  buildFavoriteTabsExportFile,
  favoriteComicsArchiveFileName,
  favoriteTabsArchiveFileName,
  parseFavoriteComicsImport,
  parseFavoriteTabsImport,
  serializeFavoriteArchive,
} from './favoritesArchive'
import { bookmarkToMobileTab, mobileTabToBookmark } from './mobileTabBookmarkBridge'
import { loadFavoriteSearchTabs, saveFavoriteSearchTabs } from './searchTabBookmarksStorage'
import type { SearchTabBookmark } from './searchTabBookmarkTypes'
import {
  formatMobileSearchTabTitle,
  loadSearchTabs,
  saveSearchTabs,
  type MobileSearchScope,
  type MobileSearchTab,
} from './searchResultTabs'
import {
  browseAlbumsList,
  browseByCategory,
  browseHome,
  browseRanking,
  getConfig,
  readKoreanTxtCatalog,
  getDownloadTaskSnapshots,
  createDownloadTask,
  getMobileSettings,
  listenSearchScanProgress,
  listSnapshotCategoryHeaders,
  listSnapshotResumeCandidates,
  pickCategoryDirectory,
  type SnapshotResumeCandidate,
  pickDownloadDirectory,
  pickImportArchiveFile,
  readImportArchiveFile,
  writeCategorySnapshotFile,
  getComic,
  searchByKeyword,
  searchByTag,
  searchSnapshotFile,
  type Comic,
  type ComicInSearch,
  type DownloadTaskSeed,
  type DownloadTaskEvent,
  type SearchResult,
  type SnapshotCategoryHeader,
} from './api'
import {
  cancelSnapshotScan,
  runSnapshotResumeQueue,
  type SnapshotResumeStrategy,
} from './snapshotScan'
import {
  gridColsForLayout,
  LAYOUT_OPTIONS,
  loadSavedFavoritesLayout,
  loadSavedFavoritesPageSize,
  loadSavedFavoritesSortOrder,
  loadSavedLayout,
  loadSavedPageSize,
  loadSavedSortOrder,
  isIdBasedSortOrder,
  LIVE_BROWSE_CUSTOM_SORT_HINT,
  PAGE_SIZE_OPTIONS,
  saveFavoritesLayout,
  saveFavoritesPageSize,
  saveFavoritesSortOrder,
  saveLayout,
  savePageSize,
  saveSortOrder,
  SEARCH_SORT_OPTIONS,
  sortSearchComics,
  type GridLayout,
  type SearchSortOrder,
} from './searchUtils'
import type { ClassifiedKoreanItem } from './koreanWebtoon'
import {
  analyzeComicsWithCatalogContext,
  createCatalogCompareContext,
  normalizePageTagLabel,
} from './koreanTxtDuplicate'
import { readerFullscreenActive } from './composables/useReaderFullscreen'
import { initLocalReadOnAppLaunch, localReadSession } from './localReadStore'
import { pickLocalReaderTitle } from './readerDisplayName'
import { extractComicSearchName } from './comicSearchName'
import {
  findLatestSnapshotHeaderForScope,
  hasComicCategorySnapshot,
  resolveCategoryScopeFromComicCategory,
} from './comicDetailSearch'

type TabId = 'home' | 'download' | 'settings'
type SubNav = 'search' | 'detail' | 'read' | 'favorites' | 'snapshots'
type ReadMode = 'online' | 'local'
type FavoritesSection = 'comics' | 'tabs'

const FAVORITES_MENU_ITEMS: { key: FavoritesSection; label: string }[] = [
  { key: 'comics', label: '收藏漫畫' },
  { key: 'tabs', label: '收藏分頁' },
]

const READ_MENU_ITEMS: { key: ReadMode; label: string }[] = [
  { key: 'online', label: '在線閱讀' },
  { key: 'local', label: '本地閱讀' },
]
type BrowseKind = 'home' | 'albums' | 'category' | 'ranking' | 'keyword' | 'snapshot' | 'none'

const activeTab = ref<TabId>('home')
const subNav = ref<SubNav>('search')
const activeTopCategory = ref<string>('')
const expandedNavParent = ref<string | null>(null)
const categoryBrowseCateId = ref<number | null>(null)
const rankingCateId = ref<number | null>(null)
const showStatusLine = ref(false)
const loadingHint = ref('')

const categoryDir = ref('')
const downloadDir = ref('')
const categoryHeaders = ref<SnapshotCategoryHeader[]>([])
const activeSnapshot = ref<SnapshotCategoryHeader | null>(null)

const keyword = ref('')
const searchScope = ref<MobileSearchScope | null>(null)
/** 使用者從「範圍」選單主動選定的下一次關鍵詞搜索範圍（切分頁還原時不沿用） */
const scopePinnedForNextSearch = ref(false)
const scopeMenuOpen = ref(false)
const sortMenuOpen = ref(false)
const pageSizeMenuOpen = ref(false)
const layoutMenuOpen = ref(false)
const favSortMenuOpen = ref(false)
const favPageSizeMenuOpen = ref(false)
const favLayoutMenuOpen = ref(false)
const readMenuOpen = ref(false)
const readMode = ref<ReadMode>('online')
const favoritesMenuOpen = ref(false)
const favoritesSection = ref<FavoritesSection>('tabs')

const searchTabs = ref<MobileSearchTab[]>([])
const activeSearchTabId = ref<string | null>(null)
const favoriteSearchTabs = ref<SearchTabBookmark[]>(loadFavoriteSearchTabs())
const favoriteComics = ref<ComicInSearch[]>(loadFavoriteComics())
const comicListMeta = ref<Map<number, string>>(new Map())
const readingComic = ref<Comic | null>(null)
const onlineReadingActive = ref(false)

/** 視窗模式：次導覽下方顯示目前閱讀檔名；全視窗時隱藏 */
const showReaderTitleBar = computed(() => {
  if (subNav.value !== 'read' || readerFullscreenActive.value) return false
  if (readMode.value === 'local') {
    if (!localReadSession.readingActive) return false
    const s = localReadSession
    return (
      pickLocalReaderTitle(
        s.readerTitle,
        s.folderSources,
        s.currentSourcePath,
        s.currentSourceIndex,
      ).length > 0
    )
  }
  if (readMode.value === 'online') {
    return onlineReadingActive.value && Boolean(readingComic.value?.title?.trim())
  }
  return false
})

const activeReaderTitle = computed(() => {
  if (readMode.value === 'local') {
    const s = localReadSession
    return pickLocalReaderTitle(
      s.readerTitle,
      s.folderSources,
      s.currentSourcePath,
      s.currentSourceIndex,
    )
  }
  return readingComic.value?.title ?? ''
})

const loading = ref(false)
const statusMessage = ref('')
const allComics = ref<ComicInSearch[]>([])
const comics = ref<ComicInSearch[]>([])
/** 官網列表每頁約 20 筆（與 PC 版一致） */
const SERVER_LIST_PAGE_SIZE = 20
const PAGE_FETCH_DELAY_MS = 300
let browsePageCache = new Map<number, SearchResult>()
/** 綁定 tab + 搜尋條件；切換分頁時必須失效，避免官網頁快取串台 */
let browsePageCacheKey = ''
let browseFetchGeneration = 0
/** 底部頁碼列顯示的「第幾頁」（依每頁顯示筆數切分總結果） */
const viewPage = ref(1)
/** 官網/API 請求的第幾頁（快照瀏覽時與 viewPage 相同） */
const serverPage = ref(1)
/** allComics 對應的官網起始頁（合併多頁時為第一段） */
const serverChunkBase = ref(1)
const totalPages = ref(1)
const totalCount = ref(0)
/** 總筆數已依尾頁實際筆數下修；避免官網估算值覆蓋正確總數 */
const totalCountRefined = ref(false)
const pageSize = ref(loadSavedPageSize())
const sortOrder = ref<SearchSortOrder>(loadSavedSortOrder())
const gridLayout = ref<GridLayout>(loadSavedLayout())
const browseKind = ref<BrowseKind>('none')

const hasCategoryDir = computed(() => categoryDir.value.trim().length > 0)
const hasSnapshots = computed(() => categoryHeaders.value.length > 0)
/** 僅在已設快照資料夾且目錄內有快照時，才允許選分類搜索範圍 */
const canUseCategoryScope = computed(() => hasCategoryDir.value && hasSnapshots.value)
const scopeLabel = computed(() => searchScope.value?.label ?? '全站')
const sortLabel = computed(
  () => SEARCH_SORT_OPTIONS.find((o) => o.key === sortOrder.value)?.label ?? '排序',
)
const pageSizeLabel = computed(() => `每頁 ${pageSize.value}`)
const layoutLabel = computed(
  () => LAYOUT_OPTIONS.find((o) => o.key === gridLayout.value)?.label ?? '顯示',
)
const cardLayout = computed<'grid' | 'list'>(() => (gridLayout.value === 'list' ? 'list' : 'grid'))
const gridCols = computed(() => (gridLayout.value === 'list' ? 1 : gridColsForLayout(gridLayout.value)))
const pageSummary = computed(() => {
  if (totalCount.value <= 0) return '無結果'
  return `${viewPage.value}/${totalPages.value} 頁 · ${totalCount.value} 筆`
})

function isLiveWebsiteBrowse(): boolean {
  const k = browseKind.value
  return k !== 'none' && k !== 'snapshot'
}

const showBrowseSortHint = computed(
  () =>
    isLiveWebsiteBrowse() &&
    !isIdBasedSortOrder(sortOrder.value) &&
    subNav.value === 'search' &&
    totalCount.value > 0,
)

function finishBrowseViewLoadStatus() {
  if (isLiveWebsiteBrowse() && !isIdBasedSortOrder(sortOrder.value)) {
    setStatus(LIVE_BROWSE_CUSTOM_SORT_HINT)
  } else {
    clearStatus()
  }
}

/** 有列表結果且需要分頁控制時（與底部頁碼列相同條件） */
const hasActiveSearchTab = computed(
  () =>
    activeSearchTabId.value !== null &&
    searchTabs.value.some((t) => t.id === activeSearchTabId.value),
)
const visibleSearchComics = computed(() => (hasActiveSearchTab.value ? comics.value : []))
const showSearchPager = computed(
  () => subNav.value === 'search' && hasActiveSearchTab.value && totalCount.value > 0,
)
const canUseKoreanDownloadMode = computed(
  () =>
    subNav.value === 'search' &&
    (browseKind.value === 'keyword' || browseKind.value === 'snapshot') &&
    totalCount.value > 0 &&
    comics.value.length > 0 &&
    !loading.value,
)

const catalogAnalysisByComicId = ref<Map<number, string>>(new Map())
const listCompareLoading = ref(false)
type ListCompareProgress = {
  phase: 'reading' | 'comparing'
  current: number
  total: number
}
const listCompareProgress = ref<ListCompareProgress | null>(null)

const listCompareProgressPercent = computed(() => {
  const p = listCompareProgress.value
  if (!p || p.total <= 0) return 0
  if (p.phase === 'reading') return 8
  return Math.min(100, Math.round((p.current / p.total) * 100))
})

const listCompareProgressLabel = computed(() => {
  const p = listCompareProgress.value
  if (!p) return '列表比對'
  if (p.phase === 'reading') return '正在讀取 TXT 列表…'
  return `比對中 ${p.current}／${p.total}`
})

const canRunListCompare = computed(
  () => subNav.value === 'search' && comics.value.length > 0 && !loading.value && !listCompareLoading.value,
)

function yieldToUi() {
  return new Promise<void>((resolve) => {
    requestAnimationFrame(() => resolve())
  })
}

function catalogAnalysisNoteFor(comicId: number): string | undefined {
  return catalogAnalysisByComicId.value.get(comicId)
}

function resolveListCompareTagLabel(): string | undefined {
  if (browseKind.value === 'snapshot' && activeSnapshot.value?.label) {
    return activeSnapshot.value.label
  }
  if (searchScope.value?.label) {
    return normalizePageTagLabel(searchScope.value.label) ?? searchScope.value.label
  }
  const kw = keyword.value.trim()
  return kw || undefined
}

async function runListCompare() {
  let config
  try {
    config = await getConfig()
  } catch (e) {
    setStatus(e)
    return
  }
  if (!config.koreanTxtDuplicateCheckEnabled) {
    setStatus('請先在設定中開啟韓漫 TXT 重複檢查')
    return
  }
  const catalogPath = (config.koreanTxtCatalogDir ?? '').trim()
  if (catalogPath === '') {
    setStatus('請先在設定中選擇韓漫 TXT 檔案')
    return
  }
  if (comics.value.length === 0) {
    setStatus('目前頁面沒有漫畫')
    return
  }

  const tabId = activeSearchTabId.value
  const comicsSnapshot = [...comics.value]
  const tagLabel = resolveListCompareTagLabel()

  listCompareLoading.value = true
  listCompareProgress.value = { phase: 'reading', current: 0, total: comicsSnapshot.length }
  await nextTick()
  try {
    const lines = await readKoreanTxtCatalog(catalogPath)
    const ctx = createCatalogCompareContext(lines, { tagLabel })
    const analysis = new Map<number, string>()
    const total = comicsSnapshot.length
    const batchSize = 40
    listCompareProgress.value = { phase: 'comparing', current: 0, total }
    await nextTick()

    for (let i = 0; i < total; i += batchSize) {
      const batch = comicsSnapshot.slice(i, i + batchSize)
      const partial = analyzeComicsWithCatalogContext(batch, ctx)
      for (const [id, msg] of partial) {
        analysis.set(id, msg)
      }
      listCompareProgress.value = {
        phase: 'comparing',
        current: Math.min(i + batchSize, total),
        total,
      }
      if (i + batchSize < total) {
        await yieldToUi()
      }
    }

    const duplicateCount = [...analysis.values()].filter((m) => m.startsWith('與列表中')).length

    if (tabId !== null && activeSearchTabId.value !== tabId) {
      const idx = searchTabs.value.findIndex((t) => t.id === tabId)
      if (idx >= 0) {
        searchTabs.value[idx] = {
          ...searchTabs.value[idx]!,
          catalogAnalysisEntries: [...analysis.entries()],
        }
        saveSearchTabs(searchTabs.value, activeSearchTabId.value)
      }
      setStatus(`已比對本頁 ${comicsSnapshot.length} 項（${duplicateCount} 項可能重複）`)
      return
    }

    catalogAnalysisByComicId.value = analysis
    persistActiveTab()
    setStatus(`已比對本頁 ${comicsSnapshot.length} 項（${duplicateCount} 項可能重複）`)
  } catch (e) {
    setStatus(e)
  } finally {
    listCompareLoading.value = false
    listCompareProgress.value = null
  }
}

const favViewPage = ref(1)
const favPageSize = ref(loadSavedFavoritesPageSize())
const favSortOrder = ref<SearchSortOrder>(loadSavedFavoritesSortOrder())
const favGridLayout = ref<GridLayout>(loadSavedFavoritesLayout())

const favSortLabel = computed(
  () => SEARCH_SORT_OPTIONS.find((o) => o.key === favSortOrder.value)?.label ?? '排序',
)
const favPageSizeLabel = computed(() => `每頁 ${favPageSize.value}`)
const favLayoutLabel = computed(
  () => LAYOUT_OPTIONS.find((o) => o.key === favGridLayout.value)?.label ?? '顯示',
)
const favCardLayout = computed<'grid' | 'list'>(() =>
  favGridLayout.value === 'list' ? 'list' : 'grid',
)
const favGridCols = computed(() =>
  favGridLayout.value === 'list' ? 1 : gridColsForLayout(favGridLayout.value),
)

const favoriteComicsSorted = computed(() =>
  sortSearchComics(favoriteComics.value, favSortOrder.value),
)

const favPageSizeEffective = computed(() => Math.max(1, favPageSize.value))

const favTotalPages = computed(() =>
  Math.max(1, Math.ceil(favoriteComics.value.length / favPageSizeEffective.value)),
)

const favoriteComicsVisible = computed(() => {
  const sorted = favoriteComicsSorted.value
  if (sorted.length === 0) return []
  const vp = Math.max(1, Math.min(favViewPage.value, favTotalPages.value))
  const ps = favPageSizeEffective.value
  const start = (vp - 1) * ps
  return sorted.slice(start, start + ps)
})

const favPageSummary = computed(() => {
  const n = favoriteComics.value.length
  if (n <= 0) return '無結果'
  return `${favViewPage.value}/${favTotalPages.value} 頁 · ${n} 筆`
})

const showFavoritesPager = computed(
  () => subNav.value === 'favorites' && favoritesSection.value === 'comics' && favoriteComics.value.length > 0,
)

const showFavoritesToolbar = computed(() => subNav.value === 'favorites')

const favPageTabNumbers = computed(() => {
  const tp = favTotalPages.value
  const n = favoriteComics.value.length
  if (tp < 1 || n <= 0) return [] as number[]
  const max = Math.min(pagerSlotCount.value, tp)
  if (tp <= max) {
    return Array.from({ length: tp }, (_, i) => i + 1)
  }
  const cp = favViewPage.value
  const half = Math.floor(max / 2)
  let start = Math.max(1, cp - half)
  let end = start + max - 1
  if (end > tp) {
    end = tp
    start = Math.max(1, end - max + 1)
  }
  return Array.from({ length: end - start + 1 }, (_, i) => start + i)
})

const favAllPageNumbers = computed(() =>
  Array.from({ length: favTotalPages.value }, (_, i) => i + 1),
)

const topCategories = SITE_CATEGORIES

const expandedParentItem = computed(() =>
  expandedNavParent.value ? topCategories.find((c) => c.label === expandedNavParent.value) : undefined,
)

const detailCreatedLabel = computed(() => {
  if (!selectedComicId.value) return ''
  const info = comicListMeta.value.get(selectedComicId.value) ?? ''
  return formatComicCreatedLabel(info)
})

const siteNavRowRef = ref<HTMLElement | null>(null)
const categoryMenuRef = ref<HTMLElement | null>(null)
const categoryMenuLeft = ref(0)
const scopeBtnRef = ref<HTMLElement | null>(null)
const scopeMenuPos = ref({ top: 0, left: 0, minWidth: 140 })

const pageJumpOpen = ref(false)
const favPageJumpOpen = ref(false)
const pageGoForFavorites = ref(false)
const pageGoInput = ref<string>('')
const downloadAllConfirmOpen = ref(false)
const downloadAllFavoritesConfirmOpen = ref(false)
const clearFavoritesConfirmOpen = ref(false)
const batchDownloadMenuOpen = ref(false)
const koreanModeDialogOpen = ref(false)
const koreanModeDialogComics = ref<ComicInSearch[]>([])
const koreanModeDialogTagLabel = ref('')
const searchScrollRef = ref<HTMLElement | null>(null)
const favScrollRef = ref<HTMLElement | null>(null)
const pagerBarRef = ref<HTMLElement | null>(null)
const pagerSummaryBtnRef = ref<HTMLElement | null>(null)
/** 隱藏探針：與 .pt-num 同樣式，用於量測單個頁碼按鈕寬度 */
const pagerMeasureRef = ref<HTMLElement | null>(null)
/** 依底欄寬度與頁碼數字寬度計算可顯示的頁碼按鈕數（ResizeObserver 更新） */
const pagerSlotCount = ref(7)

const PT_NAV_WIDTH = 26
const PT_SUMMARY_BTN_MIN = 48
const PT_NUM_GAP = 2

function pagerWindowFor(tp: number, cp: number, max: number): [number, number] {
  if (tp <= 0) return [1, 1]
  const slots = Math.max(1, Math.min(max, tp))
  if (tp <= slots) return [1, tp]
  const half = Math.floor(slots / 2)
  let start = Math.max(1, cp - half)
  let end = start + slots - 1
  if (end > tp) {
    end = tp
    start = Math.max(1, end - slots + 1)
  }
  return [start, end]
}

function widestPageLabelInWindow(tp: number, cp: number, max: number): string {
  const [start, end] = pagerWindowFor(tp, cp, max)
  let best = String(cp)
  for (let n = start; n <= end; n++) {
    const label = String(n)
    if (label.length > best.length) best = label
  }
  return best
}

function measurePageNumButtonWidth(label: string): number {
  const probe = pagerMeasureRef.value
  if (probe) {
    probe.textContent = label
    return Math.ceil(probe.offsetWidth) + PT_NUM_GAP
  }
  return Math.max(26, label.length * 7 + 12)
}

function activePagerContext(): { tp: number; cp: number } {
  if (showFavoritesPager.value) {
    return { tp: favTotalPages.value, cp: favViewPage.value }
  }
  return { tp: totalPages.value, cp: viewPage.value }
}

function updatePagerSlotCount() {
  const bar = pagerBarRef.value
  if (!bar) return
  const summaryW = Math.max(pagerSummaryBtnRef.value?.offsetWidth ?? 0, PT_SUMMARY_BTN_MIN)
  const reserved = PT_NAV_WIDTH * 2 + summaryW + 12
  const available = Math.max(0, bar.clientWidth - reserved)
  const { tp, cp } = activePagerContext()
  if (tp <= 0 || available <= 0) {
    pagerSlotCount.value = 1
    return
  }

  let numW = measurePageNumButtonWidth(widestPageLabelInWindow(tp, cp, 7))
  let slots = Math.max(1, Math.floor(available / numW))
  const widest = widestPageLabelInWindow(tp, cp, slots)
  numW = measurePageNumButtonWidth(widest)
  slots = Math.max(1, Math.floor(available / numW))
  pagerSlotCount.value = Math.min(tp, slots)
}

const pageTabNumbers = computed(() => {
  const tp = totalPages.value
  if (tp < 1 || totalCount.value <= 0) return [] as number[]
  const max = Math.min(pagerSlotCount.value, tp)
  if (tp <= max) {
    return Array.from({ length: tp }, (_, i) => i + 1)
  }
  const cp = viewPage.value
  const half = Math.floor(max / 2)
  let start = Math.max(1, cp - half)
  let end = start + max - 1
  if (end > tp) {
    end = tp
    start = Math.max(1, end - max + 1)
  }
  return Array.from({ length: end - start + 1 }, (_, i) => start + i)
})

const allPageNumbers = computed(() =>
  Array.from({ length: totalPages.value }, (_, i) => i + 1),
)

let pagerResizeObserver: ResizeObserver | null = null

watch(showFavoritesPager, (show) => {
  if (!show) return
  void nextTick(() => {
    updatePagerSlotCount()
    if (!pagerBarRef.value) return
    pagerResizeObserver?.disconnect()
    pagerResizeObserver = new ResizeObserver(() => updatePagerSlotCount())
    pagerResizeObserver.observe(pagerBarRef.value)
  })
})

watch(showSearchPager, (show) => {
  if (!show) {
    pageJumpOpen.value = false
    return
  }
  void nextTick(() => {
    updatePagerSlotCount()
    if (!pagerBarRef.value) return
    pagerResizeObserver?.disconnect()
    pagerResizeObserver = new ResizeObserver(() => updatePagerSlotCount())
    pagerResizeObserver.observe(pagerBarRef.value)
  })
})

watch([pageSummary, totalPages, viewPage, favPageSummary, favViewPage], () => {
  void nextTick(updatePagerSlotCount)
})

watch(pageJumpOpen, (open) => {
  if (open) {
    pageGoForFavorites.value = false
    pageGoInput.value = String(viewPage.value)
  }
})

watch(favPageJumpOpen, (open) => {
  if (open) {
    pageGoForFavorites.value = true
    pageGoInput.value = String(favViewPage.value)
  } else {
    pageGoForFavorites.value = false
  }
})

function scrollSearchResultsToTop() {
  void nextTick(() => {
    const el = searchScrollRef.value
    if (el) el.scrollTop = 0
  })
}

function scrollFavoritesToTop() {
  void nextTick(() => {
    const el = favScrollRef.value
    if (el) el.scrollTop = 0
  })
}

function readSearchScrollTop(): number {
  return searchScrollRef.value?.scrollTop ?? 0
}

function restoreSearchScrollTop(scrollTop: number | undefined) {
  void nextTick(() => {
    const el = searchScrollRef.value
    if (el) el.scrollTop = scrollTop ?? 0
  })
}

function clearSearchKeyword() {
  keyword.value = ''
}

watch(scopeMenuOpen, (open) => {
  if (!open) return
  void nextTick(updateScopeMenuPosition)
})

function updateScopeMenuPosition() {
  const btn = scopeBtnRef.value
  if (!btn) return
  const rect = btn.getBoundingClientRect()
  scopeMenuPos.value = {
    top: rect.bottom + 2,
    left: rect.left,
    minWidth: Math.max(rect.width, 140),
  }
}

watch(expandedNavParent, () => {
  void nextTick()
    .then(updateCategoryMenuPosition)
    .then(() => nextTick())
    .then(updateCategoryMenuPosition)
})

function updateCategoryMenuPosition() {
  const label = expandedNavParent.value
  const row = siteNavRowRef.value
  if (!label || !row) {
    categoryMenuLeft.value = 0
    return
  }
  const btn = row.querySelector(`[data-cat="${CSS.escape(label)}"]`) as HTMLElement | null
  if (!btn) return
  const menuW = categoryMenuRef.value?.offsetWidth ?? 96
  let left = btn.offsetLeft
  const maxLeft = Math.max(0, row.clientWidth - menuW - 4)
  categoryMenuLeft.value = Math.min(left, maxLeft)
}

function closeMenus() {
  scopeMenuOpen.value = false
  sortMenuOpen.value = false
  pageSizeMenuOpen.value = false
  layoutMenuOpen.value = false
  favSortMenuOpen.value = false
  favPageSizeMenuOpen.value = false
  favLayoutMenuOpen.value = false
  readMenuOpen.value = false
  favoritesMenuOpen.value = false
  pageJumpOpen.value = false
  favPageJumpOpen.value = false
  batchDownloadMenuOpen.value = false
  detailPanelRef.value?.closeSearchMenu?.()
}

function toggleBatchDownloadMenu(ev: Event) {
  ev.stopPropagation()
  const wasOpen = batchDownloadMenuOpen.value
  closeMenus()
  batchDownloadMenuOpen.value = !wasOpen
}

function onBatchDownloadAllPage() {
  batchDownloadMenuOpen.value = false
  requestDownloadAllPage()
}

function onBatchKoreanDownloadMode() {
  batchDownloadMenuOpen.value = false
  openKoreanDownloadMode()
}

function onBatchTxtListCompare() {
  batchDownloadMenuOpen.value = false
  void runListCompare()
}

function isTabBookmarked(tabId: string) {
  return favoriteSearchTabs.value.some((b) => b.sourceTabId === tabId)
}

function isFavoriteComic(id: number) {
  return favoriteComics.value.some((c) => c.id === id)
}

function rememberComicMeta(c: ComicInSearch) {
  comicListMeta.value.set(c.id, c.additionalInfo)
}

function toggleFavoriteComic(comic: ComicInSearch) {
  if (isFavoriteComic(comic.id)) {
    favoriteComics.value = favoriteComics.value.filter((c) => c.id !== comic.id)
  } else {
    favoriteComics.value = [...favoriteComics.value, comic]
  }
  saveFavoriteComics(favoriteComics.value)
}

function tabStoredKeyword(kind: BrowseKind, kw: string): string {
  return kind === 'keyword' || kind === 'snapshot' ? kw : ''
}

function captureCurrentTab(): MobileSearchTab {
  const id = activeSearchTabId.value ?? crypto.randomUUID()
  const existingIdx = searchTabs.value.findIndex((t) => t.id === id)
  const existing = existingIdx >= 0 ? searchTabs.value[existingIdx]! : null
  // 搜尋欄在非 keyword 分頁僅為草稿，持久化時以分頁已提交狀態為準，避免污染快照／分類分頁
  const draftKeywordBrowseKinds: BrowseKind[] = [
    'snapshot',
    'home',
    'albums',
    'category',
    'ranking',
    'none',
  ]
  const keywordIsDraft =
    existing !== null &&
    draftKeywordBrowseKinds.includes(existing.browseKind) &&
    existing.browseKind === browseKind.value
  const identity = keywordIsDraft
    ? {
        keyword: tabStoredKeyword(existing.browseKind, existing.keyword),
        searchScope: existing.searchScope ? { ...existing.searchScope } : null,
        browseKind: existing.browseKind,
        activeTopCategory: existing.activeTopCategory,
        categoryBrowseCateId: existing.categoryBrowseCateId,
        rankingCateId: existing.rankingCateId,
        snapshotPath: existing.snapshotPath,
        snapshotLabel: existing.snapshotLabel,
      }
    : {
        keyword: tabStoredKeyword(browseKind.value, keyword.value),
        searchScope: searchScope.value ? { ...searchScope.value } : null,
        browseKind: browseKind.value,
        activeTopCategory: activeTopCategory.value,
        categoryBrowseCateId: categoryBrowseCateId.value,
        rankingCateId: rankingCateId.value,
        snapshotPath: activeSnapshot.value?.filePath ?? null,
        snapshotLabel: activeSnapshot.value?.label ?? null,
      }
  return {
    id,
    title: formatMobileSearchTabTitle({
      browseKind: identity.browseKind,
      keyword: identity.keyword,
      searchScope: identity.searchScope,
      activeTopCategory: identity.activeTopCategory,
      snapshotLabel: identity.snapshotLabel,
    }),
    keyword: identity.keyword,
    searchScope: identity.searchScope,
    browseKind: identity.browseKind,
    activeTopCategory: identity.activeTopCategory,
    categoryBrowseCateId: identity.categoryBrowseCateId,
    rankingCateId: identity.rankingCateId,
    snapshotPath: identity.snapshotPath,
    snapshotLabel: identity.snapshotLabel,
    allComics: [...allComics.value],
    currentPage: viewPage.value,
    totalPages: totalPages.value,
    totalCount: totalCount.value,
    sortOrder: sortOrder.value,
    pageSize: pageSize.value,
    gridLayout: gridLayout.value,
    catalogAnalysisEntries: [...catalogAnalysisByComicId.value.entries()],
    scrollTop: readSearchScrollTop(),
    serverChunkBase: serverChunkBase.value,
    serverPage: serverPage.value,
    totalCountRefined: totalCountRefined.value,
  }
}

function persistActiveTab() {
  if (!activeSearchTabId.value) return
  const tab = captureCurrentTab()
  const idx = searchTabs.value.findIndex((t) => t.id === tab.id)
  if (idx >= 0) {
    searchTabs.value[idx] = tab
  } else {
    searchTabs.value.push(tab)
  }
  if (!saveSearchTabs(searchTabs.value, activeSearchTabId.value)) {
    setStatus('分頁狀態過大，已略過儲存列表（快照分頁重開後會重新載入）')
  }
}

function commitSearchTabAfterLoad(expectedTabId: string | null) {
  if (!expectedTabId || activeSearchTabId.value !== expectedTabId) return
  catalogAnalysisByComicId.value = new Map()
  const tab = captureCurrentTab()
  const idx = searchTabs.value.findIndex((t) => t.id === tab.id)
  if (idx >= 0) {
    searchTabs.value[idx] = tab
  } else {
    searchTabs.value.push(tab)
  }
  activeSearchTabId.value = tab.id
  if (!saveSearchTabs(searchTabs.value, activeSearchTabId.value)) {
    setStatus('分頁狀態過大，已略過儲存列表（快照分頁重開後會重新載入）')
  }
}

function startNewSearchTab() {
  persistActiveTab()
  activeSearchTabId.value = crypto.randomUUID()
  scopePinnedForNextSearch.value = false
  searchScope.value = null
  activeSnapshot.value = null
  keyword.value = ''
  clearBrowsePageCache()
  // 切換到新搜尋分頁時清空舊快取，避免新搜尋沿用前一個分頁結果。
  allComics.value = []
  comics.value = []
  totalCount.value = 0
  totalCountRefined.value = false
  totalPages.value = 1
  viewPage.value = 1
  serverPage.value = 1
  serverChunkBase.value = 1
  browseKind.value = 'none'
  pageSize.value = loadSavedPageSize()
  catalogAnalysisByComicId.value = new Map()
  syncBrowsePageCacheToContext()
}

async function reloadSnapshotTabFromStorage(tab: MobileSearchTab) {
  const path = tab.snapshotPath
  if (!path) return
  loading.value = true
  loadingHint.value = '正在載入快照…'
  try {
    const result = await searchSnapshotFile(path, tab.keyword.trim())
    if (activeSearchTabId.value !== tab.id) return
    allComics.value = result.comics
    totalCount.value = result.total
    viewPage.value = Math.min(tab.currentPage, Math.max(1, Math.ceil(result.total / pageSize.value)))
    serverPage.value = viewPage.value
    serverChunkBase.value = 1
    applyClientView()
    clearStatus()
    const idx = searchTabs.value.findIndex((t) => t.id === tab.id)
    if (idx >= 0) {
      searchTabs.value[idx] = captureCurrentTab()
    }
  } catch (e) {
    setStatus(e)
    allComics.value = []
    comics.value = []
  } finally {
    loading.value = false
    loadingHint.value = ''
  }
  restoreSearchScrollTop(tab.scrollTop)
}

function restoreTabServerPaging(tab: MobileSearchTab) {
  totalCountRefined.value = tab.totalCountRefined ?? false
  if (tab.browseKind === 'snapshot') {
    serverPage.value = viewPage.value
    serverChunkBase.value = 1
    return
  }
  if (typeof tab.serverChunkBase === 'number' && typeof tab.serverPage === 'number') {
    serverChunkBase.value = tab.serverChunkBase
    serverPage.value = tab.serverPage
    return
  }
  if (tab.browseKind !== 'none' && totalCount.value > 0) {
    const [spStart] = serverPageRangeForView(viewPage.value)
    serverChunkBase.value = spStart
    serverPage.value = spStart
    return
  }
  serverChunkBase.value = 1
  serverPage.value = 1
}

function browseContextKey(): string {
  const tabId = activeSearchTabId.value ?? ''
  return [
    tabId,
    browseKind.value,
    tabStoredKeyword(browseKind.value, keyword.value),
    categoryBrowseCateId.value ?? '',
    rankingCateId.value ?? '',
    activeTopCategory.value,
    searchScope.value?.filePath ?? '',
    searchScope.value?.cateId ?? '',
    String(pageSize.value),
  ].join('|')
}

/** 分頁或搜尋條件變更時清空官網頁快取，避免關鍵詞／分類分頁共用 Map */
function syncBrowsePageCacheToContext() {
  const key = browseContextKey()
  if (key === browsePageCacheKey) return
  browsePageCache.clear()
  browsePageCacheKey = key
  browseFetchGeneration++
}

function resetSearchSession() {
  activeSearchTabId.value = null
  allComics.value = []
  comics.value = []
  totalCount.value = 0
  totalCountRefined.value = false
  totalPages.value = 1
  viewPage.value = 1
  serverPage.value = 1
  serverChunkBase.value = 1
  browseKind.value = 'none'
  keyword.value = ''
  searchScope.value = null
  activeSnapshot.value = null
  activeTopCategory.value = ''
  categoryBrowseCateId.value = null
  rankingCateId.value = null
  scopePinnedForNextSearch.value = false
  catalogAnalysisByComicId.value = new Map()
  loading.value = false
  loadingHint.value = ''
  clearStatus()
}

function restoreTab(tab: MobileSearchTab) {
  scopePinnedForNextSearch.value = false
  if (tab.searchScope?.filePath) {
    searchScope.value = { ...tab.searchScope }
    syncSearchScopeFromHeaders()
  } else {
    searchScope.value = null
  }
  browseKind.value = tab.browseKind
  keyword.value = tabStoredKeyword(tab.browseKind, tab.keyword)
  activeTopCategory.value = tab.activeTopCategory
  categoryBrowseCateId.value = tab.categoryBrowseCateId
  rankingCateId.value = tab.rankingCateId
  activeSnapshot.value =
    tab.snapshotPath !== null
      ? (categoryHeaders.value.find((h) => h.filePath === tab.snapshotPath) ?? {
          filePath: tab.snapshotPath,
          label: tab.snapshotLabel ?? tab.searchScope?.label ?? '快照',
          cateId: tab.searchScope?.cateId ?? null,
        })
      : null
  viewPage.value = tab.currentPage
  totalPages.value = tab.totalPages
  totalCount.value = tab.totalCount
  sortOrder.value = tab.sortOrder
  const tabPageSize = tab.pageSize
  pageSize.value =
    typeof tabPageSize === 'number' &&
    PAGE_SIZE_OPTIONS.includes(tabPageSize as (typeof PAGE_SIZE_OPTIONS)[number])
      ? tabPageSize
      : loadSavedPageSize()
  gridLayout.value = tab.gridLayout
  catalogAnalysisByComicId.value = new Map(tab.catalogAnalysisEntries ?? [])
  subNav.value = 'search'
  restoreTabServerPaging(tab)
  syncBrowsePageCacheToContext()

  const finishRestore = () => restoreSearchScrollTop(tab.scrollTop)

  if (tab.browseKind === 'snapshot' && tab.snapshotPath && tab.allComics.length === 0) {
    allComics.value = []
    comics.value = []
    void reloadSnapshotTabFromStorage(tab).then(finishRestore)
    return
  }

  allComics.value = [...tab.allComics]
  recoverBrowseKindIfNeeded()
  const effectiveKind = browseKind.value
  const needsReload =
    effectiveKind !== 'none' &&
    effectiveKind !== 'snapshot' &&
    effectiveKind !== 'keyword' &&
    (tab.allComics.length === 0 || tab.browseKind !== effectiveKind)
  if (needsReload) {
    totalCount.value = 0
    comics.value = []
    void loadBrowseFromStart().then(finishRestore)
    return
  }
  if (browseKind.value === 'snapshot') {
    applyClientView()
    finishRestore()
  } else if (browseKind.value !== 'none' && totalCount.value > 0) {
    if (tab.allComics.length > 0) {
      applyClientView()
      finishRestore()
    } else {
      clearBrowsePageCache()
      void goViewPage(viewPage.value, true).then(finishRestore)
    }
  } else {
    applyClientView()
    finishRestore()
  }
}

function selectSearchTab(id: string) {
  if (id === activeSearchTabId.value) return
  persistActiveTab()
  const tab = searchTabs.value.find((t) => t.id === id)
  if (!tab) return
  activeSearchTabId.value = id
  restoreTab(tab)
  saveSearchTabs(searchTabs.value, activeSearchTabId.value)
}

function closeSearchTab(id: string) {
  const idx = searchTabs.value.findIndex((t) => t.id === id)
  if (idx < 0) return
  const closingActive = activeSearchTabId.value === id
  searchTabs.value.splice(idx, 1)
  if (closingActive) {
    clearBrowsePageCache()
    const next = searchTabs.value[idx] ?? searchTabs.value[idx - 1]
    if (next) {
      activeSearchTabId.value = next.id
      restoreTab(next)
    } else {
      resetSearchSession()
    }
    saveSearchTabs(searchTabs.value, activeSearchTabId.value)
  } else {
    saveSearchTabs(searchTabs.value, activeSearchTabId.value)
  }
}

function toggleSearchTabBookmark(tabId: string) {
  if (isTabBookmarked(tabId)) {
    favoriteSearchTabs.value = favoriteSearchTabs.value.filter((b) => b.sourceTabId !== tabId)
  } else {
    const tab = searchTabs.value.find((t) => t.id === tabId)
    if (!tab) return
    favoriteSearchTabs.value = [...favoriteSearchTabs.value, mobileTabToBookmark(tab)]
  }
  saveFavoriteSearchTabs(favoriteSearchTabs.value)
}

function openFavoriteTabBookmark(bookmark: SearchTabBookmark) {
  const tab = bookmarkToMobileTab(bookmark)
  searchTabs.value.push(tab)
  activeSearchTabId.value = tab.id
  restoreTab(tab)
  saveSearchTabs(searchTabs.value, activeSearchTabId.value)
  subNav.value = 'search'
  activeTab.value = 'home'
}

function openFavoriteComic(id: number) {
  const c = favoriteComics.value.find((x) => x.id === id)
  if (c) void onDetail(c.id)
}

function toggleFavoritesMenu(ev: Event) {
  ev.stopPropagation()
  const wasOpen = favoritesMenuOpen.value
  closeMenus()
  favoritesMenuOpen.value = !wasOpen
  expandedNavParent.value = null
  readMenuOpen.value = false
}

function openFavoritesSection(section: FavoritesSection) {
  favoritesSection.value = section
  favoritesMenuOpen.value = false
  expandedNavParent.value = null
  subNav.value = 'favorites'
  if (section === 'comics') favViewPage.value = 1
}

const snapshotListLoading = ref(false)
const snapshotResumeShowing = ref(false)
const snapshotResumeLoading = ref(false)
const snapshotResumeCandidates = ref<SnapshotResumeCandidate[]>([])
const snapshotScanBusy = ref(false)
const snapshotScanStatus = ref('')

async function openSnapshotUpdateDialog() {
  if (!categoryDir.value.trim()) {
    setStatus('請先在設定指定「讀取分類目錄（快照）」')
    return
  }
  snapshotResumeShowing.value = true
  snapshotResumeLoading.value = true
  snapshotResumeCandidates.value = []
  try {
    const list = await listSnapshotResumeCandidates(categoryDir.value)
    if (list.length === 0) {
      snapshotResumeShowing.value = false
      setStatus('沒有可接續掃描的快照')
      return
    }
    snapshotResumeCandidates.value = list
  } catch (e) {
    snapshotResumeShowing.value = false
    setStatus(e)
  } finally {
    snapshotResumeLoading.value = false
  }
}

async function onSnapshotResumeConfirm(payload: {
  selectedPaths: string[]
  strategy: SnapshotResumeStrategy
}) {
  if (!categoryDir.value.trim() || payload.selectedPaths.length === 0) return
  snapshotScanBusy.value = true
  snapshotScanStatus.value = '準備掃描…'
  try {
    const { completed, failed, failures } = await runSnapshotResumeQueue(
      categoryDir.value,
      snapshotResumeCandidates.value,
      payload.selectedPaths,
      payload.strategy,
      (p) => {
        snapshotScanStatus.value =
          p.phase === '排隊'
            ? `${p.phase} ${p.index}/${p.totalItems}：${p.label}`
            : `${p.label}：${p.phase} ${p.current}/${p.total}（${p.matchedCount} 本）`
      },
    )
    await refreshSnapshotList()
    if (completed > 0) {
      const failHint =
        failures.length > 0
          ? `\n失敗：${failures.map((f) => `${f.label}（${f.message}）`).join('；')}`
          : ''
      setStatus(`快照掃描完成 ${completed} 項${failed > 0 ? `，失敗 ${failed} 項` : ''}${failHint}`)
    } else if (!snapshotScanStatus.value.includes('取消')) {
      const detail =
        failures.length > 0
          ? failures.map((f) => `${f.label}：${f.message}`).join('\n')
          : '排隊掃描未成功完成任何項目'
      setStatus(detail)
    }
  } catch (e) {
    setStatus(e)
  } finally {
    snapshotScanBusy.value = false
    snapshotScanStatus.value = ''
  }
}

function cancelSnapshotUpdateScan() {
  cancelSnapshotScan()
  snapshotScanStatus.value = '正在取消…'
}

function openSnapshotList() {
  closeMenus()
  expandedNavParent.value = null
  subNav.value = 'snapshots'
  if (categoryDir.value.trim()) {
    void refreshSnapshotList()
  }
}

function snapshotHeaderDate(header: SnapshotCategoryHeader): Date | null {
  if (header.modifiedMs != null && header.modifiedMs > 0) {
    const d = new Date(header.modifiedMs)
    if (!Number.isNaN(d.getTime())) return d
  }
  if (header.savedAt) {
    const d = new Date(header.savedAt)
    if (!Number.isNaN(d.getTime())) return d
  }
  return null
}

function snapshotHeaderMeta(header: SnapshotCategoryHeader): string {
  const parts = [`共 ${header.totalCount} 本`]
  const d = snapshotHeaderDate(header)
  if (d) parts.push(d.toLocaleString())
  return parts.join(' · ')
}

async function refreshSnapshotList() {
  if (!categoryDir.value.trim()) return
  snapshotListLoading.value = true
  try {
    await refreshCategoryHeaders()
  } finally {
    snapshotListLoading.value = false
  }
}

async function onSnapshotListItemClick(header: SnapshotCategoryHeader) {
  if (!categoryDir.value.trim()) {
    setStatus('請先在設定指定「讀取分類目錄（快照）」')
    return
  }
  await runSnapshotSearch(header, { newTab: true })
}

function goSettingsForSnapshot() {
  closeMenus()
  activeTab.value = 'settings'
}

function comicInSearchFromComic(c: Comic): ComicInSearch {
  return {
    id: c.id,
    title: c.title,
    titleHtml: c.title,
    cover: c.cover,
    additionalInfo: comicListMeta.value.get(c.id) ?? '',
    isDownloaded: c.isDownloaded ?? false,
  }
}

function buildDownloadTaskSeedFromSearch(c: ComicInSearch): DownloadTaskSeed {
  return {
    id: c.id,
    title: c.title,
    cover: c.cover,
    imageCount: null,
    category: null,
  }
}

function buildDownloadTaskSeedFromComic(c: Comic): DownloadTaskSeed {
  return {
    id: c.id,
    title: c.title,
    cover: c.cover,
    imageCount: c.imageCount,
    category: c.category,
  }
}

function resolveDownloadTaskSeed(comicId: number): DownloadTaskSeed {
  const fromCurrent = comics.value.find((c) => c.id === comicId)
  if (fromCurrent) return buildDownloadTaskSeedFromSearch(fromCurrent)
  const fromAll = allComics.value.find((c) => c.id === comicId)
  if (fromAll) return buildDownloadTaskSeedFromSearch(fromAll)
  if (pickedComic.value && pickedComic.value.id === comicId) {
    return buildDownloadTaskSeedFromComic(pickedComic.value)
  }
  return {
    id: comicId,
    title: `comic-${comicId}`,
    cover: null,
    category: null,
    imageCount: null,
  }
}

function onDetailToggleFavorite() {
  if (!pickedComic.value) return
  toggleFavoriteComic(comicInSearchFromComic(pickedComic.value))
}

function toggleReadMenu(ev: Event) {
  ev.stopPropagation()
  const wasOpen = readMenuOpen.value
  closeMenus()
  readMenuOpen.value = !wasOpen
  favoritesMenuOpen.value = false
  expandedNavParent.value = null
}

async function startOnlineRead(comicId?: number) {
  const id = comicId ?? selectedComicId.value
  if (!id) return
  try {
    readingComic.value = await getComic(id)
    onlineReadingActive.value = true
    readMode.value = 'online'
    subNav.value = 'read'
  } catch (e) {
    setStatus(e)
  }
}

function stopOnlineReading() {
  onlineReadingActive.value = false
  readerFullscreenActive.value = false
  subNav.value = 'detail'
}

function pauseOnlineReading() {
  onlineReadingActive.value = false
  readerFullscreenActive.value = false
}

function selectReadMode(mode: ReadMode) {
  readMenuOpen.value = false
  subNav.value = 'read'
  if (mode === 'local') {
    pauseOnlineReading()
  }
  readMode.value = mode
  if (mode === 'online' && selectedComicId.value) {
    void startOnlineRead(selectedComicId.value)
  }
}

function extractInvokeErrorObject(o: Record<string, unknown>): string | null {
  const errMessage =
    typeof o.err_message === 'string'
      ? o.err_message
      : typeof o.errMessage === 'string'
        ? o.errMessage
        : null
  if (errMessage) {
    const title =
      typeof o.err_title === 'string'
        ? o.err_title
        : typeof o.errTitle === 'string'
          ? o.errTitle
          : ''
    return title ? `${title}：${errMessage}` : errMessage
  }
  if (typeof o.message === 'string') {
    const raw = o.message.trim()
    if (raw.startsWith('{')) {
      try {
        const inner = JSON.parse(raw) as Record<string, unknown>
        const nested = extractInvokeErrorObject(inner)
        if (nested) return nested
      } catch {
        /* use raw message */
      }
    }
    return raw
  }
  return null
}

function formatStatusMessage(msg: unknown): string {
  if (msg == null) return ''
  if (typeof msg === 'string') return msg
  if (msg instanceof Error) {
    const fromMessage = extractInvokeErrorObject({ message: msg.message })
    if (fromMessage) return fromMessage
    return msg.message || '未知錯誤'
  }
  if (typeof msg === 'object') {
    const extracted = extractInvokeErrorObject(msg as Record<string, unknown>)
    if (extracted) return extracted
    try {
      return JSON.stringify(msg)
    } catch {
      /* fall through */
    }
  }
  return String(msg)
}

function setStatus(msg: unknown) {
  const text = formatStatusMessage(msg)
  statusMessage.value = text
  showStatusLine.value = text.length > 0
}

function clearStatus() {
  statusMessage.value = ''
  showStatusLine.value = false
}

function resolveCategoryCateId(): number | undefined {
  if (categoryBrowseCateId.value !== null) return categoryBrowseCateId.value
  const root = activeTopCategory.value.split(' / ')[0] ?? ''
  return topCategories.find((c) => c.label === root)?.cateId
}

function resolveTopCategoryItem(): SiteCategoryItem | undefined {
  const root = activeTopCategory.value.split(' / ')[0] ?? ''
  return topCategories.find((c) => c.label === root)
}

function resolveBrowseKindFromContext(): BrowseKind {
  if (activeSnapshot.value?.filePath) return 'snapshot'
  // 已在官網分類／榜單／快照瀏覽時，搜尋欄文字僅供下一次搜尋，不可覆寫 browseKind
  const lockedKinds: BrowseKind[] = ['home', 'albums', 'category', 'ranking', 'snapshot']
  if (lockedKinds.includes(browseKind.value)) return browseKind.value
  if (keyword.value.trim()) return 'keyword'
  if (rankingCateId.value !== null) return 'ranking'
  if (categoryBrowseCateId.value !== null) return 'category'

  const top = resolveTopCategoryItem()
  if (!top) return 'none'
  if (top.browse === 'home') return 'home'
  if (top.browse === 'albums') return 'albums'
  if (top.browse === 'ranking') return 'ranking'
  if (top.cateId !== undefined) return 'category'
  return 'none'
}

/** 修正舊分頁／錯誤狀態：「更新」必須走 albums.html，不可誤用首頁 / */
function recoverBrowseKindIfNeeded() {
  const resolved = resolveBrowseKindFromContext()
  if (resolved === 'none') return
  if (browseKind.value !== resolved) {
    browseKind.value = resolved
    if (resolved === 'albums' || resolved === 'home' || resolved === 'ranking') {
      categoryBrowseCateId.value = null
    }
  }
}

function syncSearchScopeFromHeaders() {
  if (!canUseCategoryScope.value) {
    searchScope.value = null
    return
  }
  if (!searchScope.value?.filePath) return
  const found = categoryHeaders.value.find((h) => h.filePath === searchScope.value!.filePath)
  if (!found) {
    searchScope.value = null
    return
  }
  searchScope.value = {
    label: found.label,
    cateId: found.cateId ?? null,
    filePath: found.filePath,
  }
}

function applyClientView() {
  if (!activeSearchTabId.value || browseKind.value === 'none') {
    comics.value = []
    totalPages.value = 1
    return
  }
  const sorted = sortSearchComics(allComics.value, sortOrder.value)
  for (const c of sorted) rememberComicMeta(c)

  if (browseKind.value === 'snapshot') {
    totalCount.value = sorted.length
  } else if (sorted.length === 0) {
    totalCount.value = 0
  }

  if (totalCount.value <= 0) {
    comics.value = sorted
    totalPages.value = 1
    return
  }

  totalPages.value = Math.max(1, Math.ceil(totalCount.value / pageSize.value))
  if (viewPage.value > totalPages.value) viewPage.value = totalPages.value
  if (viewPage.value < 1) viewPage.value = 1

  const globalStart = (viewPage.value - 1) * pageSize.value
  const globalEnd =
    totalCount.value > 0
      ? Math.min(globalStart + pageSize.value, totalCount.value)
      : globalStart + pageSize.value
  const wantedCount = Math.max(0, globalEnd - globalStart)

  if (browseKind.value === 'snapshot') {
    comics.value = sorted.slice(globalStart, globalStart + wantedCount)
    return
  }

  const serverOffset = (serverChunkBase.value - 1) * SERVER_LIST_PAGE_SIZE
  const localStart = globalStart - serverOffset
  if (localStart < 0 || wantedCount <= 0) {
    comics.value = []
    return
  }
  const pageItems = sliceMergedView(allComics.value, localStart, wantedCount)
  comics.value = sortSearchComics(pageItems, sortOrder.value)
  for (const c of pageItems) rememberComicMeta(c)
}

function effectiveTotalCount(result: SearchResult): number {
  if (result.totalCount > 0) return result.totalCount
  if (result.comics.length > 0) {
    return Math.max(
      result.comics.length,
      (Math.max(result.totalPage, 1) - 1) * SERVER_LIST_PAGE_SIZE + result.comics.length,
    )
  }
  return 0
}

function refineTotalCount(provenTotal: number) {
  if (provenTotal < 0) return
  if (provenTotal === 0 && totalCount.value > 0) return
  const prev = totalCount.value
  const next = prev > 0 ? Math.min(prev, provenTotal) : provenTotal
  if (next === prev) return
  if (prev > 0 && next < prev) totalCountRefined.value = true
  totalCount.value = next
}

function shrinkTotalCountFromServerPage(serverPageNum: number, result: SearchResult) {
  if (browseKind.value === 'snapshot') return
  const pageStart = (serverPageNum - 1) * SERVER_LIST_PAGE_SIZE
  const count = result.comics.length
  if (count === 0) {
    refineTotalCount(pageStart)
  } else if (count < SERVER_LIST_PAGE_SIZE) {
    refineTotalCount(pageStart + count)
  }
}

function ingestSearchMetadata(result: SearchResult, requestedServerPage?: number) {
  syncBrowsePageCacheToContext()
  const cachePage = requestedServerPage ?? result.currentPage
  browsePageCache.set(cachePage, result)
  const total = effectiveTotalCount(result)
  if (!totalCountRefined.value || totalCount.value <= 0) {
    totalCount.value = Math.max(totalCount.value, total)
  }
  shrinkTotalCountFromServerPage(cachePage, result)
}

function applySearchResult(
  result: SearchResult,
  kind: BrowseKind,
  _label: string,
  loadedPage?: number,
  expectedTabId?: string | null,
) {
  const tabId = expectedTabId ?? activeSearchTabId.value
  if (!tabId || activeSearchTabId.value !== tabId || browseKind.value === 'none') return
  allComics.value = result.comics
  const resolvedPage = loadedPage ?? result.currentPage
  serverPage.value = resolvedPage
  serverChunkBase.value = resolvedPage
  ingestSearchMetadata(result, loadedPage)
  browseKind.value = kind === 'none' ? resolveBrowseKindFromContext() : kind
  applyClientView()
  clearStatus()
  commitSearchTabAfterLoad(expectedTabId ?? activeSearchTabId.value)
}

function serverPageRangeForView(vp: number): [number, number] {
  const startIdx = (vp - 1) * pageSize.value
  const endIdx =
    totalCount.value > 0
      ? Math.min(startIdx + pageSize.value, totalCount.value)
      : startIdx + pageSize.value
  if (endIdx <= startIdx) return [1, 1]
  const spStart = Math.floor(startIdx / SERVER_LIST_PAGE_SIZE) + 1
  const spEnd = Math.floor((endIdx - 1) / SERVER_LIST_PAGE_SIZE) + 1
  return [spStart, spEnd]
}

function maxServerListPage(): number {
  if (totalCount.value <= 0) return 1
  return Math.max(1, Math.ceil(totalCount.value / SERVER_LIST_PAGE_SIZE))
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function clearBrowsePageCache() {
  browsePageCache.clear()
  browseFetchGeneration++
}

/** 合併官網多頁；僅在「跨頁邊界」去掉重複首筆，頁內與快照/官網 HTML 一致 */
function mergeServerPageComics(spStart: number, spEnd: number): ComicInSearch[] {
  syncBrowsePageCacheToContext()
  const merged: ComicInSearch[] = []
  for (let p = spStart; p <= spEnd; p++) {
    const cached = browsePageCache.get(p)
    if (cached === undefined) continue
    let pageComics = cached.comics
    if (
      merged.length > 0 &&
      pageComics.length > 0 &&
      merged[merged.length - 1]!.id === pageComics[0]!.id
    ) {
      pageComics = pageComics.slice(1)
    }
    merged.push(...pageComics)
  }
  return merged
}

/** 依全局索引從合併段切 UI 頁（不去重，與快照/官網列表一致） */
function sliceMergedView(merged: ComicInSearch[], offset: number, wantedCount: number): ComicInSearch[] {
  return merged.slice(offset, offset + wantedCount)
}

function cachedChunkCoversView(vp: number): boolean {
  if (allComics.value.length === 0) return false
  const startIdx = (vp - 1) * pageSize.value
  const endIdx =
    totalCount.value > 0
      ? Math.min(startIdx + pageSize.value, totalCount.value)
      : startIdx + pageSize.value
  const wantedCount = endIdx - startIdx
  if (wantedCount <= 0) return false
  const [spStart] = serverPageRangeForView(vp)
  if (serverChunkBase.value !== spStart) return false
  const offset = startIdx - (spStart - 1) * SERVER_LIST_PAGE_SIZE
  return sliceMergedView(allComics.value, offset, wantedCount).length >= wantedCount
}

async function fetchBrowseResult(page: number): Promise<SearchResult> {
  recoverBrowseKindIfNeeded()
  switch (browseKind.value) {
    case 'home':
      return browseHome(page)
    case 'albums':
      return browseAlbumsList(page)
    case 'category': {
      const cateId = resolveCategoryCateId()
      if (cateId !== undefined) return browseByCategory(cateId, page)
      const top = resolveTopCategoryItem()
      if (top?.browse === 'albums') return browseAlbumsList(page)
      if (top?.browse === 'home') return browseHome(page)
      return browseHome(page)
    }
    case 'ranking':
      return browseRanking('Day', rankingCateId.value, page)
    case 'keyword':
      return searchByKeyword(keyword.value.trim(), page, searchScope.value?.cateId ?? null)
    default:
      throw new Error('請先選擇分類或搜尋')
  }
}

async function refreshSettings() {
  const s = await getMobileSettings()
  categoryDir.value = s.categoryDirectory ?? ''
  downloadDir.value = s.downloadDirectory ?? ''
  if (categoryDir.value) {
    await refreshCategoryHeaders()
  }
}

async function refreshCategoryHeaders() {
  if (!categoryDir.value) {
    categoryHeaders.value = []
    searchScope.value = null
    return
  }
  try {
    categoryHeaders.value = await listSnapshotCategoryHeaders(categoryDir.value)
    syncSearchScopeFromHeaders()
    if (!canUseCategoryScope.value) {
      searchScope.value = null
    }
  } catch (e) {
    setStatus(e)
    categoryHeaders.value = []
    searchScope.value = null
  }
}

async function loadBrowseViewChunk(vp: number) {
  if (loading.value) return
  if (browseKind.value === 'none' || !activeSearchTabId.value) {
    return
  }

  syncBrowsePageCacheToContext()
  const tabIdAtStart = activeSearchTabId.value
  const gen = browseFetchGeneration
  const browseLoadAborted = () =>
    gen !== browseFetchGeneration ||
    !tabIdAtStart ||
    activeSearchTabId.value !== tabIdAtStart ||
    browseKind.value === 'none'
  const startIdx = (vp - 1) * pageSize.value
  const endIdx =
    totalCount.value > 0
      ? Math.min(vp * pageSize.value, totalCount.value)
      : startIdx + pageSize.value
  if (totalCount.value > 0 && startIdx >= totalCount.value) return

  const wantedCount = endIdx - startIdx
  const [spStart, spEndInitial] = serverPageRangeForView(vp)
  const offset = startIdx - (spStart - 1) * SERVER_LIST_PAGE_SIZE
  // 至少多載 1 個官網頁，供邊界去重後補滿本 UI 頁
  let spEnd = spEndInitial + 1

  loading.value = true
  loadingHint.value = '正在載入…'
  try {
    let hadNetworkFetch = false
    while (true) {
      if (browseLoadAborted()) return

      const safeEnd = totalCount.value > 0 ? Math.min(spEnd, maxServerListPage()) : spEnd
      for (let p = spStart; p <= safeEnd; p++) {
        if (totalCount.value > 0 && p > maxServerListPage()) break
        if (browsePageCache.has(p)) continue
        if (hadNetworkFetch) await sleep(PAGE_FETCH_DELAY_MS)
        if (browseLoadAborted()) return
        loadingHint.value = `載入第 ${p} 頁…`
        const result = await fetchBrowseResult(p)
        if (browseLoadAborted()) return
        ingestSearchMetadata(result, p)
        hadNetworkFetch = true
      }

      if (totalCount.value <= 0) {
        const cachedFirst = browsePageCache.get(1)
        if (cachedFirst !== undefined && cachedFirst.comics.length > 0) {
          totalCount.value = effectiveTotalCount(cachedFirst)
        }
      }

      const merged = mergeServerPageComics(spStart, safeEnd)
      const slice = sliceMergedView(merged, offset, wantedCount)
      if (slice.length >= wantedCount) {
        if (browseLoadAborted()) return
        allComics.value = merged
        serverChunkBase.value = spStart
        serverPage.value = safeEnd
        applyClientView()
        finishBrowseViewLoadStatus()
        commitSearchTabAfterLoad(tabIdAtStart)
        return
      }

      if (totalCount.value > 0 && spEnd >= maxServerListPage()) {
        if (browseLoadAborted()) return
        allComics.value = merged
        serverChunkBase.value = spStart
        serverPage.value = safeEnd
        applyClientView()
        finishBrowseViewLoadStatus()
        commitSearchTabAfterLoad(tabIdAtStart)
        return
      }

      spEnd++
      if (totalCount.value <= 0 && spEnd > spStart + 20) break
    }
  } catch (e) {
    if (browseLoadAborted()) return
    setStatus(e)
    allComics.value = []
    comics.value = []
  } finally {
    loading.value = false
    loadingHint.value = ''
  }
}

async function loadServerPage(page: number) {
  if (loading.value) return
  if (browseKind.value === 'none') {
    setStatus('請先選擇分類或搜尋')
    return
  }

  syncBrowsePageCacheToContext()
  const tabIdAtStart = activeSearchTabId.value
  loading.value = true
  loadingHint.value = '正在載入…'
  try {
    loadingHint.value = `載入第 ${page} 頁…`
    const result = await fetchBrowseResult(page)
    applySearchResult(result, browseKind.value, activeTopCategory.value || '列表', page, tabIdAtStart)
  } catch (e) {
    setStatus(e)
    allComics.value = []
    comics.value = []
  } finally {
    loading.value = false
    loadingHint.value = ''
  }
}

async function ensureServerDataForView(vp: number) {
  if (!activeSearchTabId.value || browseKind.value === 'none') return
  if (cachedChunkCoversView(vp)) {
    applyClientView()
    return
  }
  await loadBrowseViewChunk(vp)
}

/** 新分頁／新搜尋：只載入目前 UI 頁需要的官網頁 */
async function loadBrowseFromStart() {
  clearBrowsePageCache()
  totalCountRefined.value = false
  viewPage.value = 1
  serverPage.value = 1
  serverChunkBase.value = 1
  await loadBrowseViewChunk(1)
}

async function goViewPage(vp: number, force = false) {
  if (loading.value) return

  syncBrowsePageCacheToContext()

  if (browseKind.value === 'snapshot') {
    if (totalCount.value <= 0) return
    const maxVp = Math.max(1, Math.ceil(totalCount.value / pageSize.value))
    const target = Math.max(1, Math.min(vp, maxVp))
    if (!force && target === viewPage.value && comics.value.length > 0) return
    viewPage.value = target
    serverPage.value = target
    applyClientView()
    persistActiveTab()
    return
  }

  if (totalCount.value <= 0) {
    const cachedFirst = browsePageCache.get(1)
    if (cachedFirst !== undefined && cachedFirst.comics.length > 0) {
      totalCount.value = effectiveTotalCount(cachedFirst)
    } else if (!force) {
      return
    }
  }

  const maxVp =
    totalCount.value > 0 ? Math.max(1, Math.ceil(totalCount.value / pageSize.value)) : Math.max(1, vp)
  const target = Math.max(1, Math.min(vp, maxVp))
  if (!force && target === viewPage.value && comics.value.length > 0) return

  viewPage.value = target
  recoverBrowseKindIfNeeded()
  if (browseKind.value === 'none') {
    setStatus('目前搜尋狀態已失效，請重新執行搜尋')
    return
  }

  await ensureServerDataForView(target)
  persistActiveTab()
}

/** 僅在搜索結果底部分頁列切頁且頁碼有變更時使用（會回彈列表至頂部） */
async function goSearchResultsPage(vp: number) {
  const before = viewPage.value
  await goViewPage(vp)
  if (viewPage.value !== before) {
    scrollSearchResultsToTop()
  }
}

function isTopRowActive(item: SiteCategoryItem): boolean {
  if (item.children?.length) {
    return (
      expandedNavParent.value === item.label ||
      activeTopCategory.value === item.label ||
      activeTopCategory.value.startsWith(`${item.label} / `)
    )
  }
  return activeTopCategory.value === item.label
}

function onTopNavClick(item: SiteCategoryItem) {
  closeMenus()
  if (item.children && item.children.length > 0) {
    expandedNavParent.value = expandedNavParent.value === item.label ? null : item.label
    favoritesMenuOpen.value = false
    void nextTick().then(updateCategoryMenuPosition)
    return
  }
  expandedNavParent.value = null
  // 已在「更新／首頁」時再點一次：直接重載官網列表（避免舊分頁卡在尚無結果）
  if (
    isTopRowActive(item) &&
    (item.browse === 'albums' || item.browse === 'home')
  ) {
    activeTopCategory.value = item.label
    browseKind.value = item.browse === 'albums' ? 'albums' : 'home'
    categoryBrowseCateId.value = null
    rankingCateId.value = null
    activeSnapshot.value = null
    searchScope.value = null
    keyword.value = ''
    subNav.value = 'search'
    if (!activeSearchTabId.value) {
      activeSearchTabId.value = crypto.randomUUID()
    }
    void loadBrowseFromStart()
    return
  }
  void runLeafBrowse(item)
}

async function runLeafBrowse(item: SiteCategoryItem) {
  closeMenus()
  keyword.value = ''
  startNewSearchTab()
  categoryBrowseCateId.value = null
  activeSnapshot.value = null
  activeTopCategory.value = item.label
  subNav.value = 'search'
  viewPage.value = 1
  serverPage.value = 1
  serverChunkBase.value = 1

  if (item.browse === 'home') {
    browseKind.value = 'home'
    await loadBrowseFromStart()
    return
  }
  if (item.browse === 'albums') {
    browseKind.value = 'albums'
    await loadBrowseFromStart()
    return
  }
  if (item.browse === 'ranking') {
    browseKind.value = 'ranking'
    rankingCateId.value = null
    await loadBrowseFromStart()
    return
  }
  if (item.cateId !== undefined) {
    browseKind.value = 'category'
    searchScope.value = null
    await loadBrowseFromStart()
  }
}

async function onParentCategoryClick(parent: SiteCategoryItem) {
  expandedNavParent.value = null
  await runLeafBrowse(parent)
}

async function onChildCategoryClick(parent: SiteCategoryItem, child: SiteCategoryItem) {
  closeMenus()
  expandedNavParent.value = null
  keyword.value = ''
  startNewSearchTab()
  activeSnapshot.value = null
  subNav.value = 'search'
  viewPage.value = 1
  serverPage.value = 1
  serverChunkBase.value = 1
  categoryBrowseCateId.value = null
  rankingCateId.value = null
  searchScope.value = null

  if (parent.browse === 'ranking') {
    activeTopCategory.value = child.label === '全部分類' ? '排行' : `排行 / ${child.label}`
    browseKind.value = 'ranking'
    rankingCateId.value = child.rankingCateId ?? null
    await loadBrowseFromStart()
    return
  }

  if (child.cateId === undefined) return
  activeTopCategory.value = `${parent.label} / ${child.label}`
  browseKind.value = 'category'
  categoryBrowseCateId.value = child.cateId
  await loadBrowseFromStart()
}

function setSubNav(mode: SubNav) {
  closeMenus()
  subNav.value = mode
}

async function submitKeywordSearch() {
  const kw = keyword.value.trim()
  if (!kw) {
    setStatus('請輸入關鍵字')
    return
  }
  closeMenus()
  expandedNavParent.value = null
  const scopedSearch = scopePinnedForNextSearch.value ? searchScope.value : null
  startNewSearchTab()
  subNav.value = 'search'
  activeTopCategory.value = ''
  categoryBrowseCateId.value = null
  rankingCateId.value = null
  viewPage.value = 1
  serverPage.value = 1
  serverChunkBase.value = 1
  keyword.value = kw

  if (scopedSearch?.filePath) {
    const header = categoryHeaders.value.find((h) => h.filePath === scopedSearch.filePath)
    if (!header) {
      setStatus('分類快照已不存在，改為全站搜索')
      searchScope.value = null
      activeSnapshot.value = null
      browseKind.value = 'keyword'
      await loadBrowseFromStart()
      return
    }
    await runSnapshotSearch(header, { newTab: false, filterKeyword: kw })
    return
  }

  activeSnapshot.value = null
  searchScope.value = null
  browseKind.value = 'keyword'
  await loadBrowseFromStart()
}

async function runSnapshotSearch(
  header: SnapshotCategoryHeader,
  options?: { newTab?: boolean; filterKeyword?: string },
) {
  closeMenus()
  expandedNavParent.value = null
  if (options?.newTab !== false) {
    startNewSearchTab()
  }
  const filterKeyword = options?.filterKeyword ?? ''
  keyword.value = filterKeyword
  const tabIdAtStart = activeSearchTabId.value
  subNav.value = 'search'
  activeSnapshot.value = header
  searchScope.value = {
    label: header.label,
    cateId: header.cateId ?? null,
    filePath: header.filePath,
  }
  activeTopCategory.value = ''
  categoryBrowseCateId.value = null
  rankingCateId.value = null
  browseKind.value = 'snapshot'
  loading.value = true
  loadingHint.value = '正在載入快照（大檔可能需要數十秒）…'
  try {
    const result = await searchSnapshotFile(header.filePath, filterKeyword)
    if (activeSearchTabId.value !== tabIdAtStart) return
    allComics.value = result.comics
    totalCount.value = result.total
    viewPage.value = 1
    serverPage.value = 1
    serverChunkBase.value = 1
    applyClientView()
    clearStatus()
    commitSearchTabAfterLoad(tabIdAtStart)
  } catch (e) {
    setStatus(e)
    allComics.value = []
    comics.value = []
  } finally {
    loading.value = false
    loadingHint.value = ''
  }
}

function onScopeSelect(header: SnapshotCategoryHeader | null) {
  scopeMenuOpen.value = false
  scopePinnedForNextSearch.value = header !== null
  if (header === null) {
    searchScope.value = null
  } else {
    searchScope.value = {
      label: header.label,
      cateId: header.cateId ?? null,
      filePath: header.filePath,
    }
    activeSnapshot.value = header
  }
  // 僅切換「範圍」顯示；開新分頁與載入結果須等使用者按「搜索」
}

function onSortSelect(order: SearchSortOrder) {
  sortOrder.value = order
  saveSortOrder(order)
  sortMenuOpen.value = false
  applyClientView()
  if (isLiveWebsiteBrowse() && !isIdBasedSortOrder(order)) {
    setStatus(LIVE_BROWSE_CUSTOM_SORT_HINT)
  } else {
    clearStatus()
  }
  persistActiveTab()
}

function onFavSortSelect(order: SearchSortOrder) {
  favSortOrder.value = order
  saveFavoritesSortOrder(order)
  favSortMenuOpen.value = false
  if (favViewPage.value > favTotalPages.value) favViewPage.value = favTotalPages.value
  clearStatus()
}

function onPageSizeSelect(size: number) {
  pageSize.value = size
  savePageSize(size)
  pageSizeMenuOpen.value = false
  viewPage.value = 1
  if (browseKind.value === 'snapshot') {
    serverPage.value = 1
    serverChunkBase.value = 1
    applyClientView()
  } else if (browseKind.value !== 'none') {
    syncBrowsePageCacheToContext()
    void ensureServerDataForView(1)
  }
  clearStatus()
  persistActiveTab()
}

function onFavPageSizeSelect(size: number) {
  favPageSize.value = size
  saveFavoritesPageSize(size)
  favPageSizeMenuOpen.value = false
  favViewPage.value = 1
  clearStatus()
}

function onLayoutSelect(layout: GridLayout) {
  gridLayout.value = layout
  saveLayout(layout)
  layoutMenuOpen.value = false
}

function onFavLayoutSelect(layout: GridLayout) {
  favGridLayout.value = layout
  saveFavoritesLayout(layout)
  favLayoutMenuOpen.value = false
}

function goPrevPage() {
  if (viewPage.value > 1) void goSearchResultsPage(viewPage.value - 1)
}

function goNextPage() {
  if (viewPage.value < totalPages.value) void goSearchResultsPage(viewPage.value + 1)
}

function goPage(n: number) {
  if (n < 1 || n > totalPages.value) return
  void goSearchResultsPage(n)
}

function goFavPage(n: number) {
  if (n < 1 || n > favTotalPages.value) return
  if (n === favViewPage.value) return
  favViewPage.value = n
  scrollFavoritesToTop()
}

function goFavPrevPage() {
  if (favViewPage.value > 1) goFavPage(favViewPage.value - 1)
}

function goFavNextPage() {
  if (favViewPage.value < favTotalPages.value) goFavPage(favViewPage.value + 1)
}

function pickFavJumpPage(n: number) {
  favPageJumpOpen.value = false
  goFavPage(n)
}

function pickJumpPage(n: number) {
  pageJumpOpen.value = false
  void goPage(n)
}

function confirmPageGoFromJump() {
  const raw = String(pageGoInput.value ?? '').trim()
  const n = parseInt(raw, 10)
  if (pageGoForFavorites.value) {
    if (!raw || Number.isNaN(n) || n < 1 || n > favTotalPages.value) {
      setStatus(`頁碼須為 1～${favTotalPages.value}`)
      return
    }
    favPageJumpOpen.value = false
    goFavPage(n)
    return
  }
  if (!raw || Number.isNaN(n) || n < 1 || n > totalPages.value) {
    setStatus(`頁碼須為 1～${totalPages.value}`)
    return
  }
  pageJumpOpen.value = false
  void goSearchResultsPage(n)
}

async function downloadAllPage() {
  try {
    if (comics.value.length === 0) {
      setStatus('本頁沒有可下載項目')
      return
    }

    const config = await getConfig()
    if ((config.downloadDir ?? '').trim() === '') {
      setStatus('請先到設定指定下載目錄')
      activeTab.value = 'settings'
      return
    }

    const snapshots = await getDownloadTaskSnapshots()
    const inQueueIds = new Set(
      snapshots
        .filter((task) => task.state === 'Pending' || task.state === 'Downloading' || task.state === 'Paused')
        .map((task) => task.comic.id),
    )

    let enqueued = 0
    let skipped = 0
    let failed = 0
    for (const c of comics.value) {
      if (c.isDownloaded || inQueueIds.has(c.id)) {
        skipped++
        continue
      }
      try {
        await createDownloadTask(buildDownloadTaskSeedFromSearch(c))
        inQueueIds.add(c.id)
        enqueued++
      } catch {
        failed++
      }
    }

    setStatus(`本頁下載：已加入 ${enqueued}，略過 ${skipped}，失敗 ${failed}`)
    if (enqueued > 0) {
      activeTab.value = 'download'
    }
  } catch (e) {
    setStatus(formatStatusMessage(e))
  }
}

function requestDownloadAllPage() {
  if (loading.value || comics.value.length === 0) {
    setStatus('本頁沒有可下載項目')
    return
  }
  downloadAllConfirmOpen.value = true
}

function cancelDownloadAllConfirm() {
  downloadAllConfirmOpen.value = false
}

function confirmDownloadAll() {
  downloadAllConfirmOpen.value = false
  void downloadAllPage()
}

async function downloadAllFavoriteComics() {
  try {
    if (favoriteComics.value.length === 0) {
      setStatus('收藏漫畫沒有可下載項目')
      return
    }

    const config = await getConfig()
    if ((config.downloadDir ?? '').trim() === '') {
      setStatus('請先到設定指定下載目錄')
      activeTab.value = 'settings'
      return
    }

    const snapshots = await getDownloadTaskSnapshots()
    const inQueueIds = new Set(
      snapshots
        .filter((task) => task.state === 'Pending' || task.state === 'Downloading' || task.state === 'Paused')
        .map((task) => task.comic.id),
    )

    let enqueued = 0
    let skipped = 0
    let failed = 0
    for (const c of favoriteComics.value) {
      if (c.isDownloaded || inQueueIds.has(c.id)) {
        skipped++
        continue
      }
      try {
        await createDownloadTask(buildDownloadTaskSeedFromSearch(c))
        inQueueIds.add(c.id)
        enqueued++
      } catch {
        failed++
      }
    }

    setStatus(`收藏全部下載：已加入 ${enqueued}，略過 ${skipped}，失敗 ${failed}`)
    if (enqueued > 0) {
      activeTab.value = 'download'
    }
  } catch (e) {
    setStatus(formatStatusMessage(e))
  }
}

function requestDownloadAllFavoriteComics() {
  if (favoriteComics.value.length === 0) {
    setStatus('收藏漫畫沒有可下載項目')
    return
  }
  downloadAllFavoritesConfirmOpen.value = true
}

function cancelDownloadAllFavoritesConfirm() {
  downloadAllFavoritesConfirmOpen.value = false
}

function confirmDownloadAllFavorites() {
  downloadAllFavoritesConfirmOpen.value = false
  void downloadAllFavoriteComics()
}

async function exportFavoriteArchive() {
  try {
    const dir = await pickCategoryDirectory(false)
    if (!dir) return

    if (favoritesSection.value === 'comics') {
      if (favoriteComics.value.length === 0) {
        setStatus('收藏漫畫尚無紀錄可導出')
        return
      }
      const content = serializeFavoriteArchive(buildFavoriteComicsExportFile(favoriteComics.value))
      const written = await writeCategorySnapshotFile(dir, favoriteComicsArchiveFileName(), content)
      setStatus(`已導出收藏漫畫：${written.split(/[/\\]/).pop() ?? written}`)
      return
    }

    if (favoriteSearchTabs.value.length === 0) {
      setStatus('收藏分頁尚無紀錄可導出')
      return
    }
    const content = serializeFavoriteArchive(buildFavoriteTabsExportFile(favoriteSearchTabs.value))
    const written = await writeCategorySnapshotFile(dir, favoriteTabsArchiveFileName(), content)
    setStatus(`已導出收藏分頁：${written.split(/[/\\]/).pop() ?? written}`)
  } catch (e) {
    setStatus(formatStatusMessage(e))
  }
}

async function importFavoriteArchive() {
  try {
    const path = await pickImportArchiveFile()
    if (!path) return
    const raw = await readImportArchiveFile(path)

    if (favoritesSection.value === 'comics') {
      const comics = parseFavoriteComicsImport(raw)
      favoriteComics.value = comics
      saveFavoriteComics(comics)
      favViewPage.value = 1
      setStatus(`已匯入收藏漫畫：${comics.length} 筆（已覆蓋舊紀錄）`)
      return
    }

    const bookmarks = parseFavoriteTabsImport(raw)
    favoriteSearchTabs.value = bookmarks
    saveFavoriteSearchTabs(bookmarks)
    setStatus(`已匯入收藏分頁：${bookmarks.length} 筆（已覆蓋舊紀錄）`)
  } catch (e) {
    setStatus(formatStatusMessage(e))
  }
}

function requestClearFavorites() {
  if (favoritesSection.value === 'comics') {
    if (favoriteComics.value.length === 0) {
      setStatus('收藏漫畫尚無紀錄')
      return
    }
  } else if (favoriteSearchTabs.value.length === 0) {
    setStatus('收藏分頁尚無紀錄')
    return
  }
  clearFavoritesConfirmOpen.value = true
}

function cancelClearFavoritesConfirm() {
  clearFavoritesConfirmOpen.value = false
}

function confirmClearFavorites() {
  clearFavoritesConfirmOpen.value = false
  if (favoritesSection.value === 'comics') {
    favoriteComics.value = []
    saveFavoriteComics([])
    favViewPage.value = 1
    setStatus('已清除收藏漫畫')
    return
  }
  favoriteSearchTabs.value = []
  saveFavoriteSearchTabs([])
  setStatus('已清除收藏分頁')
}

function getKoreanCandidates() {
  return allComics.value.filter(
    (c) => c.listCateId === 20 || c.listCateId === 21 || /韓|韩/.test(c.title),
  )
}

async function openKoreanDownloadMode() {
  try {
    loading.value = true
    let allSearchComics: ComicInSearch[]
    if (browseKind.value === 'snapshot') {
      loadingHint.value = '韓漫下載模式：正在整理快照搜尋結果…'
      allSearchComics = [...allComics.value]
    } else {
      loadingHint.value = '韓漫下載模式：正在載入第 1 頁…'
      const firstPage = await fetchBrowseResult(1)
      const totalServerPages = Math.max(1, firstPage.totalPage || 1)
      const merged: ComicInSearch[] = [...firstPage.comics]
      for (let page = 2; page <= totalServerPages; page++) {
        loadingHint.value = `韓漫下載模式：正在載入第 ${page}/${totalServerPages} 頁…`
        const result = await fetchBrowseResult(page)
        merged.push(...result.comics)
      }
      const seen = new Set<number>()
      allSearchComics = merged.filter((comic) => {
        if (seen.has(comic.id)) return false
        seen.add(comic.id)
        return true
      })
    }
    const korean = allSearchComics.filter(
      (c) => c.listCateId === 20 || c.listCateId === 21 || /韓|韩/.test(c.title),
    )
    if (korean.length === 0) {
      setStatus('本頁未偵測到韓漫項目')
      return
    }

    const config = await getConfig()
    if ((config.downloadDir ?? '').trim() === '') {
      setStatus('請先到設定指定下載目錄')
      activeTab.value = 'settings'
      return
    }
    koreanModeDialogComics.value = korean
    koreanModeDialogTagLabel.value = keyword.value.trim() || searchScope.value?.label || '韓漫系列'
    koreanModeDialogOpen.value = true
  } catch (e) {
    setStatus(formatStatusMessage(e))
  } finally {
    loading.value = false
    loadingHint.value = ''
  }
}

async function onKoreanModeDialogConfirm(payload: {
  selectedItems: ClassifiedKoreanItem[]
  rangeMin: number
  rangeMax: number
  tagLabel: string
  seriesFolder: string
}) {
  try {
    const snapshots = await getDownloadTaskSnapshots()
    const inQueueIds = new Set(
      snapshots
        .filter((task) => task.state === 'Pending' || task.state === 'Downloading' || task.state === 'Paused')
        .map((task) => task.comic.id),
    )
    const seriesFolder = payload.seriesFolder

    let enqueued = 0
    let skipped = 0
    let failed = 0
    for (const item of payload.selectedItems) {
      const comic = item.comic
      if (comic.isDownloaded || inQueueIds.has(comic.id)) {
        skipped++
        continue
      }
      try {
        await createDownloadTask(buildDownloadTaskSeedFromSearch(comic), seriesFolder)
        inQueueIds.add(comic.id)
        enqueued++
      } catch {
        failed++
      }
    }

    setStatus(`韓漫下載模式：已加入 ${enqueued}，略過 ${skipped}，失敗 ${failed}（資料夾：${seriesFolder}）`)
    if (enqueued > 0) {
      activeTab.value = 'download'
    }
  } catch (e) {
    setStatus(formatStatusMessage(e))
  }
}

async function downloadComic(comicId: number) {
  try {
    const config = await getConfig()
    if ((config.downloadDir ?? '').trim() === '') {
      setStatus('請先到設定指定下載目錄')
      activeTab.value = 'settings'
      return
    }

    const snapshots = await getDownloadTaskSnapshots()
    const existing = snapshots.find(
      (task: DownloadTaskEvent) =>
        task.comic.id === comicId &&
        (task.state === 'Pending' || task.state === 'Downloading' || task.state === 'Paused'),
    )
    if (existing) {
      setStatus(`已在下載佇列中 #${comicId}`)
      activeTab.value = 'download'
      return
    }

    await createDownloadTask(resolveDownloadTaskSeed(comicId))
    setStatus(`已加入下載佇列 #${comicId}`)
    activeTab.value = 'download'
  } catch (e) {
    setStatus(formatStatusMessage(e))
  }
}

const selectedComicId = ref<number | null>(null)
const pickedComic = ref<Comic | null>(null)
const detailLoading = ref(false)
const detailError = ref('')
const detailPanelRef = ref<{ closeSearchMenu?: () => void } | null>(null)

const detailSnapshotSearchAvailable = computed(() => {
  const comic = pickedComic.value
  if (!comic?.category.trim()) return false
  if (!hasCategoryDir.value || categoryHeaders.value.length === 0) return false
  return hasComicCategorySnapshot(comic.category, categoryHeaders.value)
})

async function loadComicDetail(id: number) {
  detailLoading.value = true
  detailError.value = ''
  pickedComic.value = null
  try {
    pickedComic.value = await getComic(id)
  } catch (e) {
    detailError.value = String(e)
  } finally {
    detailLoading.value = false
  }
}

async function onDetail(id: number) {
  selectedComicId.value = id
  const fromList = comics.value.find((c) => c.id === id) ?? allComics.value.find((c) => c.id === id)
  if (fromList) rememberComicMeta(fromList)
  subNav.value = 'detail'
  readMenuOpen.value = false
  clearStatus()
  await loadComicDetail(id)
}

function onRead(id: number) {
  selectedComicId.value = id
  readMenuOpen.value = false
  clearStatus()
  void startOnlineRead(id)
}

function onDetailRead() {
  if (!selectedComicId.value) return
  void startOnlineRead(selectedComicId.value)
}

function onDetailDownload() {
  if (!selectedComicId.value) return
  void downloadComic(selectedComicId.value)
}

async function onDetailTagSearch(tagName: string) {
  closeMenus()
  expandedNavParent.value = null
  keyword.value = tagName
  startNewSearchTab()
  const tabIdAtStart = activeSearchTabId.value
  subNav.value = 'search'
  viewPage.value = 1
  serverPage.value = 1
  serverChunkBase.value = 1

  const scopePath = searchScope.value?.filePath
  if (scopePath) {
    const header = categoryHeaders.value.find((h) => h.filePath === scopePath)
    if (header) {
      await runSnapshotSearch(header, { newTab: false, filterKeyword: tagName })
      return
    }
    searchScope.value = null
  }

  activeSnapshot.value = null
  browseKind.value = 'keyword'
  loading.value = true
  try {
    const result = await searchByTag(tagName, 1, null)
    applySearchResult(result, 'keyword', tagName, undefined, tabIdAtStart)
  } catch (e) {
    setStatus(e)
  } finally {
    loading.value = false
  }
}

async function onDetailSearch(mode: 'global' | 'snapshot') {
  const comic = pickedComic.value
  if (!comic) return
  const searchName = extractComicSearchName(comic.title)
  if (!searchName) {
    setStatus('無法從標題提取漫畫名')
    return
  }
  closeMenus()
  expandedNavParent.value = null
  keyword.value = searchName

  if (mode === 'global') {
    searchScope.value = null
    activeSnapshot.value = null
    activeTopCategory.value = ''
    categoryBrowseCateId.value = null
    rankingCateId.value = null
    startNewSearchTab()
    subNav.value = 'search'
    browseKind.value = 'keyword'
    await loadBrowseFromStart()
    return
  }

  const scope = resolveCategoryScopeFromComicCategory(comic.category)
  if (!scope) {
    setStatus(`無法識別分類「${comic.category}」`)
    return
  }
  const header = findLatestSnapshotHeaderForScope(categoryHeaders.value, scope)
  if (!header) {
    setStatus(`沒有「${scope.label}」的分類快照`)
    return
  }
  await runSnapshotSearch(header, { newTab: true, filterKeyword: searchName })
}

watch(
  () => [subNav.value, selectedComicId.value] as const,
  ([nav, id]) => {
    if (nav === 'detail' && id !== null && pickedComic.value?.id !== id && !detailLoading.value) {
      void loadComicDetail(id)
    }
  },
)

async function onPickCategoryDir() {
  const path = await pickCategoryDirectory()
  if (path) {
    categoryDir.value = path
    await refreshCategoryHeaders()
    setStatus('已讀取分類目錄')
  }
}

async function onPickDownloadDir() {
  const path = await pickDownloadDirectory()
  if (path) {
    downloadDir.value = path
    setStatus('已設定下載目錄')
  }
}

onMounted(() => {
  initLocalReadOnAppLaunch()
  const saved = loadSearchTabs()
  searchTabs.value = saved.tabs
  if (saved.activeId) {
    const tab = saved.tabs.find((t) => t.id === saved.activeId)
    if (tab) {
      activeSearchTabId.value = saved.activeId
      restoreTab(tab)
    }
  }
  void refreshSettings()
  void listenSearchScanProgress((p) => {
    if (p.finished || p.cancelled) {
      if (!loading.value) loadingHint.value = ''
      return
    }
    if (p.total > 0) {
      loading.value = true
      loadingHint.value = `掃描中 ${p.current}/${p.total}（符合 ${p.matchedCount}）`
    }
  }).catch(() => {
    /* 事件名稱隨版本可能不同，略過 */
  })
})

onUnmounted(() => {
  pagerResizeObserver?.disconnect()
  pagerResizeObserver = null
})
</script>

<template>
  <div class="app" @click="closeMenus">
    <div
      v-show="activeTab === 'home'"
      class="home"
      :class="{ 'home--reader-fs': readerFullscreenActive }"
      @click.stop
    >
      <div
        v-show="!readerFullscreenActive"
        class="home-header"
        :class="{ 'home-header--scope-open': scopeMenuOpen }"
      >
        <div ref="siteNavRowRef" class="site-nav-row">
          <header class="site-nav">
            <button
              v-for="item in topCategories"
              :key="item.label"
              type="button"
              class="site-link"
              :data-cat="item.label"
              :class="{ on: isTopRowActive(item) }"
              @click.stop="onTopNavClick(item)"
            >
              {{ item.label }}{{ item.children?.length ? ' ▾' : '' }}
            </button>
          </header>
          <div
            v-if="expandedParentItem?.children?.length"
            ref="categoryMenuRef"
            class="cat-menu-float"
            :style="{ left: `${categoryMenuLeft}px` }"
            @click.stop
          >
            <button
              type="button"
              class="cat-menu-item cat-menu-item--parent"
              @click.stop="onParentCategoryClick(expandedParentItem!)"
            >
              {{ expandedParentItem!.label }}
            </button>
            <button
              v-for="ch in expandedParentItem.children"
              :key="ch.label + String(ch.cateId ?? ch.rankingCateId ?? '')"
              type="button"
              class="cat-menu-item"
              @click.stop="onChildCategoryClick(expandedParentItem!, ch)"
            >
              {{ ch.label }}
            </button>
          </div>
        </div>

        <div class="sub-nav-row">
        <nav class="sub-nav">
          <button type="button" class="sub-link sub-link--tab" :class="{ on: subNav === 'search' }" @click.stop="setSubNav('search')">
            <span class="sub-link-text">漫畫搜索</span>
          </button>
          <button type="button" class="sub-link sub-link--tab" :class="{ on: subNav === 'detail' }" @click.stop="setSubNav('detail')">
            <span class="sub-link-text">漫畫詳情</span>
          </button>
          <div class="sub-dd sub-dd--tab">
            <button
              type="button"
              class="sub-link sub-link--tab"
              :class="{ on: readMenuOpen || subNav === 'read' }"
              @click.stop="toggleReadMenu"
            >
              <span class="sub-link-text">漫畫閱讀</span>
              <span class="sub-link-caret" aria-hidden="true">{{ readMenuOpen ? '▴' : '▾' }}</span>
            </button>
            <div v-if="readMenuOpen" class="read-menu-float" @click.stop>
              <button
                v-for="item in READ_MENU_ITEMS"
                :key="item.key"
                type="button"
                class="cat-menu-item"
                :class="{ on: readMode === item.key && subNav === 'read' }"
                @click.stop="selectReadMode(item.key)"
              >
                {{ item.label }}
              </button>
            </div>
          </div>
          <div class="sub-dd sub-dd--tab">
            <button
              type="button"
              class="sub-link sub-link--tab"
              :class="{ on: subNav === 'favorites' || favoritesMenuOpen }"
              @click.stop="toggleFavoritesMenu"
            >
              <span class="sub-link-text">我的收藏</span>
              <span class="sub-link-caret" aria-hidden="true">{{ favoritesMenuOpen ? '▴' : '▾' }}</span>
            </button>
            <div v-if="favoritesMenuOpen" class="read-menu-float" @click.stop>
              <button
                v-for="item in FAVORITES_MENU_ITEMS"
                :key="item.key"
                type="button"
                class="cat-menu-item"
                :class="{ on: favoritesSection === item.key && subNav === 'favorites' }"
                @click.stop="openFavoritesSection(item.key)"
              >
                {{ item.label }}
              </button>
            </div>
          </div>
          <button
            type="button"
            class="sub-link sub-link--tab"
            :class="{ on: subNav === 'snapshots' }"
            @click.stop="openSnapshotList"
          >
            <span class="sub-link-text">快照列表</span>
          </button>
        </nav>
        </div>

        <div v-if="showReaderTitleBar" class="reader-title-bar" :title="activeReaderTitle">
          {{ activeReaderTitle }}
        </div>

        <section v-if="subNav === 'search'" class="search-bar">
          <div class="scope-wrap">
            <button
              ref="scopeBtnRef"
              type="button"
              class="scope-btn"
              :disabled="!canUseCategoryScope"
              :title="canUseCategoryScope ? '選擇快照分類範圍' : '請先在設定指定快照資料夾並確保目錄內有快照'"
              @click.stop="canUseCategoryScope && (scopeMenuOpen = !scopeMenuOpen)"
            >
              範圍：{{ scopeLabel }}
            </button>
          </div>
          <div class="search-input-wrap">
            <input
              v-model="keyword"
              class="search-input"
              placeholder="關鍵詞／漫畫鏈結"
              @keydown.enter="submitKeywordSearch"
            />
            <button
              v-if="keyword.length > 0"
              type="button"
              class="search-clear"
              aria-label="清除關鍵字"
              @click.stop="clearSearchKeyword"
            >
              ×
            </button>
          </div>
          <button type="button" class="search-go" :disabled="loading" @click.stop="submitKeywordSearch">
            搜索
          </button>
        </section>

        <SearchResultTabBar
          v-if="subNav === 'search'"
          :tabs="searchTabs"
          :active-id="activeSearchTabId"
          :is-bookmarked="isTabBookmarked"
          @select="selectSearchTab"
          @close="closeSearchTab"
          @toggle-bookmark="toggleSearchTabBookmark"
        />

        <p v-if="loading && loadingHint" class="status status--hint">{{ loadingHint }}</p>
        <p v-else-if="showBrowseSortHint" class="status status--hint">{{ LIVE_BROWSE_CUSTOM_SORT_HINT }}</p>
        <p v-else-if="showStatusLine && statusMessage" class="status">{{ statusMessage }}</p>
      </div>

      <Teleport to="body">
        <div
          v-if="scopeMenuOpen && canUseCategoryScope && subNav === 'search'"
          class="dropdown-menu scope-dropdown scope-dropdown--portal"
          :style="{
            top: `${scopeMenuPos.top}px`,
            left: `${scopeMenuPos.left}px`,
            minWidth: `${scopeMenuPos.minWidth}px`,
          }"
          @click.stop
        >
          <button type="button" @click="onScopeSelect(null)">全站</button>
          <button
            v-for="h in categoryHeaders"
            :key="h.filePath"
            type="button"
            :class="{ on: searchScope?.filePath === h.filePath }"
            @click="onScopeSelect(h)"
          >
            {{ h.label }}
          </button>
        </div>
      </Teleport>

      <div v-show="subNav === 'search'" ref="searchScrollRef" class="comic-scroll">
        <div v-if="loading" class="loading-mask">{{ loadingHint || '載入中…' }}</div>

        <div
          class="comic-area"
          :class="{
            'comic-list': gridLayout === 'list',
            'comic-grid': gridLayout !== 'list',
            [`cols-${gridCols}`]: gridLayout !== 'list',
          }"
        >
          <MobileComicCard
            v-for="c in visibleSearchComics"
            :key="c.id"
            :comic="c"
            :layout="cardLayout"
            :favorited="isFavoriteComic(c.id)"
            :catalog-analysis-note="catalogAnalysisNoteFor(c.id)"
            @detail="onDetail"
            @read="onRead"
            @download="downloadComic"
            @toggle-favorite="toggleFavoriteComic"
          />
        </div>
        <p v-if="!loading && visibleSearchComics.length === 0" class="empty">尚無結果</p>
      </div>

      <div v-show="subNav === 'detail'" class="comic-scroll detail-scroll">
        <MobileComicDetail
          ref="detailPanelRef"
          :comic="pickedComic"
          :loading="detailLoading"
          :error="detailError"
          :created-label="detailCreatedLabel"
          :favorited="pickedComic ? isFavoriteComic(pickedComic.id) : false"
          :snapshot-search-available="detailSnapshotSearchAvailable"
          @read="onDetailRead"
          @download="onDetailDownload"
          @tag-search="onDetailTagSearch"
          @detail-search="onDetailSearch"
          @toggle-favorite="onDetailToggleFavorite"
        />
      </div>

      <div
        v-show="subNav === 'favorites' && favoritesSection === 'comics'"
        ref="favScrollRef"
        class="comic-scroll"
      >
        <div
          class="comic-area"
          :class="{
            'comic-list': favGridLayout === 'list',
            'comic-grid': favGridLayout !== 'list',
            [`cols-${favGridCols}`]: favGridLayout !== 'list',
          }"
        >
          <MobileComicCard
            v-for="c in favoriteComicsVisible"
            :key="c.id"
            :comic="c"
            :layout="favCardLayout"
            :favorited="isFavoriteComic(c.id)"
            @detail="onDetail"
            @read="onRead"
            @download="downloadComic"
            @toggle-favorite="toggleFavoriteComic"
          />
        </div>
        <p v-if="favoriteComics.length === 0" class="empty">
          尚無收藏漫畫。在搜索結果或詳情封面右上角點 ☆ 加入。
        </p>
      </div>

      <MobileFavoritesPanel
        v-show="subNav === 'favorites' && favoritesSection === 'tabs'"
        :favorite-tabs="favoriteSearchTabs"
        @open-tab="openFavoriteTabBookmark"
        @remove-tab="(sid) => { favoriteSearchTabs = favoriteSearchTabs.filter((b) => b.sourceTabId !== sid); saveFavoriteSearchTabs(favoriteSearchTabs) }"
      />

      <div v-show="subNav === 'snapshots'" class="comic-scroll snapshot-list-scroll">
        <div v-if="snapshotListLoading" class="loading-mask">讀取快照列表…</div>
        <div v-if="!hasCategoryDir" class="snapshot-list-empty">
          <p class="snapshot-list-empty-title">尚未設定快照資料夾</p>
          <p class="snapshot-list-empty-hint">
            請前往「設定」，使用「讀取分類目錄（快照）」選擇資料夾並讀取快照。
          </p>
          <button type="button" class="tool tool--primary snapshot-list-settings-btn" @click="goSettingsForSnapshot">
            前往設定
          </button>
        </div>
        <template v-else>
          <div class="snapshot-list-toolbar">
            <span class="snapshot-list-dir" :title="categoryDir">{{ categoryDir }}</span>
            <button
              type="button"
              class="tool tool--mini tool--primary"
              :disabled="snapshotListLoading || snapshotScanBusy || categoryHeaders.length === 0"
              @click="openSnapshotUpdateDialog"
            >
              更新快照
            </button>
            <button
              type="button"
              class="tool tool--mini"
              :disabled="snapshotListLoading || snapshotScanBusy"
              @click="refreshSnapshotList"
            >
              重新整理
            </button>
          </div>
          <ul v-if="categoryHeaders.length > 0" class="snapshot-list">
            <li v-for="h in categoryHeaders" :key="h.filePath">
              <button type="button" class="snapshot-list-item" @click="onSnapshotListItemClick(h)">
                <span class="snapshot-list-label">{{ h.label }}</span>
                <span class="snapshot-list-meta">{{ snapshotHeaderMeta(h) }}</span>
              </button>
            </li>
          </ul>
          <p v-else-if="!snapshotListLoading" class="empty">
            目錄內沒有可用的快照檔。請在設定重新選擇資料夾或確認檔案格式。
          </p>
        </template>
      </div>

      <MobileSnapshotResumeDialog
        v-model:showing="snapshotResumeShowing"
        :candidates="snapshotResumeCandidates"
        :loading="snapshotResumeLoading"
        @confirm="onSnapshotResumeConfirm"
      />

      <div
        v-if="snapshotScanBusy"
        class="snapshot-scan-overlay"
        @click.stop
        @touchstart.stop
      >
        <div class="snapshot-scan-card">
          <p class="snapshot-scan-title">快照掃描中</p>
          <p class="snapshot-scan-status">{{ snapshotScanStatus || '處理中…' }}</p>
          <button type="button" class="tool tool--mini" @click="cancelSnapshotUpdateScan">取消</button>
        </div>
      </div>

      <div v-show="subNav === 'read'" class="comic-scroll read-panel">
        <MobileLocalRead
          v-if="readMode === 'local'"
          key="read-local"
          class="read-mode-pane"
        />
        <MobileComicRead
          v-else-if="readMode === 'online'"
          key="read-online"
          class="read-mode-pane"
          :comic="readingComic"
          :active="true"
          :reading-active="onlineReadingActive"
          @stop="stopOnlineReading"
        />
      </div>

      <div v-if="activeTab === 'home' && !readerFullscreenActive" class="bottom-dock" @click.stop>
        <span ref="pagerMeasureRef" class="pt-num pt-num--measure" aria-hidden="true">0</span>
        <div v-if="showSearchPager" class="list-toolbar" @click.stop>
          <div class="list-toolbar-left">
            <div class="menu-wrap batch-download-wrap">
              <button
                type="button"
                class="tool tool--primary"
                :class="{ on: batchDownloadMenuOpen }"
                @click.stop="toggleBatchDownloadMenu"
              >
                批量下載模式{{ batchDownloadMenuOpen ? ' ▴' : ' ▾' }}
              </button>
              <div
                v-if="batchDownloadMenuOpen"
                class="dropdown-menu drop-up batch-download-menu"
                @click.stop
              >
                <button
                  type="button"
                  :disabled="loading || comics.length === 0"
                  @click.stop="onBatchDownloadAllPage"
                >
                  本頁全部下載
                </button>
                <button
                  type="button"
                  :disabled="!canUseKoreanDownloadMode"
                  @click.stop="onBatchKoreanDownloadMode"
                >
                  韓漫下載模式
                </button>
                <button
                  type="button"
                  :disabled="!canRunListCompare"
                  @click.stop="onBatchTxtListCompare"
                >
                  TXT列表比對
                </button>
              </div>
            </div>
          </div>
          <div class="list-toolbar-right">
            <div class="menu-wrap">
              <button type="button" class="tool tool--mini" @click.stop="layoutMenuOpen = !layoutMenuOpen">{{ layoutLabel }}</button>
              <div v-if="layoutMenuOpen" class="dropdown-menu drop-up menu-right" @click.stop>
                <button
                  v-for="opt in LAYOUT_OPTIONS"
                  :key="opt.key"
                  type="button"
                  :class="{ on: gridLayout === opt.key }"
                  @click="onLayoutSelect(opt.key)"
                >
                  {{ opt.label }}
                </button>
              </div>
            </div>
            <div class="menu-wrap">
              <button type="button" class="tool tool--mini" @click.stop="pageSizeMenuOpen = !pageSizeMenuOpen">{{ pageSizeLabel }}</button>
              <div v-if="pageSizeMenuOpen" class="dropdown-menu drop-up menu-right" @click.stop>
                <button
                  v-for="n in PAGE_SIZE_OPTIONS"
                  :key="n"
                  type="button"
                  :class="{ on: pageSize === n }"
                  @click.stop="onPageSizeSelect(n)"
                >
                  每頁 {{ n }}
                </button>
              </div>
            </div>
            <div class="menu-wrap">
              <button type="button" class="tool tool--mini" @click.stop="sortMenuOpen = !sortMenuOpen">{{ sortLabel }}</button>
              <div v-if="sortMenuOpen" class="dropdown-menu drop-up menu-right" @click.stop>
                <button
                  v-for="opt in SEARCH_SORT_OPTIONS"
                  :key="opt.key"
                  type="button"
                  :class="{ on: sortOrder === opt.key }"
                  @click="onSortSelect(opt.key)"
                >
                  {{ opt.label }}
                </button>
              </div>
            </div>
          </div>
        </div>
        <div v-if="showFavoritesToolbar" class="fav-bottom-toolbars" @click.stop>
          <div class="list-toolbar list-toolbar--fav-row">
            <div class="list-toolbar-right">
              <button
                type="button"
                class="tool tool--mini tool--fit"
                :disabled="favoritesSection === 'comics' ? favoriteComics.length === 0 : favoriteSearchTabs.length === 0"
                @click.stop="exportFavoriteArchive"
              >
                導出
              </button>
              <button type="button" class="tool tool--mini tool--fit" @click.stop="importFavoriteArchive">
                匯入
              </button>
              <button
                type="button"
                class="tool tool--mini tool--fit"
                :disabled="favoritesSection === 'comics' ? favoriteComics.length === 0 : favoriteSearchTabs.length === 0"
                @click.stop="requestClearFavorites"
              >
                清除
              </button>
            </div>
          </div>
          <div
            v-if="favoritesSection === 'comics' && favoriteComics.length > 0"
            class="list-toolbar list-toolbar--fav-row"
          >
            <div class="list-toolbar-left list-toolbar-left--nowrap">
              <button
                type="button"
                class="tool tool--primary tool--fit"
                @click.stop="requestDownloadAllFavoriteComics"
              >
                全部下載
              </button>
            </div>
            <div class="list-toolbar-right">
              <div class="menu-wrap">
                <button type="button" class="tool tool--mini" @click.stop="favLayoutMenuOpen = !favLayoutMenuOpen">{{ favLayoutLabel }}</button>
                <div v-if="favLayoutMenuOpen" class="dropdown-menu drop-up menu-right" @click.stop>
                  <button
                    v-for="opt in LAYOUT_OPTIONS"
                    :key="'fav-layout-' + opt.key"
                    type="button"
                    :class="{ on: favGridLayout === opt.key }"
                    @click="onFavLayoutSelect(opt.key)"
                  >
                    {{ opt.label }}
                  </button>
                </div>
              </div>
              <div class="menu-wrap">
                <button type="button" class="tool tool--mini" @click.stop="favPageSizeMenuOpen = !favPageSizeMenuOpen">{{ favPageSizeLabel }}</button>
                <div v-if="favPageSizeMenuOpen" class="dropdown-menu drop-up menu-right" @click.stop>
                  <button
                    v-for="n in PAGE_SIZE_OPTIONS"
                    :key="'fav-ps-' + n"
                    type="button"
                    :class="{ on: favPageSize === n }"
                    @click.stop="onFavPageSizeSelect(n)"
                  >
                    每頁 {{ n }}
                  </button>
                </div>
              </div>
              <div class="menu-wrap">
                <button type="button" class="tool tool--mini" @click.stop="favSortMenuOpen = !favSortMenuOpen">{{ favSortLabel }}</button>
                <div v-if="favSortMenuOpen" class="dropdown-menu drop-up menu-right" @click.stop>
                  <button
                    v-for="opt in SEARCH_SORT_OPTIONS"
                    :key="'fav-sort-' + opt.key"
                    type="button"
                    :class="{ on: favSortOrder === opt.key }"
                    @click="onFavSortSelect(opt.key)"
                  >
                    {{ opt.label }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
        <footer v-if="showSearchPager" class="home-pager" @click.stop>
          <div ref="pagerBarRef" class="page-tab-bar page-tab-bar--dock">
            <button type="button" class="pt-nav" :disabled="viewPage <= 1 || loading" @click="goPrevPage">‹</button>
            <div class="pt-nums-cluster">
              <div class="pt-nums">
                <button
                  v-for="n in pageTabNumbers"
                  :key="n"
                  type="button"
                  class="pt-num"
                  :class="{ on: n === viewPage }"
                  @click="goPage(n)"
                >
                  {{ n }}
                </button>
              </div>
              <button
                type="button"
                class="pt-nav pt-nav--next"
                :disabled="viewPage >= totalPages || loading"
                @click="goNextPage"
              >
                ›
              </button>
            </div>
            <div class="page-jump-wrap">
              <button
                ref="pagerSummaryBtnRef"
                type="button"
                class="pt-summary-btn"
                :class="{ on: pageJumpOpen }"
                @click.stop="pageJumpOpen = !pageJumpOpen"
              >
                {{ pageSummary }}
              </button>
              <div v-if="pageJumpOpen" class="page-jump-pop" @click.stop>
                <div class="page-jump-goto">
                  <input
                    v-model="pageGoInput"
                    class="page-jump-goto-input"
                    type="text"
                    inputmode="numeric"
                    pattern="[0-9]*"
                    :placeholder="`1～${totalPages}`"
                    @keydown.enter.prevent="confirmPageGoFromJump"
                  />
                  <button type="button" class="page-jump-goto-btn" @click.stop="confirmPageGoFromJump">
                    前往
                  </button>
                </div>
                <button
                  v-for="n in allPageNumbers"
                  :key="'jump-' + n"
                  type="button"
                  class="page-jump-item"
                  :class="{ on: n === viewPage }"
                  @click="pickJumpPage(n)"
                >
                  第 {{ n }} 頁
                </button>
              </div>
            </div>
          </div>
        </footer>
        <footer v-if="showFavoritesPager" class="home-pager" @click.stop>
          <div ref="pagerBarRef" class="page-tab-bar page-tab-bar--dock">
            <button type="button" class="pt-nav" :disabled="favViewPage <= 1" @click="goFavPrevPage">‹</button>
            <div class="pt-nums-cluster">
              <div class="pt-nums">
                <button
                  v-for="n in favPageTabNumbers"
                  :key="'fav-' + n"
                  type="button"
                  class="pt-num"
                  :class="{ on: n === favViewPage }"
                  @click="goFavPage(n)"
                >
                  {{ n }}
                </button>
              </div>
              <button
                type="button"
                class="pt-nav pt-nav--next"
                :disabled="favViewPage >= favTotalPages"
                @click="goFavNextPage"
              >
                ›
              </button>
            </div>
            <div class="page-jump-wrap">
              <button
                ref="pagerSummaryBtnRef"
                type="button"
                class="pt-summary-btn"
                :class="{ on: favPageJumpOpen }"
                @click.stop="favPageJumpOpen = !favPageJumpOpen"
              >
                {{ favPageSummary }}
              </button>
              <div v-if="favPageJumpOpen" class="page-jump-pop" @click.stop>
                <div class="page-jump-goto">
                  <input
                    v-model="pageGoInput"
                    class="page-jump-goto-input"
                    type="text"
                    inputmode="numeric"
                    pattern="[0-9]*"
                    :placeholder="`1～${favTotalPages}`"
                    @keydown.enter.prevent="confirmPageGoFromJump"
                  />
                  <button type="button" class="page-jump-goto-btn" @click.stop="confirmPageGoFromJump">
                    前往
                  </button>
                </div>
                <button
                  v-for="n in favAllPageNumbers"
                  :key="'fav-jump-' + n"
                  type="button"
                  class="page-jump-item"
                  :class="{ on: n === favViewPage }"
                  @click="pickFavJumpPage(n)"
                >
                  第 {{ n }} 頁
                </button>
              </div>
            </div>
          </div>
        </footer>
        <nav class="bottom-tabs">
          <button type="button" :class="{ on: activeTab === 'home' }" @click="activeTab = 'home'">主頁</button>
          <button type="button" :class="{ on: activeTab === 'download' }" @click="activeTab = 'download'">下載</button>
          <button type="button" :class="{ on: activeTab === 'settings' }" @click="activeTab = 'settings'">設定</button>
        </nav>
      </div>
    </div>

    <section v-show="activeTab === 'download'" class="tab-panel tab-panel--fill">
      <h2 class="tab-h2">下載進度</h2>
      <MobileDownloadPanel />
    </section>

    <section v-show="activeTab === 'settings'" class="tab-panel tab-panel--fill">
      <h2 class="tab-h2">設定</h2>
      <MobileSettingsPanel
        :category-dir="categoryDir"
        :category-count="categoryHeaders.length"
        @pick-category="onPickCategoryDir"
      />
    </section>

    <nav v-if="activeTab !== 'home'" class="bottom-tabs bottom-tabs--solo">
      <button type="button" :class="{ on: activeTab === 'home' }" @click="activeTab = 'home'">主頁</button>
      <button type="button" :class="{ on: activeTab === 'download' }" @click="activeTab = 'download'">下載</button>
      <button type="button" :class="{ on: activeTab === 'settings' }" @click="activeTab = 'settings'">設定</button>
    </nav>

    <MobileKoreanDownloadDialog
      :showing="koreanModeDialogOpen"
      :comics="koreanModeDialogComics"
      :tag-label="koreanModeDialogTagLabel"
      @update:showing="koreanModeDialogOpen = $event"
      @confirm="onKoreanModeDialogConfirm"
    />

    <Teleport to="body">
      <div
        v-if="listCompareLoading"
        class="page-go-overlay list-compare-overlay"
        @click.stop
        @touchstart.stop
      >
        <div class="page-go-dialog list-compare-dialog" @click.stop @touchstart.stop>
          <p class="page-go-title">列表比對</p>
          <p class="list-compare-status">{{ listCompareProgressLabel }}</p>
          <div class="list-compare-track" :class="{ 'list-compare-track--indeterminate': listCompareProgress?.phase === 'reading' }">
            <div
              class="list-compare-bar"
              :class="{ 'list-compare-bar--indeterminate': listCompareProgress?.phase === 'reading' }"
              :style="listCompareProgress?.phase === 'comparing' ? { width: `${listCompareProgressPercent}%` } : undefined"
            />
          </div>
        </div>
      </div>
      <div
        v-if="downloadAllConfirmOpen"
        class="page-go-overlay"
        @click.self="cancelDownloadAllConfirm"
        @touchstart.self="cancelDownloadAllConfirm"
      >
        <div class="page-go-dialog" @click.stop @touchstart.stop>
          <p class="page-go-title">確認本頁下載</p>
          <p class="download-confirm-text">
            即將把本頁 {{ comics.length }} 筆加入下載佇列，是否繼續？
          </p>
          <div class="page-go-actions">
            <button
              type="button"
              class="page-go-btn page-go-btn--ghost"
              @click.stop="cancelDownloadAllConfirm"
            >
              取消
            </button>
            <button
              type="button"
              class="page-go-btn page-go-btn--ok"
              @click.stop="confirmDownloadAll"
            >
              確定下載
            </button>
          </div>
        </div>
      </div>
      <div
        v-if="downloadAllFavoritesConfirmOpen"
        class="page-go-overlay"
        @click.self="cancelDownloadAllFavoritesConfirm"
        @touchstart.self="cancelDownloadAllFavoritesConfirm"
      >
        <div class="page-go-dialog" @click.stop @touchstart.stop>
          <p class="page-go-title">確認全部下載</p>
          <p class="download-confirm-text">
            即將把收藏漫畫內 {{ favoriteComics.length }} 筆全部加入下載佇列（含全部頁數），是否繼續？
          </p>
          <div class="page-go-actions">
            <button
              type="button"
              class="page-go-btn page-go-btn--ghost"
              @click.stop="cancelDownloadAllFavoritesConfirm"
            >
              取消
            </button>
            <button
              type="button"
              class="page-go-btn page-go-btn--ok"
              @click.stop="confirmDownloadAllFavorites"
            >
              確定下載
            </button>
          </div>
        </div>
      </div>
      <div
        v-if="clearFavoritesConfirmOpen"
        class="page-go-overlay"
        @click.self="cancelClearFavoritesConfirm"
        @touchstart.self="cancelClearFavoritesConfirm"
      >
        <div class="page-go-dialog" @click.stop @touchstart.stop>
          <p class="page-go-title">確認清除</p>
          <p class="download-confirm-text">
            {{
              favoritesSection === 'comics'
                ? `確定要清除收藏漫畫內的 ${favoriteComics.length} 筆紀錄嗎？`
                : `確定要清除收藏分頁內的 ${favoriteSearchTabs.length} 筆紀錄嗎？`
            }}
          </p>
          <div class="page-go-actions">
            <button
              type="button"
              class="page-go-btn page-go-btn--ghost"
              @click.stop="cancelClearFavoritesConfirm"
            >
              取消
            </button>
            <button
              type="button"
              class="page-go-btn page-go-btn--ok"
              @click.stop="confirmClearFavorites"
            >
              確定清除
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 100vw;
  height: 100dvh;
  height: 100vh;
  background: #121212;
  color: #eee;
  font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
  overflow: hidden;
  position: relative;
}

.home {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.home-header {
  flex-shrink: 0;
  padding-top: max(env(safe-area-inset-top, 0px), 26px);
  background: #121212;
  position: relative;
  z-index: 12;
  border-bottom: 1px solid #222;
}

.home-header--scope-open {
  z-index: 50;
}

.scope-dropdown--portal {
  position: fixed;
  z-index: 1000;
  max-height: 40vh;
  overflow: auto;
  margin-top: 0;
}

.site-nav {
  display: flex;
  flex-wrap: nowrap;
  width: 100%;
  overflow: hidden;
  justify-content: space-between;
  align-items: stretch;
  gap: 0;
}

.site-link,
.sub-link,
.tool,
.scope-btn,
.search-go,
.search-clear,
.block-btn,
.cat-menu-item,
.pt-nav,
.pt-num,
.pt-summary-btn,
.page-jump-item {
  touch-action: manipulation;
  -webkit-tap-highlight-color: rgba(110, 181, 255, 0.2);
  cursor: pointer;
}

.site-link {
  flex: 1 1 0;
  min-width: 0;
  padding: 4px 2px;
  border: none;
  background: transparent;
  color: #ccc;
  font-size: clamp(11px, 3.1vw, 13px);
  white-space: nowrap;
  line-height: 1.15;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: center;
}

.site-link.on {
  color: #fff;
  font-weight: 600;
  box-shadow: inset 0 -2px 0 #3d6ef5;
}

.site-nav-row {
  position: relative;
}

.cat-menu-float {
  position: absolute;
  top: 100%;
  z-index: 80;
  min-width: 88px;
  max-width: 55vw;
  background: #1c1c1c;
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
}

.cat-menu-item {
  display: block;
  width: 100%;
  padding: 8px 16px;
  border: none;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  background: transparent;
  color: rgba(255, 255, 255, 0.88);
  font-size: 13px;
  line-height: 1.4;
  text-align: center;
  white-space: nowrap;
}

.cat-menu-item.on {
  color: #fff;
  background: rgba(255, 255, 255, 0.08);
}

.cat-menu-item:active {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.cat-menu-item--parent {
  font-weight: 600;
  border-bottom: 1px solid rgba(255, 255, 255, 0.15);
}

.sub-nav-row {
  position: relative;
  z-index: 50;
  overflow: visible;
}

.sub-nav {
  display: flex;
  flex-wrap: nowrap;
  width: 100%;
  overflow: visible;
  justify-content: space-between;
  align-items: stretch;
  gap: 0;
  padding: 2px 4px 4px;
  min-height: 28px;
}

.sub-link {
  border: none;
  background: transparent;
  color: #888;
  line-height: 1.15;
}

.sub-nav > .sub-link--tab,
.sub-nav > .sub-dd--tab {
  flex: 1 1 0;
  min-width: 0;
}

.sub-link--tab {
  box-sizing: border-box;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.1em;
  width: 100%;
  min-width: 0;
  padding: 5px 2px;
  font-size: clamp(12px, 3.4vw, 14px);
  white-space: nowrap;
  border-bottom: 2px solid transparent;
}

.sub-link-text {
  flex: 1 1 auto;
  min-width: 0;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sub-link-caret {
  flex: 0 0 auto;
  font-size: 0.72em;
  opacity: 0.85;
  line-height: 1;
}

.sub-link.on {
  color: #6eb5ff;
  border-bottom-color: #3d6ef5;
}

.sub-nav .read-menu-float .cat-menu-item {
  font-size: clamp(12px, 3.4vw, 14px);
  padding: 8px 12px;
}

.sub-dd {
  position: relative;
  display: flex;
  min-width: 0;
}

.sub-dd--tab .sub-link--tab {
  flex: 1 1 0;
}

.read-menu-float {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 100;
  min-width: 96px;
  background: #1c1c1c;
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
}

.search-bar,
.home-header :deep(.result-tab-bar),
.home-header .status {
  position: relative;
  z-index: 1;
}

.search-bar {
  display: flex;
  align-items: stretch;
  gap: 3px;
  padding: 4px 6px 3px;
}

.scope-wrap,
.menu-wrap {
  position: relative;
}

.dropdown-menu.scope-dropdown {
  z-index: 120;
}

.menu-grow {
  flex: 0 0 auto;
}

.scope-btn {
  width: 100%;
  min-height: 30px;
  padding: 4px 5px;
  border: 1px solid #444;
  border-radius: 5px 0 0 5px;
  background: #252525;
  color: #bbb;
  font-size: 10px;
  text-align: left;
}

.dropdown-menu {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 40;
  max-height: 40vh;
  overflow: auto;
  min-width: 140px;
  margin-top: 2px;
  background: #2a2a2a;
  border: 1px solid #555;
  border-radius: 6px;
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45);
}

.dropdown-menu--right,
.menu-right {
  left: auto;
  right: 0;
}

.drop-up {
  top: auto;
  bottom: 100%;
  margin-top: 0;
  margin-bottom: 2px;
}

.dropdown-menu button {
  display: block;
  width: 100%;
  padding: 8px 10px;
  border: none;
  background: transparent;
  color: #eee;
  text-align: left;
  font-size: 11px;
}

.dropdown-menu button.on {
  color: #6eb5ff;
  background: rgba(61, 110, 245, 0.12);
}

.search-input-wrap {
  flex: 1;
  min-width: 0;
  position: relative;
  display: flex;
  align-items: stretch;
}

.search-input {
  flex: 1;
  min-width: 0;
  width: 100%;
  min-height: 30px;
  padding: 4px 28px 4px 8px;
  border: 1px solid #444;
  border-left: none;
  border-right: none;
  background: #1e1e1e;
  color: #fff;
  font-size: 13px;
}

.search-clear {
  position: absolute;
  right: 2px;
  top: 50%;
  transform: translateY(-50%);
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #aaa;
  font-size: 16px;
  line-height: 1;
}

.search-clear:active {
  color: #fff;
  background: rgba(255, 255, 255, 0.08);
}

.search-go {
  flex: 0 0 auto;
  min-width: 52px;
  padding: 0 10px;
  min-height: 30px;
  border: none;
  border-radius: 0 5px 5px 0;
  background: #3d6ef5;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
}

.active-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 0 6px 3px;
}

.tag {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 2px 6px;
  border-radius: 4px;
  background: #252525;
  font-size: 10px;
}

.tag button {
  border: none;
  background: transparent;
  color: #888;
  padding: 0 2px;
  min-width: 20px;
  min-height: 20px;
}

.list-toolbar {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 4px;
  padding: 4px 6px;
  border-top: 1px solid #333;
  background: #141414;
}

.fav-bottom-toolbars {
  border-top: 1px solid #333;
  background: #141414;
}

.fav-bottom-toolbars .list-toolbar--fav-row {
  border-top: none;
}

.fav-bottom-toolbars .list-toolbar--fav-row + .list-toolbar--fav-row {
  border-top: 1px solid #2a2a2a;
}

.list-toolbar-left {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.list-toolbar-left--nowrap {
  flex-wrap: nowrap;
  flex-shrink: 0;
}

.list-toolbar-right {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 4px;
  margin-left: auto;
  flex-shrink: 0;
}

.tool--mini {
  padding: 4px 6px;
  font-size: 9px;
  white-space: nowrap;
}

.tool--fit {
  width: auto;
  min-width: 0;
  padding-inline: 6px;
  white-space: nowrap;
  flex-shrink: 0;
}

.tool {
  padding: 4px 8px;
  border: 1px solid #444;
  border-radius: 5px;
  background: #252525;
  color: #ccc;
  font-size: 10px;
  line-height: 1.2;
}

.tool--primary {
  border-color: #3d6ef5;
  background: #3d6ef5;
  color: #fff;
}

.tool--primary.on {
  background: #2f5ad4;
  border-color: #5b8cff;
}

.batch-download-wrap {
  position: relative;
  z-index: 35;
}

.batch-download-menu {
  left: 0;
  right: auto;
  min-width: 9.5rem;
}

.batch-download-menu button {
  text-align: left;
  white-space: nowrap;
}

.batch-download-menu button:disabled {
  opacity: 0.45;
}

.tool:disabled {
  opacity: 0.45;
}


.status {
  margin: 0;
  padding: 0 6px 4px;
  font-size: 10px;
  color: #e88;
  line-height: 1.3;
  white-space: pre-wrap;
  word-break: break-word;
}

.status--hint {
  color: #8ab4f8;
}

.page-tab-bar {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 3px 6px 4px;
  background: #0f2848;
  border-bottom: 1px solid #2a4a6e;
}

.page-tab-bar--dock {
  width: 100%;
  border-bottom: none;
  padding: 2px 4px;
  min-height: 28px;
  gap: 2px;
}

.pt-nums-cluster {
  display: flex;
  flex: 1;
  align-items: center;
  min-width: 0;
  gap: 0;
}

.pt-nav--next {
  flex-shrink: 0;
  margin-left: 0;
}

.scope-btn:disabled {
  opacity: 0.55;
}

.page-go-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.55);
  padding: 16px;
}

.page-go-dialog {
  width: min(280px, 100%);
  padding: 14px;
  border-radius: 8px;
  background: #1e1e1e;
  border: 1px solid #444;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}

.page-go-title {
  margin: 0 0 10px;
  font-size: 13px;
  font-weight: 600;
  color: #eee;
  text-align: center;
}

.page-go-input {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 10px;
  border: 1px solid #555;
  border-radius: 5px;
  background: #111;
  color: #eee;
  font-size: 14px;
}

.page-go-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.page-go-btn {
  flex: 1;
  padding: 8px;
  border-radius: 5px;
  font-size: 12px;
  border: 1px solid #444;
  background: #333;
  color: #ddd;
}

.page-go-btn--ok {
  border-color: #3d6ef5;
  background: #3d6ef5;
  color: #fff;
}

.page-go-btn--ghost {
  background: transparent;
}

.download-confirm-text {
  margin: 0;
  color: #cfd8e3;
  font-size: 12px;
  line-height: 1.45;
  text-align: center;
}

.list-compare-overlay {
  z-index: 210;
}

.list-compare-dialog {
  width: min(300px, 100%);
}

.list-compare-status {
  margin: 0 0 12px;
  color: #b8c5d6;
  font-size: 12px;
  line-height: 1.4;
  text-align: center;
}

.list-compare-track {
  height: 8px;
  border-radius: 4px;
  background: #2a2a2a;
  overflow: hidden;
}

.list-compare-bar {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, #3d6ef5, #5b8cff);
  transition: width 0.12s ease-out;
}

.list-compare-track--indeterminate .list-compare-bar--indeterminate {
  width: 36% !important;
  animation: list-compare-slide 1.1s ease-in-out infinite;
}

@keyframes list-compare-slide {
  0% {
    transform: translateX(-120%);
  }
  100% {
    transform: translateX(320%);
  }
}

.detail-scroll {
  padding-bottom: calc(108px + env(safe-area-inset-bottom, 0px));
}

.pt-nav {
  flex: 0 0 auto;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid #3a5a80;
  border-radius: 3px;
  background: #1a3050;
  color: #cde;
  font-size: 13px;
}

.pt-nums {
  display: flex;
  flex: 0 1 auto;
  gap: 2px;
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
}

.pt-num {
  flex: 0 0 auto;
  min-width: 26px;
  height: 24px;
  padding: 0 5px;
  border: 1px solid #3a5a80;
  border-radius: 3px;
  background: #1a3050;
  color: #9bc;
  font-size: 11px;
}

.pt-num.on {
  background: #3d6ef5;
  border-color: #3d6ef5;
  color: #fff;
}

.pt-num--measure {
  position: absolute;
  left: -9999px;
  top: 0;
  visibility: hidden;
  pointer-events: none;
}

.page-jump-wrap {
  position: relative;
  flex: 0 0 auto;
  min-width: 0;
}

.pt-summary-btn {
  height: 24px;
  padding: 0 8px;
  border: 1px solid #3a5a80;
  border-radius: 3px;
  background: #1a3050;
  color: #9bc;
  font-size: 10px;
  white-space: nowrap;
  flex-shrink: 0;
  width: max-content;
}

.pt-summary-btn.on {
  border-color: #3d6ef5;
  color: #fff;
}

.page-jump-pop {
  position: absolute;
  bottom: 100%;
  right: 0;
  z-index: 55;
  min-width: 100%;
  width: max-content;
  max-width: min(92vw, 280px);
  max-height: 40vh;
  margin-bottom: 2px;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  background: #1c1c1c;
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 -4px 12px rgba(0, 0, 0, 0.45);
}

.page-jump-goto {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  position: sticky;
  top: 0;
  background: #1c1c1c;
  z-index: 1;
}

.page-jump-goto-input {
  flex: 1;
  min-width: 0;
  height: 28px;
  padding: 0 8px;
  border: 1px solid #555;
  border-radius: 4px;
  background: #111;
  color: #eee;
  font-size: 13px;
}

.page-jump-goto-btn {
  flex: 0 0 auto;
  height: 28px;
  padding: 0 10px;
  border: 1px solid #3a5a80;
  border-radius: 4px;
  background: #1a3050;
  color: #cde;
  font-size: 12px;
  white-space: nowrap;
}

.page-jump-item {
  display: block;
  width: 100%;
  padding: 6px 8px;
  border: none;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  background: transparent;
  color: #ccc;
  font-size: 11px;
  text-align: center;
}

.page-jump-item.on {
  background: #3d6ef5;
  color: #fff;
}

.comic-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  -webkit-overflow-scrolling: touch;
  position: relative;
  z-index: 1;
  padding-bottom: calc(108px + env(safe-area-inset-bottom, 0px));
}

.snapshot-list-scroll {
  padding: 8px 10px calc(108px + env(safe-area-inset-bottom, 0px));
}

.snapshot-list-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}

.snapshot-scan-overlay {
  position: fixed;
  inset: 0;
  z-index: 1400;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  padding: 16px;
}

.snapshot-scan-card {
  background: #1e1e1e;
  border: 1px solid #3c4043;
  border-radius: 12px;
  padding: 16px 18px;
  max-width: 320px;
  width: 100%;
  text-align: center;
}

.snapshot-scan-title {
  margin: 0 0 8px;
  font-size: 14px;
  font-weight: 700;
}

.snapshot-scan-status {
  margin: 0 0 12px;
  font-size: 11px;
  color: #9aa0a6;
  line-height: 1.4;
  word-break: break-all;
}

.snapshot-list-dir {
  flex: 1;
  min-width: 0;
  font-size: 10px;
  color: #888;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.snapshot-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.snapshot-list-item {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid #333;
  border-radius: 6px;
  background: #1e1e1e;
  color: #eee;
  text-align: left;
}

.snapshot-list-item:active {
  background: #2a3548;
  border-color: #3d6ef5;
}

.snapshot-list-label {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.3;
}

.snapshot-list-meta {
  font-size: 11px;
  color: #9ab;
}

.snapshot-list-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 40vh;
  padding: 24px 16px;
  text-align: center;
}

.snapshot-list-empty-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: #eee;
}

.snapshot-list-empty-hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: #aaa;
  max-width: 280px;
}

.snapshot-list-settings-btn {
  margin-top: 4px;
  padding: 8px 16px;
  font-size: 13px;
}

.favorites-panel {
  align-items: stretch;
  justify-content: flex-start;
  padding-top: 12px;
}

.fav-tab-btn {
  display: block;
  width: 100%;
  margin-bottom: 6px;
  padding: 8px 10px;
  border: 1px solid #444;
  border-radius: 6px;
  background: #252525;
  color: #ddd;
  font-size: 12px;
  text-align: left;
}

.sub-panel {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  padding-bottom: calc(40px + env(safe-area-inset-bottom, 0px));
}

.sub-panel-msg {
  margin: 0;
  font-size: 13px;
  color: #888;
  text-align: center;
  line-height: 1.5;
}

.loading-mask {
  position: absolute;
  inset: 0;
  z-index: 8;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.35);
  font-size: 13px;
}

.comic-area {
  width: 100%;
  padding: 3px;
  gap: 3px;
}

.comic-grid {
  display: grid;
}

.comic-grid.cols-2 {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}
.comic-grid.cols-4 {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}
.comic-grid.cols-6 {
  grid-template-columns: repeat(6, minmax(0, 1fr));
}
.comic-grid.cols-8 {
  grid-template-columns: repeat(8, minmax(0, 1fr));
}
.comic-grid.cols-10 {
  grid-template-columns: repeat(10, minmax(0, 1fr));
}

.comic-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.empty {
  text-align: center;
  color: #555;
  font-size: 12px;
  padding: 16px;
}

.bottom-dock {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  background: #1a1a1a;
  padding-bottom: env(safe-area-inset-bottom, 0);
}

.home-pager {
  display: flex;
  flex-direction: row;
  align-items: stretch;
  padding: 0;
  border-top: 1px solid #2a4a6e;
}

.tab-panel--fill {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1;
}

.home--reader-fs .read-panel {
  flex: 1;
  min-height: 0;
  padding: 0;
}

.comic-scroll.read-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-bottom: calc(34px + env(safe-area-inset-bottom, 0px));
}

.read-panel :deep(.reader-shell) {
  flex: 1;
  min-height: 0;
  height: 100%;
}

.read-mode-pane {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.reader-title-bar {
  flex-shrink: 0;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 600;
  color: #eee;
  background: #1a2744;
  border-bottom: 1px solid #2a4a6e;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-h2 {
  margin: 0;
  padding: 10px 12px 6px;
  font-size: 15px;
}

.tab-panel {
  flex: 1;
  overflow: auto;
  padding: 12px;
  padding-top: max(env(safe-area-inset-top, 0px), 28px);
  padding-bottom: calc(52px + env(safe-area-inset-bottom, 0));
}

.tab-panel h2 {
  margin: 0 0 10px;
  font-size: 17px;
}

.hint {
  margin: 0 0 6px;
  font-size: 11px;
  color: #888;
}

.block-btn {
  width: 100%;
  padding: 10px;
  margin-bottom: 6px;
  border-radius: 7px;
  border: 1px solid #555;
  background: #252525;
  color: #fff;
}

.block-btn.primary {
  background: #3d6ef5;
  border-color: #3d6ef5;
}

.bottom-tabs {
  display: flex;
  border-top: 1px solid #333;
  background: #1a1a1a;
}

.bottom-tabs--solo {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 30;
  padding-bottom: env(safe-area-inset-bottom, 0);
}

.bottom-tabs button {
  flex: 1;
  padding: 5px 4px 4px;
  border: none;
  background: transparent;
  color: #999;
  font-size: 12px;
  line-height: 1.15;
}

.bottom-tabs button.on {
  color: #fff;
  font-weight: 600;
}
</style>
