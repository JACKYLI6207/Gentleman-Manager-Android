<script setup lang="ts">
import { nextTick, onActivated, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { Comic } from '../api'
import { getReaderImage } from '../api'
import { useReaderAspectRatio } from '../composables/useReaderAspectRatio'
import { useReaderChromeAutoHide } from '../composables/useReaderChromeAutoHide'
import { useReaderFullscreen } from '../composables/useReaderFullscreen'
import {
  isReaderScrollLocked,
  registerReaderScrollControl,
  unregisterReaderScrollControl,
  updateReaderScrollProgress,
} from '../readerProgressBridge'
import {
  computeContentRatio,
  computeVisiblePageIndex,
  seekToContentRatio,
} from '../readerProgressMath'
import { getReaderPages } from '../readerUtils'
import MobileReaderProgressBar from './MobileReaderProgressBar.vue'
import ReaderAspectRatioMenu from './ReaderAspectRatioMenu.vue'
import '../readerShared.css'

const props = defineProps<{
  comic: Comic | null
  active: boolean
  readingActive: boolean
}>()

const emit = defineEmits<{
  stop: []
}>()

const { isFullscreen, toggleFullscreen, exitFullscreen } = useReaderFullscreen()
const {
  chromeVisible,
  onReaderScrollForChrome,
  onReaderTouchStart,
  onReaderTouchMove,
  onReaderTouchEnd,
  onReaderTouchCancel,
} = useReaderChromeAutoHide()
const { scrollClass: aspectScrollClass } = useReaderAspectRatio()

const pageSrc = ref<Map<number, string>>(new Map())
const loadingSet = ref<Set<number>>(new Set())
const failedSet = ref<Set<number>>(new Set())
const scrollRef = ref<HTMLElement | null>(null)
let session = 0
let observer: IntersectionObserver | undefined
const queue: number[] = []
let activeLoads = 0

const pages = () => (props.comic ? getReaderPages(props.comic.imgList) : [])

function revokeAll() {
  for (const url of pageSrc.value.values()) URL.revokeObjectURL(url)
  pageSrc.value = new Map()
}

function reset() {
  session += 1
  revokeAll()
  loadingSet.value = new Set()
  failedSet.value = new Set()
  queue.length = 0
  activeLoads = 0
  observer?.disconnect()
}

function stopReading() {
  reset()
  exitFullscreen()
  emit('stop')
}

async function loadPage(index: number) {
  const comic = props.comic
  if (!comic || !props.active || !props.readingActive) return
  const list = pages()
  const page = list[index]
  if (!page || pageSrc.value.has(index) || loadingSet.value.has(index)) return
  const mySession = session
  loadingSet.value = new Set(loadingSet.value).add(index)
  try {
    const bytes = await getReaderImage(comic.id, page.url)
    if (mySession !== session) return
    const blob = new Blob([new Uint8Array(bytes)])
    const url = URL.createObjectURL(blob)
    const next = new Map(pageSrc.value)
    next.set(index, url)
    pageSrc.value = next
  } catch {
    if (mySession === session) {
      failedSet.value = new Set(failedSet.value).add(index)
    }
  } finally {
    const ls = new Set(loadingSet.value)
    ls.delete(index)
    loadingSet.value = ls
    pump()
  }
}

function enqueue(index: number) {
  if (!props.readingActive) return
  if (!queue.includes(index)) queue.push(index)
  pump()
}

function pump() {
  while (activeLoads < 4 && queue.length > 0) {
    const idx = queue.shift()!
    activeLoads += 1
    void loadPage(idx).finally(() => {
      activeLoads -= 1
      pump()
    })
  }
}

function setupObserver() {
  observer?.disconnect()
  const root = scrollRef.value
  if (!root) return
  observer = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue
        const i = Number((e.target as HTMLElement).dataset.index)
        if (!Number.isNaN(i)) enqueue(i)
      }
    },
    { root, rootMargin: '400px 0px', threshold: 0 },
  )
}

function setPageRef(index: number, el: Element | { $el?: unknown } | null) {
  const node =
    el instanceof HTMLElement ? el : (el as { $el?: unknown } | null)?.$el
  if (!(node instanceof HTMLElement)) return
  observer?.observe(node)
  enqueue(index)
}

function primeInitialPages() {
  const total = pages().length
  for (let i = 0; i < Math.min(4, total); i++) enqueue(i)
}

function updateScrollProgressBridge() {
  if (!props.readingActive || !props.active) return
  const root = scrollRef.value
  const total = pages().length
  if (!root || total <= 0) return
  const pageIndex = computeVisiblePageIndex(root)
  const contentRatio = computeContentRatio(root, total, pageIndex)
  updateReaderScrollProgress({
    active: true,
    ratio: contentRatio,
    totalPages: total,
    currentPage: pageIndex + 1,
    pageLabel: `${pageIndex + 1}/${total}頁`,
  })
}

