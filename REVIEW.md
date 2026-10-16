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
5. **上游 v4.8.4 整批合併**：見第 9 節，adoption pending（本機無 MSVC/cargo，Rust 變更無法在 `dev_check` 驗證；與 fork 地區回退分歧）。

---

## 9. 上游 v4.8.4 審查（2026-09-30）

範圍：`27ee35b`（v4.8.1）→ `0269f04`（tag `v4.8.4`）共 40 commit；PR `#3541`–`#3557`（9 筆）；issue `#3528`–`#3566`（30 筆）。release 軌，不審查 tag 之後的 `d7438d61`。

**總決策：adoption pending，本輪不合併任何上游程式碼。** 理由：(1) v4.8.4 是同一批 proxy pipeline 重構（簽章 sentinel、端點順序、工具封包），commit 互相依賴，無法乾淨挑出單一小修復；(2) 上游 `bbbf734`／`635767f` 改為 daily 優先，與 fork 在 `client.rs` 的地區錯誤回退（第 7.4 節）分歧，需擇一，不可疊加；(3) 本機無 cargo/MSVC，`dev_check` 只覆蓋前端建置，Rust 變更只能靠 CI 事後驗證。維護者決定要跟 v4.8.4（或更後的 release）時，建議整批合併並先在分支上跑 CI。

### 9.1 Commit（40 筆，依類別）

| 類別 | Commit | 決策 |
|---|---|---|
| Release／版號／changelog／merge（12） | `9c17149` `ca7598e` `ad07a38` `06f458d` `0269f04` `7c0b574` `08187c3` `cd30a5b` `ee7b9a6` `9243338` `6bf17c1` `a9c1ffd` | not-applicable：版務與 merge 標記；`6bf17c1`／`a9c1ffd` 是 #3520／#3521 的空 merge，實質內容見下方 `399e334`／`0708ec2` |
| Gemini 簽章／thinking／工具封包 pipeline（12） | `775158d` `7d6f8c7` `fa5fbf3` `1ba7f17` `dbbfb07` `5321c49` `639209b` `441510d` `2bb52ee` `c8529c4` `68e7310` `219b850` | adoption pending：相依的整批重構（見總決策）；對應使用者回報的 `thought_signature` 400（#3529／#3531／#3535） |
| 端點順序／per-account client（2） | `bbbf734` `635767f` | adoption pending：與 fork 地區回退分歧，需擇一；`635767f` 另動 `AGENTS.md`、PR 模板、`RELEASE_GUIDE.md`，合併時須保留 fork 版本 |
| 功能（4） | `399e334`（週配額重置統計）`0708ec2`（帳號優先度設定）`7ed6b83` `34dcacf`（更新檢查雙軌／自訂端點） | follow-upstream：非缺陷修復，涉及 UI、設定與 Rust；更新檢查指向上游 release，fork 不採用 |
| 非 Windows 編譯修復（3） | `ee83986` `999af6c` `1899489` | not-applicable：修的是上游同批 commit 引入的編譯錯誤，隨整批合併才有意義 |
| 文件／連結（7） | `d6f9e5e` `3b4ff8e` `9fcbc5b` `572a9e8` `6a0b72e` `e505152` `4561638` | not-applicable：README 為 fork 自有版本；Docker beta 拉取說明與 JeikCode 導覽屬上游作者專案推廣 |

### 9.2 Pull requests

| PR | 決策 | 理由 |
|---|---|---|
| #3541 #3542 #3544 | follow-upstream | 上游已合併，屬 v4.8.4 之後的 proxy 修復（HAR 工具鏈、`data:image` 誤判、`toolConfig`）；隨下個 release 整批帶入 |
| #3543 #3554 | follow-upstream | 上游已關閉未合併；待上游另案修復 |
| #3549 #3553 #3557 | not-applicable | `--minimized` 啟動、移除 WebView2、prompts 同步（2582 檔）皆已關閉未合併 |
| #3552 | not-applicable | 僅上游 release workflow 的 Rust cache |

### 9.3 Issues

| Issue | 決策 | 理由 |
|---|---|---|
| #3529 #3531 #3535 | follow-upstream | `thought_signature` 缺失 400；對應 C 類簽章重構，隨整批合併 |
| #3537 #3539 #3540 #3558 #3560 #3561 #3562 | follow-upstream | 上游 pipeline 缺陷（orphaned tool_result、`googleSearch` 注入、`data:image` 誤判、結尾為 model turn、多圖 tool_result 順序、`toolConfig` 被剝除）；未在 fork 先行修補 |
| #3545 #3547 #3556 #3564 | follow-upstream | 功能請求／顯示 bug（圖片上限、`/v1/model/{id}`、儀表板 5h／週配額顯示）；上游未列入 v4.8.4 |
| #3551 #3536 | not-applicable | 針對 v4.8.5／v4.8.6 beta 的回歸；本 fork 追 release 軌（#3536 已關閉） |
| #3538 | not-applicable | 針對 `hermes_sync.rs` 的 yaml-rt workaround 移除建議；本 fork 未修改該檔，隨上游處理 |
| #3563 #3566 #3546 #3550 | not-applicable | RFC／指南／公告，非缺陷 |
| #3528 #3530 #3532 #3533 #3534 #3548 #3555 #3559 #3565 | not-applicable | 帳號登入／授權／403／預熱／環境求助；#3559 回報刷新即恢復，屬 Google 端異常（延續 #3527） |

共 30 筆 issue、9 筆 PR、40 筆 commit 全數 triage；水位：`reviewed_release=v4.8.4`、`reviewed_through=0269f045f4e35f34b9ee3b3bcd6c0ff1e741378f`、`reviewed_pr_through=3557`、`reviewed_issue_through=3566`、`reviewed_date=2026-09-30`。**Baseline 代表已審查，不代表已合併。**

---

Maintained per `AGENTS.md` 協作約定：修 bug 需回註本檔並附 commit hash；對應 commit：`1334af79`、`2a9b9e03`、`bca736e9`、`49b6f567`、`a7a7ab0e`、`5fba79d0`、`81909ac4`、`aef80c8b`。

---

## 10. 上游 v4.9.6 審查（2026-10-07）

範圍：前次水位 `0269f04`（v4.8.4）至 `b601f5e7`（v4.9.6）共 **174 commit**；另審 PR `#3567`–`#3622`（34 筆）及 issue `#3569`–`#3621`（22 筆）。依 `tools/check_upstream_updates.py` 的 release 軌，v4.9.6 tag 是本次邊界；tag 之後的修正只列為後續追蹤。上游 v4.9.6 release 說明確認它是 v4.9.5 內容的重新發布，並非獨立的程式功能批次。

**總決策：已審查，整批程式碼採用仍 pending。** 本 fork 仍在 v4.8.1 衍生樹；上游這段改動橫跨 proxy pipeline、模型路由、Cowork、帳號處理、桌面端與 UI，共 140 個檔案、約 31,206 行新增／9,802 行刪除。既有第 9 節的地區錯誤回退與上游端點優先序分歧仍未解決。這輪只完成審查與水位，不假稱已合併或已驗證上游整樹。日後若採用，應另開隔離分支處理衝突、CI 與回歸，不能零散 cherry-pick 相依提交。下列每組列出該類**全部**短 SHA；分組計數 `70+18+67+9+8+2=174`。

