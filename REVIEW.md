# 證據式覆核報告 (Review)

- **覆核時間**：2026-09-26
- **覆核對象**：`SanHsien/Antigravity-Manager`（fork from `lbjlaq/Antigravity-Manager` @ `v4.8.1` / `27ee35b7`），
  疊加 overlay commit `68ae072e`（"chore(fork): establish SanHsien Windows maintenance overlay ..."）。
- **執行環境**：Windows 11 原生（PowerShell / Node.js v26.7.0 / npm 11.19.0 / Python 3.14 / Git 2.55）。
- **本次覆核者**：獨立於 `68ae072e` 作者的覆核 pass，不採信該 commit 內含的舊版 `REVIEW.md`「100% 綠燈」結論，逐項重跑驗證。

---

## 1. 覆核結論

**舊版「全數通過 (Ship Ready) / 100% 綠燈」結論：過度樂觀（overstated）。**

- overlay 本身新增的 4 項工具（前端建置、Python 編譯、連結檢查、上游巡檢）在 commit 當下確實全綠，這部分屬實。
- 但舊版報告**完全沒有提到**：(a) 同一次 push 觸發的 `Deploy static content to Pages` workflow 當場失敗，且往後每次 push 到 main 都會繼續失敗；(b) Rust/Tauri 是否有被驗證過——`dev_check.ps1` 從不編譯 `src-tauri`，舊報告未交代這個落差是否由別處補上。
- 「100% 綠燈」被寫成一次性靜態結論，但 `check_upstream_updates.py --strict` 的設計就是「一旦有新的上游 issue/PR 未 triage 就會變紅」——這是正確的設計行為，不是 bug，但代表這份報告的「綠燈」保質期極短，寫成定論具有誤導性。

---

## 2. 實跑證據

### 2.1 `npm ci`（先前 `node_modules` 不存在，這是本次覆核首次安裝）

```
added 821 packages, and audited 822 packages in 49s
24 vulnerabilities (2 low, 15 moderate, 7 high)
```

### 2.2 `powershell -File tools\dev_check.ps1`（今日實跑，非沿用舊輸出）

```
==> Checking Frontend Build (npm run build)
...
✓ built in 2m 52s
==> Compiling Python tools
==> Checking fork document links
OK   FORK.md / NOTICE.md / REVIEW.md / CLAUDE.md / GEMINI.md / SECURITY.md / CONTRIBUTING.md / AGENTS.md
共 8 份 overlay 文件，0 份有缺檔。
==> Checking upstream updates
...
## Upstream issues
Triaged through #3519.
1 new item(s) to triage.
| #3523 | 你们有遇到死循环的情况吗 感觉昨晚开始gemini开始死循环了？ |

Upstream check failed with exit code 1
```

**結論：`dev_check.ps1` 現在是紅燈**，原因是上游新增 issue #3523 尚未 triage，`--strict` 依設計回傳非 0。這不是工具壞掉，是工具正確地抓到一個需要人工決定的項目（見「待主人決策」）。

### 2.3 CI 狀態（`gh run list --branch main`）

| Workflow | 結論 | 連結 |
|---|---|---|
| CI（前端建置 + 3 平台 Rust fmt/clippy/check + Tauri debug build） | ✅ success | https://github.com/SanHsien/Antigravity-Manager/actions/runs/36170029785 |
| Upstream check | ✅ success（覆核當下尚未有 #3523） | https://github.com/SanHsien/Antigravity-Manager/actions/runs/36170029819 |
| Deploy static content to Pages | ❌ **failure** | https://github.com/SanHsien/Antigravity-Manager/actions/runs/36170029860 |

Pages 失敗原因（實際錯誤訊息）：

```
##[error]Get Pages site failed. Please verify that the repository has Pages
enabled and configured to build using GitHub Actions ...
HttpError: Not Found
```

`gh api repos/SanHsien/Antigravity-Manager --jq '.has_pages'` → `false`：這個 repo 從未啟用 GitHub Pages，該 workflow 從 upstream 繼承而來，在本 fork 上必然失敗。

### 2.4 Rust/Tauri 驗證落差是否有人補

