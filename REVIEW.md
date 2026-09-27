# 證據式覆核報告 (Review)

- **覆核時間**：2026-09-26（第二輪，接續同日第一輪的獨立覆核）
- **覆核對象**：`SanHsien/Antigravity-Manager`（fork from `lbjlaq/Antigravity-Manager` @ `v4.8.1` / `27ee35b7`）
- **執行環境**：Windows 11 原生（PowerShell / Node.js v26.7.0 / npm 11.19.0 / Python 3.14 / Git 2.55）
- **本輪異動 commit**：`1334af79`（改寫本檔＋修 Deploy Pages 閘門，已被下一步取代）→ `2a9b9e03`（移除 Deploy Pages workflow、清 npm audit）→ 本次 commit（觸發上游 issue triage、恢復 dev_check 綠燈、重寫本檔）

---

## 1. 覆核結論

**目前狀態：`dev_check.ps1` 綠燈，`npm audit` 0 vulnerabilities，CI 三個 workflow（CI / Upstream check）綠燈，Deploy Pages workflow 已移除不再是紅燈來源。**

上一輪覆核發現的兩個問題本輪都已處理：
1. `Deploy static content to Pages` workflow 因這個 fork 從未啟用 GitHub Pages 而必然失敗——維護者確認本 fork 不需要 Pages 網站，**直接刪除該 workflow**（而非上一輪暫定的條件閘門）。
2. `npm audit` 原有 24 筆漏洞（2 low / 15 moderate / 7 high），透過 `npm audit fix`（無 `--force`）與移除未使用的 `vitepress` devDependency 全部清除，見第 2、3 節。

---

## 2. `npm audit` 修復過程與證據

### 2.1 修復前（本輪覆核首次 `npm ci` 後）

```
npm ci
added 821 packages, and audited 822 packages in 49s
24 vulnerabilities (2 low, 15 moderate, 7 high)
```

### 2.2 `npm audit fix`（無 `--force`，semver-safe）

```
added 3 packages, removed 17 packages, changed 50 packages, and audited 808 packages in 16s
3 vulnerabilities (2 moderate, 1 high)
```

清掉 21 筆。剩餘 3 筆（`esbuild <=0.24.2` moderate、`vite <=6.4.2` high/moderate 混合、`vitepress <=1.6.4` moderate）**全部**來自 `vitepress` 私有依賴樹（`node_modules/vitepress/node_modules/{vite,esbuild}`），`npm audit fix` 回報 `No fix available`。

### 2.3 移除未使用的 `vitepress`

- 全 repo 搜尋（`package.json`、`src/`、`docs/`、`tools/`、npm scripts）找不到任何地方引用 `vitepress`——它是完全沒被呼叫的 devDependency。
- `vitepress` 目前最新穩定版仍是 `1.6.4`（同樣受影響），下一版只有 `2.0.0-alpha.*`，沒有非 alpha 的修復版本可升，所以「升級」這條路不通。
- 決定：**從 `package.json` 移除 `vitepress`**，重新 `npm install` 產生 lockfile。

```
npm install
removed 62 packages, and audited 746 packages in 2s
found 0 vulnerabilities
```

- 移除後跑 `npm run build`（`tsc && vite build`）確認未破壞任何東西：

```
✓ 17041 modules transformed.
...
✓ built in 1m 23s
```

移除決定與理由已記錄於 `FORK.md`「相依套件偏移」一節（依主人指示，所有偏離上游 `package.json`/`package-lock.json` 的地方都要留痕）。

### 2.4 Dependabot alerts（`gh api repos/SanHsien/Antigravity-Manager/dependabot/alerts`）

- Dependabot security alerts 與 automated security fixes 已由主人／主 session 開啟（本次覆核僅用 `gh api` 讀取驗證，未重新切換設定）：
  - `GET .../vulnerability-alerts` → `204 No Content`（已啟用）
  - `GET .../automated-security-fixes` → `200 OK`（已啟用）
- 目前列出的 alerts 全部是 **npm 生態系**（`colord`、`browserslist`、`mermaid`、`dompurify`、`vite`、`js-cookie`、`lodash`/`lodash-es`、`uuid`、`rollup`、`picomatch`、`yaml`、`esbuild` 等，多筆 `state: open`，少數 `auto_dismissed`）。這批是 Dependabot 對現有 `package-lock.json` 的历史掃描結果，與本次 `npm audit` 抓到的 24 筆有重疊但編號體系不同（Dependabot 以 GHSA 為單位、按套件版本區間列出，不會因為某個套件被移除就自動清空舊 alert，需要等下次掃描或手動 dismiss）。**這些 Dependabot alert 條目不在本次「待主人決策」範圍內處理**，因為 `npm audit` 這條路徑（實際 lockfile 狀態）已驗證歸零；是否要手動到 GitHub UI 把 Dependabot 上對應已解決套件的 alert 標記為 resolved，留給主人決定。
- **Cargo 生態系 alerts：目前 0 筆**（`gh api .../dependabot/alerts --jq '[.[] | select(.dependency.package.ecosystem=="cargo")] | length'` → `0`）。可能是還在建立索引（主人在同一時段剛啟用），也可能 Rust 依賴目前沒有已知漏洞。本機沒裝 `cargo`，依指示不動 `Cargo.lock`/`Cargo.toml`。**列為待觀察的 open item**，之後重新查一次確認是否跑出新項目。

