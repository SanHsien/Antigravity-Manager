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

已閱讀 [Issue #3629](https://github.com/lbjlaq/Antigravity-Manager/issues/3629)。決策：needs-info／follow-upstream。作者從 executable 找到 Gemini defense 的 `ok go on` 字串，回報它可能干擾 Agent 等待人類確認的狀態；但未提供版本、原始請求、gateway 實際出站 payload 或 fork 重現。

fork 的 `src-tauri/src/proxy` 精準搜尋未找到 `ok go on` 或該兩條完整 Defense 訊息；這不等於所有補齊訊息行為均不存在，更不能據此宣稱本輪已修復。後續應比對來源版本與 request mapper，使用隔離 payload fixture 確认補齊訊息是否被當作人類授權；不得把合成訊息當成操作者同意，也不直接採用未驗證的開關／替代文字方案。issue 水位更新至 `#3629`，其他水位不變。
