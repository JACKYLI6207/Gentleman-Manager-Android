# Gentleman Manager Android（紳士管理器 · 安卓版）

Gentleman Manager Android 是以 **Tauri 2 Mobile + Vue 3 + Rust** 製作的 Android 管理工具，用於在行動裝置上瀏覽、搜尋、下載、整理與閱讀內容。本倉庫為 **Android 獨立開源版**，與桌面版 [Gentleman-Manager](https://github.com/JACKYLI6207/Gentleman-Manager) 分開維護，互不干擾 PC 端原始碼。

本倉庫是適合放在 GitHub 上繼續開發的輕量原始碼版本，不包含 `node_modules/`、`dist/`、`src-tauri/target/`、Gradle `build/`、APK、ZIP、快照 JSON 或本機備份檔。clone 後請依照 [DEVELOPMENT.md](./DEVELOPMENT.md) 安裝依賴與建置。

## 目前功能

- 漫畫搜尋：關鍵字全站搜尋，可選快照分類範圍。
- 分類瀏覽：首頁、更新、同人誌、單行本、雜誌短篇、韓漫、排行榜等。
- 搜尋結果：多分頁、列表/網格、排序、每頁數量、動態分頁列（依螢幕寬度與頁碼位數調整）。
- 收藏：收藏漫畫、收藏搜尋分頁。
- 快照列表：讀取本機資料夾內 `.gm-snapshot.json` 標頭，點擊後離線瀏覽/搜尋（需自行從 PC 版匯出）。
- 快照更新掃描：支援接續/更新式掃描（資料存於使用者指定資料夾）。
- 漫畫詳情：封面、分類、標籤、線上閱讀與下載。
- 下載佇列：暫停、繼續、取消；支援 Server 2 ZIP 與 JPEG 打包。
- 韓漫批次：TXT 列表比對、韓漫下載模式。
- 閱讀：線上閱讀、本地 ZIP/CBZ/資料夾閱讀，支援全螢幕與閱讀進度。
- 設定：下載目錄、快照資料夾（SAF）、API 網域、代理等。

## v1.0 重點

- Android 版首次獨立開源發佈。
- 完整四架構 Release APK（見 [Releases](https://github.com/JACKYLI6207/Gentleman-Manager-Android/releases)）。
- 分頁列依單個頁碼寬度與螢幕寬度自動決定顯示數量。
- 「我的收藏」下拉選單改為與「漫畫閱讀」相同之左側對齊。

## 技術棧

- 前端：Vue 3、TypeScript、Vite
- 行動端：Tauri 2 Android
- 後端：Rust、Tokio、reqwest
- 原生：Kotlin（資料夾選擇等插件）
- 套件管理：pnpm

## 開發環境

請先安裝：

- [Node.js](https://nodejs.org/) LTS
- [pnpm](https://pnpm.io/installation)
- [Rust](https://www.rust-lang.org/tools/install)
- [Android Studio](https://developer.android.com/studio)（SDK、NDK）
- [Tauri Mobile 前置條件](https://v2.tauri.app/start/prerequisites/)

首次設定：

```powershell
git clone https://github.com/JACKYLI6207/Gentleman-Manager-Android.git
cd Gentleman-Manager-Android
pnpm install
pnpm tauri android init
```

開發除錯：

```powershell
pnpm tauri android dev
```

建置 APK（Windows）：

```powershell
pnpm android:build:fast   # arm64 快速測試
pnpm android:build:full   # 四架構完整版
```

詳見 [DEVELOPMENT.md](./DEVELOPMENT.md)。

## 倉庫內容說明

此倉庫保留開發所需的原始碼、設定、圖示、建置腳本與 lockfile。以下內容刻意不提交：

- `node_modules/`、`dist/`、`src-tauri/target/`、`src-tauri/gen/android/build/`
- `*.apk`、`*.zip`
- `*.gm-snapshot.json` 及任何含第三方網址/標題/描述之快照或匯出資料
- 下載成品、日誌、本機 `.env` 與暫存檔
- `*.bak*` 備份檔

## 注意事項

- 本專案以 Android 9（API 28）～ Android 14（API 34）為主要測試目標。
- 快照、下載目錄需透過系統文件選擇器（SAF）由使用者自行指定。
- 個人建置的 APK 可能被防毒軟體誤判，建議自行從原始碼建置或只信任作者發布的 Release。
- 本專案不內建、不代管、不隨 Release 發佈任何第三方內容、漫畫圖片、下載成品或真實站點快照。
- 快照讀取、掃描、下載與線上功能均在使用者本機/所選目錄執行；資料來源、保存、分享與使用責任由使用者自行確認。
- 請自行確認使用方式符合所在地法律、目標網站服務條款與第三方權利要求。

## 資料與內容政策

此倉庫只提供工具原始碼、建置設定、圖示與文件。工具可能在使用者本機或所選資料夾產生搜尋結果、快照、下載紀錄、日誌、ZIP 或其他資料，但這些資料不屬於本倉庫內容，也不由本專案代管或背書。

**請勿**將真實站點快照、含第三方標題/標籤/網址/描述的資料包、下載檔案或其他可能涉及第三方權利與平台規範的內容提交到本倉庫或 Issue。若需示範資料格式，應使用虛構、脫敏或最小化的 sample data。

## 授權與免責

授權條款見 [LICENSE](./LICENSE)。

本專案基於 MIT 授權之原始專案修改與延伸，保留原作者 [lanyeeee](https://github.com/lanyeeee) 之著作權聲明。

本工具僅供學習、研究與個人管理用途。專案作者不鼓勵、不協助、不代管任何未經授權的內容取得、散布、再上傳或商業使用。

本專案與任何第三方內容平台、作者、出版方、權利人均無從屬、合作、授權或背書關係。所有第三方名稱、網址、標題、標籤與內容權利均歸其各自權利人所有。

使用者需自行承擔使用本工具、匯入外部快照、使用線上功能、下載資料、保存資料、分享資料或發布衍生檔案所造成的所有風險與責任。作者不對任何資料遺失、帳號限制、網站封鎖、法律爭議、著作權爭議、平台規範違反、第三方權益問題或其他直接/間接損失負責。

## 與桌面版的關係

- 桌面版倉庫：[Gentleman-Manager](https://github.com/JACKYLI6207/Gentleman-Manager)
- Android 版可讀取 PC 版匯出至本機的 `.gm-snapshot.json`，但兩者 Git 倉庫獨立，Release 分開發佈。