---

## 3. GitHub Pages 移除

- `gh api repos/SanHsien/Antigravity-Manager --jq '.has_pages'` → `false`：這個 fork 從未啟用過 GitHub Pages。
- 繼承自上游的 `.github/workflows/deploy-pages.yml` 因此在每次 push `main` 時都失敗：

  ```
  ##[error]Get Pages site failed. Please verify that the repository has Pages
  enabled and configured to build using GitHub Actions ...
  HttpError: Not Found
  ```

- 維護者決定本 fork 不需要 Pages 網站 → **直接刪除該 workflow**（上一輪覆核暫時加了條件閘門，這次改為直接刪除，更乾淨）。
- 已確認 repo 內（README、`docs/`、`web_site/`、`package.json`、`src-tauri` 設定、`index.html`）沒有任何 `github.io` / Pages 相關連結需要改寫——`lbjlaq.github.io` 只出現在 GitHub repo 設定的 homepage 欄位，不在任何檔案內。該欄位已由主人清空（`gh api repos/SanHsien/Antigravity-Manager --jq '.homepage'` 現在回傳空字串），本次覆核未變更任何 repo 設定，只是驗證這件事已完成。
- `web_site/`（`index.html`、`qa.html`、圖片）是上游原始靜態頁原始碼，維持不動——沒有東西建置或部署它，留著只是縮小與上游的 diff，不影響本 fork 運作。

---

## 4. 上游 issue/PR Triage

依 `tools/upstream_baseline.json` 的既有機制（`reviewed_pr_through` / `reviewed_issue_through`），本輪 triage：

| 項目 | 判定 | 理由 |
|---|---|---|
| PR：無新項目（`> #3521`） | — | `gh pr list` 確認上游沒有新 PR |
| Issue **#3523**「你们有遇到死循环的情况吗 感觉昨晚开始gemini开始死循环了？」 | **Skip / not applicable** | 上游維護者自己回報的產品行為問題（Gemini 陷入迴圈、不呼叫編輯工具），無重現步驟、無留言、無 label。本 fork 的 overlay 只碰文件/工具/CI，未修改任何 pipeline 或協定邏輯，這個問題與本 fork 的維護範圍無關；待上游自己 triage 或釋出修復後，下次 `check_upstream_updates.py` 抓到新 release 時一併重新檢視。 |
| Issue **#3524**「反代报错400 User location is not supported for the API use」 | **Skip / not applicable** | 使用者所在地區不被 Google API 支援的求助；根因與 #3525 相同，一併追蹤。 |
| Issue **#3525**「sandbox 返回地区错误但 daily 可用：HTTP 400 阻止上游回退（v4.8.1）」 | **Defer / track upstream** | 真實 bug：sandbox 回地區錯誤時沒有回退到 daily。上游 PR #3526 已提出修正、尚未合併；待上游合併或發行新版時依 release 追蹤一併帶入，不在 fork 先行 cherry-pick。 |
| PR **#3526**「fix(proxy): prefer daily upstream before sandbox」 | **Defer / track upstream** | #3525 的修正，OPEN 未合併。合併後隨上游 release 帶入。 |
| Issue **#3527**「怎么突然开始全部 403了」 | **Skip / track upstream** | 多位使用者同時回報全面 403、重新授權無效，留言指向 Google 端近期異常，不是本 fork 程式錯誤；上游若有修正隨 release 帶入。 |

處理後 `tools/upstream_baseline.json.reviewed_issue_through` 已由 `3519` 提升為 `3527`（#3523–#3527），`reviewed_pr_through` 由 `3521` 提升為 `3526`。

---

## 5. `dev_check.ps1` 最終實跑（本輪，非沿用舊輸出）

```
==> Checking Frontend Build (npm run build)
✓ 15025 modules transformed.
✓ built in 19.40s
==> Compiling Python tools
==> Checking fork document links
共 8 份 overlay 文件，0 份有缺檔。
==> Checking upstream updates
## Upstream pull requests
Triaged through #3521.
No new items above that number.
## Upstream issues
Triaged through #3527 (issues) / #3526 (PRs).
No new items above that number.

WINDOWS DEV CHECK GREEN
```

---

## 6. CI 狀態

