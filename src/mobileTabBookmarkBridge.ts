import type { ComicInSearch } from './api'
import type { MobileBrowseKind, MobileSearchTab } from './searchResultTabs'
import { loadSavedPageSize } from './searchUtils'
import {
  comicsForBookmarkStorage,
  sanitizeTabStateForBookmark,
  type SearchTabBookmark,
} from './searchTabBookmarkTypes'
import type { RankingPeriod, SearchResultTabState, SearchSource } from './searchResultTabTypes'

function mobileBrowseToSearchSource(tab: MobileSearchTab): SearchSource {
  switch (tab.browseKind) {
    case 'home':
      return { type: 'albums', list: 'home' }
    case 'albums':
      return { type: 'albums', list: 'albums' }
    case 'category':
      return { type: 'category', cateId: tab.categoryBrowseCateId ?? 0 }
    case 'ranking':
      return { type: 'ranking', period: 'Day', cateId: tab.rankingCateId }
    case 'snapshot':
    case 'keyword':
      return tab.searchScope?.cateId != null
        ? { type: 'keyword', cateId: tab.searchScope.cateId }
        : { type: 'keyword' }
    default:
      return { type: 'keyword' }
  }
}

function browseKindFromSearchSource(source: SearchSource, hasScoped: boolean): MobileBrowseKind {
  if (hasScoped) return 'snapshot'
  switch (source.type) {
    case 'albums':
      return source.list === 'home' ? 'home' : 'albums'
    case 'category':
      return 'category'
    case 'ranking':
      return 'ranking'
    case 'tag':
      return 'keyword'
    default:
      return 'keyword'
  }
}

export function mobileTabToPcTabState(tab: MobileSearchTab): SearchResultTabState {
  const isSnapshot = tab.browseKind === 'snapshot'
  const stored = comicsForBookmarkStorage(tab.allComics)
  return {
    id: tab.id,
    title: tab.title,
    keywordOrComicLinkInput: tab.keyword,
    tagOrLinkInput: '',
    activeTagSearchSource: 'name',
    searchSource: mobileBrowseToSearchSource(tab),
    searchScopeCategory:
      tab.searchScope?.cateId != null
        ? { cateId: tab.searchScope.cateId, label: tab.searchScope.label }
        : null,
    activeTagLabel: '',
    activeCategoryLabel: tab.activeTopCategory,
    rankingPeriod: 'Day',
    viewPage: tab.currentPage,
    allSearchComics: isSnapshot ? [] : stored,
    sortedComics: stored,
    visibleComics: stored,
    pageCacheEntries: [],
    sortOrder: tab.sortOrder,
    catalogAnalysisEntries: tab.catalogAnalysisEntries ?? [],
    totalServerPagesHint: tab.totalPages,
    totalCountHint: tab.totalCount,
    lastCommittedViewPage: tab.currentPage,
    searchSession: 0,
    coverLoadLimit: 20,
    listScrollTop: 0,
    scopedOfflineMatches: isSnapshot ? stored : undefined,
  }
}

export function mobileTabToBookmark(tab: MobileSearchTab): SearchTabBookmark {
  const tabState = sanitizeTabStateForBookmark(mobileTabToPcTabState(tab))
  return {
    id: crypto.randomUUID(),
    sourceTabId: tab.id,
    savedAt: new Date().toISOString(),
    title: tab.title,
    tabState,
  }
}

export function bookmarkToMobileTab(bookmark: SearchTabBookmark): MobileSearchTab {
  const s = bookmark.tabState
  const scoped = s.scopedOfflineMatches
  const allComics = scoped?.length ? [...scoped] : [...s.allSearchComics]
  const isSnapshot = Boolean(scoped?.length) || browseKindFromSearchSource(s.searchSource, false) === 'snapshot'
  const browseKind = scoped?.length
    ? 'snapshot'
    : browseKindFromSearchSource(s.searchSource, false)

  let snapshotPath: string | null = null
  let snapshotLabel: string | null = null
  if (browseKind === 'snapshot' && s.searchScopeCategory) {
    snapshotLabel = s.searchScopeCategory.label
  }

  return {
    id: crypto.randomUUID(),
    title: s.title,
    keyword: s.keywordOrComicLinkInput,
    searchScope: s.searchScopeCategory
      ? {
          label: s.searchScopeCategory.label,
          cateId: s.searchScopeCategory.cateId,
          filePath: snapshotPath ?? '',
        }
      : null,
    browseKind,
    activeTopCategory: s.activeCategoryLabel,
    categoryBrowseCateId: s.searchSource.type === 'category' ? s.searchSource.cateId : null,
    rankingCateId: s.searchSource.type === 'ranking' ? s.searchSource.cateId : null,
    snapshotPath,
    snapshotLabel,
    allComics,
    currentPage: s.viewPage,
    totalPages: Math.max(1, Math.ceil(allComics.length / 20)),
    totalCount: s.totalCountHint || allComics.length,
    sortOrder: s.sortOrder,
    pageSize: loadSavedPageSize(),
    gridLayout: 'grid4',
    catalogAnalysisEntries: s.catalogAnalysisEntries?.length ? [...s.catalogAnalysisEntries] : undefined,
  }
}
