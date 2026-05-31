<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Comic } from '../api'

const props = defineProps<{
  comic: Comic | null
  loading: boolean
  error: string
  createdLabel?: string
  favorited?: boolean
  snapshotSearchAvailable?: boolean
}>()

const emit = defineEmits<{
  read: []
  download: []
  tagSearch: [tagName: string]
  toggleFavorite: []
  detailSearch: [mode: 'global' | 'snapshot']
}>()

const searchMenuOpen = ref(false)

function toggleSearchMenu() {
  searchMenuOpen.value = !searchMenuOpen.value
}

function closeSearchMenu() {
  searchMenuOpen.value = false
}

function pickSearch(mode: 'global' | 'snapshot') {
  if (mode === 'snapshot' && !props.snapshotSearchAvailable) return
  closeSearchMenu()
  emit('detailSearch', mode)
}

watch(
  () => props.comic?.id,
  () => {
    closeSearchMenu()
  },
)

defineExpose({ closeSearchMenu })
</script>

<template>
  <div class="detail">
    <p v-if="loading" class="detail-msg">載入詳情中…</p>
    <p v-else-if="error" class="detail-msg detail-msg--err">{{ error }}</p>
    <p v-else-if="!comic" class="detail-msg">請在搜索結果點「詳情」，或先瀏覽列表</p>
    <template v-else>
      <h2 class="detail-title">{{ comic.title }}</h2>
      <div class="detail-top">
        <div class="detail-cover-wrap">
          <button
            type="button"
            class="fav-star"
            :class="{ on: favorited }"
            :title="favorited ? '取消收藏' : '收藏漫畫'"
            @click.stop="emit('toggleFavorite')"
          >
            {{ favorited ? '★' : '☆' }}
          </button>
          <img v-if="comic.cover" class="detail-cover" :src="comic.cover" :alt="comic.title" />
          <div v-else class="detail-cover detail-cover--ph" />
        </div>
        <div class="detail-meta">
          <p>ID：{{ comic.id }}</p>
          <p>分類：{{ comic.category }}</p>
          <p>頁數：{{ comic.imageCount }}P</p>
          <p v-if="createdLabel">創建：{{ createdLabel }}</p>
          <p v-if="comic.isDownloaded" class="downloaded">已下載</p>
          <div class="detail-actions">
            <button type="button" class="detail-btn primary" @click="emit('read')">閱讀</button>
            <button type="button" class="detail-btn primary" @click="emit('download')">下載</button>
            <div class="detail-search-wrap">
              <button type="button" class="detail-btn primary" @click.stop="toggleSearchMenu">搜索</button>
              <div v-if="searchMenuOpen" class="detail-search-menu" @click.stop>
                <button type="button" class="detail-search-item" @click="pickSearch('global')">
                  全站搜索
                </button>
                <button
                  type="button"
                  class="detail-search-item"
                  :class="{ 'detail-search-item--disabled': !snapshotSearchAvailable }"
                  :disabled="!snapshotSearchAvailable"
                  :title="
                    snapshotSearchAvailable
                      ? '在該漫畫分類快照內搜索漫畫名稱'
                      : '此分類尚無快照，請先到快照列表建立'
                  "
                  @click="pickSearch('snapshot')"
                >
                  快照搜索
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div v-if="comic.tags.length" class="detail-tags">
        <span class="detail-tags-label">標籤</span>
        <div class="detail-tag-list">
          <button
            v-for="tag in comic.tags"
            :key="tag.url"
            type="button"
            class="detail-tag"
            @click="emit('tagSearch', tag.name)"
          >
            {{ tag.name }}
          </button>
        </div>
      </div>
      <div v-if="comic.intro" class="detail-intro" v-html="comic.intro" />
    </template>
  </div>
</template>

<style scoped>
.detail {
  padding: 8px 10px 16px;
  color: #ddd;
}

.detail-msg {
  margin: 24px 0;
  text-align: center;
  font-size: 12px;
  color: #999;
}

.detail-msg--err {
  color: #e88;
}

.detail-title {
  margin: 0 0 8px;
  font-size: 15px;
  line-height: 1.35;
  font-weight: 700;
  word-break: break-word;
}

.detail-top {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}

.detail-cover-wrap {
  position: relative;
  flex: 0 0 100px;
  width: 100px;
}

.fav-star {
  position: absolute;
  top: 4px;
  right: 4px;
  z-index: 2;
  width: 28px;
  height: 28px;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.55);
  color: #ccc;
  font-size: 16px;
  line-height: 28px;
  text-align: center;
}

.fav-star.on {
  color: #ffc107;
}

.detail-cover {
  display: block;
  width: 100%;
  max-height: 140px;
  object-fit: contain;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.06);
}

.detail-cover--ph {
  height: 120px;
}

.detail-meta {
  flex: 1;
  min-width: 0;
  font-size: 11px;
  line-height: 1.5;
  color: #bbb;
}

.detail-meta p {
  margin: 0 0 4px;
}

.downloaded {
  color: #8fd18f;
}

.detail-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.detail-btn {
  padding: 6px 12px;
  border: 1px solid #444;
  border-radius: 5px;
  background: #333;
  color: #eee;
  font-size: 11px;
}

.detail-btn.primary {
  border-color: #3d6ef5;
  background: #3d6ef5;
  color: #fff;
}

.detail-search-wrap {
  position: relative;
}

.detail-search-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  z-index: 20;
  min-width: 112px;
  padding: 4px 0;
  border: 1px solid #444;
  border-radius: 6px;
  background: #252525;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
}

.detail-search-item {
  display: block;
  width: 100%;
  padding: 8px 12px;
  border: none;
  background: transparent;
  color: #eee;
  font-size: 11px;
  text-align: left;
}

.detail-search-item:active:not(:disabled) {
  background: rgba(61, 110, 245, 0.25);
}

.detail-search-item--disabled,
.detail-search-item:disabled {
  color: #666;
  cursor: not-allowed;
}

.detail-tags {
  margin-top: 12px;
}

.detail-tags-label {
  display: block;
  margin-bottom: 6px;
  font-size: 11px;
  font-weight: 600;
}

.detail-tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.detail-tag {
  padding: 3px 8px;
  border: 1px solid #444;
  border-radius: 12px;
  background: #252525;
  color: #ccc;
  font-size: 10px;
}

.detail-intro {
  margin-top: 12px;
  font-size: 11px;
  line-height: 1.45;
  word-break: break-word;
  color: #aaa;
}

.detail-intro :deep(a) {
  color: #8ab4f8;
}
</style>
