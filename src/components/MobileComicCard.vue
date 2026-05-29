<script setup lang="ts">
import type { ComicInSearch } from '../api'

defineProps<{
  comic: ComicInSearch
  layout?: 'grid' | 'list'
  favorited?: boolean
  catalogAnalysisNote?: string
}>()

const isDuplicateNote = (note: string | undefined) =>
  note !== undefined && note.startsWith('與列表中')

const emit = defineEmits<{
  detail: [id: number]
  read: [id: number]
  download: [id: number]
  toggleFavorite: [comic: ComicInSearch]
}>()
</script>

<template>
  <article class="card" :class="{ 'card--list': layout === 'list' }">
    <template v-if="layout === 'list'">
      <div class="list-row">
        <div class="cover-wrap list-cover">
          <button
            type="button"
            class="fav-star"
            :class="{ on: favorited }"
            :title="favorited ? '取消收藏' : '收藏漫畫'"
            @click.stop="emit('toggleFavorite', comic)"
          >
            {{ favorited ? '★' : '☆' }}
          </button>
          <img
            v-if="comic.cover"
            class="cover list-img"
            :src="comic.cover"
            :alt="comic.title"
            loading="lazy"
          />
          <div v-else class="cover placeholder list-img" />
        </div>
        <div class="list-main">
          <h3 class="title" :title="comic.title" v-html="comic.titleHtml || comic.title" />
          <p class="info">{{ comic.additionalInfo }}</p>
          <p
            v-if="catalogAnalysisNote"
            class="catalog-note"
            :class="isDuplicateNote(catalogAnalysisNote) ? 'catalog-note--dup' : 'catalog-note--ok'"
            :title="catalogAnalysisNote"
          >
            {{ catalogAnalysisNote }}
          </p>
          <div class="actions">
            <button type="button" class="act primary" @click.stop="emit('detail', comic.id)">詳情</button>
            <button type="button" class="act primary" @click.stop="emit('read', comic.id)">閱讀</button>
            <button type="button" class="act primary" @click.stop="emit('download', comic.id)">下載</button>
          </div>
        </div>
      </div>
    </template>
    <template v-else>
      <div class="cover-wrap">
        <button
          type="button"
          class="fav-star"
          :class="{ on: favorited }"
          :title="favorited ? '取消收藏' : '收藏漫畫'"
          @click.stop="emit('toggleFavorite', comic)"
        >
          {{ favorited ? '★' : '☆' }}
        </button>
        <img v-if="comic.cover" class="cover" :src="comic.cover" :alt="comic.title" loading="lazy" />
        <div v-else class="cover placeholder" />
        <span v-if="comic.isDownloaded" class="badge badge-dl">已下載</span>
      </div>
      <div class="body">
        <h3 class="title" :title="comic.title" v-html="comic.titleHtml || comic.title" />
        <p class="info">{{ comic.additionalInfo }}</p>
        <p
          v-if="catalogAnalysisNote"
          class="catalog-note"
          :class="isDuplicateNote(catalogAnalysisNote) ? 'catalog-note--dup' : 'catalog-note--ok'"
          :title="catalogAnalysisNote"
        >
          {{ catalogAnalysisNote }}
        </p>
        <div class="actions">
          <button type="button" class="act primary" @click.stop="emit('detail', comic.id)">詳情</button>
          <button type="button" class="act primary" @click.stop="emit('read', comic.id)">閱讀</button>
          <button type="button" class="act primary" @click.stop="emit('download', comic.id)">下載</button>
        </div>
      </div>
    </template>
  </article>
</template>

<style scoped>
.card {
  display: flex;
  flex-direction: column;
  min-width: 0;
  height: 100%;
  background: #1e1e1e;
  border: 1px solid #333;
  border-radius: 4px;
  overflow: hidden;
}

.card--list {
  height: auto;
}

.list-row {
  display: flex;
  gap: 8px;
  padding: 6px 8px;
}

.list-cover {
  position: relative;
  flex: 0 0 52px;
  background: rgba(255, 255, 255, 0.06);
}

.list-img {
  width: 52px;
  height: 70px;
  object-fit: cover;
  display: block;
}

.list-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.cover-wrap {
  position: relative;
  width: 100%;
  background: rgba(255, 255, 255, 0.06);
}

.cover {
  width: 100%;
  aspect-ratio: 3 / 4;
  object-fit: cover;
  display: block;
}

.cover.placeholder {
  aspect-ratio: 3 / 4;
}

.fav-star {
  position: absolute;
  top: 2px;
  right: 2px;
  z-index: 2;
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.55);
  color: #ccc;
  font-size: 13px;
  line-height: 22px;
}

.fav-star.on {
  color: #f0c040;
}

.badge,
.badge-dl {
  position: absolute;
  top: 2px;
  left: 2px;
  font-size: 8px;
  padding: 1px 3px;
  border-radius: 2px;
  background: rgba(0, 0, 0, 0.65);
  color: #8fd18f;
}

.body {
  display: flex;
  flex-direction: column;
  flex: 1;
  gap: 2px;
  padding: 4px;
}

.title {
  margin: 0;
  font-size: 10px;
  line-height: 1.25;
  font-weight: 600;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  word-break: break-all;
}

.title :deep(em) {
  color: #6eb5ff;
  font-style: normal;
}

.info {
  margin: 0;
  font-size: 9px;
  color: #aaa;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.catalog-note {
  font-size: 9px;
  line-height: 1.25;
  margin: 2px 0 0;
  padding: 2px 4px;
  border-radius: 3px;
  word-break: break-word;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.catalog-note--dup {
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.12);
  border: 1px solid rgba(251, 191, 36, 0.35);
}

.catalog-note--ok {
  color: #86efac;
  background: rgba(34, 197, 94, 0.1);
  border: 1px solid rgba(34, 197, 94, 0.25);
}

.actions {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 2px;
  margin-top: auto;
}

.act {
  width: 100%;
  min-width: 0;
  padding: 3px 1px;
  border: none;
  border-radius: 3px;
  font-size: 10px;
  line-height: 1.2;
  white-space: nowrap;
  overflow: visible;
  background: #3a3a3a;
  color: #eee;
}

.list-main .act {
  flex: 0 0 auto;
  padding: 4px 8px;
  font-size: 11px;
}

.act.primary {
  background: #3d6ef5;
  color: #fff;
}
</style>
