<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'
import {
  scanLanRemotePcs,
  testRemotePcConnection,
  type RemotePcListItem,
} from '../api'
import MobileRemoteBrowse from './MobileRemoteBrowse.vue'

const scanning = ref(false)
const status = ref('')
const pcs = ref<RemotePcListItem[]>([])
const browsePc = ref<RemotePcListItem | null>(null)
let scanGeneration = 0

async function refreshList() {
  const gen = ++scanGeneration
  scanning.value = true
  status.value = '正在掃描區網內的 PC…'
  pcs.value = []
  browsePc.value = null
  try {
    const discovered = await scanLanRemotePcs()
    if (gen !== scanGeneration) return
    if (discovered.length === 0) {
      status.value =
        '未找到 PC。請確認：① 手機與 PC 同一 Wi‑Fi（建議 192.168.x.x 同一網段）；② PC 設定顯示「執行中」；③ Windows 防火牆允許本程式私人網路。'
      return
    }
    pcs.value = discovered.map((pc) => ({
      ...pc,
      connected: null,
      message: '測試連線中…',
      connectedHost: null,
    }))
    status.value = `找到 ${discovered.length} 台 PC，正在測試連線…`
    await Promise.all(
      pcs.value.map(async (pc, index) => {
        const result = await testRemotePcConnection(pc.hosts, pc.port)
        if (gen !== scanGeneration) return
        pcs.value[index] = {
          ...pc,
          connected: result.connected,
          message: result.message,
          connectedHost: result.connectedHost,
        }
      }),
    )
    if (gen !== scanGeneration) return
    const okCount = pcs.value.filter((p) => p.connected === true).length
    status.value = `掃描完成：${okCount} / ${pcs.value.length} 台能連線`
  } catch (e) {
    if (gen !== scanGeneration) return
    status.value = String(e)
  } finally {
    if (gen === scanGeneration) scanning.value = false
  }
}

function openManage(pc: RemotePcListItem) {
  if (pc.connected !== true) return
  browsePc.value = pc
}

onBeforeUnmount(() => {
  scanGeneration++
})

void refreshList()
</script>

<template>
  <div class="remote-manage-root">
  <div v-if="browsePc" class="remote-browse-shell">
    <MobileRemoteBrowse :pc="browsePc" @exit="browsePc = null" />
  </div>
  <div v-else class="remote-manage">
    <div class="remote-manage-toolbar">
      <p class="remote-manage-hint">
        請與 PC 連接<strong>同一 Wi‑Fi</strong>。PC 版請在「設定」勾選「開放遠端管理」並儲存，且保持程式執行。
      </p>
      <button type="button" class="tool tool--primary" :disabled="scanning" @click="refreshList">
        {{ scanning ? '掃描中…' : '重新掃描' }}
      </button>
    </div>
    <p v-if="status" class="remote-manage-status">{{ status }}</p>
    <ul v-if="pcs.length > 0" class="remote-pc-list">
      <li v-for="(pc, i) in pcs" :key="`${pc.hosts.join(',')}:${pc.port}:${i}`" class="remote-pc-item">
        <div class="remote-pc-main">
          <span class="remote-pc-name">{{ pc.name }}</span>
          <span class="remote-pc-addr">{{ pc.hosts.join(' / ') }}:{{ pc.port }}</span>
        </div>
        <div class="remote-pc-footer">
          <span
            class="remote-pc-badge"
            :class="{
              'remote-pc-badge--ok': pc.connected === true,
              'remote-pc-badge--fail': pc.connected === false,
              'remote-pc-badge--pending': pc.connected === null,
            }"
          >
            {{
              pc.connected === true
                ? '能連線'
                : pc.connected === false
                  ? '不能連線'
                  : '辨識中'
            }}
          </span>
          <button
            v-if="pc.connected === true"
            type="button"
            class="remote-pc-manage-btn"
            @click="openManage(pc)"
          >
            管理
          </button>
        </div>
        <p v-if="pc.message && pc.connected !== true" class="remote-pc-msg">{{ pc.message }}</p>
      </li>
    </ul>
  </div>
  </div>
</template>

<style scoped>
.remote-manage-root {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.remote-browse-shell {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.remote-manage {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  padding: 12px 14px 24px;
}

.remote-manage-toolbar {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 12px;
}

.remote-manage-hint {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  opacity: 0.85;
}

.remote-manage-status {
  margin: 0 0 12px;
  font-size: 13px;
  opacity: 0.8;
}

.remote-pc-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.remote-pc-item {
  border: 1px solid var(--gm-border, rgba(255, 255, 255, 0.12));
  border-radius: 10px;
  padding: 12px;
  background: rgba(0, 0, 0, 0.15);
}

.remote-pc-main {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 8px;
}

.remote-pc-name {
  font-weight: 600;
  font-size: 15px;
}

.remote-pc-addr {
  font-size: 12px;
  opacity: 0.7;
  font-family: ui-monospace, monospace;
}

.remote-pc-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.remote-pc-badge {
  display: inline-block;
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.1);
}

.remote-pc-badge--ok {
  background: rgba(72, 187, 120, 0.25);
  color: #9ae6b4;
}

.remote-pc-badge--fail {
  background: rgba(245, 101, 101, 0.2);
  color: #feb2b2;
}

.remote-pc-badge--pending {
  opacity: 0.75;
}

.remote-pc-manage-btn {
  flex-shrink: 0;
  padding: 6px 16px;
  border-radius: 8px;
  border: none;
  background: #3182ce;
  color: #fff;
  font-size: 14px;
  font-weight: 500;
}

.remote-pc-msg {
  margin: 8px 0 0;
  font-size: 12px;
  opacity: 0.7;
  line-height: 1.4;
}
</style>
