# 路線圖

> 本 repo 後續工作的單一真相源。做完一項，就在同一個 commit 刪掉那一列；新工作只加在這裡。
> 現況段的數字是 2026-10-29 依 `main` `624ae2b61dff` 查的快照，開工前先重查 GitHub。

## 現況
`lbjlaq/Antigravity-Manager` 的 Windows 原生維護 fork（Tauri v2 + React + Rust 的帳號切換與協定代理），追上游 release 軸，已審到 `v5.0.0`（`tools/upstream_baseline.json`）；本 fork 最新 release 是 `v5.0.0`（2026-10-16），之後 `main` 已累積 83 個 commit（含 8 組上游移植與 #3754、#3758 修正，見 [`REVIEW.md`](REVIEW.md) 00.2–00.4）但未發版。
2026-10-29 查：open PR 1 筆（#56，Dependabot `rustls` 0.23.45 → 0.23.46）、Dependabot open alert 4 筆（low 3、medium 1，全是 `rustls`）、code scanning 0；CI／CodeQL／Upstream check 最近一次 success；issues 已停用，open issue 數「未查到」。上游已出到 `v5.0.1-beta.7`，尚無 v5.0.1 正式版（查法：`gh pr list -R SanHsien/Antigravity-Manager`、`gh api repos/SanHsien/Antigravity-Manager/dependabot/alerts?state=open`、`gh release list -R lbjlaq/Antigravity-Manager`）。

2026-10-29 本輪完成指定大檔的 fork 差異審查：限流測試已在 `624ae2b6` 全數進 CI；本輪另修工具 schema 的 `parameters_json_schema` 別名漏檢，先紅燈再以現有 CI selector 本機實跑 37 過／0 失敗。審查來源、範圍與上游原有問題見 [`DECISIONS.md`](DECISIONS.md)。

## 近期：可以直接動手
| 項目 | 為什麼 | 驗收方式 | 建議執行者 | 證據 |
| --- | --- | --- | --- | --- |
| 審查並合併 Dependabot PR #56（`rustls` 0.23.46） | open alert 全部指向 `rustls`，這筆 patch 升級是對應的修補 | `gh pr diff 56` 讀過，只動 `src-tauri/Cargo.lock` 的 `rustls`（若有連帶升級要在合併訊息寫明）；PR 上 CI 綠後合併；`gh api repos/SanHsien/Antigravity-Manager/dependabot/alerts?state=open` 不再有 `rustls` | Claude Sonnet | PR #56；2026-10-29 `gh api repos/SanHsien/Antigravity-Manager/dependabot/alerts?state=open` 查詢（全為 `rustls`，修補版 0.23.46） |
| 代理密碼解密失敗且 URL 沒帶帳密時，不把 `ag_enc_` 密文當 basic auth 送給代理 | `build_proxy_config` 只在 URL 有帳密時攔截 `ag_enc_`，否則照樣 `basic_auth(&auth.username, &auth.password)`，密文外送到第三方代理；P2 的「不採用」決定只涵蓋故障轉移語意，沒有處理這一段 | 移植上游測試 `build_proxy_config_blocks_ciphertext_when_no_url_auth`（先在 CI 紅）再修；既有 `test_build_proxy_config_fallback_to_url_auth_on_decrypt_failure` 保持綠；CI 新增對應 `task_` 步驟並實跑；`REVIEW.md` P2 段註記修復 commit | Claude Sonnet | `src-tauri/src/proxy/proxy_pool.rs:382-389`；[`REVIEW.md`](REVIEW.md) 完整分診表 `47a0f182` 列、P2 段 |
| 修輕量模式重建主視窗黑屏（P11） | fork 的 `exit_lightweight_mode` 對剛重建、`visible:false` 的視窗立即 `show()`，與上游 `e18d31fc` 修掉的缺陷相同 | 依 [`REVIEW.md`](REVIEW.md) P11 改法（含 3 秒保底強制顯示）；`npm run build` 綠、CI 綠；維護者在 Windows 11 實機切到輕量模式再從托盤叫出，視窗正常顯示 | Claude Sonnet | `src-tauri/src/modules/lightweight.rs:67-72`；`src/components/common/ThemeManager.tsx:13-19`；[`REVIEW.md`](REVIEW.md) P11 |
| 補 `v5.0.0` 之後的發版說明 | `v5.0.0` 之後的使用者可見變更（429 延遲、Gemini `?key=` 去敏、壞圖 400 自癒、Web 模式設定儲存等）還沒進任何 changelog；`AGENTS.md` 要求正式版同時更新 `CHANGELOG.md`、`CHANGELOG_EN.md`、`README.md`、`README_EN.md` | 四個檔都有新版本節，逐條對得到 [`REVIEW.md`](REVIEW.md) 00.2–00.4 的 fork commit；`pwsh -NoProfile -File tools\dev_check.ps1` 綠 | Claude Sonnet | `git log --oneline v5.0.0..main`；`CHANGELOG.md:6`（最新仍是 v5.0.0）；`AGENTS.md`「Standard Release Workflow」 |
| 發下一個 fork release 並實機驗收 | 讓上面的修正進到自動更新通道；安裝檔與切號需要 Windows 實機 | `npm run bump` 後在要打 tag 的 commit 跑完 Pre-flight Checks；`release.yml` success 且產出 `nsis`／`msi`；維護者實機安裝後完成一次帳號切換與一次代理請求 | 維護者 | `.github/workflows/release.yml`；`docs/RELEASE_GUIDE.md` |

