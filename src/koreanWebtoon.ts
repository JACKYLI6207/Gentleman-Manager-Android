import type { ComicInSearch } from './api'
import { toTraditionalChinese } from './chineseText'

function normalizeEpisodeTitle(title: string): string {
  return title
    .replace(/[\u002D\u007E\u02DC\u2010-\u2015\u2212\uFE58\uFE63\uFF0D\u301C\uFF5E\u3030]/g, '-')
    .replace(/\s+/g, ' ')
    .trim()
}

function stripHtml(text: string): string {
  return text.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim()
}

function parseImageCount(additionalInfo: string): number {
  const match = additionalInfo.match(/(\d+)\s*張/)
  return match !== null ? parseInt(match[1], 10) : 0
}

function parseEpisodeRangeFromTitle(title: string): { start: number; end: number; isChapterEnding: boolean } | null {
  const normalized = normalizeEpisodeTitle(title)

  const batch = normalized.match(/(\d+)\s*-\s*(\d+)\s*(?:[话話回集]|$)/)
  if (batch !== null) {
    return {
      start: parseInt(batch[1], 10),
      end: parseInt(batch[2], 10),
      isChapterEnding: false,
    }
  }

  const single = normalized.match(/(\d+)\s*[话話回集]/)
  if (single !== null) {
    const n = parseInt(single[1], 10)
    const afterEpisode = normalized.slice(normalized.indexOf(single[0]) + single[0].length)
    const isChapterEnding =
      /\[完結\]/.test(normalized) ||
      /\[.*完結.*\]/.test(afterEpisode) ||
      /(?:^|\s)完結(?:\s|$|[\]\)）])/.test(afterEpisode)
    return {
      start: n,
      end: n,
      isChapterEnding,
    }
  }

  return null
}

function parseEpisodeRange(comic: ComicInSearch): { start: number; end: number; isChapterEnding: boolean } | null {
  const candidates = [comic.title, stripHtml(comic.titleHtml)]
    .map(normalizeEpisodeTitle)
    .filter((t) => t.length > 0)
  for (const title of candidates) {
    const range = parseEpisodeRangeFromTitle(title)
    if (range !== null) return range
  }
  return null
}

export function prefixBeforeRange(title: string): string {
  const normalized = normalizeEpisodeTitle(title)
  const rangeMatch = normalized.match(/(\d+)\s*-\s*(\d+)\s*(?:[话話回集]|$)/)
  if (rangeMatch === null) {
    return normalized.trim()
  }
  const idx = normalized.indexOf(rangeMatch[0])
  return normalized.slice(0, idx).trim()
}

function prefixTokenCount(prefix: string): number {
  return prefix.split(/[\s　]+/).filter((t) => t.length > 0).length
}

function hasSeriesCompleteMarker(title: string): boolean {
  const normalized = normalizeEpisodeTitle(title)
  const batch = normalized.match(/(\d+)\s*-\s*(\d+)\s*(?:[话話回集]|$)/)
  if (batch !== null) {
    if (/\[完結\]/.test(normalized) || /\[.*完結.*\]/.test(normalized)) {
      return true
    }
    const tail = normalized.slice(normalized.indexOf(batch[0]) + batch[0].length)
    return /完/.test(tail)
  }
  if (/\[完結\]/.test(normalized) || /第\d+季完結/.test(normalized)) {
    return false
  }
  return false
}

function isAnthologyTitle(
  title: string,
  imageCount: number,
  rangeStart: number,
  rangeEnd: number,
  globalMax: number,
): boolean {
  const span = rangeEnd - rangeStart + 1
  const prefix = prefixBeforeRange(title)
  const tokens = prefixTokenCount(prefix)

  if (tokens >= 3 && span >= 20) {
    return true
  }

  if (globalMax >= 10 && rangeStart <= 2 && rangeEnd >= globalMax - 1 && hasSeriesCompleteMarker(title)) {
    return true
  }

  if (
    globalMax >= 3 &&
    rangeStart <= 1 &&
    rangeEnd >= globalMax &&
    hasSeriesCompleteMarker(title) &&
    /(\d+)\s*-\s*(\d+)\s*(?:[话話回集]|$)/.test(normalizeEpisodeTitle(title))
  ) {
    return true
  }

  if (
    globalMax >= 50 &&
    rangeStart <= 3 &&
    rangeEnd >= globalMax - 2 &&
    imageCount >= Math.max(1500, globalMax * 15)
  ) {
    return true
  }

  return false
}

export type KoreanDownloadStrategy = 'episodes' | 'anthology'