| 類別 | Commit（全部） | 決策與依據 |
| --- | --- | --- |
| 發版、merge、CI、格式與上游依賴（70） | `87de06cf` `ff8c3870` `100322e3` `10c44ef7` `ba2a3668` `f51a92ca` `e24ed63f` `7850e046` `5e719a9c` `4c709c97` `e4f455c4` `c6d6687e` `c3951922` `b661d301` `f89d262c` `a2464cf3` `c373e161` `3d015d84` `62a23b53` `f67422da` `e99a6586` `4fbdab0b` `69f7a0fb` `02ad2b49` `a3ddf8d5` `9e41d7c8` `de0f0e22` `a978bcc1` `a6637bcd` `bd48eb00` `a2dfd515` `45fc6d37` `72699a7f` `c829cf38` `6eca4510` `3ec6824d` `99c9528a` `6e8b982a` `2201173a` `f666237c` `6a360c3a` `42d9a732` `1d81d64f` `4e10eee3` `be403fe6` `3732e117` `5f952e0b` `04d2fd2c` `b5de8c8f` `e63e8bc0` `39608f2d` `8ab0d37e` `1873899c` `d74ba8f0` `325e787a` `083039d2` `f094b29c` `84b0b71f` `6b3b00a9` `6702090d` `e927fb8d` `f04bce68` `ab176a84` `ff56fed1` `e82405a4` `f2a97ad3` `b5eed068` `b4ff91f1` `56513c7f` `b601f5e7` | not-applicable：上游版號、merge 標記、release workflow 與格式整理不能單獨移植。`e24ed63f` 的 yaml-rt 依賴升級應在採用上游整樹時重新驗證，而非以格式／版務提交直接套入 fork。 |
| 上游文件與品牌（18） | `76124c29` `4568c352` `23d8c11e` `16d3cdbf` `ffdbe996` `d7438d61` `1fbe00f4` `ffc557f0` `b4dafd8c` `a82ee988` `8813f0ad` `1011b684` `67c5fdf9` `9ea713db` `79dc54c3` `277c0062` `6b589798` `80bf086d` | not-applicable：README、changelog、抓包與 Cowork 規格屬上游版本或作者資料；fork 文件與治理指引獨立維護，採用功能時再對照技術文件。 |
| Proxy、模型、Cowork、設定與 gateway（67） | `fe8aee68` `d04fc5fb` `eb5c1e1d` `af0e3224` `0c36bcac` `cc6fb217` `c9be9d05` `41855f7c` `5f0bae66` `3f00eec7` `6b56f5f5` `49f855a7` `3059a2c9` `b57872fc` `ea515250` `45bb067d` `34c2cfc6` `0c88f1b2` `3f8363a4` `2c7cf017` `c5ad0622` `026d4097` `a690dfaa` `3e92accd` `e642d257` `9a547bd4` `36f5706e` `d2150fa3` `7103e28d` `d00085dd` `f3ab5873` `6ea73c46` `9d1690cf` `09bab118` `30654785` `5b337a32` `3be28fe6` `4a1c88b7` `669d6c45` `d0fccd32` `5d5d3537` `0e5a0ca7` `96eb11f1` `5cfc607d` `913c7dda` `84eb5413` `9ca5ad5e` `8fdf6d61` `4e8487df` `331d5982` `fd9c30c6` `322587be` `af5b791b` `11cf588a` `f3d74cf2` `9912ccd9` `2303ed33` `a80cd495` `58a4e1a8` `8f5ff297` `dbd0786c` `aadfa547` `ecdb1862` `0dc45751` `a4903ef5` `215443a5` `04198598` | follow-upstream／adoption pending：同一條 pipeline 的簽章、模型、HTTP/2、429、SSE、Cowork 與路由修正相依。`9912ccd9` 明確修回 `#3571` 引入的危險 HTTP/2 設定，不能只挑前者；`e642d257` 刪除 z.ai/MCP（22 檔、約 2,506 行刪除），會改變 fork 現有功能，採用前須另做產品決策。`913c7dda`／`a80cd495` 的路徑與 process 隔離值得後續優先審查，但非本輪 CodeQL alert 的直接修補。 |
| 桌面端與整合（9） | `675703a6` `38dc82e6` `6e421677` `cb6f1135` `d1ec3fd4` `99d32a68` `68ce996f` `a163b61d` `64bb357f` | follow-upstream／adoption pending：PATH／keyring／資料庫隔離、啟動模式、Linux 圖形與 macOS 重簽需跨平台驗證；本機無 MSVC，不能只憑靜態審查宣稱已驗收。 |
| UI、monitor、i18n（8） | `a60e7d12` `75112aeb` `5994437b` `5d2ac56b` `7c07bb3f` `dad8f006` `4d2b5717` `58a22ea0` | follow-upstream／adoption pending：配額、mini view 與 monitor 改動須跟新資料模型及 UI 一起驗證。 |
| 其他（2） | `4736f80a` `31d219f7` | `4736f80a` not-applicable：只修上游新批次引入的型別／格式門禁；`31d219f7` defer：新增大量錯誤與入參日誌，採用前須先審核憑證與個資遮罩。 |

### 10.1 上游 PR（34 筆）

| 決策 | PR | 理由 |
| --- | --- | --- |
| follow-upstream／adoption pending | `#3567` `#3568` `#3571` `#3574` `#3578` `#3580` `#3584` `#3589` `#3590` `#3591` `#3592` `#3595` `#3599` `#3600` `#3601` `#3602` `#3603` `#3604` `#3606` `#3607` `#3608` `#3609` `#3611` | 已合併到 v4.9.6 邊界的程式碼；依上表按整批處理。`#3571` 的 HTTP/2 PING 之後被 `9912ccd9` 修正。 |
| follow-upstream（tag 之後） | `#3613` `#3615` `#3618` `#3620` | 上游已合併，但不在本次 v4.9.6 tag；等待下一個 release 審查。 |
| follow-upstream（未合併） | `#3616` | 仍開啟的 token stats 功能 PR，不能當作已發布修補。 |
| follow-upstream（tag 之後，未合併） | `#3622` | 2026-10-07 新開、目標 beta 的 Claude SSE 超時／重試修復，對應未解 `#3621`；尚未進入 v4.9.6 release，不能當作 fork 已修。 |
| not-applicable | `#3579` `#3582` | 上游 README 與 Cowork RFC 文件；fork 文件獨立。 |
| not-adopt | `#3583` `#3588` `#3594` | 上游已關閉而未合併；`#3588` 功能由 `#3592` 接替，`#3594` Claude 規格由後續提交處理。 |

### 10.2 上游 issues（22 筆）

| 決策 | Issue | 理由 |
| --- | --- | --- |
| follow-upstream／adoption pending | `#3570` `#3573` `#3577` `#3585` `#3586` `#3587` `#3593` `#3598` `#3605` `#3610` | 上游已關閉的功能缺陷；相關修正分散於上述版本，fork 尚未採用，不能宣稱已修。 |
| follow-upstream（後續 release） | `#3612` `#3614` `#3617` `#3619` `#3621` | v4.9.6 後續 SSE、Cowork、語系與競態回報；`#3621` 仍開啟，下一次 release 繼續追。 |
| needs-info／follow-upstream | `#3572` `#3581` | 登入授權與 Fedora UI 問題沒有足夠 fork 重現資料；先保留觀察，不移植特定修補。 |
| not-applicable | `#3569` `#3575` `#3576` `#3596` `#3597` | 撤回問候、Cowork RFC、社群與第三方訂閱問題，無 fork 程式修補。 |

審查水位：`reviewed_release=v4.9.6`、`reviewed_through=b601f5e7375343343922ccab5e50653bc3450c3b`、PR through `#3622`、issue through `#3621`，日期 `2026-10-07`。**水位只代表逐項審查與記錄，不代表已合併或已驗證上游程式碼。**

### 10.3 新增 issue 審查（2026-10-08）

