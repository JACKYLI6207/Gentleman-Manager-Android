<script setup lang="ts">
import type { SearchTabBookmark } from '../searchTabBookmarkTypes'

defineProps<{
  favoriteTabs: SearchTabBookmark[]
}>()

const emit = defineEmits<{
  openTab: [bookmark: SearchTabBookmark]
  removeTab: [sourceTabId: string]
}>()
</script>

<template>
  <div class="fav-panel">
    <p v-if="favoriteTabs.length === 0" class="empty">尚無收藏分頁。在搜索結果分頁標題前點 ☆ 加入。</p>
    <button
      v-for="b in favoriteTabs"
      :key="b.id"
      type="button"
      class="fav-row"
      @click="emit('openTab', b)"
    >
      <span class="fav-title">{{ b.title }}</span>
      <span class="fav-time">{{ b.savedAt.slice(0, 10) }}</span>
      <button type="button" class="fav-del" title="移除" @click.stop="emit('removeTab', b.sourceTabId)">×</button>
    </button>
  </div>
</template>

<style scoped>
.fav-panel {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
.empty {
  text-align: center;
  color: #999;
  font-size: 12px;
  padding: 20px 8px;
}
.fav-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px;
  margin-bottom: 6px;
  border: 1px solid #444;
  border-radius: 6px;
  background: #252525;
  color: #eee;
  text-align: left;
}
.thumb {
  width: 36px;
  height: 48px;
  object-fit: cover;
  border-radius: 3px;
  flex-shrink: 0;
}
.fav-title {
  flex: 1;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fav-time {
  font-size: 10px;
  color: #888;
}
.fav-del {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: #aaa;
  font-size: 16px;
}
</style>
