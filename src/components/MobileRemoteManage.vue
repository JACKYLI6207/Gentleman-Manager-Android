<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  enterRemoteWifiMode,
  leaveRemoteWifiMode,
  scanLanRemotePcs,
  testRemotePcConnection,
  type RemotePcListItem,
} from '../api'
import MobileRemoteBrowse from './MobileRemoteBrowse.vue'

const scanning = ref(false)
const status = ref('')
const pcs = ref<RemotePcListItem[]>([])
const browsePc = ref<RemotePcListItem | null>(null)
const scanLog = ref('')
const showScanLog = ref(true)
const copyLogHint = ref('')
const scanLogRef = ref<HTMLTextAreaElement | null>(null)
const manualIp = ref('')
const manualPort = ref(8765)
const manualTesting = ref(false)
let scanGeneration = 0

const scanLogCopyText = computed(() => scanLog.value)

function appendLog(section: string, lines: string[]) {
  if (!scanLog.value) return
  scanLog.value += `\n\n--- ${section} ---\n${lines.join('\n')}`
}

async function copyScanLog() {
  const text = scanLogCopyText.value
  if (!text) return
  copyLogHint.value = ''
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
      copyLogHint.value = '已複製到剪貼簿'
    } else {
      throw new Error('clipboard unavailable')
    }
  } catch {
    const el = scanLogRef.value
    if (el) {
      el.focus()
      el.select()
      el.setSelectionRange(0, el.value.length)
      try {
        if (document.execCommand('copy')) {
          copyLogHint.value = '已複製到剪貼簿'
        } else {
          copyLogHint.value = '請長按文字區全選後複製'
        }
      } catch {
        copyLogHint.value = '請長按文字區全選後複製'
      }
    } else {
      copyLogHint.value = '請長按文字區全選後複製'
    }
  }
  window.setTimeout(() => {
    copyLogHint.value = ''
  }, 2000)
}

async function refreshList() {
  const gen = ++scanGeneration
  scanning.value = true
  status.value = '正在掃描區網內的 PC…'
  pcs.value = []
  browsePc.value = null
  scanLog.value = ''
  copyLogHint.value = ''
  try {
    const result = await scanLanRemotePcs()
    if (gen !== scanGeneration) return
    scanLog.value = result.log
    showScanLog.value = true

    const discovered = result.pcs
    if (discovered.length === 0) {
      status.value =
        '未找到 PC。請查看下方 LOG 並複製回報，或確認：① 同一 Wi‑Fi ② PC 遠端管理「執行中」③ 防火牆允許私人網路。'
      return
    }
    pcs.value = discovered.map((pc) => ({
      ...pc,
      connected: null,
      message: '測試連線中…',
      connectedHost: null,
    }))
    status.value = `找到 ${discovered.length} 台 PC，正在測試連線…`
    const testLines: string[] = []
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
        testLines.push(
          `${pc.name} (${pc.hosts.join(' / ')}:${pc.port}) → ${
            result.connected ? `OK ${result.connectedHost ?? ''}` : result.message
          }`,
        )
      }),
    )
    if (gen !== scanGeneration) return
    appendLog('連線測試', testLines)
    const okCount = pcs.value.filter((p) => p.connected === true).length
    status.value = `掃描完成：${okCount} / ${pcs.value.length} 台能連線`
  } catch (e) {
    if (gen !== scanGeneration) return
    status.value = String(e)
    scanLog.value += `\n\n--- 錯誤 ---\n${String(e)}`
    showScanLog.value = true
  } finally {
    if (gen === scanGeneration) scanning.value = false
  }
}

function openManage(pc: RemotePcListItem) {
  if (pc.connected !== true) return
  browsePc.value = pc
}

async function connectManualIp() {
  const ip = manualIp.value.trim()
  if (!ip) {
    status.value = '請輸入 PC 的 IP 位址（例如 192.168.0.100）'
    return
  }
  manualTesting.value = true
  status.value = `正在連線 ${ip}:${manualPort.value}…`
  try {
    const result = await testRemotePcConnection([ip], manualPort.value)
    scanLog.value += `\n\n--- 手動連線 ---\n${ip}:${manualPort.value} → ${result.message}`
    showScanLog.value = true
    if (!result.connected) {
      status.value = result.message
      return
    }
    const existing = pcs.value.findIndex(
      (p) => p.port === manualPort.value && p.hosts.includes(ip),
    )
    const item: RemotePcListItem = {
      name: `PC (${ip})`,
      hosts: [ip],
      port: manualPort.value,
      connected: true,
      message: result.message,
      connectedHost: result.connectedHost ?? ip,
    }
    if (existing >= 0) {
      pcs.value[existing] = item
    } else {
      pcs.value = [item, ...pcs.value]
    }
    status.value = `已連線 ${ip}，可點「管理」進入`
  } catch (e) {
    status.value = String(e)
  } finally {
    manualTesting.value = false
  }
}

onBeforeUnmount(() => {
  scanGeneration++
  void leaveRemoteWifiMode()
})