export type ClassifiedKoreanItem = {
  comic: ComicInSearch
  kind: 'episode' | 'anthology'
  rangeStart: number
  rangeEnd: number
  isChapterEnding: boolean
  imageCount: number
}

export type KoreanWebtoonAnalysis = {
  tagLabel: string
  episodes: ClassifiedKoreanItem[]
  anthologies: ClassifiedKoreanItem[]
  rangeMin: number
  rangeMax: number
  coherenceWarning?: string
  marksComplete: boolean
}

export function analyzeKoreanWebtoon(comics: ComicInSearch[], tagLabel: string): KoreanWebtoonAnalysis {
  const traditionalTagLabel = toTraditionalChinese(tagLabel)
  const rows: Array<{
    comic: ComicInSearch
    range: { start: number; end: number; isChapterEnding: boolean }
    imageCount: number
  }> = []
  for (const comic of comics) {
    const range = parseEpisodeRange(comic)
    if (range === null) continue
    rows.push({
      comic,
      range,
      imageCount: parseImageCount(comic.additionalInfo),
    })
  }

  let globalMax = 0
  for (const row of rows) {
    globalMax = Math.max(globalMax, row.range.end)
  }

  const episodes: ClassifiedKoreanItem[] = []
  const anthologies: ClassifiedKoreanItem[] = []
  for (const { comic, range, imageCount } of rows) {
    const item: ClassifiedKoreanItem = {
      comic,
      kind: 'episode',
      rangeStart: range.start,
      rangeEnd: range.end,
      isChapterEnding: range.isChapterEnding,
      imageCount,
    }
    if (isAnthologyTitle(comic.title, imageCount, range.start, range.end, globalMax)) {
      item.kind = 'anthology'
      anthologies.push(item)
    } else {
      episodes.push(item)
    }
  }
  episodes.sort((a, b) => a.rangeStart - b.rangeStart)
  anthologies.sort((a, b) => b.imageCount - a.imageCount)

  let rangeMin = Number.POSITIVE_INFINITY
  let rangeMax = 0
  for (const e of episodes) {
    rangeMin = Math.min(rangeMin, e.rangeStart)
    rangeMax = Math.max(rangeMax, e.rangeEnd)
  }
  for (const a of anthologies) {
    rangeMin = Math.min(rangeMin, a.rangeStart)
    rangeMax = Math.max(rangeMax, a.rangeEnd)
  }
  if (!Number.isFinite(rangeMin)) {
    rangeMin = 1
    rangeMax = globalMax || 1
  }

  const mergedCoverage: Array<{ start: number; end: number }> = []
  for (const item of [...episodes, ...anthologies].sort((a, b) => a.rangeStart - b.rangeStart)) {
    const last = mergedCoverage[mergedCoverage.length - 1]
    if (last !== undefined && item.rangeStart <= last.end + 1) {
      last.end = Math.max(last.end, item.rangeEnd)
    } else {
      mergedCoverage.push({ start: item.rangeStart, end: item.rangeEnd })
    }
  }
  const gaps: string[] = []
  for (let i = 1; i < mergedCoverage.length; i++) {
    const prev = mergedCoverage[i - 1]
    const curr = mergedCoverage[i]
    if (curr.start > prev.end + 1) {
      gaps.push(`第 ${prev.end + 1}–${curr.start - 1} 話`)
    }
  }
  if (mergedCoverage.length > 0 && mergedCoverage[0].start > 1) {
    gaps.unshift(`第 1–${mergedCoverage[0].start - 1} 話`)
  }

  const marksComplete =
    episodes.some((e) => e.isChapterEnding) ||
    anthologies.some((a) => a.rangeEnd >= rangeMax && hasSeriesCompleteMarker(a.comic.title))

  let coherenceWarning: string | undefined
  if (gaps.length > 0) {
    coherenceWarning = `話數可能不連續，缺少：${gaps.join('、')}`
  }
  if (!marksComplete && episodes.length > 0) {
    coherenceWarning = [coherenceWarning, '未偵測到完結話數（[完結] 或系列「完」）'].filter(Boolean).join('；')
  }

  return {
    tagLabel: traditionalTagLabel,
    episodes,
    anthologies,
    rangeMin,
    rangeMax,
    coherenceWarning,
    marksComplete,
  }
}

export function defaultCheckedIds(analysis: KoreanWebtoonAnalysis, strategy: KoreanDownloadStrategy): number[] {
  if (strategy === 'anthology') return analysis.anthologies.map((a) => a.comic.id)
  return analysis.episodes.map((e) => e.comic.id)
}