**是，由 CI 補上，不是本地。** `ci.yml` 的 `check-rust`（`cargo fmt --check` / `cargo clippy` / `cargo check`，ubuntu + windows-2025 + macos matrix）與 `build-tauri`（`tauri build --debug --no-bundle`）三平台皆跑過且本次全數 `success`（見上表連結）。本機不裝 `cargo` 是刻意決定（沿用主人指示），`dev_check.ps1` 本來就不該重複做 CI 已覆蓋的事——但舊版 REVIEW.md 完全沒寫這段推理，只留下「有沒有測 Rust」的疑問，這是文件缺失，不是驗證缺失。

---

## 3. 本次交付改動清單（overlay，commit `68ae072e`）

1. **維護與門禁工具**：`tools/dev_check.ps1`、`tools/upstream_baseline.json`、`tools/check_upstream_updates.py`、`tools/check_links.py`。
2. **規範與治理手冊**：`FORK.md`、`AGENTS.md`、`CLAUDE.md` / `GEMINI.md`、`.cursor/rules/no-upstream-pr.mdc`、`NOTICE.md`、`CONTRIBUTING.md`、`SECURITY.md`。
3. **CI 自動化**：`.github/workflows/upstream-check.yml`（每週排程監控上游更新）。

以上均為文件與工具層級改動，未觸及 `src-tauri` 或 `src/` 的產品邏輯，安全性抽查未發現新增風險（`.gitignore` 唯一變動是保留 `.cursor/` 並忽略 `upstream-review-report.md`，均無害）。

---

## 4. 問題清單（依嚴重度）

| 嚴重度 | 檔案:行 | 問題 | 狀態 |
|---|---|---|---|
| Medium | `.github/workflows/deploy-pages.yml` | Pages 未啟用，每次 push main 必然失敗，長期留著會讓 Actions 頁面持續掛紅 X，掩蓋真正的失敗訊號 | **已修復**（見下） |
| Low | 舊版 `REVIEW.md` | 未揭露 Pages workflow 失敗、未說明 Rust 驗證由 CI 而非本地覆蓋、把一次性通過寫成靜態「100%」結論 | 本次覆核重寫取代 |
| Low | 依賴 | `npm audit`：24 vulnerabilities（2 low / 15 moderate / 7 high），fork 未開 Dependabot alerts | 見第 6 節建議 |
| Info | `tools/check_upstream_updates.py` | `--strict` 設計上會因新 issue/PR 而變紅，屬預期行為非 bug，但需要文件講清楚（本次已在此報告說明） | 已說明 |

---

## 5. 本次修復

- `.github/workflows/deploy-pages.yml`：`deploy` job 加上 `if: github.repository == 'lbjlaq/Antigravity-Manager' || vars.ENABLE_PAGES_DEPLOY == 'true'`，比照 `release.yml` 既有的 Docker push 閘門樣式，避免 fork 上的每次 push 都留下無法修的紅燈。若之後想在本 fork 啟用 Pages，設定 repo variable `ENABLE_PAGES_DEPLOY=true` 並到 repo Settings 開啟 Pages（Build and deployment: GitHub Actions）即可。

---

## 6. 待主人決策（Open Items）

1. **上游 issue #3523**（"你们有遇到死循环的情况吗"）尚未 triage，`dev_check.ps1 --strict` 會持續回報為紅燈直到主人決定 adopt/skip 並手動調高 `tools/upstream_baseline.json` 的 `reviewed_issue_through`。本次覆核不代為決定。
2. 是否要在本 fork 啟用 GitHub Pages（若不需要，可考慮直接刪除 `deploy-pages.yml`，比留著加條件閘門更乾淨）。
3. `npm audit` 的 24 項漏洞是否要處理／哪些可以 `npm audit fix`、哪些需要手動評估重大版本升級。

---

## 7. Dependabot 建議

**建議開啟。** `gh api repos/SanHsien/Antigravity-Manager/vulnerability-alerts` 回傳 `404 Not Found`，代表這個 fork 從未開過 Dependabot security alerts；`npm audit` 已顯示 24 項漏洞（7 high）。本次覆核只給建議，不代為變更 repo 設定：

```
gh api -X PUT repos/SanHsien/Antigravity-Manager/vulnerability-alerts
```

---

Maintained per `AGENTS.md` 協作約定：修 bug 需回註本檔並附 commit hash（見上方「本次修復」與 commit message）。