onMounted(async () => {
  try {
    const wifiMsg = await enterRemoteWifiMode()
    scanLog.value = `[Wi‑Fi 區網模式] ${wifiMsg}\n`
    showScanLog.value = true
  } catch {
    // ignore
  }
  void refreshList()
})
</script>

<template>
  <div class="remote-manage-root">
  <div v-if="browsePc" class="remote-browse-shell">
    <MobileRemoteBrowse :pc="browsePc" @exit="browsePc = null" />
  </div>
  <div v-else class="remote-manage">
    <div class="remote-manage-toolbar">
      <p class="remote-manage-hint">
        請與 PC 連接<strong>同一 Wi‑Fi</strong>，建議<strong>暫時關閉行動數據</strong>。
        若其他電腦模擬器能找到、只有本機找不到，可能是路由器<strong> AP 隔離</strong>（手機無法訪問區網內其他裝置）。
        可用手機瀏覽器試開 <code>http://PC的IP:8765/api/v1/health</code> 驗證。
      </p>
      <button type="button" class="tool tool--primary" :disabled="scanning" @click="refreshList">
        {{ scanning ? '掃描中…' : '重新掃描' }}
      </button>
      <div class="remote-manual-connect">
        <p class="remote-manual-label">掃描不到？手動輸入 PC IP（同 Wi‑Fi 網段）</p>
        <div class="remote-manual-row">
          <input
            v-model="manualIp"
            class="remote-manual-input"
            type="text"
            inputmode="decimal"
            placeholder="192.168.0.x"
            :disabled="manualTesting || scanning"
          />
          <input
            v-model.number="manualPort"
            class="remote-manual-port"
            type="number"
            min="1"
            max="65535"
            :disabled="manualTesting || scanning"
          />
          <button
            type="button"
            class="tool remote-manual-btn"
            :disabled="manualTesting || scanning"
            @click="connectManualIp"
          >
            {{ manualTesting ? '連線中…' : '連線' }}
          </button>
        </div>
      </div>
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

    <div v-if="scanLog" class="remote-scan-log-wrap">
      <div class="remote-scan-log-header">
        <button type="button" class="remote-scan-log-toggle" @click="showScanLog = !showScanLog">
          {{ showScanLog ? '▼' : '▶' }} 掃描 LOG（可複製）
        </button>
        <div class="remote-scan-log-actions">
          <button type="button" class="tool tool--ghost remote-scan-copy" @click="copyScanLog">
            複製 LOG
          </button>
          <span v-if="copyLogHint" class="remote-scan-copy-hint">{{ copyLogHint }}</span>
        </div>
      </div>
      <textarea
        v-show="showScanLog"
        ref="scanLogRef"
        class="remote-scan-log"
        readonly
        :value="scanLog"
        rows="14"
      />
    </div>
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

.remote-manual-connect {
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px dashed var(--gm-border, rgba(255, 255, 255, 0.18));
  background: rgba(0, 0, 0, 0.1);
}

.remote-manual-label {
  margin: 0 0 8px;
  font-size: 12px;
  opacity: 0.8;
}

.remote-manual-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.remote-manual-input {
  flex: 1;
  min-width: 0;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid var(--gm-border, rgba(255, 255, 255, 0.15));
  background: rgba(0, 0, 0, 0.2);
  color: inherit;
  font-family: ui-monospace, monospace;
  font-size: 14px;
}

.remote-manual-port {
  width: 72px;
  padding: 8px 6px;
  border-radius: 8px;
  border: 1px solid var(--gm-border, rgba(255, 255, 255, 0.15));
  background: rgba(0, 0, 0, 0.2);
  color: inherit;
  font-size: 14px;
}

.remote-manual-btn {
  flex-shrink: 0;
  padding: 8px 14px;
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

.remote-scan-log-wrap {
  margin-top: 16px;
  border: 1px solid var(--gm-border, rgba(255, 255, 255, 0.12));
  border-radius: 10px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.2);
}

.remote-scan-log-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  border-bottom: 1px solid var(--gm-border, rgba(255, 255, 255, 0.08));
}

.remote-scan-log-toggle {
  border: none;
  background: transparent;
  color: inherit;
  font-size: 13px;
  font-weight: 600;
  padding: 4px 0;
}

.remote-scan-log-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.remote-scan-copy {
  font-size: 12px;
  padding: 4px 10px;
}

.remote-scan-copy-hint {
  font-size: 12px;
  opacity: 0.75;
  color: #9ae6b4;
}

.remote-scan-log {
  display: block;
  width: 100%;
  box-sizing: border-box;
  margin: 0;
  padding: 10px 12px;
  border: none;
  background: rgba(0, 0, 0, 0.25);
  color: inherit;
  font-family: ui-monospace, monospace;
  font-size: 11px;
  line-height: 1.45;
  resize: vertical;
  min-height: 180px;
  max-height: 50vh;
  user-select: text;
  -webkit-user-select: text;
}
</style>