| Issue | 決策 | 依據與後續 |
| --- | --- | --- |
| [`#3623`](https://github.com/lbjlaq/Antigravity-Manager/issues/3623) | follow-upstream／needs fork reproduction | 回報 v4.9.6 將不存在的 `claude-sonnet-4-6-thinking` 列出並回傳誤導性 429。fork 的 `src-tauri/src/proxy/common/model_mapping.rs` 也有同名映射，因此存在相關風險；上游帳號配額與三次請求紀錄不是 fork 的 runtime 證據。後續須對照帳號可用模型、列表產生及錯誤分類，在隔離測試中驗證後修正；本輪不擅改模型路由。 |
| [`#3624`](https://github.com/lbjlaq/Antigravity-Manager/issues/3624) | follow-upstream／needs timing evidence | 回報 thinking 約 3.5 秒批次送出，原因仍是推測。fork 的 `src-tauri/src/proxy/mappers/claude/streaming.rs` 已有 `thinking_delta`／`signature_delta` 處理，但僅此不能證明或排除延遲。後續須比對上游到達與下游送出時間，區分模型端與 mapper 緩衝；不憑回報直接改簽章或 flush 行為。 |

本次只審查上述兩筆新增 issue；issue 水位提升至 `#3624`，release、commit 與 PR 水位維持不變。決策為追蹤與待重現，未宣稱問題已修復。

### 10.4 Beta 串流回歸與帳號冷卻報告（2026-10-08）

已閱讀下列 PR 的說明、檔案範圍及串流錯誤／超時相關 diff，並對照 fork 的 Claude／OpenAI handler 與 mapper。這輪只記錄決策，不採用上游程式碼。

| 項目 | 決策 | 依據與後續 |
| --- | --- | --- |
| [PR #3625](https://github.com/lbjlaq/Antigravity-Manager/pull/3625) | follow-upstream／未採用 | 針對 beta #3622 的假截斷收尾，改為傳出錯誤並把滑動超時 45 秒改為 120 秒；涉及 lifecycle、handler、mapper、設定與語系。fork 尚未採用該 lifecycle，不能把此修補直接套入，也不能把作者的測試當成 fork 驗收。後續 release 審查需保留「錯誤後不追加成功尾幀」的回歸案例。 |
| [PR #3628](https://github.com/lbjlaq/Antigravity-Manager/pull/3628) | follow-upstream／未採用 | 同一 beta 回歸的另一方案：移除 lifecycle 與全域 120 秒熔斷，回復串流邏輯並使用 `overloaded_error`。與 #3625 方案相互影響，不能兩份任意 cherry-pick。fork 的 Claude mapper 已有 `overloaded_error`，但未做 runtime 串流驗收；等待上游收斂後獨立檢查錯誤後尾幀、長等待及 OpenAI 路徑。 |
| [Issue #3626](https://github.com/lbjlaq/Antigravity-Manager/issues/3626) | follow-upstream／beta 回歸待追蹤 | 回報 #3622 引入的假 `end_turn`／`message_stop`、45 秒滑動超時及 120 秒全域截止時間。fork 未採用該 beta lifecycle；回報不能證明 fork 同樣受影響。保留為後續採用前的測試要求，不宣稱本輪已修或已重現。 |
| [Issue #3627](https://github.com/lbjlaq/Antigravity-Manager/issues/3627) | needs-info／待 fork 重現 | 回報額度充足但帳號池全數冷卻，等待 182805 秒。缺少帳號池與限流時間證據，不能據此縮短冷卻或繞過限流。後續用遮罩後的帳號狀態、冷卻截止時間與實際 provider 回應，區分排程、配額與帳號停用原因。 |

PR 水位提升至 `#3628`，issue 水位提升至 `#3627`；release／commit 水位維持 v4.9.6／`b601f5e7`。上述紀錄表示已審查，並非已採用或通過 fork 的 runtime 驗收。

### 10.5 合成人類訊息回報（2026-10-09）

已閱讀 [Issue #3629](https://github.com/lbjlaq/Antigravity-Manager/issues/3629) 與 maintainer 回覆。決策：follow-upstream／上游已有修正，fork 未採用。作者回報 Gemini defense 的 `ok go on` 可能干擾 Agent 等待人類確認的狀態；maintainer 已確認根因，並指出修正 [d8b6e3f9](https://github.com/lbjlaq/Antigravity-Manager/commit/d8b6e3f90ee8cbd4700c75c876c6998b9f8537f8) 移除末尾 model／assistant turn 的合成 user turn，但仍保留空 user turn 的填補處理。已確認 `v4.9.7-beta.5` tag 存在；本次審查時 GitHub Release API 尚未列出該 release，不把 tag 存在當成正式 release 或 fork runtime 驗收。

fork 的 `src-tauri/src/proxy` 精準搜尋未找到 `ok go on` 或該兩條完整 Defense 訊息；但 `mappers/common_utils.rs` 的 `ensure_gemini_payload_ends_with_user` 確有補齊 user turn，使用 `TRANSIT_DEFENSE_FALLBACK_TEXT = "Please continue your analysis."`。文字不同不代表安全問題不存在，本輪也未修復或驗證它是否被下游視為人類授權。後續應比對來源版本與 request mapper，使用隔離 payload fixture 確認補齊訊息是否被當作人類授權；不得把合成訊息當成操作者同意，也不直接採用未驗證的開關／替代文字方案。issue 水位更新至 `#3629`，其他水位不變。

### 10.6 驗證封禁與管理 API 欄位（2026-10-09）

已閱讀 [PR #3632](https://github.com/lbjlaq/Antigravity-Manager/pull/3632) 的完整三檔 diff，以及 [Issue #3630](https://github.com/lbjlaq/Antigravity-Manager/issues/3630)／[Issue #3631](https://github.com/lbjlaq/Antigravity-Manager/issues/3631) 的回報。決策：follow-upstream／待隔離驗證，未採用。

- fork 的 `refresh_all_quotas_logic` 同樣會跳過 `quota.is_forbidden`，與 #3630 所述循環相關；但沒有本 fork 的實際帳號重現。PR 會在驗證封禁過期後清掉 `is_forbidden` 並允許重新探測，可能增加上游請求，不能僅凭過期就宣稱帳號已解封。後續測試須區分永久禁止、尚在驗證期及已過期可重試三種狀態。
- fork 的 `AccountResponse` 確實缺少 `validation_url`，而前端 `AccountErrorDialog` 有讀取它；#3631 的欄位缺口具有靜態依據。PR 補齊三個 response 建構點並清除過期 URL；採用前須驗證管理 API 序列化、URL 安全檢查與清理一致性，不使用真實帳號作 smoke。

本輪只記錄審查，沒有移植該 PR 或宣稱 runtime 已修復。PR 水位至 `#3632`，issue 水位至 `#3631`；release／commit 水位不變。

### 10.7 Beta 後續項目逐項審查（2026-10-11）

已逐一閱讀下列 issue／PR 的說明與實際 diff，並以 fork 現有對應模組作靜態比對。本輪只更新 `follow-upstream` 紀錄，**沒有移植任何上游程式碼**；release／commit 水位仍維持 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 diff 與 fork 對照、後續理由 |
| --- | --- | --- |
| [Issue #3633](https://github.com/lbjlaq/Antigravity-Manager/issues/3633) | defer／follow-upstream | 回報 Claude Code 串流終止、重試與狀態問題，後續由 #3634 的 lifecycle 改寫處理。fork 未採用該 lifecycle，且沒有 fork runtime 串流重現；保留「錯誤後不得追加成功尾幀」作為日後採用前回歸條件。 |
| [PR #3634](https://github.com/lbjlaq/Antigravity-Manager/pull/3634) | defer／follow-upstream | 已關閉且曾合併 beta；實際變更多達 10 檔（Claude/OpenAI handler、mapper、common 與文件），加入 heartbeat、terminal state machine、429/403 分流與 timeout/budget 行為。fork 的 Claude mapper 雖有 `message_stop_sent`，但沒有相同 `stream_lifecycle`／heartbeat 架構；此為相依的協定生命週期改寫，不能片段移植。 |
| [Issue #3635](https://github.com/lbjlaq/Antigravity-Manager/issues/3635) | defer／follow-upstream | 針對 overloaded/error 傳播，與 #3634 同一串流狀態問題相關。fork 已有部分 `overloaded_error` mapper 邏輯，但沒有問題 payload 的 runtime 證據；且需與先前 #3625／#3628 的互斥方案一併判定，不能單獨改尾幀行為。 |
| [PR #3636](https://github.com/lbjlaq/Antigravity-Manager/pull/3636) | not-applicable | 已關閉且合併的 10 檔 React/UI 動畫（entry／hover，含 motion 相關改動），不是安全或功能缺陷修補。fork 已有自己的 UI 與本輪已驗收的 layout；導入會造成無授權的視覺設計變更。 |
| [Issue #3637](https://github.com/lbjlaq/Antigravity-Manager/issues/3637) | needs-info／follow-upstream | Standalone language server 熱切換回報只提供特定 Windows vendor app 經驗，沒有可驗證的目標 executable／版本／父子程序或未儲存緩衝資料。fork 有 integration/process 路徑，但不可據此直接調整程序生命週期；待可重現資料或上游修補。 |
| [PR #3638](https://github.com/lbjlaq/Antigravity-Manager/pull/3638) | not-applicable | 已關閉未合併的 token stats 擴充，實際跨 63 檔、約 +5,407/-1,573，涵蓋資料模型、設定、後端、UI、文件與 heatmap。是未發行大型功能而非可獨立修補，且會和 fork 自有文件／在地化相衝突。 |
| [Issue #3639](https://github.com/lbjlaq/Antigravity-Manager/issues/3639) | defer／follow-upstream | 請求在帳號配額歸零時自動切換；#3642 提出 beta 實作。沒有 fork 帳號排程、速率帳務與使用者選擇的驗收證據，不應藉由猜測直接改切換行為。 |
| [PR #3640](https://github.com/lbjlaq/Antigravity-Manager/pull/3640) | defer／follow-upstream | OPEN beta，`token_manager.rs` 約 +175/-34，將 quota reset 查詢改為 per-account/per-model。fork 有相同 `get_quota_reset_time` 與呼叫點，故具相關性；但 API 語意與模型選擇需 unit/integration 測試，不能只移植函式簽章。 |
| [PR #3641](https://github.com/lbjlaq/Antigravity-Manager/pull/3641) | defer／follow-upstream | OPEN beta，3 個 Rust 檔約 +84，擴展 Imagen grace retry 與 cooldown rescue。fork 已有 `GraceRetry` 與帳號協調邏輯；直接疊加可能重複 retry／冷卻排程，需 account-pool/queue 時序測試後才可評估。 |
| [PR #3642](https://github.com/lbjlaq/Antigravity-Manager/pull/3642) | defer／follow-upstream | OPEN beta，`token_manager.rs` 約 +200/-8，處理固定帳號配額歸零後的自動切換。fork 已有 model quota 與 target filtering；仍需釐清固定帳號語意、限流與使用者意圖，沒有真實帳號或隔離 scheduler 測試時不採用。 |
| [PR #3643](https://github.com/lbjlaq/Antigravity-Manager/pull/3643) | not-applicable | OPEN beta，新增 Veo video 端點、模型 mapping 與 OpenAI handler 路由（5 檔約 +327）。不是本輪安全鏈或既有 bug 修補，也沒有此 API capability 的產品需求；不擴大 fork 功能面。 |
| [PR #3644](https://github.com/lbjlaq/Antigravity-Manager/pull/3644) | defer／follow-upstream | OPEN beta，2 檔約 +63，將 Cowork deep compact 的 immunity／scan 閾值調整為 60／5。fork 沒有對應 compact detector 常數與 RFC state machine，孤立移植只會引入不可驗證的行為差異。 |

水位更新為 PR 至 `#3644`、issue 至 `#3639`，只表示每項已讀說明、相關 diff 與 fork 對照後作出採用判定；**不表示這些上游程式碼已合併、已修復或通過 fork runtime 驗收。**

### 11. Dependency freshness major deferrals 與 Action patch（2026-10-11）

本輪針對 freshness report 的十個過期 major deferral，讀取各發布者的 registry metadata／release notes及 fork 實際引用；只更新有到期點的治理紀錄，沒有變更產品依賴或 Rust runtime。全部十項的 major migration 風險仍成立：

| 項目 | 最新審查版本與來源 | fork 對照與判定 | 下次重查條件 |
| --- | --- | --- | --- |
| `@lobehub/ui` | [`5.57.3`](https://www.npmjs.com/package/@lobehub/ui/v/5.57.3) | v5 要求 Node `>=22.22` 與 `antd ^6.1.1`，但 fork CI 是 Node 20、直接使用 antd 5；defer。 | 核准 Node/antd 協同遷移後，build 與 light/dark UI 驗收。 |
| `@vitejs/plugin-react` | [`6.1.2`](https://github.com/vitejs/vite-plugin-react/releases/tag/plugin-react%406.1.2) | v6 peer 為 Vite `^8`；defer，不能與 Vite 8/Rolldown toolchain 分拆。 | 與 Vite 8 一併遷移並 clean-install build。 |
| `framer-motion` | [Motion upgrade guide](https://motion.dev/docs/react-upgrade-guide)／`14.1.0` | 11→14 跨越 v12 render scheduling 與 v13 Emotion prop-filtering；fork 有 animated modal/mini/debug UI 及 Emotion 相依；defer。 | 動畫互動與 accessibility focused regression。 |
| `i18next` | [`26.4.3`](https://github.com/i18next/i18next/releases/tag/v26.4.3) | `react-i18next` 17 同時將 peer floor 提至 `>=26.2`；fork 的初始化與全部 locale type surface 必須配對驗證；defer。 | 與 `react-i18next` 升級、typecheck、切換語言驗收。 |
| `lucide-react` | [Lucide v1 guide](https://lucide.dev/guide/version-1)／`1.55.0` | v1 移除品牌圖示並將裝飾圖示預設 `aria-hidden`；fork 59 個檔案引用 icons，屬可見及可及性行為面；defer。 | 全部 import audit，加上鍵盤／螢幕閱讀器驗收。 |
| `react-i18next` | [`17.0.16`](https://www.npmjs.com/package/react-i18next/v/17.0.16) | v17 peer 需要 i18next `>=26.2`；不能單獨升級；defer。 | 與 i18next 26 同批驗證。 |
| `vite` | [`8.3.4`](https://github.com/vitejs/vite/releases/tag/v8.3.4) | v8 引入 Rolldown toolchain dependency，且 React plugin v6 peer 為 Vite 8；defer。 | Vite/plugin-react 協同遷移、clean-install frontend build。 |
| `toml` | [`1.1.8+spec-1.1.0`](https://crates.io/crates/toml/1.1.8%2Bspec-1.1.0) | TOML 1.1 新增 inline-table、escape、datetime parse 語意；defer，避免未驗證地改寫設定解析。 | 與 `toml_edit` 升級、managed-config fixtures、完整 Tauri build。 |
| `toml_edit` | [`0.25.17+spec-1.1.0`](https://crates.io/crates/toml_edit/0.25.17%2Bspec-1.1.0) | `cli_sync.rs` 直接用 `DocumentMut`/`Item` 改寫 CLI TOML；格式保留與 parser 行為需要驗證；defer。 | 與 `toml` 升級、preservation fixtures、完整 Tauri build。 |
| `windows` | [`0.62.2`](https://crates.io/crates/windows/0.62.2) | 0.x minor 為 major-series；fork 直接使用 Windows shortcut/COM bindings；defer。 | Windows shortcut test 與 Windows Tauri debug/signed build。 |

Actions 的官方 release notes已覆核並直接採用 patch：download/upload artifact `v8.0.2`／`v7.0.2` 改善 429 Retry-After；setup-node `v7.1.0` 增加版本驗證與 manifest retry；CodeQL `v4.38.3` 更新 bundle 並修正 custom-query bundle 選擇。本輪更新的 Action `uses:` 都維持不可變 commit SHA 與版本註解，沒有轉為 floating tag；release workflow 沒有被執行。

### 12. 新增 upstream beta／issue 逐項審查（2026-10-11）

已讀取下列五個 OPEN beta PR 的說明與實際 diff，以及 [Issue #3648](https://github.com/lbjlaq/Antigravity-Manager/issues/3648)；並對照 fork 的相應模組。這一輪只更新治理水位，**沒有移植上游程式碼或變更產品 runtime**；release／commit 基線維持 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 diff、fork 對照與後續 |
| --- | --- | --- |
| [PR #3645](https://github.com/lbjlaq/Antigravity-Manager/pull/3645) | defer／follow-upstream | 對 `token_manager.rs` 加入 weekly reset 欄位、24 小時 EDF 優先權與約 344 行排程變更。fork 有 model quota／reset 排程，但沒有此週期資料契約或對應測試；這會改變帳號選擇與限流行為，須以遮罩 quota fixture 與 scheduler 時序測試整批驗證，不能片段採用。 |
| [PR #3646](https://github.com/lbjlaq/Antigravity-Manager/pull/3646) | defer／follow-upstream | 對 `model_mapping.rs` 正規化 Claude Pro tier alias 至 Gemini 上游 ID。fork 有同一 mapping surface，但 custom mapping、實體 upstream ID 與 provider alias 的優先順序相依；上游 PR 仍 OPEN 且未有 fork 的問題請求重現，待以 alias matrix／實際 provider error fixture 驗證後再決定。 |
| [PR #3647](https://github.com/lbjlaq/Antigravity-Manager/pull/3647) | not-applicable | 將 Cowork compact 的免疫 lease／掃描深度改為 90 秒／五則，連同 README。fork 沒有相同 Cowork compact detector state machine；單獨加入常數不會修到既有路徑，且屬 beta 行為調校，不擴大功能面。 |
| [PR #3649](https://github.com/lbjlaq/Antigravity-Manager/pull/3649) | 需獨立產品安全修補；本批未採用 | 上游只把 Gemini 空輪次 fallback 改為惰性 `.`。fork `src-tauri/src/proxy/mappers/common_utils.rs` 的 `TRANSIT_DEFENSE_FALLBACK_TEXT = "Please continue your analysis."` 仍被 `ensure_gemini_payload_ends_with_user` 注入空／末尾 model／空 user turn，並由 Claude、Gemini、OpenAI handlers 呼叫；這是已確認的合成 user 指令面，不能視為操作者授權。後續應在獨立產品變更以隔離 payload fixture 驗證惰性填補、協定合法性與 Agent confirmation 狀態，再採用最小修補。 |
| [Issue #3648](https://github.com/lbjlaq/Antigravity-Manager/issues/3648) | needs fork reproduction／follow-upstream | 回報上游忽略 `thinkingBudget=4000` 而產生約 82k thought tokens。fork `mappers/gemini/wrapper.rs` 確會讀／注入 `thinkingBudget`，但目前只使 request 的 `maxOutputTokens > thinkingBudget` 並依模型 static cap 限制 request；沒有 provider response thought-token 計數或 stream cutoff。這證實同類成本風險可達，但沒有 fork 帳號 telemetry／response payload，不能宣稱已重現或安全地中止 stream；後續需以可重現的 provider 回應 fixture 定義計數、終止與錯誤語意。 |
| [PR #3650](https://github.com/lbjlaq/Antigravity-Manager/pull/3650) | defer／follow-upstream | 調整 Claude Code session 的 usage scaling 與 95% compact probe，改動 handler compact／quota 路徑。fork 有自己的 thinking/compact 與 quota 行為，沒有對應帳號 session 的 usage data；直接套用可能改變自動壓縮與用量判斷，待上游收斂及隔離 compact regression 後評估。 |

PR 水位提升至 `#3650`、issue 水位提升至 `#3648`；release／commit 水位不變。水位只表示已逐項讀取、對照與作出決策，**不表示 OPEN beta 已採用、fork runtime 已修復，或 #3649 的產品安全修補已完成。**

### 13. Dependency freshness 六項重審（2026-10-13）

本輪只處理 freshness report 的六個到期項目。已讀取官方 registry/crates metadata、相容性文件與 fork 實際引用；**沒有調高相依版本、修改 lockfile、Cargo 或產品碼**。下列 `deferredLatest` 均精準更新至本輪檢查版本，下一個更新版本會再次要求實質重審。

| 項目 | 官方來源與 fork 對照 | 判定與下一步 |
| --- | --- | --- |
| [`@lobehub/ui` 5.62.1](https://www.npmjs.com/package/@lobehub/ui/v/5.62.1) | registry metadata 要求 Node `>=22.22.0`、`antd ^6.1.1`；fork 有五個檔案直接 import `@lobehub/icons`，且 `npm ls` 顯示目前 `@lobehub/icons` 的 UI v5 peer 與 resolved UI v4 不相容。 | defer；不是單純延後。應獨立決定移除未直接 import 的 UI package/lockfile，或做 Node、UI、antd、icons 的協同遷移，再 clean-install、build 與 light/dark 驗收。 |
| [`antd` 6.6.6](https://ant.design/docs/react/migration-v6/) | 官方 v6 指南要求 `@ant-design/icons >=6`，並列出 CSS variables、DOM 與 component API 調整；fork 直接宣告 antd/icons v5，且是上述 Lobe peer topology 的一部分。 | defer；不得把 antd 單獨升到 v6。與 Lobe cleanup 或 antd/icons v6 協同工作一併做，包含 clean install 和視覺驗收。 |
| [`framer-motion` 14.2.0](https://motion.dev/docs/react-upgrade-guide) | 2026-10-13 補查 14.2.0 metadata；fork 在 DebugConsole、MiniView、ApiKeyFun、UserToken 有直接 import。官方升級指南指出 v13 對 Emotion prop filtering 需顯式注入，v14 則收緊內部相依版本配對。 | defer；14.2.0 沒有消除 major migration 風險。日後以互動動畫、prop forwarding 與 accessibility regression 覆核。 |
| [`i18next` 26.4.4](https://www.i18next.com/misc/migration-guide) | fork 在 `src/i18n.ts` 初始化並在多個畫面切換語言；目前 `react-i18next` v16（宣告 `^16.5.0`；lock `16.5.4`）peer 是 i18next `>=25.6.2`，並不強制 v17。 | defer；改正先前錯誤的 paired-v17 敘述。這是獨立 core major migration，需 typecheck/build 與語言切換驗收。 |
| [`lucide-react` 1.56.0](https://lucide.dev/guide/version-1) | fork 有 59 個直接 import；官方 v1 移除 brand icons，且預設 `aria-hidden`。 | defer；仍須完整 import audit 及鍵盤／螢幕閱讀器驗收，不能因 patch 更新而跳過 v0→v1 影響。 |
| [`sha2` 0.11.1](https://docs.rs/crate/sha2/0.11.1) | crates metadata 為 digest `0.11`、Rust `1.88`、edition `2024`；fork 在 crypto、cache、session、schema、thinking-store 路徑使用 SHA-256。 | defer；hash 輸出屬 runtime/persistent identifier 行為，須以 coordinated Cargo update、hash-stability fixture 與 Windows Tauri build 驗證，不能只為 freshness 調高版本。 |

本輪的 package/lockfile、Node、Cargo 與產品 runtime 工作均超出本次治理檔 ownership，應另立工作；本紀錄不把 deferral 當成已完成的遷移或相容性承諾。

### 14. 新增 upstream beta／issue 批次審查（2026-10-14）

本輪逐一讀取下列九個 PR 的 metadata、完整 diff 與檔案清單，並讀取兩個可存取 issue；各原始輸入保存在本次 checkpoint `upstream-3651-3663`。沒有移植上游程式碼、調整產品、manifest、workflow 或 checker。release／commit 基線仍為 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [PR #3651](https://github.com/lbjlaq/Antigravity-Manager/pull/3651) | reject | 已關閉未合併、目標 beta，實際跨 20 檔 `+1535/-73`：混合 HTTP API validation 欄位、帳號／device storage、CLI sync、handler、token manager、測試及其他 proxy 功能。這不是可獨立採用的 #3630／#3631 修補；fork 的 API 欄位與 validation 清理若要處理，須另以最小 API serialization／狀態轉換 fixture 審查。 |
| [PR #3652](https://github.com/lbjlaq/Antigravity-Manager/pull/3652) | defer | 已合併 beta，4 個 pipeline／mapper／model-spec／token-manager 檔 `+149/-35`，實際含 safety category 過濾、Claude tool sentinel、shell alias 與 Pro output cap。它改變安全分類與工具協定，不以 upstream 測試代替 fork payload fixture；待 release 與隔離 request/response regression 後再評估。 |
| [PR #3655](https://github.com/lbjlaq/Antigravity-Manager/pull/3655) | reject | 已關閉未合併，只改 `AccountCard`／`AccountTable` 的 weekly quota pill 排版（2 檔 `+82/-39`）。沒有 fork bug 或可驗收需求，不擴大自有 UI。 |
| [PR #3656](https://github.com/lbjlaq/Antigravity-Manager/pull/3656) | reject | 已關閉未合併，68 檔 `+6160/-1578` 同時重寫帳號排程、proxy handlers、token stats、設定、UI、README 與 scripts。minute stats 與 multi-agent spillover 不是小修補，且與 #3662／#3663 重疊；不可片段採用。 |
| [PR #3657](https://github.com/lbjlaq/Antigravity-Manager/pull/3657) | defer | 已關閉未合併，5 個 Rust 模組 `+303/-36`，加入跨 quota／project resolver／proxy pool 共用的 endpoint health tracker，並改動 `loadCodeAssist` 與 quota timeout／retry 次序。fork 有相近路徑，但沒有 provider timing、重試與帳號池 fixture；先保留 upstream 追蹤。 |
| [PR #3658](https://github.com/lbjlaq/Antigravity-Manager/pull/3658) | reject | 已關閉未合併，11 檔 `+174/-14` 把實際 upstream model 寫入 proxy DB、三個 handler、warmup、monitor 與 UI。這是監控功能擴張（對應 #3654），沒有 fork 產品需求；也不能在未定義 log retention／隱私語意下新增持久化欄位。 |
| [PR #3660](https://github.com/lbjlaq/Antigravity-Manager/pull/3660) | defer／獨立 security follow-up | 已關閉未合併，3 檔 `+70/-5` 以 `ConnectInfo` 嚴格判定 TCP loopback、拒絕偽造 forwarded header，並在停服時只放行 loopback `/internal/`。fork 靜態確認現有 auth 與 service-status 都無條件放行 `/internal/`，而 local warmup 本身呼叫 `127.0.0.1`；缺陷已確認但**尚未修復**。後續必須獨立以外部／偽造 header／IPv4、IPv6、mapped loopback／停服 fixture 驗證最小修補。 |
| [PR #3662](https://github.com/lbjlaq/Antigravity-Manager/pull/3662) | defer | 已合併 beta，9 檔 `+346/-18` 在三個 handler 與 token manager 引入 in-flight guard、priority spillover 與 `max_spillover_accounts`。這會改變帳號選擇、公平性與配額消耗；fork 現有排程不可直接疊加，待有 account-pool 時序 fixture 再評估。 |
| [PR #3663](https://github.com/lbjlaq/Antigravity-Manager/pull/3663) | defer | 已合併 beta，8 檔 `+264/-6` 新增 minute bucket 統計 query、Tauri commands 與 token-stats UI。fork 已有 hourly/daily/weekly 統計；新增 minute-level 儲存／查詢與本地時區語意是產品與資料契約變更，沒有本輪需求或 migration 驗收。 |
| [Issue #3654](https://github.com/lbjlaq/Antigravity-Manager/issues/3654) | reject | 僅請求顯示實際路由模型名稱，對應 #3658 的持久化／monitor 功能，沒有 fork 驗收需求；不因單一請求新增 proxy log 資料面。 |
| [Issue #3659](https://github.com/lbjlaq/Antigravity-Manager/issues/3659) | defer／獨立 security follow-up | 回報非 loopback `/internal/warmup` 無鑑權與停服仍可達。已用 fork 現行 auth、service-status、server `ConnectInfo` 與 local warmup 呼叫作靜態對照，確認根因；不把 #3660 視為已採用或已修。 |
| [Issue #3661](https://github.com/lbjlaq/Antigravity-Manager/issues/3661) | reject／無可審 delta | GitHub REST 精準讀取 `issues/3661` 回覆 404，且同次 `gh issue list --state all` 的 `#3651+` 清單只有 #3654、#3659、#3665；目前沒有可讀 issue 內容或 diff。此編號保留為不存在／已刪除的水位缺口，不虛構議題或修補。 |

PR 水位提升至 `#3663`，issue 水位提升至 `#3661`，只表示上述範圍已有逐項結論；release／commit 基線不變。執行 strict checker 時另發現 **PR #3664** 與 **Issue #3665**，兩項不在本批授權範圍，故本輪不審、不提高水位跨過它們，也不宣稱 upstream gate 已清除。

### 15. 剩餘 upstream gate 審查（2026-10-14）

本輪只處理前一批 focused strict checker 列出的 [PR #3664](https://github.com/lbjlaq/Antigravity-Manager/pull/3664) 與 [Issue #3665](https://github.com/lbjlaq/Antigravity-Manager/issues/3665)。已讀取 PR metadata、五檔完整 diff 與 issue 原文；原始輸入保存在 checkpoint `upstream-3664-3665`。沒有移植上游程式碼或改動產品／checker／workflow。release／commit 基線仍為 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [PR #3664](https://github.com/lbjlaq/Antigravity-Manager/pull/3664) | defer | 已合併 beta，5 個 Rust 檔 `+253/-46`：切號 fallback 再寫憑據、保留大小寫 restart args、OpenAI 429 fast mark、`QuotaExhausted` 後即時將 `model_quotas` 設為 0，及 optimistic reset 保留未到期的 quota lock。fork 已有 `credentials_applied`、`mark_rate_limited_fast`、`QuotaExhausted` 與 `clear_for_optimistic_reset` 相鄰路徑，表示風險相關；但切號重啟、帳號池選擇、quota 快取與鎖定期限具相依時序，且沒有 fork account-pool／provider fixture 或 Windows Rust runtime 驗證，不能片段採用。後續須獨立以 cold-restart、大小寫 workspace、429 重選、quota-zero 與 reset-time fixture 評估。 |
| [Issue #3665](https://github.com/lbjlaq/Antigravity-Manager/issues/3665) | reject | 只有「Gemini4」請求，無描述、模型 ID、provider capability、錯誤 payload 或預期路由契約。fork 的 generic Gemini flash 規則已涵蓋部分 4.x 名稱判定，但這不能證明真實 upstream model 可用、thinking／quota／fallback 語意正確；不因無規格請求擴張 model mapping。 |

PR 水位提升至 `#3664`，issue 水位提升至 `#3665`；只表示兩項已讀取並有結論，release／commit 基線不變，亦不表示 #3664 已採用或 fork runtime 已修復。

### 16. Dependency freshness：兩個新 release 重評（2026-10-14）

root freshness run 只列出這兩個新 release。本輪讀取官方 metadata 與有界 tag diff；此紀錄**不宣稱**任一 patch release 已在本 fork 完成相容性驗證。

| 套件 | 已讀證據 | 判定與明確重查條件 |
| --- | --- | --- |
| `@lobehub/ui` 5.64.2 | [npm 5.62.1](https://registry.npmjs.org/@lobehub/ui/5.62.1) 與 [npm 5.64.2](https://registry.npmjs.org/@lobehub/ui/5.64.2) 都要求 Node `>=22.22.0`、`antd ^6.1.1`、`@lobehub/icons ^5`、React 19 與 Motion 12。官方 [v5.62.1...v5.64.2 compare](https://github.com/lobehub/lobe-ui/compare/v5.62.1...v5.64.2) 為 19 commits／107 files，包含 DatePicker、theme 與 layout 變更。fork 仍直接宣告 antd v5、resolved UI 4.34.1、有既有 icon/UI peer mismatch，且 CI 為 Node 20。 | **defer**，reviewed watermark 前進至 5.64.2。僅在核准 package/lockfile cleanup，或完成 Node／antd／icons／UI 協同遷移後，以 clean install、frontend build 及 light/dark UI acceptance 重查。 |
| `lucide-react` 1.57.0 | [npm 1.56.0](https://registry.npmjs.org/lucide-react/1.56.0) 與 [npm 1.57.0](https://registry.npmjs.org/lucide-react/1.57.0) 的 React peer range 相同。官方 [1.56.0...1.57.0 compare](https://github.com/lucide-icons/lucide/compare/1.56.0...1.57.0) 為 12 commits／35 files，主要是新增 icon assets 與 versioning，並未驗證本 fork 獨立的 0.561.0-to-v1 migration；有界 source audit 為 59 個 `lucide-react` use sites。 | **defer**，reviewed watermark 前進至 1.57.0。既有 v1 accessibility-default／brand-icon-removal 風險仍需在核准 v1 migration 後，以完整 import audit、keyboard 與 screen-reader acceptance 重查。 |

### 17. 最終 maintenance gate 的兩項新增 delta（2026-10-15）

本輪只處理 root final run 列出的 [PR #3666](https://github.com/lbjlaq/Antigravity-Manager/pull/3666) 與 i18next 26.4.5；原始官方輸入、完整 PR diff、fork 對照與命令紀錄保存在 checkpoint `maintenance-two-deltas-20261015`。沒有採用上游產品 patch、變更 manifest／lockfile／workflow／checker，release／commit 基線維持 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [`i18next` 26.4.5](https://www.npmjs.com/package/i18next/v/26.4.5) | defer | 官方 [v26.4.5 release](https://github.com/i18next/i18next/releases/tag/v26.4.5) 修正 `hasLoadedNamespace(ns, { lng })` 對 resolved／unsupported language code 的 readiness：否則 react-i18next 的 `useTranslation(ns, { lng })` 可能永遠不 ready，造成 Suspense retry／SSR 卡住。這是實質上游修正，但 fork 目前是內嵌本地 resources aliases、未設定 `supportedLngs`，且 source 沒有帶 `lng` 的 `useTranslation` 呼叫，不能把它當作已驗證的 v26 採用理由。v26 仍會影響 locale 初始化、型別與切換；現有 react-i18next 16 peer `>=25.6.2` 並不強制 v17。僅於獨立 core migration 進行 typecheck／build 與 language-switch acceptance 時重查。 |
| [PR #3666](https://github.com/lbjlaq/Antigravity-Manager/pull/3666) | defer | 已合併 beta，5 個 Rust 檔 `+302/-100`：對低於 10% quota 的 EDF／P2C 候選移除 imminent-reset 提權、將 proxy toggle 改為帳號檔與 `accounts.json` index 的 locked 同步，並在已輸出內容後的 upstream error／idle timeout 以 Claude terminal triad 結束串流。fork 的 token manager 也有 quota／reset 排序及 P2C；command 仍直接寫 account 檔，雖已有可同步 index 的 `modules::account::toggle_proxy_status`；但 fork 沒有相同 `stream_lifecycle.rs`，Claude mapper 有不同的 thinking-without-content recovery。直接摘取會同時改變帳號公平性、持久化原子性與跨協定 client stream 語意。後續僅能在獨立產品候選，以低 quota/reset EDF-P2C、account/index atomicity、content/tool-after-error 與 idle-timeout 的 protocol fixtures 評估。 |

PR 水位提升至 `#3666`；issue 水位仍為 `#3665`，release／commit 基線不變。水位只表示完整 diff 與 fork 對照已作出結論，**不表示 #3666 已採用、i18next v26 已遷移或 runtime 已驗證。**

### 18. Full gate 新增 upstream PR #3667／#3668（2026-10-15）

本輪只處理 root final Full 列出的 [PR #3667](https://github.com/lbjlaq/Antigravity-Manager/pull/3667) 與 [PR #3668](https://github.com/lbjlaq/Antigravity-Manager/pull/3668)。兩項的 metadata、完整 diff、檔案清單與 fork 對照保存在 checkpoint `upstream-3667-3668-20261015`。兩項都是已關閉、未合併的 beta 變更；沒有採用產品 patch、改動 runtime／manifest／workflow／checker，issue／release／commit 基線維持不變。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [PR #3667](https://github.com/lbjlaq/Antigravity-Manager/pull/3667) | defer／獨立 cost-safety 候選 | 已關閉未合併，9 個 Rust 檔 `+388/-17`：新增 thinking-budget runaway circuit breaker、thought-token 增量累計、Claude/OpenAI budget 傳遞與通用 `stream_lifecycle` 熔斷。fork 已有全域 thinking-budget config、model-spec budget 解析與 request 注入，且 #3648 已確認 provider 可能無視 budget；但 fork 沒有相同 lifecycle 驅動器，現有 #3648 紀錄也沒有 provider response thought-token 計數或安全 cutoff 語意。不能以文字長度估算、閾值預設或 upstream beta 測試直接改寫本 fork。後續需獨立以遮罩 provider stream fixture 驗證累積 thought-token、閾值／budget 邊界、純 thinking 中止、content/tool 後豁免、帳號 quota 成本與各協定錯誤語意。 |
| [PR #3668](https://github.com/lbjlaq/Antigravity-Manager/pull/3668) | defer／獨立 protocol 候選 | 已關閉未合併，2 個 Rust 檔 `+180/-92`：在通用 lifecycle 對 content 或 thinking 後的 error／idle timeout／非正常 EOF 執行 protocol-specific finalize；Claude 另插入 interruption notice 以閉合 thinking block。fork 沒有該 `stream_lifecycle.rs`，Claude mapper 已有 thinking-without-content recovery 與直接 timeout 路徑，且 #3666 已被記為相鄰但未採用的 triad 行為。直接移植會同時改變 Claude／OpenAI／Gemini 的中斷、重試、history 與 tool-block 語意。後續只能以各協定的 pre-output error、thinking-only、content、tool-use、idle timeout 與 premature EOF fixture 驗證完整 closure，並明確決定 interruption notice 是否屬產品契約。 |

PR 水位提升至 `#3668`；issue 水位仍為 `#3665`，release／commit 基線不變。水位只表示兩份完整 diff 已有採用判定，**不表示 #3667／#3668 已採用、#3648 已修復，或任何 client stream 行為已通過 runtime 驗證。**

### 19. Full gate 新增 upstream PR #3669／#3670／#3672 與 Issue #3671／#3673–#3675（2026-10-15）

本輪只處理 root Full 列出的 7 項。以 authenticated GitHub REST 實際讀取 3 份 PR metadata、完整 diff、檔案清單，及 4 份 issue 原文與全部當時留言；輸入 URL、SHA-256 與命令／exit 紀錄保存在 checkpoint `upstream-triage-20261015`。沒有採用上游程式碼或改動產品、manifest、lockfile、workflow、checker。release／commit 基線仍為 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [PR #3669](https://github.com/lbjlaq/Antigravity-Manager/pull/3669) | reject／已由 #3670 取代 | 已關閉未合併，28 檔 `+1890/-15`：新增 Chat route、SSE client、localStorage history、三協定本機 proxy 呼叫、導航／設定／12 locale、CSP loopback 與 `react-markdown`／`remark-gfm` root dependencies。fork 沒有 `pages/Chat`、chat components/service/store；雖 lockfile 已有兩套件的 transitive entries，並不代表可採用此 UI。此 PR 是較早版本，#3670 已以同一功能擴充附件／Rust payload 路徑；不保留兩份平行產品候選。若產品 owner 明確要求內建 proxy Chat，僅以 #3670 作為獨立設計起點，先定義 desktop CSP、三協定 streaming／abort、history/privacy、無服務狀態與繁中-only locale acceptance。 |
| [PR #3670](https://github.com/lbjlaq/Antigravity-Manager/pull/3670) | defer／獨立 Chat 與附件產品候選 | 開啟中，33 檔 `+2533/-16`、4 commits：在 #3669 Chat 面加入 IndexedDB 附件、PDF／圖片三協定 payload，並改 `context_manager.rs`、OpenAI content model/request 以接受 inline file；同時改 CSP、drag-drop、root dependencies、route、設定與 locales。fork 沒有上述 Chat 檔案，且現有 OpenAI mapper 與 token/context 行為已偏離；直接 cherry-pick 會同時改資料儲存、inline payload 大小計算、CSP、UI 與多協定安全界面。僅在 owner 核准產品需求後，以附件 MIME／大小、IndexedDB 清理、OpenAI／Claude／Gemini streaming+abort、desktop CSP 與繁中 UI fixture 驗收。 |
| [PR #3672](https://github.com/lbjlaq/Antigravity-Manager/pull/3672) | defer／獨立 Responses 相容性修補 | 已合併，單一 Rust 檔 `+77/-50`，修正 `/v1/chat/completions` 收到 `instructions`／`input` 而無 `messages` 時，把 array `input.to_string()` 壓成單一 user 的 role-collapse；改為重用 `convert_codex_to_openai_request`，並為 string input 補 2 測試。fork 現行 handler 亦有相同簡化分支，但本 fork converter 另含 interaction ledger、custom/tool call、圖片輸出與 terminal-prefill 規則；上游 converter 的 string／非 array 行為不能無差別覆蓋。後續獨立修補必須以 Chat endpoint 的 instructions+多輪 user/assistant、string、非 array、tool/image、ledger 與 Gemini role payload fixture 驗收。 |
| [Issue #3671](https://github.com/lbjlaq/Antigravity-Manager/issues/3671) | defer／隨 #3672 的獨立相容性修補 | 原文精確重現 Responses array 在 Chat endpoint 失去 role 邊界，已由 #3672 關閉。fork 對照確認同一 `input.to_string()` 路徑存在；這是可追蹤缺陷，非已修復聲明。觸發條件與 fixture 同 #3672，不在 ledger 批次直接改產品。 |
| [Issue #3673](https://github.com/lbjlaq/Antigravity-Manager/issues/3673) | reject | 勝算雲商務／贊助接洽及聯絡方式，沒有可審產品 delta、錯誤報告或 fork 維護需求；不將第三方合作訊息轉成 provider 或配置改動。 |
| [Issue #3674](https://github.com/lbjlaq/Antigravity-Manager/issues/3674) | defer／獨立 provider-discovery 調查 | 原始回報為找不到「Gmini4」；上游留言稱 beta 以官方 Hub User-Agent 調整 discovery，後續留言仍稱最新 beta 缺 Gemini 3.1 Pro。fork 已有 `gemini-4` generic mapping 與受控出站 User-Agent，但沒有該帳號、可驗證的 model-list payload、精確 model ID 或 provider contract。不能因 issue 文案改 UA、硬編 model 名稱或宣稱 4.x 可用。僅在有授權帳號與遮罩 discovery fixture 時，驗證 request UA、回傳清單、mapping／quota 與 fallback。 |
| [Issue #3675](https://github.com/lbjlaq/Antigravity-Manager/issues/3675) | defer／與 #3674 同一 discovery 證據條件 | Jio Pro 帳號缺 Gemini 3.1 Pro 的簡短回報，留言重複 #3674 的缺模型資訊，未附 response、帳號能力、model ID 或重現條件。保留為 provider availability 信號；不推論為本 fork regression，待 #3674 所列的授權 discovery fixture 一併驗證。 |

PR 水位提升至 `#3672`，issue 水位提升至 `#3675`；只表示上述 7 項均已讀取並有明確採用／延後／拒絕結論。release／commit 基線不變，**不表示 Chat、附件、Responses role 修補或任何 Gemini 4／3.1 Pro discovery 已採用、已修復或已通過 runtime 驗證。**

### 20. Full gate 新增 upstream PR #3678 與 Issue #3676／#3677（2026-10-15）

本輪只處理 root Full 列出的 [PR #3678](https://github.com/lbjlaq/Antigravity-Manager/pull/3678) 與 [Issue #3676](https://github.com/lbjlaq/Antigravity-Manager/issues/3676)、[Issue #3677](https://github.com/lbjlaq/Antigravity-Manager/issues/3677)。以 authenticated GitHub 讀取 PR metadata、19 檔完整 diff、檔案清單，以及兩份 issue 原文與全部當時留言；端點、SHA-256 與命令／exit 紀錄保存在 checkpoint `upstream-triage-3678-20261015`。沒有採用上游程式碼、變更產品、manifest、lockfile、workflow 或 checker。release／commit 基線仍為 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [PR #3678](https://github.com/lbjlaq/Antigravity-Manager/pull/3678) | defer／獨立 diagnostics 產品候選 | 開啟中，19 檔 `+1007/-12`：在 Supported Models 加入單一／並行三個的 request probe 與 model check、原始 body／headers／scheduler account／mapped model 顯示、12 語系，並把 desktop CSP 擴至 loopback、CORS 改為 expose all response headers、Vite 加 `/v1` proxy。fork 已有 `ApiProxy` Supported Models、`X-Account-Email`／`X-Mapped-Model` 與 `cors_layer()` 的 `allow_origin(Any)`；上游的 `expose_headers(Any)` 會讓跨 origin caller 讀取既有 account telemetry，probe 的 raw response 與 Test all 也會產生真實帳號配額／routing 成本。fork 另採繁中-only locale policy，不能直接併入其 12 locale UI。僅在 owner 核准此產品面後，以 auth on/off、loopback／非 loopback、可讀 header 最小 allowlist、API key 不外洩、三協定 success/429/retired-model/stream/abort、concurrency/Stop 與不重送 fixture，連同 desktop CSP 與繁中 UI acceptance 評估。 |
| [Issue #3676](https://github.com/lbjlaq/Antigravity-Manager/issues/3676) | reject／無可審規格 | 原文只有「token 統計頁面能否依使用者 token 的 API key 查詢」標題與圖片，無預期輸出、帳號／管理員授權、查詢範圍、資料保留或錯誤重現，且沒有留言或對應 diff。fork 的 user token DB 保存 token，security log 僅保存 `api_key_hash`；不能把截圖當成可安全實作的明文 key 查詢需求。若 owner 明確提出需求，須先定義權限、hash-only lookup／遮罩、audit、retention 與無匹配行為，再以多租戶 token fixture 驗證不可枚舉或洩露 key。 |
| [Issue #3677](https://github.com/lbjlaq/Antigravity-Manager/issues/3677) | defer／獨立 models capability contract 候選 | 原文要求把帳號 `quota.models[]` 的 image／thinking／token-limit metadata 透過 `/v1/models` 等清單回應。fork 實際的 `get_all_dynamic_models` 合併 raw quota、normalized bucket、custom mapping 與內建 ID 後只回 `Vec<String>`；OpenAI、Claude 清單皆輸出固定四欄，Gemini 則輸出靜態 limits。這不是只加欄位：跨帳號 quota 可能互相衝突，tier alias／內建與 custom ID 未必有同一 capability，且三協定 schema 不同。僅在 owner 定義公開 metadata schema、帳號衝突優先序與 cache/privacy 契約後，以完整／缺欄舊帳號、衝突帳號、tier alias、custom/builtin、only-raw mode 與 OpenAI／Claude／Gemini compatibility fixture 實作並驗收。 |

PR 水位提升至 `#3678`，issue 水位提升至 `#3677`；只表示此三項已逐項讀取並作出結論。release／commit 基線不變，**不表示 request probe、明文 API key 查詢或 models capability metadata 已採用、已修復或已通過 runtime 驗證。**

### 21. Full gate 新增 upstream Issue #3679（2026-10-16）

本輪只讀取 [Issue #3679](https://github.com/lbjlaq/Antigravity-Manager/issues/3679) 原文與留言（metadata `comments=0`）。回報稱原始 request 沒有 `unknown`，但轉出報文出現 `function.name: "unknown"` 而被上游拒絕；沒有附可重放的 request／provider response、協定、模型、stream 狀態或實際 function-call payload。輸入 URL、SHA-256、fork 檔案 hash 與命令／exit 紀錄保存在 checkpoint `upstream-triage-3679-20261016`。沒有採用上游程式碼或改動產品、manifest、lockfile、workflow 或 checker；release／commit 基線仍為 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [Issue #3679](https://github.com/lbjlaq/Antigravity-Manager/issues/3679) | defer／獨立 malformed tool-call 修補候選 | fork 可確認 OpenAI 非串流 `src-tauri/src/proxy/mappers/openai/response.rs:92` 與兩個串流 function-call 分支 `streaming.rs:165`、`:757`，都在 Gemini `functionCall.name` 缺失或非字串時以 `unwrap_or("unknown")` 產出下游 function name；`response.rs:26-31` 的 `resolve_shell_tool_name` 又保持該值透傳。這與回報的輸出形狀相符，但 issue 沒有 payload，不能證明是哪一分支、哪一 provider 回應或 name 應如何還原；直接以 client tool 清單猜配，會在多工具、namespace／shell alias、retries 或 tool-call id 下改變協定語意。僅在獨立產品修補中，以 Gemini functionCall 有效 name、缺 name、非字串 name、空 client tool／單一工具／多工具、非串流與兩個串流路徑的 fixture，明確決定 fail-closed error、丟棄 malformed call 或可證明映射的契約，並驗證不輸出 literal `unknown`、不虛構 tool call、SSE completion／error 語意與 interaction ledger。 |

issue 水位提升至 `#3679`；PR 水位仍為 `#3678`，release／commit 基線不變。水位只表示 #3679 已讀取並有結論，**不表示 unknown function name 已修復、任何 provider response 已重現，或上述獨立產品修補已採用或通過 runtime 驗證。**

### 22. Full gate 新增 upstream PR #3680（2026-10-16）

本輪只讀取 [PR #3680](https://github.com/lbjlaq/Antigravity-Manager/pull/3680) 的 metadata、5 個 commits 與 9 檔完整 diff（`+481/-89`）；PR 開啟中、目標為 upstream `beta`。輸入 URL、SHA-256、fork 檔案 hash 與命令／exit 紀錄保存在 checkpoint `upstream-triage-3680-20261016`。沒有採用上游程式碼或改動產品、manifest、lockfile、workflow 或 checker；release／commit 基線仍為 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際 delta、fork 對照與後續 |
| --- | --- | --- |
| [PR #3680](https://github.com/lbjlaq/Antigravity-Manager/pull/3680) | defer／拆分為 routing/retry、account-state 與 Claude contract 的獨立產品候選 | 此 PR 同時改 `modules/{account,quota}.rs`、`proxy/{upstream/client,project_resolver,handlers/common,handlers/claude,common/model_mapping,token_manager}.rs` 與 retry tests：最終 diff 把端點設為 Daily→Sandbox→Prod、Sandbox 400 視為可 fallback、候選端點加 12 秒 response-header watchdog、health cooldown 改 300 秒、請求級無 delay 429 在第一輪全池後直接 `NoRetry`、entity/index 停用狀態互相同步、剝除 Claude model `[1m]` 類後綴並合併 client `anthropic-beta`／頂級 `effort`。commit 歷史卻先後主張 Daily→Prod→Sandbox、Prod→Daily→Sandbox、再切回 Daily→Sandbox→Prod，不能把 PR 標題或個別 benchmark 主張當成穩定端點契約。fork 現行 `upstream/client.rs:74-77` 是 Sandbox→Daily→Prod 且兩個 client 皆為 60 秒 TCP keepalive；`handlers/common.rs:246-319` 已讓多帳號第一輪全池快切、第二輪依 `Retry-After`／quota 再退避；`token_manager.rs:260-334` 僅按 entity 載入，`modules/account.rs:1208-1225` 亦未回寫 index；`model_mapping.rs`／`handlers/claude.rs:1300-1305` 尚未有 bracket canonicalization 或 client beta 合併。直接合併會在沒有 provider 實證下同時改端點地域失敗、長流首包、帳號檔案權威性、全池 rate-limit 成本與出站 Claude headers。僅在獨立批次以 fake endpoint 驗證 Daily/Sandbox/Prod 的 400／408／429／5xx、11/12/13 秒首包、最後候選與 health cooldown；以單／多帳號、explicit retry delay、hard quota、request-level 429 與 account rotation fixture 決定 retry 契約；以 index/entity 一致、漂移、毀損、並行 reload／quota update、停用持久化 fixture 決定原子來源；並以 bracket model、top-level effort、client/official beta 去重與非 Claude header fixture 驗證出站契約後，才可分項評估。 |

PR 水位提升至 `#3680`；issue 水位仍為 `#3679`，release／commit 基線不變。水位只表示 #3680 已讀取並有結論，**不表示端點排序、429 fail-fast、帳號狀態同步或 Claude Code 相容性已採用、已修復或通過 runtime 驗證。**

### 23. Full gate 新增 upstream Issue #3681／#3682（2026-10-16）

本輪只讀取 [Issue #3681](https://github.com/lbjlaq/Antigravity-Manager/issues/3681) 與 [Issue #3682](https://github.com/lbjlaq/Antigravity-Manager/issues/3682) 的原文、metadata 與當時全部留言（分別為 1、0 則）。輸入 URL、SHA-256、fork 檔案 hash 與命令／exit 紀錄保存在 checkpoint `upstream-3681-3682-20261016`。沒有採用上游程式碼或改動產品、manifest、lockfile、workflow 或 checker；release／commit 基線仍為 `v4.9.6`／`b601f5e7`。

| 項目 | 判定 | 實際內容、fork 對照與後續 |
| --- | --- | --- |
| [Issue #3681](https://github.com/lbjlaq/Antigravity-Manager/issues/3681) | defer／帳號安全與 provider enforcement 觀測 | 原文只回報 Google 以第三方軟體封鎖 3 個帳號並回傳 403，附申訴表單連結；唯一留言只有圖片，沒有可遮罩的 response body／headers、帳號驗證／生命週期、request fingerprint 或申訴結果。fork 的 `modules/quota.rs:329-347` 已把 quota 403 標為 `is_forbidden`，且明確不再剝除 project 後重送；warmup 在 `:804-814` 以統一帳號停用處理，`proxy/handlers/common.rs:329-330` 則將 401／403 視為帳號切換前的短暫延遲。這是安全敏感的 provider enforcement，不可藉由改 UA、重試、帳號指紋或申訴流程盲目繞過，也不能由單一回報宣稱 fork 已重現或已修復。僅在取得遮罩的 provider 403 body／headers 與明確帳號狀態分類後，以 fake provider fixture 驗證 forbidden 與暫態 403、停止／輪換／cooldown、持久化帳號狀態及不增加繞過請求，才開獨立產品批次。 |
| [Issue #3682](https://github.com/lbjlaq/Antigravity-Manager/issues/3682) | defer／獨立 deep-schema safety 修補候選 | 原文提供超過深度 10 的巢狀 tool schema，最深層 itemless array 造成 upstream 400。fork `proxy/common/json_schema.rs:3` 同樣定義 `MAX_RECURSION_DEPTH = 10`；`clean_json_schema_recursive` 在 `:164-171` 超限即返回，而 itemless-array 的 Gemini fallback 在 `:296-308` 才插入，因此該路徑與回報相符。fork 既有 `test_nested_array_without_items_gets_gemini_fallback` 只覆蓋未跨深度上限的案例。不可直接提高上限或無界遞迴：需獨立修補以超過 10 層、無 ref 的 itemless-array fixture 證明 fallback 仍插入，同時以 ref cycle／大型 schema fixture 證明深度與堆疊界限保留，並以 Gemini 與共用 tool-schema 輸出 fixture 驗證不再形成無效 `items`／400 語意。未有 owner 核准的獨立產品批次前，不採用。 |

issue 水位提升至 `#3682`；PR 水位仍為 `#3680`，release／commit 基線不變。水位只表示這兩項已讀取並有明確判定，**不表示 Google 帳號封鎖已重現或修復，亦不表示深層 tool schema fallback 已採用或通過 provider runtime 驗證。**