| Workflow | 前一輪結論 | 本輪 push 後 |
|---|---|---|
| CI（前端建置 + 3 平台 Rust fmt/clippy/check + Tauri debug build） | ✅ success | 待新 commit push 後以 `gh run list` 覆核 |
| Upstream check | ✅ success | 同上 |
| Deploy static content to Pages | ❌ failure（已知必然失敗） | 已移除，不再產生 workflow run |

（本節於 push 後以實際 `gh run list` 結果為準，見送出後的 commit 訊息與後續覆核。）

---

## 7. 第二輪：依賴安全與地區錯誤回退（2026-09-26）

### 7.1 Dependabot PR 合併

Cargo 生態系 alert 建好索引後出現 15 筆（全在 `src-tauri/Cargo.lock`）。Dependabot 開的 10 個 PR 逐一讀完 diff（只改 lockfile、來源皆 crates.io／npmjs.org、每個 PR 7 項 CI 綠燈）後 squash merge：#10 `rustls-webpki`、#11 `quinn-proto`、#8 `tar`、#9 `serde_with`、#7 `dompurify`、#3 `colord`、#2 `postcss-selector-parser`、#4 `decode-uri-component`/`query-string`、#5 `baseline-browser-mapping`、#1 `browserslist`。

#8、#9 另帶入 `windows-sys`／`windows-core` 的較舊版本（皆在各 crate 宣告的版本範圍內，CI 三平台綠燈），只是多保留幾份舊版 Windows 綁定，無安全影響。

### 7.2 手動升級（本機 rustup minimal，無 MSVC，編譯驗證靠 CI）

| 項目 | 變更 | commit |
|---|---|---|
| Tauri origin confusion（Windows/Android 會把遠端頁面當本地來源） | `tauri` 2.10.2 → 2.11.1（`cargo update -p tauri --precise 2.11.1`，連帶 tauri-build/runtime/wry/tao）；npm `@tauri-apps/api` ^2.11.1、`@tauri-apps/cli` ^2.11.5 對齊版本檢查 | `aef80c8b` |
| `rand` 0.8.5 unsound | `rand` 0.8.5 → 0.8.8 | `aef80c8b` |

### 7.3 標記可接受風險（Dependabot dismissed: tolerable_risk，附理由）

| Alert | 原因 |
|---|---|
| #55 `lru` 0.13 | 被 `rquest` 5.1（`lru ^0.13`）綁住；`rquest` 已停更並改名 `wreq`，根治需更換 HTTP client。 |
| #54 `glib` 0.18 | 來自 Tauri Linux GTK3 堆疊，fork 無法單獨升級；僅 Linux。 |
| #63 `rand` 0.7.3 | `phf_codegen` 的 build-dependency（建置期程式碼產生），執行期 `rand` 已升到 0.8.8。 |

### 7.4 地區錯誤回退（上游 #3525；不採用上游 PR #3526）

上游 PR #3526 把端點順序改成 daily 優先，會改變所有使用者的行為。本 fork 改為保留原順序，只有收到 HTTP 400 且內容含 `User location is not supported` 時才切到下一個端點；其他 400 讀出後原樣重建回傳（`rebuild_response`，移除已解碼的 `content-encoding`／`content-length`）。

- `src-tauri/src/proxy/upstream/client.rs`：`is_location_unsupported()`、`rebuild_response()`、回退分支與兩個單元測試；`Cargo.toml` 加 `http = "1"`（依賴圖中已有 1.4.0）。
- commit `81909ac4`（首次 CI 因 rustfmt 格式失敗）→ `aef80c8b` 套用 rustfmt，本機 `cargo fmt -- --check` 通過。

### 7.5 CI（`aef80c8b`，run `36249443064`）

Build Frontend、Check Rust Code（ubuntu／macos／windows-2025：fmt＋clippy＋check＋測試）、Build Tauri App（三平台）全部 success。

### 7.6 上游分流補記

#3524–#3527 記錄於第 4 節表格；`tools/upstream_baseline.json`：issues through `3527`、PRs through `3526`。本機 `tools\dev_check.ps1` → `WINDOWS DEV CHECK GREEN`。

---

## 8. 待觀察（Open Items）

1. **HTTP client 遷移**：`rquest` 已停更（改名 `wreq`），`lru` 警示要根治需換 client，屬上游層級改寫，待上游處理或另案評估。
2. **地區錯誤回退**：無法在本機以真實帳號重現，只有單元測試＋CI 驗證；上游若合併 #3526 或另有修法，同步時需比對後擇一，避免兩套邏輯疊加。
3. **上游全面 403（#3527）**：Google 端異常，持續觀察上游 release。
4. 是否要重新引入文件產生工具（`vitepress` 或替代品）；目前完全未使用，已移除。

---

Maintained per `AGENTS.md` 協作約定：修 bug 需回註本檔並附 commit hash；對應 commit：`1334af79`、`2a9b9e03`、`bca736e9`、`49b6f567`、`a7a7ab0e`、`5fba79d0`、`81909ac4`、`aef80c8b`。
