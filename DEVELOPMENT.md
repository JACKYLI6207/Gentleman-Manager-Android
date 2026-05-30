# 開發環境（Android 開源倉庫）

本目錄為 **Gentleman Manager Android** 的輕量原始碼樹，適合推送到 GitHub 並在 clone 後繼續開發。不包含 `node_modules`、Rust `target`、Gradle `build/`、APK、ZIP 或本機備份。

## 環境需求

- [Node.js](https://nodejs.org/) LTS
- [pnpm](https://pnpm.io/installation)
- [Rust](https://www.rust-lang.org/tools/install)
- [Android Studio](https://developer.android.com/studio)（含 SDK、NDK）
- 環境變數 `ANDROID_HOME` 指向 Android SDK
- Windows 建議 JDK 17（Temurin 等）

## 首次設定

```powershell
git clone https://github.com/JACKYLI6207/Gentleman-Manager-Android.git
cd Gentleman-Manager-Android
pnpm install
pnpm tauri android init
```

## 日常開發

```powershell
pnpm tauri android dev
```

瀏覽器僅前端預覽：`pnpm dev`（不含 Rust 功能）。

## 建置 APK（Windows）

| 種類 | 指令 | 產物 |
|------|------|------|
| 快速測試（arm64） | `pnpm android:build:fast` | `Gentleman-Manager-Android-v{version}-fast.apk` |
| 完整（四架構） | `pnpm android:build:full` | `Gentleman-Manager-Android-v{version}.apk` |

或 PowerShell：`powershell -ExecutionPolicy Bypass -File .\build-apk.ps1 -Mode Fast` / `-Mode Full`

## 倉庫體積說明

| 不納入 Git | 取得方式 |
|------------|----------|
| `node_modules/` | `pnpm install` |
| `src-tauri/target/` | 建置時自動產生 |
| `dist/` | `pnpm build` |
| `src-tauri/gen/android/build/` | Gradle 建置 |
| `*.apk` / `*.zip` | 本機建置或 GitHub Releases |
| `*.gm-snapshot.json` | 使用者自行從 PC 版匯出至本機，**勿提交** |

## 快照資料

App 可讀取使用者本機資料夾內由 **桌面版** 匯出的 `.gm-snapshot.json` 標頭與內容。此類檔案可能含第三方標題、標籤或網址描述，**不得**放入本倉庫。