## 中期：等條件成立再做
| 項目 | 觸發條件 | 驗收方式 | 建議執行者 | 證據 |
| --- | --- | --- | --- | --- |
| 整批審上游 v5.0.1（含 beta 的 #3761、#3765、#3767、#3771、#3773–#3775 與號池 429／503 修正 `e2c89ff8`／`8ce78b2f`） | 上游發佈 v5.0.1 正式版（`Upstream check` 因 release 軸有新版本而失敗） | `REVIEW.md` 新增一節逐筆記採用／不採用與重新開案條件；採用項各有 CI 實跑的 `task_` 測試；`tools/upstream_baseline.json` 的 `reviewed_release`／`reviewed_through` 推進；`Upstream check` 轉綠 | Claude Opus | [`REVIEW.md`](REVIEW.md) 00.5、00.6；`tools/upstream_baseline.json`（`track: release`） |
| 提前移植號池 429／503 的入站模型門禁 | 維護者遇到整個號池同時 429／503（不等上游 release） | 移植 `e2c89ff8`／`8ce78b2f` 的做法，讓已下線模型（如 `gemini-2.5-pro`）不再把帳號送進長冷卻；CI 有對應測試；維護者實機確認號池恢復 | Claude Opus | [`REVIEW.md`](REVIEW.md) 00.5 #3768／#3772；`src-tauri/src/proxy/cli_sync.rs:1353`、`src-tauri/src/proxy/common/model_mapping.rs:57` |
| 提前移植 Antigravity IDE 2.6.0 切號修正（#3771） | 維護者回報 IDE 2.6.0 切號後仍未登入或黑屏 | 補 `id_token_email` 等欄位，`agy` 行程判定（只偵測行程，不經 `agy` 呼叫任何服務）收窄到 `.gemini\bin` 或 Antigravity 安裝目錄（不採上游只看名稱的判定）；CI 綠；維護者實機切號成功 | Claude Sonnet | [`REVIEW.md`](REVIEW.md) 00.5 #3771、#3762、#3770；`src-tauri/src/modules/process.rs:566-628` |
| 無 quotaInfo 的模型被丟棄（P8a／P8b） | 帳號的模型清單出現 `gemini-3.1-pro-high/low` 但沒有 `gemini-pro-agent`，或日誌出現 `No accounts available ... gemini-pro-agent` | 先依 [`REVIEW.md`](REVIEW.md) P8 用 debug 日誌在維護者帳號實測；成立後抽出 `build_model_quotas` 純函式並補 P8 列的五種測試，加白名單避免內部遺留模型混入 | Claude Sonnet | [`REVIEW.md`](REVIEW.md) P8；`src-tauri/src/modules/quota.rs:388` |
| 前端 major 依賴遷移（Vite 8＋`@vitejs/plugin-react` 6、antd 6＋icons 6、i18next 26＋react-i18next 17 等成組升級） | 該組的延後條目被新版本推過期（新鮮度檢查轉紅），或有安全告警只能靠 major 修 | 每組一次遷移：乾淨安裝、`npm run build`、`tools\dev_check.ps1` 綠，並在 `.github/dependency-deferrals.json` 刪除該組條目 | Codex | `.github/dependency-deferrals.json`（各條目的 `reason` 寫明要成組升級） |
| 觀察 Claude count_tokens 的上游占位回應 | 維護者另外要求修正計數，或確認客戶端因零值誤判上下文用量 | 先對非空訊息補會失敗的計數測試，再獨立評估上游估算實作；本輪僅記錄，不實作 | 待指派 | [`DECISIONS.md`](DECISIONS.md) `UPSTREAM-OBS-001` |

## 刻意不做
| 不做什麼 | 理由 | 什麼情況下重新考慮 |
| --- | --- | --- |
| 恢復 macOS／Linux／Docker 建置、GitHub Pages、簡體中文語系 | v5.0.0 已收斂為 Windows 原生（`nsis`／`msi`），fork 不出其他平台包（`CHANGELOG.md` v5.0.0、[`FORK.md`](FORK.md)「與上游的差異」） | fork 決定支援其他平台，且有人能實機驗收 |
| 移植 fork 沒有對應程式的上游功能：QoS、Argon 模型、Mini 窗、CLI、Linux 顯示守衛、`stream_lifecycle` 家族 | 缺陷所在的元件在 fork 不存在，照抄會引入整套新功能（[`REVIEW.md`](REVIEW.md) 00、00.3） | fork 引入對應元件，或加入 `gemini-4-argon` 等模型 |
| 綁定代理不健康時一律改走公用池（上游 P2 改法） | 會讓「關閉故障轉移」的帳號也換出口 IP，與設定語意相反（[`REVIEW.md`](REVIEW.md) P2「決定」） | 維護者要求「專屬帳號絕不直連」：改在 `upstream/client.rs` 對綁定帳號的 `Ok(None)`／`Err` 回錯誤，並移植上游 4 個測試 |
| 對上游開 PR 或推 `upstream` | 對外只打 `SanHsien/Antigravity-Manager`（`AGENTS.md` 對外邊界硬閘門、`.cursor/rules/no-upstream-pr.mdc`） | 維護者在當次對話明確同意回貢某一筆修正 |

## 維護方式
上游：追 release 軸，`Upstream check` 每週跑；beta 內容等上游正式版再整批審，只有維護者實際遇到時才提前移植（見中期表）。PR／issue 逐筆讀 diff 後記進 [`REVIEW.md`](REVIEW.md) 並推進水位。依賴：Dependabot PR 先讀 diff 再合併；major 延後寫進 `.github/dependency-deferrals.json`。Rust 改動先載入已驗證的 Windows 工具鏈，做本機目標測試與 fmt／Clippy，再確認 CI 的 `task_`／`tool_schema` 步驟；不能只以編譯通過宣稱測試通過。
