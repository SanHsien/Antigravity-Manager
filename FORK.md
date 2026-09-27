# 關於這份 fork

上游：[`lbjlaq/Antigravity-Manager`](https://github.com/lbjlaq/Antigravity-Manager)（CC-BY-NC-SA-4.0 License）。

`Antigravity-Manager` 是一套專業級 AI 帳號管理與協定代理系統（Tauri v2 + React + Rust），支援將常見的 Web 端 Session（Google Gemini、Anthropic Claude、OpenAI 等）轉換為相容 OpenAI / Gemini 格式的本地標準化 API 端點，並提供智慧多帳號輪替、配額監控與自動故障轉移（Failover）。

本 repo 為 **SanHsien 的 Windows 原生維護型 fork**，確保在 Windows 11 原生環境（非 WSL）下的穩定運作、自動化水位追蹤與健全治理。

---

## 為什麼 fork

1. **Windows 11 原生相容性與開發驗證**：
   確保前端 TypeScript / Vite 建置與後續本機桌面編譯流程在 Windows 11 原生（PowerShell）環境下順暢無阻。
2. **完整維護門禁與自動化**：
   建立 `tools/dev_check.ps1` 一鍵式驗證門禁，整合前端類型與生產打包檢查（`npm run build`）、Python 輔助工具編譯、Markdown 相對連結校驗與上游水位巡檢。
3. **上游水位雙軌追蹤**：
   建立 `tools/upstream_baseline.json` 與 `tools/check_upstream_updates.py`，完整涵蓋 commit、PR 與 issue 三個維度的水位監控與每週 GitHub Actions 定時檢查。
4. **AI 治理與多 Agent 規範**：
   落地 `AGENTS.md` 單一真相源，並提供 `CLAUDE.md`、`GEMINI.md` 以及 `.cursor/rules/no-upstream-pr.mdc`，防止 AI 代理人誤向上游提交 PR 或推送分支。
5. **在地化與文檔治理**：
   提供繁體中文指引與治理文件，並維持文件相對連結正確性。

---

## 與上游的差異

| 項目 | 本 fork (`SanHsien/Antigravity-Manager`) | 上游 (`lbjlaq/Antigravity-Manager`) |
|---|---|---|
| **預設治理規範** | 增加 `FORK.md`、`NOTICE.md`、`REVIEW.md`、`SECURITY.md`、`CONTRIBUTING.md` | 僅上游原始說明與規範 |
| **一鍵式驗證 Gate** | `tools/dev_check.ps1`（前端建置、工具編譯、連結檢查、上游巡檢） | 無單一 PowerShell 一鍵門禁 |
| **上游追蹤水位** | `tools/upstream_baseline.json` + `tools/check_upstream_updates.py` + 每週 Actions | 無 |
| **AI 代理規範** | `AGENTS.md`（疊加維護 overlay）、`CLAUDE.md`、`GEMINI.md`、`.cursor/rules/no-upstream-pr.mdc` | 僅上游通用規範 |
| **連結檢查** | `tools/check_links.py` | 無 |
| **GitHub Pages** | 不部署，`deploy-pages.yml` 已移除 | 有 `deploy-pages.yml`（發布 `web_site/` 到 `lbjlaq.github.io`） |
| **`package.json` devDependencies** | 移除 `vitepress`（見下方「相依套件偏移」） | 保留 `vitepress` |
| **中文語系** | 只有繁體中文（`zh-TW`），`src/locales/zh.json` 已刪除；任何 `zh`/`zh-CN`/`zh-Hans`/`zh-*` 一律解析為 `zh-TW`（前端 `src/i18n.ts`、Rust `src-tauri/src/modules/i18n.rs`），舊設定檔的 `language: "zh"` 會在載入時自動遷移為 `"zh-TW"` | 同時提供簡體 `zh.json` 與繁體 `zh-TW.json` |
| **README** | Fork 自有版本（`README.md` 繁體中文、`README_EN.md` 英文），移除贊助商／打賞／作者其他專案推廣／Trendshift 徽章／貢獻者頭像牆，只保留必要的授權出處說明 | 簡體中文為主的 README，含贊助商、打賞（Buy Me a Coffee、支付寶/微信收款碼）、推薦專案（作者自家其他 repo）、Trendshift 徽章、逐一貢獻者頭像 |

---

## README（本 fork 自有內容，高合併衝突區）

`README.md`（繁體中文）與 `README_EN.md`（英文）是本 fork 自行維護的版本，**不是**單純同步上游 README 再翻譯——內容結構已對齊，但移除了以下與「Windows 原生維護型 fork」定位無關或屬作者個人推廣性質的區塊：

- 💖 贊助商區塊（PackyCode / APIKEY.FUN / Claude API / AICodeMirror 等 affiliate 連結與優惠碼）
- ☕ 支持專案（Buy Me a Coffee 按鈕、支付寶／微信收款 QR code）
- 🚀 推薦專案（連到作者 lbjlaq 自己的另一個 repo，屬作者自我推廣）
- 頂部 Trendshift 徽章（純推廣性質的星數排行榜徽章）
- 👥 核心貢獻者區塊的逐一頭像牆，改為純文字連到上游 repo 的 Contributors 頁面（減少每次同步都要對齊一長串頭像連結的維護負擔）
- 底部「幫我點星星」呼籲橫幅

保留／新增的授權出處說明（CC BY-NC-SA 4.0 要求 attribution，不可移除）：
- README 底部：`本專案 fork 自 lbjlaq/Antigravity-Manager，依 CC BY-NC-SA 4.0 授權。`（含連結）
- `NOTICE.md` 既有的 Original Author / Upstream Repository / License 區塊維持不變

**未來同步上游 README 改動時**：不要直接覆蓋整份檔案，逐段比對上游新增/修改的功能說明段落，手動合併進本 fork 版本，同時保留上述已移除的區塊維持移除狀態。

---

## 相依套件偏移（deviation from upstream package.json / package-lock.json）

本 fork 為了清掉 `npm audit` 的已知漏洞，對上游 `package.json` / `package-lock.json` 做了以下偏移，皆已驗證 `npm run build`（`tsc && vite build`）通過：

- **移除 `vitepress`（`^1.6.4`）**：repo 內搜尋（含 `package.json`、`src/`、`docs/`、`tools/`）找不到任何 script 或程式碼引用它，是完全未使用的 devDependency。它綁死的自帶 `vite`/`esbuild` 版本是 `npm audit` 剩下 3 筆漏洞（1 high + 2 moderate：`GHSA-fx2h-pf6j-xcff`、`GHSA-67mh-4wv8-2f99` 等）的唯一來源，上游沒有非 alpha 的修復版本可升（`vitepress` 目前最新穩定版仍是 `1.6.4`，下一版是 `2.0.0-alpha.*`）。移除後 `npm audit` 歸零，`npm run build` 照常通過。若之後真的需要產生文件網站，重新加回並升級到穩定的修復版即可。
- 其餘 21 筆 `npm audit` 項目透過 `npm audit fix`（無 `--force`）以 semver-safe 的方式解決，沒有額外偏移上游版本號策略。

## GitHub Pages（不部署）

- 這個 fork 從未啟用 GitHub Pages（`gh api repos/SanHsien/Antigravity-Manager --jq '.has_pages'` → `false`），繼承自上游的 `.github/workflows/deploy-pages.yml` 因此每次 push `main` 必然失敗（`Get Pages site failed`）。維護者決定本 fork 不需要 Pages 網站，該 workflow 已直接移除，而非加條件閘門保留。
- Repo 設定裡的 GitHub homepage 欄位原本指向 `lbjlaq.github.io`（上游官網），已由維護者清空；本 repo 檔案內（README、docs、`web_site/`）本來就沒有任何 `github.io` 連結需要改寫。
- `web_site/`（`index.html`、`qa.html`、圖片）是上游原始靜態頁內容，未部署也不影響本 fork 運作，維持不動以縮小與上游的 diff；只有觸發部署的 workflow 被移除。

---

## Remote 與分支規範

每個本機 clone 請確認以下 remote 配置：

- `origin`：`https://github.com/SanHsien/Antigravity-Manager.git`（主要工作與交付目標）
- `upstream`：`https://github.com/lbjlaq/Antigravity-Manager.git`（上游對照目標）

```powershell
gh repo set-default SanHsien/Antigravity-Manager   # 每個 clone 先跑一次
gh repo set-default --view                         # 必須顯示 SanHsien/Antigravity-Manager
```

### 對外邊界與 PR 守則

- **所有 PR、push、release 一律只打 `SanHsien/Antigravity-Manager`，絕不打上游。**
- `gh` 工具在 fork clone 下的預設 repo 會指向母 repo，必須使用 `gh repo set-default SanHsien/Antigravity-Manager` 覆蓋。
- 建立 PR 時請明確指定：
  ```powershell
  gh pr create --repo SanHsien/Antigravity-Manager --base main --head <分支>
  ```
  建完**務必讀取輸出 URL**，確認 owner 為 `SanHsien`。
- **唯一例外**：維護者在**當次對話**明確指示回貢上游通用 bug 修復時，才開立上游 PR。

---

## 本機開發與日常驗收

本專案需要 Node.js 20+（目前環境 Node 26+）與 Python 3.10+。

```powershell
# 執行 Windows 一鍵式驗證門禁
.\tools\dev_check.ps1

# 本地啟動前端開發伺服器
npm run dev

# 檢查上游最新 release / commit / PR / issue
python tools\check_upstream_updates.py --strict
```
