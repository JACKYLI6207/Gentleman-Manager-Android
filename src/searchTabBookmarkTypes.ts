import type { ComicInSearch, SearchResult } from './api'
import type { SearchResultTabState, SearchSource } from './searchResultTabTypes'

const COVER_LOAD_BATCH = 20
export const SCOPED_COLLECTED_PAGE_SIZE = 20

export interface SearchTabBookmark {
  id: string
  sourceTabId: string
  savedAt: string
  title: string
  tabState: SearchResultTabState
}

export function isScopedSearchSource(source: SearchSource): boolean {
  if (source.type === 'keyword' && source.cateId !== undefined) return true
  return source.type === 'tag' && source.cateId !== undefined
}

function isRemoteCoverUrl(cover: string): boolean {
  const trimmed = cover.trim()
  return trimmed.startsWith('http://') || trimmed.startsWith('https://')
}

export function comicForBookmarkStorage(comic: ComicInSearch): ComicInSearch {
  if (isRemoteCoverUrl(comic.cover) || comic.cover === '') return comic
  return { ...comic, cover: '' }
}

export function comicsForBookmarkStorage(comics: ComicInSearch[]): ComicInSearch[] {
  return comics.map(comicForBookmarkStorage)
}

export function sanitizeTabStateForBookmark(tab: SearchResultTabState): SearchResultTabState {
  const scoped = isScopedSearchSource(tab.searchSource)
  if (scoped) {
    return {
      ...tab,
      allSearchComics: [],
      sortedComics: [],
      visibleComics: [],
      scopedOfflineMatches:
        tab.scopedOfflineMatches !== undefined
          ? comicsForBookmarkStorage(tab.scopedOfflineMatches)
          : comicsForBookmarkStorage(tab.allSearchComics),
      pageCacheEntries: [],
      catalogAnalysisEntries: [],
      coverLoadLimit: COVER_LOAD_BATCH,
      searchResult: undefined,
      listScrollTop: 0,
    }
  }
  const searchResult =
    tab.searchResult === undefined
      ? undefined
      : { ...tab.searchResult, comics: comicsForBookmarkStorage(tab.searchResult.comics) }
  return {
    ...tab,
    allSearchComics: comicsForBookmarkStorage(tab.allSearchComics),
    sortedComics: comicsForBookmarkStorage(tab.sortedComics),
    visibleComics: comicsForBookmarkStorage(tab.visibleComics),
    pageCacheEntries: [...tab.pageCacheEntries],
    catalogAnalysisEntries: [],
    coverLoadLimit: COVER_LOAD_BATCH,
    searchResult,
    listScrollTop: 0,
  }
}