function seekByRatio(ratio: number) {
  const root = scrollRef.value
  const total = pages().length
  if (!root || total <= 0) return
  const targetIndex = seekToContentRatio(root, total, ratio)
  enqueue(targetIndex)
  for (let d = 1; d <= 3; d++) {
    if (targetIndex - d >= 0) enqueue(targetIndex - d)
    if (targetIndex + d < total) enqueue(targetIndex + d)
  }
}

function onReaderScroll() {
  onReaderScrollForChrome()
  if (!props.readingActive || !props.active || isReaderScrollLocked()) return
  updateScrollProgressBridge()
}

async function activateReader() {
  if (!props.readingActive || !props.comic) return
  await nextTick()
  await nextTick()
  scrollRef.value?.scrollTo({ top: 0 })
  setupObserver()
  primeInitialPages()
  await nextTick()
  updateScrollProgressBridge()
}

watch(
  () => [props.comic?.id, props.active, props.readingActive] as const,
  async () => {
    reset()
    if (!props.comic || !props.active || !props.readingActive) {
      unregisterReaderScrollControl()
      return
    }
      registerReaderScrollControl({
        seek: seekByRatio,
        savePosition: () => {
          const root = scrollRef.value
          const total = pages().length
          if (!root || total <= 0) return
          const pageIndex = computeVisiblePageIndex(root)
          updateScrollProgressBridge()
          void pageIndex
        },
      })
    await activateReader()
  },
  { immediate: true },
)

function resumeIfNeeded() {
  if (props.comic && props.active && props.readingActive) {
    void activateReader()
  }
}

onMounted(() => resumeIfNeeded())
onActivated(() => resumeIfNeeded())

onBeforeUnmount(() => {
  unregisterReaderScrollControl()
  reset()
  exitFullscreen()
})
</script>

<template>
  <div class="online-read-root">
  <div v-if="!comic" class="reader-idle">
    <p>請在詳情或搜索結果點「閱讀」</p>
  </div>
  <div
    v-else
    :class="['reader-shell', { 'reader-shell--fullscreen': isFullscreen && readingActive }]"
  >
    <div v-if="!readingActive" class="reader-header">
      <div class="reader-header-title">
        <span class="idle-label">在線閱讀</span>
      </div>
    </div>

    <div
      ref="scrollRef"
      class="reader-scroll"
      :class="aspectScrollClass"
      @scroll="onReaderScroll"
      @touchstart.passive="onReaderTouchStart"
      @touchmove.passive="onReaderTouchMove"
      @touchend="onReaderTouchEnd"
      @touchcancel="onReaderTouchCancel"
    >
      <div v-if="!readingActive" class="reader-idle">
        <p>請在「漫畫詳情」點擊「閱讀」開始</p>
      </div>
      <template v-else>
        <div
          v-for="(page, index) in pages()"
          :key="`${comic.id}-${index}`"
          :ref="(el) => setPageRef(index, el)"
          class="reader-page"
          :data-index="index"
        >
          <img
            v-if="pageSrc.get(index)"
            :src="pageSrc.get(index)"
            :alt="page.caption"
            class="reader-img"
          />
          <p v-else-if="failedSet.has(index)" class="reader-ph">載入失敗，請向下捲動重試</p>
          <p v-else-if="loadingSet.has(index)" class="reader-ph reader-ph--hint">載入中…</p>
        </div>
      </template>
    </div>

    <div
      v-if="readingActive"
      class="reader-bottom-chrome"
      :class="{
        'reader-bottom-chrome--fullscreen': isFullscreen,
        'reader-bottom-chrome--hidden': isFullscreen && !chromeVisible,
      }"
    >
      <div class="reader-header reader-header--bottom">
        <ReaderAspectRatioMenu />
        <div class="reader-header-actions">
          <button type="button" class="reader-btn" @click="toggleFullscreen">
            {{ isFullscreen ? '視窗模式' : '全視窗' }}
          </button>
          <button type="button" class="reader-btn" @click="stopReading">停止閱讀</button>
        </div>
      </div>
      <MobileReaderProgressBar v-if="!isFullscreen" />
    </div>
  </div>
  </div>
</template>

<style scoped>
.online-read-root {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  height: 100%;
}
.page-meta {
  font-weight: 400;
  font-size: 12px;
  opacity: 0.7;
  margin-left: 6px;
}
.idle-label {
  opacity: 0.6;
}
</style>
