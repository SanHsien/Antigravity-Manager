# 證據式覆核報告 (Review)

## 00. 2026-10-26 Antigravity-Manager 上游 commit 分診（v4.9.8 之後 -> v5.0.0，`0067fe04`）

- 範圍：`git log 9f8e4f24d61b33fa80b8078d104c942752a5fa0b..upstream/main --no-merges`，共 67 筆（最新 `fd6fab80`）。
- fork 基準：`C:/GitHub/Antigravity-Manager` HEAD `df98b51a`，工作樹乾淨；全程唯讀（`git show`、grep、讀檔），未改 repo、未 push、未對上游做任何寫入。
- 判準：移植「做法」不是 diff；每筆先在 fork 對應檔 grep 確認缺陷是否存在。判定詞沿用 REVIEW.md：採用候選／延後／拒絕／監看（另加「已存在」）。
- 本機實況佐證（唯讀）：`~/.antigravity_tools/accounts/*.json` 共 4 個帳號（3 FREE、1 PRO）；`proxy_logs.db` 僅 5 筆（全為 200，皆為 `/internal/warmup`）。本機實際流量極低，下方「價值」排序以「缺陷在 fork 程式碼中確實存在」加「影響面」判斷，不以本機症狀為準。
- 本機驗證能力：本機 `cargo` 無 MSVC `link.exe`（bash PATH 上的 `link.exe` 是 Git 的 coreutils，不是 MSVC），所有 Rust 單元測試只能靠 CI；`rustfmt` 在本 bash 找不到。純前端改動可用 `npm run build`（`tsc && vite build`）本機驗。

### 00.1 Release、PR、issue 判讀（水位：release v5.0.0／PR 3751／issue 3750）

| 項目 | 判定 | 證據與重新開案條件 |
|---|---|---|
| `0067fe04`（v5.0.0 release bump）與 11 筆 beta bump | 拒絕 | 版本號與 changelog metadata，只由獨立 release packet 評估。 |
| PR #3751（QoS 巡檢 DashMap 自死鎖、Headless Argon 熔斷；合併於上游 `beta`，在 v5.0.0 之後，另有 `v5.0.0-beta.3` 同在 beta 軌） | 拒絕 | 5 檔 +183/-71 全在 `qos_guardian.rs`／QoS 版 `token_manager`；本 fork 無 QoS 排程（`grep -ri qos_guardian src-tauri/src` 無命中，唯一 `qos` 字樣是 `lib.rs:47` 的 Windows EcoQoS）。fork 採用 QoS 系列時重開。 |
| PR #3747（已關閉；Argon 虛假配額、macOS 懸浮窗） | 拒絕 | 動到 fork 沒有的 Argon／Mini 窗；`quota.rs` 的無 `quotaInfo` 問題另見 00 的 `7fcb1a2a`。 |
| PR #3746（open；入站 Gzip 解壓） | 監看 | 功能新增（Ref #425／#146），非缺陷；fork 無 `RequestDecompression`。上游合併且有明確使用者需求時再評估。 |
| Issue #3749／#3750／#3744／#3745／#3743／#3748 | 拒絕 | 全部指向 QoS、Argon、Mini 窗或託盤互動；fork 無對應程式碼。 |


### 00.2 採用項修復紀錄（2026-10-26，皆已進 main，CI 實跑新增測試通過）

| 項目 | 上游 | fork commit | 與上游差異 |
|---|---|---|---|
| P1 429 延遲取 max、`resets in`、掃全部 details | `7ed0f10e` 第 1 段 | `49204cba` | 不把 `QuotaFailure` 映射成配額耗盡（會把每分鐘限流誤鎖長時間）；未加 `handlers/common.rs` 的 individual quota |
| P4 驗證封鎖到期不再永久自鎖 | `d6974bc3` | `c6d53b62` | 只在驗證封鎖到期時重探；一般 403／proxy 停用帳號不重探（上游會全部重探） |
| P3 Gemini `?key=` 含日誌去敏 | `505deb09` | `d1611b8e` | 管理端點不接受 query key；監控日誌、IP 日誌一律去敏（含 percent-encoded 參數名） |
| P5 會話指紋穩定 | `f86c2485` | `2a6c8ee2`、`b75f4075` | 文字錨點優先，既有文字會話 id 不變；簽名 URL 去 query；`[System` 後任何空白皆視為標籤 |
| P10 Claude 429 外洩 | `16d31898` 僅 Claude handler | `ae762fb1` | 另修 200 header 後 peek 失敗仍回 200 的洩漏；未取 `common.rs` 多日配額快速轉移 |
| P7 Web 模式儲存設定熱更新 | `d206e57e` | `5065411f` | 另補 `circuit_breaker`；未做 hub 指紋 |
| P9 強制 tool_choice 移除 Claude thinkingConfig | `cae8fb9b` 第 3 項 | `2fa371d7` | 不做「400 後剝除重試」 |
| P13 thought 片段簽名查詢 | `aa6cd5b1` 小項 | `78d7cce2` | 改為掃最近 50 筆解壓後 `contains`（區分大小寫） |

**已決（2026-10-26，維護者授權代決）**：P2 不採用（維持現行行為，見 P2「決定」）；P6 採保守版（僅在已收到 usageMetadata 時補 finish_reason，見 P6「決定」）。**延後**：P8（無 quotaInfo 模型，需真實 API 資料）、P11（輕量模式黑屏，需 Windows 實機）、P12（warmup 路由，現況不觸發）延後。

**既有失敗測試（base 即紅，CI 從未執行）**：`proxy::rate_limit::tests::task_preserves_explicit_long_image_quota_deadline`、`proxy::rate_limit::tests::test_server_error_does_not_accumulate_failure_count`；CI 新步驟以 `--skip` 排除前者。修正需先決定預期語意（顯式長配額期限是否僅限圖像模型）。


---

### 結論先行：建議採用清單（依價值排序）

| 序 | SHA | 做法（一句話） | 價值 | 本機可測 |
|---|---|---|---|---|
| 1 | `7ed0f10e`（僅第 1 段 429 解析） | `Retry-After` 標頭與回應本文延遲取 max、補 `resets in` 文字樣式、掃描全部 `details`；fork 的 `retry.rs`／`rate_limit.rs` 與上游修前一模一樣 | 高：標頭 8s 蓋過本文數小時的配額重置，帳號早退解鎖後 429 連環 | 否（CI） |
| 2 | `47a0f182`（僅防 Direct 洩漏與密文護欄） | 綁定專屬代理的帳號，代理不健康時不回退直連，改為先試公用池、再不行仍走綁定代理；`ag_enc_` 解密失敗且無 URL 憑證時不送密文 | 高：專屬 IP 帳號會洩漏來源 IP（`upstream/client.rs:253-292` 對 `Ok(None)` 回預設 client） | 否（CI） |
| 3 | `505deb09` | 新增 `extract_api_key()`，支援 `?key=`、`x-goog-api-key`、小寫 `bearer`；另加 fork 專屬要求：query key 不得進監控日誌，管理端點不接受 query key | 中高：Gemini 官方 REST 客戶端用 `?key=` 會被誤 401；但需補日誌去敏，否則會把金鑰寫進 `proxy_logs.db` | 否（CI） |
| 4 | `d6974bc3`（REVIEW 已列採用候選） | 驗證封鎖到期後放行批次刷新、到期清除時一併清 `quota.is_forbidden` | 中高：帳號永久自鎖，只能手改 JSON | 否（CI） |
| 5 | `f86c2485` | 會話指紋改為「摘要化多模態」「單字元也算有效輸入」「`[System` 未閉合不截斷」；fork 的 Gemini 路徑兜底是 `request.to_string()` 整包雜湊，每輪必漂移 | 中：影響 thinking_store／prompt cache 連續性 | 否（CI） |
| 6 | `073d6a97`（僅 OpenAI Chat 串流收尾，且需改良） | EOF 時若無 finishReason 但已有內容／工具呼叫，補發 `finish_reason`（建議只在已收到 usageMetadata 時補，避免掩蓋中斷） | 中：下游客戶端判 `finish=unknown`；fork 的 Responses 串流刻意標 `upstream_interrupted`，需保留語意 | 否（CI） |
| 7 | `d206e57e`（僅 Web 模式熱更新缺口，hub 指紋略） | `admin_save_config` 補 `only_raw_quota_models`、`debug_logging`、UA、`capture_health_logs`、代理池綁定同步、預設 client 重建；另補 `circuit_breaker`（上游也漏） | 中：Web／Docker 儲存設定後部分欄位不生效 | 否（CI） |
| 8 | `7fcb1a2a`（僅「無 quotaInfo 的模型被丟棄」，Argon／i18n 略；需先實測確認） | `fetch_quota_with_cache` 取消「必須有 quotaInfo」門檻，改用 quota_groups 補額度；**本機 PRO 帳號有 Gemini 群組（5h 74%），卻無任何 gemini-3.1-pro 模型**，與此缺陷吻合但未證實 | 中（待驗證）：Pro 模型從清單消失，會讓調度器 503 | 否（CI） |
| 9 | `cae8fb9b`（僅第 3 項：強制 tool_choice 與 Claude thinking 衝突） | 目標為 Claude 且 `toolConfig.mode == ANY` 時移除 `generationConfig.thinkingConfig` | 中低：Anthropic 規則「forced tool_choice 不可開 thinking」；fork 的 OpenAI mapper 會把 `required`／物件形式映射成 ANY | 否（CI） |
| 10 | `16d31898`（僅 Claude handler 的 429 外洩） | `claude.rs` 補 `FailureStatusTracker`（`common.rs:120` 已有，gemini／openai handler 已用，Claude 沒用） | 中低：混合 429 與逾時時，Claude 客戶端收到 429 而進入 1 分鐘退避 | 否（CI） |
| 11 | `e18d31fc` | 輕量模式重建主視窗時不立即 `show()`，由前端首幀後顯示；`ThemeManager` 補 `unminimize`／`setFocus` | 低中：Windows WebView2 黑底閃現；fork 為 Windows 原生，直接相關 | TSX 部分可 `npm run build`；Rust 部分 CI；視覺需手動實機 |
| 12 | `bec5c554` | warmup 先經 `resolve_model_route` 再送上游 | 低：本機 quota 清單的模型名都是具名／有 `-tiered` 後綴，現況不會 404；只在上游回傳裸名時才觸發 | 否（CI） |
| 13 | `aa6cd5b1`（僅 `lookup_signature_by_thought_snippet` 一小項） | 對 GZIP 壓縮的 thought 不可用 SQL `LIKE`，改取近 50 筆解包後比對 | 低：只影響 UI 簽名顯示兜底（`monitor.rs:389`） | 否（CI） |

其餘（QoS／Argon／Mini 窗／簡中／CLI／Linux 專用／stream_lifecycle 家族）fork 無對應程式碼，全部拒絕或延後，見下方全表。

最需要決策者注意的三點：
1. `505deb09` 單獨照搬會讓 `?key=` 進 `proxy_logs.db`（`monitor.rs:736/912` 把完整 URI 存成 `url`）與 `ip_filter.rs:111`，必須同批去敏。
2. `073d6a97` 的合成 `finish_reason` 與 fork 既有「Responses 串流無 finishReason 即標 incomplete／upstream_interrupted」的設計取向相反，建議只在 clean EOF（已有 usage）時合成。
3. `7fcb1a2a`／`bba1e3d6` 是同一個 Issue（#3737）的兩半；本機證據顯示 Pro 模型沒進清單，先實測再動手，單做模型改名（bba1e3d6）無效。

---

### 完整分診表（依 git log 由新到舊，67 筆）

欄位：`SHA | 判定 | 證據（上游動了哪些檔／行數；fork 對應檔實況） | 觸發條件／採用要點`

| SHA | 判定 | 證據 | 觸發條件／採用要點 |
|---|---|---|---|
| `fd6fab80` `6279e5e1` `fe2ace7d` `4b9c983e` `f356c3e9` `61ddc8ef` `01a60a28` `8a998cb3` `b04fea67` `74bb2324` `6b75dcb4`（共 11 筆 chore(release)） | 拒絕 | 各為版本號／changelog／lockfile／tauri.conf 變更（9–15 檔，+21~+89 行） | 僅由 fork 獨立 release packet 評估 |
| `e18d31fc` | 採用候選 | 上游：`modules/lightweight.rs`(13)、`ThemeManager.tsx`(2)。fork：`lightweight.rs:67-72` `exit_lightweight_mode` 對剛重建的視窗立即 `show()/unminimize()/set_focus()`；`tauri.conf.json:23` 主視窗 `visible:false`；`ThemeManager.tsx:13-19` 已於 100ms 後 `show()`。同缺陷存在 | 見實作方案 P11 |
| `bc7f5e59` | 拒絕 | 上游 5 檔：`commands/mod.rs`(45)、`capabilities/default.json`、`lib.rs`、`MiniView.tsx`、`main.tsx`；修 Mini 窗喚醒主控台的跨執行緒死鎖。fork 無獨立 `mini` 窗（`tauri.conf.json` 無 mini；`MiniView.tsx` 是主窗內 `isMiniView` 版面，`Layout.tsx:13`）；`exit_lightweight_mode` 的呼叫點只有 `tray.rs:92,237`、`lib.rs:305`（事件迴圈執行緒），沒有 tokio 非同步指令呼叫 | 重開條件：fork 新增從 async command 呼叫 `exit_lightweight_mode` 的入口時，改用 `run_on_main_thread` |
| `d1f41575` | 拒絕 | `openai.rs`(6)、`inbound.rs`(4)：修 `c819efa5` 的可變性綁定與格式；母 commit 未採 | 隨 `c819efa5` |
| `c819efa5` | 延後 | 上游 4 檔（+283）：Responses API `text.format` 結構化輸出映射、`reasoning.effort=low` 時 thinkingBudget<=1024、tools 與 responseSchema 互斥、Responses 回應整包圍欄解殼。fork：`openai/request.rs:1198-1206` 只處理 Chat 的 `response_format`；`openai/models.rs:36` 有 `response_format`，無 `text` 欄位 | 重開條件：Codex（Responses API）打 fork 時出現結構化輸出 JSON 解析失敗或對話命名逾時；需 Responses 結構化輸出、圍欄解殼零誤殺的對抗測試 |
| `91c342c6` | 拒絕 | `model_specs.rs`(11)：把 Claude 預算 `<0` 的 `Some(-1)` 分支砍成 `None`，是上游對自己先前語意的回改。fork `model_specs.rs:355-430` 的 `resolve_custom_budget` 無 `tb_config.effort` 回退鏈，Claude 分支結構不同，無對應回歸 | 無 repro，不動 |
| `b0fa557c` | 拒絕 | `App.tsx`(24)：修 Mini 窗載入 `index.html` 觸發 React Router 404；fork 無獨立 mini 窗 | 同 `bc7f5e59` |
| `447490f9` | 拒絕 | `pipeline/stream_lifecycle.rs`(61)；fork `proxy/pipeline/` 只有 `inbound.rs mod.rs policy.rs usage.rs`，無 `stream_lifecycle.rs` | REVIEW 既有「stream lifecycle 家族」延後條件 |
| `bba1e3d6` | 延後 | 上游 4 檔（+96/-44）：`variant_mapping.rs` 3.1 Pro Medium/High 由 `gemini-pro-agent` 改 `gemini-3.1-pro-high`；`claude.rs` 補 `resolve_dynamic_model_for_account`。fork：`variant_mapping.rs:323,612,618` 與 `model_mapping.rs:50,105` 仍用 `gemini-pro-agent`；`claude.rs` 無 `resolve_dynamic_model_for_account`（`gemini.rs:304`、`openai.rs:2243,4132,5354,5860` 有）。**本機 4 個帳號的 quota 清單既無 `gemini-pro-agent` 也無 `gemini-3.1-pro-*`**（僅 `gemini-2.5-pro`），改名對現有帳號無影響；日誌無 `No accounts available ... gemini-pro-agent` | 重開條件：帳號清單出現 `gemini-3.1-pro-high/low` 但無 `gemini-pro-agent`，或日誌出現該 503。先做 `7fcb1a2a` 的實測。若觸發，見 P8b |
| `7fcb1a2a` | 採用候選（待實測；僅 quota 解析部分） | 上游 16 檔（+207/-121）：`modules/quota.rs`(263) 重構、`GroupQuotaCard` 與 12 語系 key。fork `quota.rs:388` `if let Some(quota_info) = info.quota_info` 沒有 quotaInfo 就整個模型丟棄；`quota.rs:439` 已有 `fetch_quota_summary`。本機 PRO 帳號 `80c5da24`：群組 `Gemini Models` 有 `gemini-5h 0.74`／`gemini-weekly 0.82`，models 共 19 筆卻無 3.1 Pro | 見 P8a；先以 debug 日誌確認 `fetchAvailableModels` 是否回傳無 `quotaInfo` 的 Pro 模型。i18n 與 Argon 群組部分拒絕（fork 無 Argon、無 zh.json） |
| `33c6ab4a` | 拒絕 | 13 檔 locale＋`ApiProxy.tsx`；含 `zh.json`（fork 已刪），導覽順序純 UI 偏好 | 無 |
| `3ed27de4` | 拒絕 | `GroupQuotaCard.tsx`(33)、`zh.json`(4)；fork 無 `zh.json`，且為 Argon 外觀 | 無 |
| `304f445b` | 拒絕 | 32 檔 +1904：Mini 獨立視窗、`tray.rs`(293)、`lib.rs`(73)、`inbound.rs`(158) 含 Claude 拓撲防禦。fork 無獨立 mini 窗；`inbound.rs` 部分與 fork 的 `align_google_request_prefix_topology`（`inbound.rs:673`）簽名不同、無 `_with_model` | 拆出 `inbound.rs` 子項另案評估才可重開 |
| `08fc86e7` | 拒絕 | `quota.rs`(20)、`token_manager.rs`(68)、`AccountTable.tsx`：Argon 獨立配額組調度；fork 無 argon | 同 Argon 家族 |
| `e9c7ecd4` | 拒絕 | 15 檔：QoS 小黑屋剩餘時長／手動移除，依賴 `qos_guardian.rs`（fork 不存在，已 `ls` 確認） | 無 |
| `ec2272a7` | 拒絕 | `quota.rs`(52)、`token_manager.rs`(150)、`model_mapping.rs`(10)、3 個 UI：配額分組能力繼承（Argon 為主） | 無 |
| `5fcab3e5` | 拒絕 | 24 檔 +1310/-909：`model_specs.json`、`official_models.json`、`model_mapping.rs`(512)、QoS 排程、Argon 隔離、`cli_sync.rs`(251)。fork 無 QoS／Argon，路由表分歧大 | 重開條件：owner 明確要求跟進上游模型元資料整體架構；需分拆為獨立 packet |
| `fc75d8c1` | 拒絕 | `inbound.rs`(28)：修 `42523542` 的 borrow checker | 隨 `42523542` |
| `ee7b0484` | 拒絕 | rustfmt（`inbound.rs`、`thinking_store.rs`） | 無 |
| `42523542` | 延後 | `inbound.rs`(186)、`thinking_store.rs`(125)、`proxy_db.rs`(11)：Claude 全鏈 0 哨兵、跨模型切換簽名處理、哨兵升級 Base64 且不落庫、廢除 `<think>` 包裝。fork `thinking_store.rs:23` 仍為 `skip_thought_signature_validator` 字串常數，簽名生命週期自訂（REVIEW 既有多筆延後） | 重開條件：Claude 並行工具呼叫或 Gemini<->Claude 切換出現簽名 400；需跨模型切換簽名 fixture，且哨兵不落庫的回歸測試 |
| `d206e57e` | 採用候選 | 上游 `server.rs`(+37)。fork `admin_save_config`（`server.rs:1779-1861`）只熱更新 custom_mapping／upstream_proxy／security／experimental／proxy_pool 與全域 config；缺 `only_raw_quota_models`、`debug_logging`、UA、`capture_health_logs`、預設 client 重建、`sync_bindings_from_config`；桌面端 `commands/mod.rs:430-520` 已全做 | 見 P7 |
| `505deb09` | 採用候選 | 上游 `auth.rs`(+232/-35)。fork `auth.rs:120-137` 只取 header（`Authorization`、`x-api-key`、`x-goog-api-key`），無 query；`url = "2.5.7"` 已在 `Cargo.toml:39`；`monitor.rs:736/912` 以完整 URI 為 `url` 入庫、`ip_filter.rs:111` 亦記完整 URI | 見 P3 |
| `20df8616` | 拒絕 | `integration.rs`(131)、`version.rs`(240)、`account.rs`(21)：Linux 憑證與 CLI 版本探測；fork 為 Windows 原生 | 無 |
| `af8e9112` | 延後 | `useAccountStore.ts`(+41)：冷啟動快照水合避免空狀態閃現。fork `src/stores/useAccountStore.ts` 存在，本機僅 4 帳號，閃現不明顯 | 重開條件：帳號數量大、冷啟動可見空清單閃現；純前端，可 `npm run build` 本機驗 |
| `a58bd81d` | 延後 | `commands/mod.rs`(21)、`account.rs`(70)；依賴 `e933f63d` 的 `batch_import_accounts_logic`，另含 `list_accounts` 改 `try_read` | 隨 `e933f63d` |
| `c9f84b6c` | 拒絕 | `models/mod.rs`(3)：re-export 批次匯入結構，隨 `e933f63d` | 隨 `e933f63d` |
| `e933f63d` | 延後 | 上游 12 檔 +1037/-170：`batch_import_accounts_logic`（6 路並發＋抖動＋429 退避）、Axum `/accounts/batch-import`、CLI `import/export`、`refresh_all_quotas_logic` 改 `buffer_unordered`、`detect_active_clients_status` 降頻。fork：`refresh_all_quotas_logic`（`account.rs:2429`）用 `join_all`＋Semaphore(5)，無 `cli_handler.rs`；匯入路徑限制在 `server.rs:3295`（`ANTIGRAVITY_DB_IMPORT_DIR`）只針對 DB 路徑匯入，`batch-import` 接 JSON entries 不碰檔案路徑，本身不衝突；**CLI `import <file>` 會讀任意路徑並寫 `<file>-backup`，若移植須保持僅限本機 CLI、不得掛成遠端路由** | 重開條件：帳號數破百且實測記憶體暴增／匯入逾時；僅取 `refresh_all_quotas_logic` 單次寫索引一項較安全（獨立 packet）；本機僅 4 帳號，無症狀 |
| `30cae0d8` | 拒絕 | `handlers/common.rs`(16) rustfmt | 無 |
| `b917ff01` | 拒絕 | 28 檔 +1386：QoS 自適應排程、`qos_guardian.rs`(723)；fork 無 | 無 |
| `16d31898` | 採用候選（僅 Claude 429 外洩；多日配額快換延後） | 上游 `claude.rs`(9)、`common.rs`(85)。fork：`FailureStatusTracker` 在 `common.rs:120-142`，`gemini.rs:183,271,412,535,834,1085`、`openai.rs:2131,...,3054` 已使用；`claude.rs` 無 `failure_statuses`，最終狀態取 `last_status`（`claude.rs:2039-2052,2086-2099`）。多日 quotaResetDelay 硬配額（`common.rs:223`）fork 仍要求 `parsed_delay.is_none()`，但 pool>1 的第一輪本來就 50ms 換號，差異只在第二輪與 2 帳號池，本機 4 帳號影響小 | 見 P10；多日配額項重開條件：2 帳號池或單帳號出現長延遲仍等待 12s／30s |
| `c8ae4224` | 拒絕 | `release.yml`(+1)：preview 的 `removeArtifacts: true`。fork `release.yml:289-291` 為統一發版 job，且 preview 極少 | 重開條件：prerelease 資產數逼近 GitHub 上限 |
| `2560efcd` | 拒絕 | tray 的 import 路徑修正與全面 fmt（`monitor.rs`(80)、`openai.rs`(36) 等）；母 commit `6d4f1013` 未採 | 隨 `6d4f1013` |
| `6d4f1013` | 拒絕 | 25 檔：跨平台托盤配額感知（`tray.rs`(202)、`MiniView.tsx`），含 Homebrew Cask 與版本 bump；fork `tray.rs:281-292` 已有自己的托盤配額邏輯，且不要 Cask | 無 |
| `cae8fb9b` | 採用候選（僅第 3 項）；其餘拒絕 | 上游 8 檔：①Linux 行程自識別（`process.rs`(122)）②Linux Wayland 顯示守護（`linux_display_guard.rs`(196)，fork 無）③`inbound.rs`(160)＋`policy.rs`(20)＋`claude.rs`／`openai.rs` 強制 tool_choice 與 thinking 衝突自癒。fork：`openai/request.rs:1383-1394` 將 `required` 與物件形式映射為 `ANY`；`inbound.rs:763` 保留 `ANY`；全專案 grep 無任何「ANY 時移除 thinkingConfig」；`claude/request.rs` 完全未處理 `tool_choice` | 見 P9。①②Linux 專用拒絕 |
| `f693015a` | 延後 | `monitor.rs`(+441)、`config.rs`(4)：第三方閘道（NewAPI/OneAPI）探活請求在 `capture_health_logs` 關閉時過濾。fork `monitor.rs:63,854` 已有 `capture_health_logs` 開關（只處理 `GET /health`） | 重開條件：fork 被 NewAPI 類閘道探測並造成監控面板刷屏；需成功探活被過濾、失敗仍記錄的 fixture |
| `a1d7a655` | 拒絕 | `prompt_sanitizer.rs`(6)：把不支援的 regex lookahead `(?=\d)` 改為 `\d+\.\d+`。fork `prompt_sanitizer.rs` grep 無 `(?=`／`(?!`，且身份歸一正規式為舊版，未引入該 lookahead | 無缺陷，無需處理 |
| `aff0a9ef` | 拒絕 | `IDENTITY_FAST_MARKERS` 型別修正，母 commit `63cbbb42` 未採 | 隨 `63cbbb42` |
| `63cbbb42` | 延後 | `prompt_sanitizer.rs`(+288)＋版本 bump：多客戶端身份歸一、掃描窗擴至 8 句／1500 字元、框架白名單（JeikCode）。fork 的 sanitizer 有自己的防誤殺註解與測試（`prompt_sanitizer.rs:94-110`），白名單針對上游專屬名稱 | 重開條件：fork 需支援 OpenCode／Cursor 等身份歸一；需保留現有「`… agent, and the config was created by X` 不得整段誤刪」回歸，並加 33k 級提示詞的效能測試 |
| `54fd90b0` | 拒絕 | 全專案 `cargo fmt`（13 檔 +190/-101）；fork 有自己的 `dev_check.ps1` 與格式門禁，批量 fmt 會與 fork 分歧處產生衝突 | 無 |
| `03dab1bc` `a739ca07` | 拒絕 | 上游 CHANGELOG 文字 | 無 |
| `073d6a97` | 採用候選（僅 OpenAI Chat 串流收尾）；`common_utils` 半部分監看 | 上游：`openai/streaming.rs`(+118)、`common_utils.rs`(+218/-57)。fork：`create_openai_sse_stream_with_anchor`（`streaming.rs:73-376`）收尾（`:354-372`）只補 usage 與 `[DONE]`，無 `has_emitted_finish_reason`，上游若沒下發 finishReason 就沒有終態 chunk。`common_utils.rs:1628` `ensure_gemini_payload_ends_with_user` 是上游「暫時廢棄」之前的舊版（末尾 model 一律補 `.` user），已能避免 400，上游新增的「空尾輪剔除」「functionResponse 例外」非必要 | 見 P6。`common_utils` 半部分重開條件：客戶端送空 model 尾輪造成對話污染 |
| `37fc65d6` | 拒絕 | `lib.rs`(21)、Cask：CLI symlink；fork 無 CLI | 無 |
| `dc9e9c97` | 拒絕 | 17 檔 +1023：`cli_handler.rs`(293)、`agy` 帳號切換；fork `modules/` 無 `cli_handler.rs`（已確認） | 無 |
| `bec5c554` | 採用候選（低優先） | 上游 `warmup.rs`(+73/-23)。fork `warmup.rs:81-82,92,147,158,170,178` 全用 `req.model` 原名；`model_mapping.rs:42` 有 `gemini-3.8-flash -> gemini-3.8-flash-tiered` 映射，但 warmup 未經過 `resolve_model_route`。本機 quota 模型名為具名或 `-tiered`，現況不 404 | 見 P12 |
| `8a496c6c` | 延後 | `upstream/client.rs`(99)、`token_manager.rs`(54)：看門狗標頭逾時收斂 30s、慢端點降級、終態失敗 30s 冷卻。fork 逾時契約不同（REVIEW 批次 0.1 已延後同類） | 重開條件：實測出現 85s~140s 壅塞掛起；需 fake-clock 端點降級 fixture |
| `ef5a8751` | 延後 | `client.rs`(+183)：動態看門狗預算與容量耗盡快探；同上 | 同 `8a496c6c` |
| `c8c80083` | 延後 | `openai.rs`(+136)、`auth.rs`(10)、`server.rs`(6)：新增 `/props`、`/version`（llama.cpp／vLLM 探測，Hermes 類客戶端用），`/v1/models` 補 context_length 欄位並把探測路徑視為免鑑權。fork `server.rs` 無 `/props`／`/version`，也無 `OfficialModelCatalog`，`handle_list_models`（`openai.rs:5004-5027`）不帶長度欄位 | 重開條件：有客戶端（如 Hermes）指向 fork 且因 404 退化；移植時需用 fork 的 model_specs 自行解析 n_ctx，免鑑權路徑只回靜態版本字串 |
| `47a0f182` | 採用候選（防 Direct 洩漏與密文護欄）；其餘監看 | 上游 `proxy_pool.rs`(+180/-20)。fork：`proxy_pool.rs:248-250` `get_bound_proxy` 在 `auto_failover && !is_healthy` 回 `Ok(None)`；`get_proxy_for_account:206-235` 接著走公用池（已排除綁定代理，`:268-284`），池空則 `Ok(None)`；`upstream/client.rs:253-292` 對 `Ok(None)`／`Err` 都回預設 client＝直連。`build_proxy_config:382-393` 僅在 `parsed_auth.is_some()` 時攔 `ag_enc_`，否則 `basic_auth(&auth.username, &auth.password)` 把密文送給代理。`set_username`／`set_password` 順序（`:362-363`）：url 2.5 兩種順序輸出應相同，上游宣稱的缺陷未證實；health check 逾時放寬（`:594` 只有 `.timeout(10s)`）屬調參 | 見 P2 |
| `a5ddccd3` | 拒絕 | `pipeline/estimator.rs`(+42)；fork 無 `estimator.rs`（已確認） | 無 |
| `76327e93` | 拒絕 | `session_manager.rs` 純 fmt，隨 `f86c2485` | 隨 `f86c2485` |
| `61c81822` | 已存在（countTokens 信封）；預算繼承拒絕 | 上游：`inbound.rs` 新 `align_count_tokens_envelope`、`client.rs` countTokens 分支、`model_specs.rs` 的 `.or(tb_config.effort)`。fork：`gemini.rs:1230-1241` `execute_count_tokens` 已只送 `{"request": ...}` 並移除 `safetySettings`（註解「已驗證」），且 fork 無 `align_official_envelope`，無信封污染路徑。`ThinkingBudgetConfig.effort`（`config.rs:572`）在 fork 是舊版相容欄位，前端 `ThinkingBudget.tsx` 無 UI 可設，`model_specs.rs` 無 4 處 `tb_config.effort` 回退 | 提交說明自述為 AI 靜態審計、未經生產驗證。預算部分重開條件：前端新增全域 effort 設定入口 |
| `74f3a9fd` | 延後 | `claude/request.rs`(73)、`client.rs`(59)：孤兒 tool_result 函式名由 `tool_use_id` 改 `"unknown"`。fork `claude/request.rs:1456-1460` 仍 `unwrap_or_else(\|\| tool_use_id.clone())`，但上游改法依賴其 pipeline 的「unknown 降級為文字」，fork 無該降級，貿然改名可能同樣 400；提交說明自述未經生產驗證 | 重開條件：實際捕獲 Vertex 400 且根因為孤兒 tool_result；需 fork 自有降級邏輯與測試 |
| `f86c2485` | 採用候選 | 上游 `session_manager.rs`(+351/-64)。fork 三項缺陷皆在：①`clean_text.len() >= 2` 於 `session_manager.rs:83,182,286`；②兜底用 `format!("{:?}", msg.content)`（`:94,107,193,205`）與 `request.to_string()`（`:297`），多模態 base64 整包序列化；③`sanitize_user_text_for_fingerprint` `[System` 無閉合時 `s.truncate(start)`（`:34-41`）。Gemini 路徑尤其嚴重：首則 user 為單字元或純圖片時，兜底雜湊整個 body，每輪指紋都變 | 見 P5 |
| `158e8e9b` | 拒絕 | `openai.rs`(32)：補 `acquire_in_flight`（P2C 負載計數）與 `is_responses_api_path` 路徑別名。fork 無 `acquire_in_flight`、`is_responses_api_path`（grep 無） | 無 |
| `7ed0f10e` | 採用候選（第 1 段）；其餘延後 | 上游 9 檔：①`retry.rs`／`rate_limit.rs` 429 延遲解析 ②`token_manager.rs` 即時標記配額 0% ③`thinking_store.rs` 效能與 TTL ④`collector.rs` 思考合併、`gemini.rs` 輪次索引。fork：①`retry.rs:107-149` 標頭回傳優先，結構化次之，文字最後；`RE_TEXT_DELAY_PATTERNS`（`retry.rs:13`）無 `resets in`；`rate_limit.rs:484-486` Baseline 為 `header.or_else(body)`；`rate_limit.rs:745-` `details.first()`；`token_manager.rs:27-39` `classify_rate_limit_reason` 無 `individual quota`／`resets in`。③`thinking_store.rs:645` 內層迴圈仍每次 `normalize_ws(&rec.thought)`，`:130-135` 水位 TTL 被 `sessions.len() <= MAX_SESSIONS` 短路 | 見 P1；③效能與 TTL 解耦為可選小項（重開條件：長會話思考水合耗時可量測）；②④延後 |
| `aa6cd5b1` | 延後（一小項採用） | 上游 7 檔（+433/-74）：`auto_heal.rs`、`stream_lifecycle.rs`、`inbound.rs` 角色交替合併、工具宣告保序、`has_available_account`、`auto_switch.rs`、`proxy_db.rs`。fork：`proxy/pipeline/` 無 `auto_heal.rs`、`stream_lifecycle.rs`，無 `normalize_function_response_roles`，無 `auto_switch.rs`；`inbound.rs:700-736` 工具宣告是刻意「按 name 字典序排序」做前綴穩定（保序與其設計相反）；`has_available_account`（`token_manager.rs:2952`）在 fork 無任何呼叫端（grep），修了也無效；唯一對應缺陷是 `proxy_db.rs:956-970` `lookup_signature_by_thought_snippet` 用 `thought LIKE` 比對可能已 GZIP 的 BLOB（`proxy_db.rs:16,34-54` `MIN_GZIP_THOUGHT=384`），呼叫端 `monitor.rs:389` | 小項見 P13；其餘重開條件：fork 引入 auto-heal 或 stream lifecycle 時一併評估 |

---

### 實作方案（只規劃，未實作）

共通：所有 Rust 測試只能走 CI；本機只能做靜態檢查與前端 `npm run build`。提交前走 fork 規則 `powershell -File tools\dev_check.ps1`（本機若缺 MSVC 則該步驟由 CI 代跑）。每項為獨立 commit，並回註 REVIEW.md（採用 commit hash 與日期）。

#### P1  `7ed0f10e` 第 1 段：429 延遲解析（最高價值）

- 檔案與函式：
  - `src-tauri/src/proxy/upstream/retry.rs`
    - `RE_TEXT_DELAY_PATTERNS`（`:13` 附近）新增 `(?i)resets in ([^.,;\]\n]+)`。
    - `parse_retry_delay_with_source`（`:107-149`）：不要在標頭有值時提早 return。分別算 `header_delay`、`body_structured`（`extract_structured_delay_recursive`）、`body_text`（結構化為 None 才試文字），最後 `match (header, body)`：兩者皆有取 `raw_ms` 較大者；其餘取有值者。`source` 沿用勝出者。
  - `src-tauri/src/proxy/rate_limit.rs`
    - `Baseline` 分支（`:484-486`）：`header_sec` 與 `parse_retry_time_from_body_baseline` 取 max。
    - `parse_retry_time_from_body_baseline`（`:745-`）：`details.first()` 改為迭代全部 `details`，取第一個含 `metadata.quotaResetDelay` 的；文字備援補 `resets in`，正規式改用 `once_cell::Lazy<Regex>`（上游每次呼叫 `Regex::new`）。
    - `classify_reason` 內 `details` 迴圈（上游約 `:693`）：補 `@type` 含 `QuotaFailure` 判 `QuotaExhausted`（先確認 fork 對應位置）。
  - `src-tauri/src/proxy/token_manager.rs:27-39` `classify_rate_limit_reason`：`explicit_quota_exhausted` 加 `individual quota`、`resets in`。
  - 可選：`proxy/handlers/common.rs:223` 的硬配額判定加 `individual quota`（上游同改）。
- 測試（Rust，CI）：
  - `retry.rs`：本文 `"Individual quota reached. Resets in 1h19m6s."` 單獨解析為 4_746_000ms；加 `Some("8")` 標頭仍為 4_746_000；JSON `quotaResetDelay: "4746s"` 加標頭 8 亦同。
  - 保護既有行為：`retry.rs` 現有測試 `parse_retry_delay_with_source("", Some("3"))` 仍為 3200ms 實等待；標頭大於本文時取標頭。
  - `rate_limit.rs`：多 `details`（ErrorInfo 在 index 0，quotaResetDelay 在 index 1）與標頭 `8` 時 Baseline 回 3600。
  - 檢查 `rate_limit.rs:1085-1110` 既有 `current`／`baseline` 斷言（標頭 `None`），預期不受影響。
- 本機可測：否。
- 風險備註：「標頭 8s 為 Google 前門反 DDoS」是上游單方說法，作者在提交說明中自述未於生產驗證。取 max 的行為保守（只會延長鎖定），風險為帳號鎖定時間變長，可接受。

#### P2  `47a0f182`：專屬代理不洩漏直連、密文不外送

- 檔案與函式：`src-tauri/src/proxy/proxy_pool.rs`
  - `get_bound_proxy`（`:240-257`）：改為
    1. `entry.enabled && entry.is_healthy` -> 回綁定代理。
    2. 不健康且 `config.auto_failover` -> 先 `select_proxy_from_pool(config)`，成功則 `warn!` 後回該代理。
    3. 其餘一律仍回綁定代理（`warn!` 說明為防直連洩漏），不再回 `Ok(None)`。
  - `build_proxy_config`（`:348-410`）：`ag_enc_` 分支改為 `starts_with("ag_enc_")` 單獨判斷；有 `parsed_auth` 用 URL 憑證，否則 `warn!` 並不呼叫 `basic_auth`（不送密文）。
  - 可選：把 URL 解析抽成 `pub fn parse_and_clean_proxy_url(raw) -> (String, Option<(String,String)>)` 以利單測；`set_password`／`set_username` 的順序先不動，需在 CI 用測試證明差異才改。
  - 另注意：`upstream/client.rs:253-292` 對 `Err` 與 `build_client_with_proxy` 失敗仍回預設 client；若要真正封閉洩漏，專屬綁定帳號在這兩種情況應回錯誤而非直連。這是上游也沒處理的延伸缺口，建議另開 packet。
  - 決策點：不健康時改走公用池會讓專屬帳號換出口 IP；若希望「專屬帳號絕不換 IP」，則步驟 2 應省略，直接用綁定代理（較嚴格，但無容錯）。
- **決定（2026-10-26）：不採用，維持現行行為，不改程式。** 現行 `get_bound_proxy` 的兩種設定都不會讓專屬帳號靜默換 IP：`auto_failover` 關閉時，不論健康與否一律用綁定代理（不換 IP、不直連）；`auto_failover` 開啟時，使用者已明示接受容錯，不健康才改走公用池（`select_proxy_from_pool` 已排除所有被綁定的代理，不會借用別帳號的專屬 IP）。上游改法會讓「關閉故障轉移」的帳號也換 IP，與設定語意相反。已知殘留缺口：`auto_failover` 開啟、綁定代理不健康且公用池為空時，`Ok(None)` 會讓 `upstream/client.rs` 回預設 client（直連）；這是使用者開啟容錯後的邊緣情況，且上游也未處理。觸發重開：維護者要求「專屬帳號絕不直連」，屆時於 `upstream/client.rs` 對綁定帳號的 `Ok(None)`／`Err` 回錯誤而非直連，並移植上游 4 個測試。
- 測試（CI）：上游 4 個測試可直接移植（`bound_account_prevents_direct_leak_when_unhealthy`、`bound_account_fails_over_to_healthy_pool_proxy`、`build_proxy_config_blocks_ciphertext_when_no_url_auth`、URL 清洗兩個）；fork 現有 `test_build_proxy_config_fallback_to_url_auth_on_decrypt_failure`（`proxy_pool.rs:~705`）要保持綠燈。
- 本機可測：否。

#### P3  `505deb09`：Gemini `?key=`（含日誌去敏）

- 檔案與函式：
  - `src-tauri/src/proxy/middleware/auth.rs`
    - 新增 `pub fn extract_api_key<B>(request: &Request<B>) -> Option<Cow<'_, str>>`：順序 Authorization（`Bearer`／`bearer`／裸 token）、`x-api-key`、`x-goog-api-key`、query `key`（`url::form_urlencoded::parse`）。fork 專屬：函式加參數 `allow_query: bool`，管理端點（`force_strict == true`）傳 `false`，避免 admin 密碼進 URL。
    - 取代 `auth.rs:71-81`（`ProxyAuthMode::Off` 的 UserToken 辨識）與 `:120-137` 兩段重複的 header 取值；`:155-170` 之後的 `api_key.map(...)` 改 `api_key.as_deref()`，`api_key.unwrap()` 改 `as_deref().unwrap()`。
  - `src-tauri/src/proxy/middleware/monitor.rs:736,912` 與 `ip_filter.rs:111`：記錄 `url`／`path` 前先去敏，新增小函式 `redact_query_secret(uri: &str) -> String`，把 `key=`、`api_key=` 的值換成 `***`（可放 `proxy/middleware/mod.rs`）。注意 `monitor.rs:773-775` 依 URI 解析模型名（`/v1beta/models/xxx:...`），去敏後的字串只用於入庫，解析仍用原字串。
- 測試（CI）：
  - `auth.rs` 目前無 `mod tests`，新增：移植上游 `test_extract_api_key_sources`（9 種來源與優先序）；`allow_query=false` 時 `?key=` 回 None；`test_auth_middleware_with_gemini_query_key`（正確 key 200、錯 key 403、無 key 401，用 `tower::ServiceExt::oneshot`，fork 已有 `tower = 0.4 util`）。
  - 新增 `redact_query_secret` 單測：`/v1beta/models/x:streamGenerateContent?alt=sse&key=abc` -> `...&key=***`。
- 本機可測：否。
- 備註：本機 `proxy_logs.db` 目前 `url LIKE '%key=%'` 為 0 筆，修前無歷史洩漏。

#### P5  `f86c2485`：會話指紋

- 檔案：`src-tauri/src/proxy/session_manager.rs`
- 改法：
  1. 新增三個摘要函式（Claude／OpenAI／Gemini parts）：文字原樣；`image`／`document`／`inlineData` 只取 `[type:mime:長度]`；`tool_result` 取 `[tool_result:id:is_error]`。fork 的 `ContentBlock::{Text, Image{source}, Document{source}, ToolResult{..}}` 與 `OpenAIContentBlock::{Text, ImageUrl, AudioUrl, InputAudio, VideoUrl}` 皆存在（`claude/models.rs:96-136`、`openai/models.rs:143-156`），欄位名需對齊（`ImageSource.media_type`／`data` 先讀 `claude/models.rs:162` 確認）。
  2. `:83,182,286` 的 `clean_text.len() >= 2` 改 `!clean_text.trim().is_empty()`。
  3. 兜底：Claude／OpenAI 的 `format!("{:?}", ...)`（`:94,107,193,205`）改用摘要；Gemini 的 `request.to_string()`（`:297`）改為「首個 user 的 parts 摘要 -> 最後一個 content 的 parts 摘要」兩段兜底。
  4. `sanitize_user_text_for_fingerprint`（`:34-41`）：只在 `[System` 後緊接 `:`／空白／`]` 時才視為標籤；找不到 `]` 時不截斷，僅推進搜尋起點。
  5. 注意指紋改變會讓單字元／純圖片會話的既有 session_id 變動一次（thinking_store 命中重置一次），可接受。
- 測試（CI）：移植 `test_unclosed_system_tag_and_code_preserved`（`[System.out.println(...)]` 不被刪）、`test_single_char_input_multi_turn_stability_all_protocols`（三協定第 1 輪與第 2 輪同 session_id）；另加「同一張大圖 base64 + 不同後續文字，指紋一致且不序列化整包」。fork 既有 `test_sanitize_user_text_for_fingerprint`（`session_manager.rs:347`）要確認仍綠。
- 本機可測：否。

#### P6  `073d6a97`：OpenAI Chat 串流終態 finish_reason

- **決定（2026-10-26）：採保守版，已實作。** 串流收尾時若未曾送出 finish_reason 且已收到 usageMetadata（Gemini 只在最後一幀附 usage，代表回應其實完成、只是結束標記遺失），補一個 finish_reason chunk：有工具呼叫為 `tool_calls`，否則 `stop`。沒有 usage 則不補，維持未完成，不把中斷偽裝成正常結束（與 Responses 串流的 `upstream_interrupted` 一致）。未採用上游的「一律補」與 `has_content` 條件。測試：`streaming.rs` 的 `task_eof_*` 五個，CI 步驟「Test OpenAI stream EOF finish_reason」。

- 檔案：`src-tauri/src/proxy/mappers/openai/streaming.rs`，函式 `create_openai_sse_stream_with_anchor`（`:73-376`）。
- 改法：
  1. 迴圈外新增 `let mut has_emitted_finish_reason = false; let mut has_content = false;`（`:94` 附近與 `emitted_tool_calls` 並列）。
  2. 解析 `finishReason`（`:246`）補 `.or_else(|| candidate.get("finish_reason"))`。
  3. `:281-299` 發出內容 chunk 時：`!content_out.is_empty()` 設 `has_content = true`；`finish_reason.is_some()` 設 `has_emitted_finish_reason = true`。
  4. `:354` `thinking_acc.commit` 之後、`if include_usage` 之前，`if !error_occurred && !has_emitted_finish_reason && (has_content || !emitted_tool_calls.is_empty())` 補發 `{"delta":{},"finish_reason": tool_calls|stop}`；`!include_usage` 時把 `final_usage.take()` 附在該 chunk（與 `:292-297` 一致）。
  5. 決策點（建議）：fork 的 Responses 串流刻意把「無 finishReason」標成 `incomplete`＋`upstream_interrupted`（`streaming.rs:1046-1090`）。為避免 Chat 這邊把中斷偽裝成 `stop`，建議加條件 `final_usage.is_some()`（收到 usageMetadata 才視為正常 EOF）；無 usage 時維持現狀不補。這與上游行為不完全一致，需決策者定奪。
- 測試（CI）：移植 `test_openai_stream_emits_fallback_finish_reason_when_upstream_omits_it`（文字 chunk＋usage-only chunk，斷言 `["stop"]` 與 `[DONE]`）；補工具呼叫案例斷言 `tool_calls`；補「純文字無 usage、無 finishReason 時不補」（若採決策點 5）；既有測試 `streaming.rs:~1920`（`test-malformed-session`，斷言 `final_finish_reason == "stop"`）要仍綠。
- 本機可測：否。`legacy` completions 串流（`create_legacy_sse_stream`，`:378`）上游也沒處理，不在本案。

#### P7  `d206e57e`：Web 模式儲存設定熱更新

- 檔案：`src-tauri/src/proxy/server.rs`，函式 `admin_save_config`（`:1779-1861`）。
- 在現有各段之後補（`AppState` 欄位皆已存在：`only_raw_quota_models`、`debug_logging`、`upstream`、`monitor`、`proxy_pool_manager`、`token_manager`，見 `server.rs:89-111`）：
  1. `*state.only_raw_quota_models.write().await = new_config.proxy.only_raw_quota_models;`
  2. `*state.debug_logging.write().await = new_config.proxy.debug_logging.clone();`
  3. `upstream_proxy` 段後：`state.upstream.rebuild_default_client(Some(new_config.proxy.upstream_proxy.clone())).await; state.upstream.clear_client_cache();`（`client.rs:133,164` 已有；上游多呼叫的 `utils::http::invalidate_shared_clients` 在 fork 不存在，略過）。
  4. `proxy_pool` 段後：`state.proxy_pool_manager.sync_bindings_from_config().await; state.upstream.clear_client_cache();`
  5. `state.upstream.set_user_agent_override(new_config.proxy.user_agent_override.clone()).await;`
  6. `state.monitor.set_capture_health_logs(new_config.proxy.capture_health_logs);`
  7. fork 額外（上游也漏）：`state.token_manager.update_circuit_breaker_config(new_config.circuit_breaker.clone()).await;`（桌面端 `commands/mod.rs:~505` 有做）。
  8. 上游還同步 `hub_version` 指紋（`update_hub_fingerprint`）；fork 無 `hub_version` 模組，略過。
- 測試：`admin_save_config` 依賴 `AppState` 全量建構，不易單測。建議抽成 `async fn apply_hot_reload(state: &AppState, cfg: &AppConfig)` 讓桌面端與 Web 端共用，並以最小 AppState（若現有測試輔助可用）斷言 `only_raw_quota_models` 等欄位被更新；否則以 CI 編譯＋手動 Web 模式驗證。
- 本機可測：否。

#### P8  `7fcb1a2a`／`bba1e3d6`（Issue #3737 兩半，需先實測）

- P8a（`quota.rs`）：
  - 先驗證：在 `fetch_quota_with_cache`（`quota.rs:~381`）臨時以 debug 日誌印出被丟棄的 `(name, has_quota_info)`，對本機 PRO 帳號跑一次配額刷新，確認 `gemini-3.1-pro-*` 是否以「無 quotaInfo」形式出現。若上游本來就沒回傳，則本案不成立。
  - 改法（確認後）：把 `fetch_quota_summary`（`:439`）移到模型迴圈前；迴圈對符合前綴篩選（`gemini`／`claude`／`gpt`／`image`／`imagen`，`:398-402`）的模型一律保留：`model_pct`（有 quotaInfo 才有）與 `group_pct`（用現有 `:444-510` 的群組比對邏輯抽成 `fn pick_group_bucket(name, groups) -> Option<(i32, String)>`）取較嚴者，模型級明確 0% 不被正數群組覆蓋；兩者皆無時記 100%、reset 空字串。
  - 風險：上游靠 `is_model_compliant_with_baseline`＋官方模型清單避免內部遺留項（如 `gemini-3-flash-agent`、`chat_*`）混入；fork 無此過濾，放寬 quotaInfo 門檻後這些條目可能出現在 UI／調度器。因此必須同步加白名單（沿用現有名稱前綴再加官方清單），不能只拿掉 `if let Some(quota_info)`。
  - 測試：抽出純函式 `build_model_quotas(models, groups, filter)`，覆蓋「有 quotaInfo／無 quotaInfo 取群組／兩者皆無／內部條目被濾掉／模型級 0% 不被覆蓋」。CI。
- P8b（`bba1e3d6`，P8a 確認 3.1 Pro 在清單中後才做）：`variant_mapping.rs` 新增 `SPEC_31_PRO_HIGH{id:"gemini-3.1-pro-high", thinking_budget:10001, max_output_tokens:65535}`，3.1 Pro 的 Medium／High 改用之；`SPEC_PRO_AGENT` 保留並 `#[allow(dead_code)]`；同步更新 `variant_mapping.rs:656-662,846-901,1000,1019-1086` 的期望值（共十餘處 `gemini-pro-agent`）；`claude.rs` 在選號後補 `token_manager.resolve_dynamic_model_for_account(&account_id, &mapped_model).await`（選號處約 `claude.rs:1125-1180`，與 `gemini.rs:304` 對稱）。另外 `model_mapping.rs:50,105` 的 `gemini-3.1-pro-high -> gemini-pro-agent` 映射上游未動，需另行決定。CI。
- 本機證據：`resolve_dynamic_model_for_account`（`token_manager.rs:1123`）本來就會在帳號可用模型含 `gemini-3.1-pro-high` 時改寫，故補 Claude 對稱呼叫比改名更安全，可單獨先做。

#### P9  `cae8fb9b` 第 3 項：強制 tool_choice 與 Claude thinking 衝突

- 檔案：`src-tauri/src/proxy/mappers/openai/request.rs`（`:1383-1394` 決定 mode，`:1483` 呼叫 pipeline）。
- 改法（最小範圍）：在 `align_google_request_prefix_topology(&mut inner_request)` 之前，若目標模型字串含 `claude` 且 `inner_request["toolConfig"]["functionCallingConfig"]["mode"] == "ANY"`，則移除 `inner_request["generationConfig"]["thinkingConfig"]`（`tracing::warn!` 一行）。fork 的 pipeline 函式沒有 `target_model` 參數，所以放在 mapper 內比改 pipeline 簽名小。Claude 原生協定路徑（`claude/request.rs`）fork 根本未處理 `tool_choice`，不受影響，不需動。
- 測試（CI）：Claude 模型＋`tool_choice:"required"`＋thinking 開啟 -> 結果無 `thinkingConfig`、`toolConfig` 仍為 ANY；Gemini 模型同樣輸入 -> 保留 `thinkingConfig`；`tool_choice:"auto"` -> 不動。
- 備註：此為 Anthropic 公開規則，但 Antigravity 上游的 Claude 端點是否嚴格套用未驗證；上游另有「遇衝突 400 就地移除重試」的自癒，fork 先不做。
- 本機可測：否。

#### P10  `16d31898`：Claude handler 429 外洩

- 檔案：`src-tauri/src/proxy/handlers/claude.rs`
- 改法：`handle_messages` 在 `last_status`（`:903`）旁新增 `let mut failure_statuses = FailureStatusTracker::default();`（`gemini.rs:16` 的 import 寫法可參照）。在所有設定 `last_status = status` 處（`:1383`、`:1722`）補 `failure_statuses.record(status)`；上游呼叫 `Err` 分支與全域預算逾時處補 `record(StatusCode::GATEWAY_TIMEOUT)`（fork 對應位置需讀 `claude.rs:1700-1760` 確認哪些路徑有逾時／網路錯誤）。收尾兩處（`:2039-2052`、`:2086-2099`）把 `last_status` 換成 `failure_statuses.final_status()`，保留原 403 -> 503 的特例。
- 測試（CI）：`common.rs:375-390` 已有 tracker 單測；Claude handler 為整合邏輯，建議把「最終狀態選擇」抽成純函式後測：`[429, 504] -> 504`、`[429, 429] -> 429`、`[403] -> 503`。
- 本機可測：否。

#### P11  `e18d31fc`：輕量模式重建主視窗黑屏

- 檔案：`src-tauri/src/modules/lightweight.rs`、`src/components/common/ThemeManager.tsx`
- 改法：
  1. `exit_lightweight_mode`（`lightweight.rs:67`）在 `ensure_main_window` 前記 `let already_running = app.get_webview_window("main").is_some();`，只有 `already_running` 時才 `show()/unminimize()/set_focus()`；新建視窗保持 `visible:false`，由前端在首幀後顯示。
  2. `ThemeManager.tsx:17`（`await getCurrentWindow().show()` 之後）補 `await win.unminimize().catch(()=>{}); await win.setFocus().catch(()=>{});`（fork 無 `mini` 標籤，不需要上游的 `win.label !== 'mini'` 判斷）。
  3. 風險：若前端載入失敗，新建視窗永遠不顯示。加保底：Rust 端對新建視窗啟動 `tokio::time::sleep(3s)` 後仍未 visible 則強制 `show()`。
- 驗證：TSX 部分 `npm run build` 可本機；視覺需實機（切到輕量模式再從托盤叫出，Windows 11）。Rust 部分 CI。

#### P12  `bec5c554`：warmup 先解析路由

- 檔案：`src-tauri/src/proxy/handlers/warmup.rs`（`handle_warmup`，`:81-178`）
- 改法：函式開頭 `let mapped_model = crate::proxy::common::model_mapping::resolve_model_route(&req.model, &*state.custom_mapping.read().await);`（fork 的簽名是兩參數，`model_mapping.rs:269`；上游用的 `resolve_model_route_with_effort` 在 fork 不存在）；之後 `is_claude`／`is_image` 同時判 `mapped_model` 與 `req.model`，請求體 `model` 與 `wrap_request` 第三參數、`prefer_non_stream` 判斷、日誌 `mapped_model`、`X-Mapped-Model` 皆改用 `mapped_model`。`state.custom_mapping` 為 `AppState` 欄位（`gemini.rs:189` 同樣用法）。
- 測試（CI）：`resolve_model_route("gemini-3.8-flash", {})` 回 `gemini-3.8-flash-tiered`（`model_mapping.rs:42` 的映射），具名模型不變，自訂映射優先。
- 本機可測：否。

#### P13  `aa6cd5b1` 小項：思考簽名片段查詢（GZIP）

- 檔案：`src-tauri/src/modules/proxy_db.rs`，`lookup_signature_by_thought_snippet`（`:956-970`）。
- 改法：改為 `SELECT id, thought, signature FROM thinking_records WHERE signature IS NOT NULL ORDER BY id DESC LIMIT 50`，逐筆 `unpack_thought(&bytes)`（`:54`）後 `contains(clean)`，命中即回傳；`thought` 欄位以 `Vec<u8>` 讀取。fork 的版本沒有 `normalize_and_heal_signature` 的回寫，直接回 `signature`。
- 測試（CI）：移植 `test_lookup_signature_by_thought_snippet_gzip_compressed`（`>= MIN_GZIP_THOUGHT` 的長思考，片段命中回簽名、不存在回 None）。fork 測試輔助名稱（`TestDataDir`、`save_thinking_record` 簽名）需先讀 `proxy_db.rs` 的 `thinking_sqlite_tests`。
- 本機可測：否。價值低，可併入下一個 proxy_db 相關 commit。

#### 補充：`d6974bc3`（REVIEW 已列採用候選）的精確修法（P4）

- 檔案與函式：
  1. `src-tauri/src/modules/account.rs`，`refresh_all_quotas_logic` 的 `.filter`（`:2449-2463`）：只在「`q.is_forbidden && account.validation_blocked && now < account.validation_blocked_until.unwrap_or(0)`」時跳過；`is_forbidden` 但封鎖已到期（或 `validation_blocked == false`）一律放行刷新並 `log_info("Retrying ... 重新探測")`。注意 fork 的 `validation_blocked_until` 是 `Option<i64>`（`models/account.rs:59`）。
  2. `src-tauri/src/proxy/token_manager.rs`，`load_single_account` 的「Block expired」分支（`:557-575`）：在記憶體 JSON 與 `update_account_json` 閉包兩處都加 `quota.is_forbidden = false`（`account.get_mut("quota").and_then(|v| v.as_object_mut())`）。
  3. 現況成因確認：fork 的 `is_forbidden` 寫入點有兩處——配額抓取遇 403（`account.rs:2367`，直接存回）與 `mark_account_forbidden`（`account.rs:2096-2125`，同時 `proxy_disabled = true`）；`token_manager.rs:465-476` 把 `is_forbidden` 視為停用。因此清除 `is_forbidden` 只解除「刷新被跳過」，不會自動恢復 `proxy_disabled`；是否一併清除需另行決定（建議不清，由成功刷新後使用者／既有恢復邏輯處理）。
- 測試（CI）：把 filter 抽成 `fn should_skip_forbidden_in_batch_refresh(account: &Account, now: i64) -> bool` 後單測：封鎖中跳過、已到期放行、`validation_blocked=false` 但 `is_forbidden=true` 放行。擴充 `token_manager.rs:~4560` 既有「expired 清除」測試，斷言 `updated["quota"]["is_forbidden"] == false`。
- 本機可測：否。本機目前無任何帳號處於封鎖狀態（4 個帳號 `is_forbidden=0`、`validation_blocked=0`），無法重現，修前先靠 fake clock 單測。

---

### 附：未採用但可能被問到的點

- `e933f63d`／`a58bd81d` 與匯入路徑限制：`ANTIGRAVITY_DB_IMPORT_DIR` 只約束 `admin_import_custom_db`（`server.rs:3295-3301`）。批次匯入 API 收 JSON 的 `{email, refresh_token}`，不碰路徑，所以在 API 層面不衝突；真正的衝突點是 CLI `import <file>` 會讀取任意路徑並在旁邊寫 `-backup`，以及批次匯入把大量 refresh token 透過管理端點寫入（需維持現有管理端鑑權）。若日後採用，CLI 與遠端路由必須分開，遠端不得接受路徑參數。
- `61c81822` 與 `74f3a9fd`、`158e8e9b` 三筆的提交說明皆自述為「AI 靜態審計發現、未經生產驗證」；`aa6cd5b1`、`f86c2485` 同為 AI 協作產物。採用時請保留這項風險註記，先寫能重現缺陷的失敗測試再改碼。
- 本機證據的限制：4 個帳號、5 筆日誌，只能證明「程式碼缺陷存在」與「某些症狀吻合」（P8），不能證明「已發生」。

---

## 0. 2026-10-21 `v4.9.8` release 逐批覆核

- **範圍與界線**：仍只覆核 `b601f5e7375343343922ccab5e50653bc3450c3b..9f8e4f24d61b33fa80b8078d104c942752a5fa0b` 的 release commit；首個歷史批次閱讀 12 個實際 patch（merge 一律以第一父差異計），其後批次已完成全部 162 筆覆核；沒有移植任何產品碼。完整 SHA、檔案清單、第一父基底與可重現 patch blob SHA-1 保存於 `checkpoint-tailwind4-braces/upstream-v498-release-review/coverage.json`。
- **coverage 校正**：舊 ledger 的 15 個 exact 映射有兩筆誤算：`443a162…` 是含 rate-limit ceil／token manager／P2C spillover 的 combined merge，不是 PR #3662 head；`1ef91e8…` 是 beta.7 release metadata，不是 PR #3687。兩者改為本批實讀。PR #3687 正確映射為 `4f3e1f90168f57179c30a9e9dd0e272ecd093161`（saved patch-id `9fbba6d4e52dd8869a3f23e6b27549e4cdf9b2d5`），判定仍為延後，僅表示已審閱，不表示採用。
- **目前 coverage（本輪候選）**：148 筆實讀、14 筆既有 exact-patch 映射，合計 **162/162**；`v4.9.8` 的每一筆 release commit 均已有實讀判讀或精準同 patch 映射。下方逐批舊計數是歷史停點，保留其可重現證據，不能取代此總結。
- **目前 tickets（本輪候選）**：PR #3708／#3714 與 Issue #3709／#3710／#3711／#3713／#3715–#3720 共 12 項均已逐項判讀。此處的「reviewed」只代表治理覆核，**不表示產品採用、合併或已修復**；未實作候選、延後與既有缺口仍明列於各表。

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `443a1627623b6c5138e1162f02c2f2e09c7d7757` | 延後 | 第一父 patch 跨 Claude/Gemini/OpenAI handler、rate limit、token manager 與設定；加入毫秒 ceil、P2C spillover。fork 沒有對等多帳號排程契約。需 fake-clock、in-flight spillover、帳號隔離／不重試 fixture。 |
| `1ef91e8240963e05ccec1ea03ece8452ecfd50c0` | 拒絕 | beta.7 release/version、lockfile、Tauri 設定與 UI metadata；非 fork release packet。僅在 owner 獨立 release packet 重開。 |
| `60311656b99bfdfcaf0ecd155a40090907eed888` | 延後 | Claude compaction 的 signature clear 與 thinking sentinel；fork 的 session/thinking lifecycle 自訂。需 Claude/Gemini compaction continuation、cache-clear、signature-isolation fixture。 |
| `0936a9b1979079f57c7d91a6bfce26d614a79b06` | 延後 | 實驗性 global compaction/signature strip、server/config/UI 串接；不能以靜態相似採用。需多協定 compaction、false-positive fold marker、signature race 與設定持久化驗收。 |
| `06f280c73b1c38255ed63fd6221c6ccd4c84085e` | 延後 | #3615 merge 第一父 patch 涵蓋 Cowork estimator、DB、session/signature lifecycle 及 compaction；需完整 client-visible lifecycle state-machine acceptance。 |
| `4a8b5589025f5a58cd9fccb4d5c4ce3c4df9555e` | 拒絕 | beta.1 release/version metadata；只由獨立 owner release packet 評估。 |
| `86faa82a68c4adf8484c75aa6bd6587cde75e580` | 延後 | fork 實際支援 en、zh-TW 與同一批非 zh locale；但上游 `AgentSettings` 路徑不存在，且 fork 必須維持所有 `zh-* → zh-TW`。需先映射 AgentSettings/Dashboard keys，排除簡中又不產生 fallback regression，並跑 locale build／渲染 warning 驗收。 |
| `d5f5dc47d3d28e436c38103ee80a36ac834e1161` | 延後 | #3618 merge 第一父差異即前列 `86faa…` 的具體 AgentSettings/Dashboard localization patch；fork 的非 zh locale 本來就存在，不能以「多語不適用」拒絕。條件同前列 key/path、zh-TW migration 與渲染驗收。 |
| `635f7b37798e215e2d897d8a6f3be8f9aae97812` | 拒絕 | beta.2 release/version metadata；只由獨立 owner release packet 評估。 |
| `5dd6958ac06d1fd53fe97ef8047f2140d05dc4d6` | 延後 | i18n rapid-toggle 的 persistence queue／rollback，與 fork config store 的多語意義耦合。需 rapid toggle、failure/retry、waiter-order UI fixture。 |
| `12e74b95e871c614ea5f17728d1a7313b3e7bcfa` | 延後 | queue waiter／BCP47 normalization follow-up；依賴未採用的 rapid-toggle 改動。條件同前列 fixture。 |
| `6621a9490f2a6f9365d56f1504c6903bb8ceb1bd` | 監看 | #3620 merge 第一父差異即前列 rapid-switch patch；無獨立可採用內容。條件同前列 fixture。 |

### 0.1 第三批：stream lifecycle／model catalog 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `2312b9119c60d437e16111199622772692c144ab` | 拒絕 | beta.3 release/version metadata；只由獨立 owner release packet 評估。 |
| `9645ef9119a6aeb3be6227ff7d3061311ebdc284` | 延後 | Claude SSE 改為 3s heartbeat／45s idle，並改 error/retry/frame 收尾；fork 仍有 20s／100s 契約。需 no-content、thinking-only、content-emitted interruption fixture。 |
| `885c68b2cffe53d96221cf3c3574f376cb39b19a` | 延後 | Claude/OpenAI first-chunk peek、請求預算與輪替變更；fork 有不同 120/300s 路徑。需 fake heartbeat、timeout、rotation、cross-protocol retry-budget fixture。 |
| `0ba72459e1248826429b10479c1e52e418713a19` | 延後 | 以新 shared stream lifecycle 重寫兩個 streaming adapter；fork 無此 state machine。需 SSE frame ordering、尾端 buffer、first error、thinking interruption 與 session isolation acceptance。 |
| `2427cd13ecbb94e442609ac053c57452cdcad0c2` | 延後 | trailing-signature block/index 修補依賴未採 lifecycle；需 signature-only、signature→tool/text、normal-stop frame-order fixture。 |
| `58e145ef61ea2ff06a223a3741403cbc9f19072d` | 延後 | 180/180/45 三階段 timeout state machine 依賴未採 lifecycle，與 fork timeout 不同。需 fake-clock TTFT／thinking-transition／steady-stream／heartbeat fixture。 |
| `134cb0d3d5fc833ca65d587d923e1349658d6668` | 延後 | OpenAI transition timeout/probe 依賴同一 state machine。需 thinking→tool/content transition 的 idle 與 terminal-frame fixture。 |
| `9bdcb06abc817d90aa8da2d54462b9bdf6f95db6` | 延後 | timeout config/UI 暴露未採 lifecycle，且 patch 含簡中；fork 必須維持 `zh-* → zh-TW`。需 config migration/default/env/UI persistence 與 fake-clock acceptance。 |
| `f0248cd06cce4c9b083a7fa9f78b69e170a14656` | 延後 | 是未採 timeout settings 的九語文案 follow-up；fork 支援這些 locale，但需先決定 source keys。採用 `9bd…` 後才以 locale-key completeness build 重開。 |
| `c278086a88ae881d1501b2bc7c7f7272a8653f35` | 監看 | #3622 merge 第一父差異彙整本批已分別覆核的 lifecycle patches；merge 標題不當採用證明。 |
| `0412bbf91be29a93f3e1289e7a01180e4459b5c8` | 延後 | 動態 model catalog／alias 移除跨 model specs、CLI/OpenCode sync；靜態名稱不能證明 429 根因。需 offline discovery、alias canonicalization、stale-model suppression 與相容性 fixture。 |
| `35e127539c5f9b41c54c86f373fc7edb8213bd33` | 拒絕 | beta.4 release/version metadata；只由獨立 owner release packet 評估。 |

### 0.2 第四批：stream 透明錯誤／token stats／403 重新探測 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `9901c3979f418540cae6a6505161edeff46661ef` | 延後 | 改為透明傳遞中途 stream error、取消假收尾；fork error/frame 契約不同且依賴未採 lifecycle。需 Claude/OpenAI no-content、thinking-only、content interruption 的 SDK retry/frame fixture。 |
| `a6135ddcf1da81406b10c55d8deaaedb05c607d8` | 延後 | 將 steady-stream timeout 改 120s，假設上游 lifecycle phase；需 fake-clock large-output、heartbeat、cancel 驗收。 |
| `eeb64f01d6f508eaeec71c74fb824c6c99b73594` | 延後 | 將未採 lifecycle timeout 暴露至 OpenAI/UI/locale，且含簡中。需先有 lifecycle、config migration/env/UI persistence 與 zh-TW mapping acceptance。 |
| `8d2c4ae58bf3325fbbb1571b9c4d3156f859dba3` | 監看 | #3625 merge 第一父差異匯集已分別覆核的 lifecycle/config patches；merge 非採用證明。 |
| `d99198be01fec82264ce8add1f3a12010969c1d8` | 延後 | 改帳號輪替、outer timeout、error mapping 與 test dependency；fork selection safety 自訂。需 multi-account fake-clock、sticky escape、504/overloaded、無重複 request fixture。 |
| `7baff3d198e817efd452ab9d96e30ed9551b4d70` | 監看 | #3628 merge 第一父差異即前列 `d991…`；無獨立採用。 |
| `d8b6e3f90ee8cbd4700c75c876c6998b9f8537f8` | 拒絕 | 上游移除 model-turn inert user padding；fork 已保留並驗證 `.` 防禦。除非真 provider 契約證明無效且有替代 security regression，否則不移除。 |
| `1e4c8adfa899a4eebb08e0ff505b45a8fc9d75fa` | 拒絕 | beta.5 release/version metadata；只由獨立 owner release packet 評估。 |
| `eeb477542cbe7d4711b8bf983746911fbd3f717e` | 延後 | token stats 新增 all-time aggregation、91-day heatmap、pricing storage/UI；fork 有既有統計頁但無此 DB／隱私／價格契約。需 bounded SQLite、pricing persistence、responsive UI acceptance。 |
| `a7fbdb6c107194ec8089a38638f900e13e7e6b5d` | 延後 | token stats review follow-up，仍依賴未採 dashboard，並帶簡中。先有 source dashboard contract 才重開。 |
| `2d706ad412952976c5e76fe891929766f9786d69` | 監看 | #3616 merge 第一父差異匯集上述 token stats patch；無獨立採用。 |
| `d6974bc3b9a37477c9d1d4aa9906c24cefffc288` | 採用候選 | fork 有相同 `validation_blocked` 到期與 `is_forbidden` 路徑，可能讓過期 403 永久自鎖。另立帳號狀態批次，以 fake clock 證明到期只重探一次／清除 forbidden，且手動／proxy disable 不被放行。 |

### 0.3 第五批：validation URL／UI motion／stream retry 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `6e562cdb6b8b32c9978d8588cd33a18bc439dff3` | 採用候選 | fork 已持久化 `validation_url`，但 proxy `AccountResponse` 三個構造點未輸出，前端只能解析 reason fallback。需獨立 local-API/UI 批次，驗證 active URL、expiry 清除與無 stale URL。 |
| `35bbbe0744a94f05cb7e9c79ba5bf19d931e3455` | 拒絕 | beta.6 release/version metadata；只由獨立 owner release packet 評估。 |
| `9a4dbacea31db92ad91f7b17aed9d071e44f7cee` | 延後 | Dashboard/account card 加 motion hover/tap；需 reduced-motion、keyboard、touch 與 desktop render acceptance。 |
| `593adec49c1a64d3f2453d9770d5c3ca0f5dc04a` | 延後 | 七個 stateful pages 加入場動效；需 reduced-motion、focus navigation、route transition、低階桌面效能驗收。 |
| `1a2a5cb8281c5856c1cbd3ccc03f0be207af5759` | 延後 | Dashboard GPU/container follow-up 依賴未採 animation design；條件同前列 route/performance acceptance。 |
| `890fa062592be6bf02b92776029a58417532f6e4` | 監看 | #3636 merge 第一父差異彙集已分別覆核 UI patches；merge 非採用證明。 |
| `73f5512bd05879b9d870ceaae59fee36597871ec` | 拒絕 | 上游 stream 診斷文檔假設未採 lifecycle，且不符合 fork 證據治理格式；僅在 fork 委託同類調查時重開。 |
| `ca9a523d3e677698df1321cc7da07450cb51a818` | 延後 | stream propagation/error-code 改寫經 absent lifecycle；需各協定中途錯誤、retry、無 synthetic terminal frame fixture。 |
| `5dff7b5f459ecc6605b873a8189fc55c42882df7` | 延後 | adaptive peek/retry/header 與 function-response reminder folding 改帳號選擇、Gemini tool-turn；需 fake transport／tool-sequence fixture。 |
| `31495efa88fcfd228a4f791bac75482116fd97d5` | 監看 | #3634 merge 第一父差異彙集本批 stream/docs patches；待獨立協定設計採用後才重開。 |
| `0eb3337c41d60685f31f833d22a5ad903d2699f0` | 延後 | token-stats persistence/ranges/cost UI 依賴未採 dashboard，且含簡中；需 bounded storage/aggregation 與 zh-TW UI acceptance。 |
| `224040fa23f9f9101510477bfe91d7b027dcb29c` | 延後（第十四批完成 component 覆核） | 除 beta.7 metadata 外，`integration.rs` 將熱切換從 `is_ide && running` 反轉為 `!is_ide && running`：IDE 改為完整關閉→寫 SQLite→重啟，Standalone 改為先寫 Keyring→殺 language_server→best-effort 補寫。fork 仍以 IDE 熱切換、Classic 完整重啟；未驗證其 target resolution、Keyring/SQLite source、supervisor 與 credential-write/restart order 契約，故不採碼。 |

### 0.4 第六批：model quota／retry／compaction／padding 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `61fc48558816c740cb1662690b5683040c27ea4d` | 延後 | `token_manager.rs` 將 reset lock 綁定至指定模型且僅接受該模型 0% quota；fork 仍採帳號級 `get_quota_reset_time`。需 fake-clock 證明多模型 429 不會誤鎖、真 0% 會精確鎖定，且無跨帳號污染。 |
| `29e65fd36a68df165a4f3dd8e056dd0c2f59013a` | 延後 | `handlers/common.rs` 在多帳號第一輪以 <=2.5s GraceRetry 取代 50ms 輪替，`token_manager.rs` 為 image scheduler 加冷卻等待重選；fork 已有不同 GraceRetry／image selection 路徑。需 fake-clock、deadline、池耗盡與一次 grace 消耗 fixture。 |
| `5c7998c8bde25924d74511f023c09530748aea61` | 延後 | 以 per-model quota 改 P2C、preferred/sticky/60 秒 session fallback；fork已有 capability filter／session binding，但 selection 順序與相容契約不同。需 sticky escape、preferred override、0% metadata 缺失與無重複 request acceptance。 |
| `1fea3bd799f20d212b5a67225b2c418b6ff59868` | 拒絕目前 patch | 上游 Claude Cowork deep-compact 延長 lease／掃描五訊息／擴充命令；fork使用不同的 Layer-3 XML summary compression，沒有對等 lease/continuation state。僅在 owner 要求統一 compaction lifecycle 並有 continuation／cache／signature fixture 時重開。 |
| `9d652296a3dd9881676c92b6eb8b991c76c3ea8b` | 延後 | 將 `weekly_reset_time` 與 EDF 加入 token selection；patch 寫入 `bucket_id`／`reset_time`，測試 payload 卻為 `bucketId`／`resetTime`，provider 資料契約未證實。需真實遮罩 quota fixture、fake clock、週/5h bucket 區分與 fairness acceptance。 |
| `8cddf33eaf5d14b161f7701ad718cd9825938a06` | 已存在 | 上游把 fallback `ok go on` 改為 `.`；fork `mappers/common_utils.rs:1619` 已是惰性 `.`，並有 `test_transit_defense_fallback_text_is_inert_protocol_padding`。保留既有 security regression，不重複移植。 |
| `f7f784e30ff7e642117ec98faa75ad39875c0492` | 監看 | #3640 merge 的第一父差異與 `61fc…` 相同；不因 merge 再次採用，條件同模型級 reset fake-clock fixture。 |
| `21a1cca7b2ed598bbde98149e47a394952bdcf90` | 監看 | #3641 merge 納入 micro-gap GraceRetry/image cooldown；fork retry policy 不同，條件同 `29e65…` 的 deadline／pool fixture。 |
| `336d31163d4a19a88e55b493b65db3d4579f7c9c` | 監看 | #3642 merge 納入 per-model quota session switch；條件同 `5c799…` 的 session/selection acceptance。 |
| `0973ad98762ad6655727e2f2d0f2b975b28d16c1` | 拒絕目前 patch | 僅修正未採 #3639 測試將 model quota key 正規化；不在未採產品行為上移植測試。採用父功能時一併重開。 |
| `befe6225800823d8b28e45338213597fbb47db50` | 監看 | #3644 merge 納入未採 Cowork compact patch；無獨立可採用內容，條件同 `1fea…` 的 lifecycle 重新設計。 |
| `55f774e84d249893b55f8e3fb3eec675e8f190ff` | 監看 | #3645 merge 納入 weekly EDF；無獨立可採用內容，條件同 `9d652…` 的 provider quota schema 與 fairness fixture。 |

### 0.5 第七批：quota compatibility／model route／HTTP API／transport 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `b0fd10c15848bf19726cb560f21c7e22971c4ca8` | 拒絕目前 patch | 只修未採 weekly EDF／Cowork compact 的 camelCase quota 欄位與測試編譯；不在未採 scheduler/compaction 行為上移植相容修補。若採用父設計，連同 provider-shaped quota fixture 一併重開。 |
| `aa84203026fa5e7b059c3ed9a3e9a438653c4b3d` | 已存在 | #3649 merge 第一父 patch SHA-1 `65312707…` 與第六批 `8cddf…` 精確相同；fork 已有 inert `.` padding 與 focused regression，引用既有證據，不重複採用。 |
| `f719d242779d73d817c70ac8426e123614381741` | 延後 | 讓 route-with-effort 將 public tier alias 再映射為 physical upstream ID（如 `gemini-pro-agent`）；fork model mapping 路由不同，需 custom mapping 優先、effort alias、實際 upstream 400/capability fixture。 |
| `31cc4f4306351571f8efc49ff794506b84b5495f` | 延後 | Claude Code 偵測後關閉 usage scaling，改 client 95% auto-compact 的可見 token 壓力；fork仍有自身 scaling/Layer-3 壓縮路徑。需 Code/Cowork/一般 chat classifier 與 compaction-threshold acceptance。 |
| `169f144a0d3b11d2cfb4fa5ce111d1e32afcf1b0` | 拒絕目前 patch | 將未採 Cowork immunity lease 60 秒延長至 90 秒，並推薦固定 Claude Code 版本；fork無同一 lease state machine，且版本建議不能取代 transport 驗證。 |
| `658cfa825b4d8424d4c8cd60449008b414d2deff` | 拒絕 | 新 Windows build/release workflow 要求 `contents: write`、動態改 Tauri config、以浮動 Action ref／`npm install` 發布 prerelease；不符合 fork 的 immutable pin/release 治理。 |
| `23b571cf6f8df979b4683575332cffbf6199bf17` | 延後 | 混合 HTTP AccountResponse validation URL、storage auto-create、CLI sync、靜默啟動與 admin hot-sync 七模組；fork確實缺 API validation fields，但必須拆出最小 local API/UI／expiry／storage migration 批次，不能整包採用。 |
| `15f4bc938ecfa0286097ec6246a3d1583bc683a7` | 延後（第十五批完成 component 覆核） | 39 檔 beta.8 已逐類覆核：release metadata 拒絕；commands/device/http/token-manager 僅格式跟隨；其餘 reasoning-token 資料鏈、`thinkingBudget=0`、dashboard/pricing 與 1,452 locale key 均因 fork 契約不符而延後。詳見第十五批；不採產品碼。 |
| `cb6366d8f717ace2f52b069eaef6b9f5150f7047` | 拒絕 | beta.9 release/version metadata；僅由獨立 owner release packet 評估。 |
| `cfea6e8d974635a93b78b07e63b196566dbba11d` | 延後 | AccountCard/Table 於 weekly view 同時堆疊 5H 與週 quota pills；fork UI/layout、可及性與小螢幕契約需獨立 visual/responsive acceptance。 |
| `b08b17276b63aeb54ef3de19b13882ec3445861b` | 拒絕目前 patch | changelog 僅敘述未採 weekly stacked UI；待來源 UI 設計採用後才更新 fork release notes。 |
| `7de745127c6e3bddff2bac3356a1d3dd1868998c` | 延後 | endpoint health cooldown、5 秒 connect timeout 與 fallback reorder 改變 internal endpoint retry 行為；fork仍有其 endpoint chain。需 fake transport 驗證 4xx/429/5xx、cooldown expiry、全端點失敗與無跨帳號污染。 |

### 0.6 第八批：physical model／internal warmup／endpoint health／model catalog 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `f97fae822439623ab8af8bfd760e3d4538fdd217` | 拒絕目前 patch | changelog 僅敘述未採用的 endpoint-health tracker；待來源實作有獨立 acceptance 才更新 release notes。 |
| `8db23c3ef110359678820cbd8c83ac911a1c1ea4` | 延後 | 跨三協定／warmup 增加 `X-Upstream-Model`、SSE/JSON `modelVersion` 擷取、`request_logs` migration 與 Monitor UI。fork缺同一資料保存／header 信任／隱私契約；需 migration/rollback、stream/non-stream/header 衝突、redaction 與 DB failure fixture。 |
| `1600640a5d746ec8c293a8be128db647afaa4c66` | 拒絕目前 patch | changelog 僅記錄前列 physical-model visibility；來源未採用前不帶入 fork release notes。 |
| `278d4273173e49c21ed1abcc29f026c48bb5865d` | 已存在（較強替代） | 上游僅以 TCP loopback 保護 internal warmup；fork現有 capability middleware 另要求 process-local secret、拒絕 duplicate/missing header，且在 CORS／authOff／stopped 前執行。保留 fork 較強的 tunnel-safe 契約。 |
| `1b1a96ebae589870b1e4d475f5e437a1aec3a61b` | 已存在（較強替代） | 只格式化前列 loopback guard；fork已不採其 TCP-only 設計，維持 credentialed loopback guard 的既有 regression。 |
| `f83c1f06ff17517b1976da5b2c2b4b4b2828c762` | 已存在（較強替代） | 上游在 authOff 前拒絕非 loopback，但仍把 loopback 當身分；fork已覆蓋外部／forged header／OPTIONS／stopped，並加不可猜 process-local credential。 |
| `95b7300a52a094bc7d45efaa2dbd61eaa20d4975` | 延後 | 將所有 host 的 endpoint-health 全域共享，並把 quota/project resolve 壓至 3/5 秒；會改變 fork 的 quota、project 與 retry timeout/冷卻選擇。需 fake transport 覆核 host normalization、4xx/429/5xx、跨帳號隔離、timeout 與 recovery。 |
| `1ecfa28d36c53aa90e335e75ae60a232c962e47c` | 延後（第十六批完成 component 覆核） | beta.10 的 39 檔已分為 metadata、response-header log/export、quota authority/account surfaces、dashboard/a11y 與 locale；無任一類可隨 release 採用。詳見第十六批；不採產品碼。 |
| `f57d48c0528a872d48820d96b535c7c9c9c02ee1` | 拒絕 | 實際僅 10 個 version/changelog/lock/Tauri/UI metadata 檔；只由獨立 owner release packet 評估。 |
| `c44d96b02bb794ce012dfde97d21100a564609fe` | 延後 | 動態 deprecated-model rules、Gemini 4 Argon、token selection 與 UI catalog 同時移除／重映舊模型。需遮罩 provider catalog snapshot、custom alias precedence、capability/400、cold-start 與 stale-model suppression fixture。 |
| `1b419043a3d67784e5e4adb046cab3c212a575bb` | 延後 | 三協定、endpoint client 與 Monitor UI 增加多端點 TTFT/fallback timeline，會暴露 endpoint URL／重定義 timing。需 fake transport 的 fallback ordering、timeout/error redaction、stream/non-stream timeline 與 responsive/a11y fixture。 |
| `e57e5fae2cc07704dbeb1582ac868c0f05c9b8d1` | 拒絕 | 實際僅 10 個 version/changelog/lock/Tauri/UI metadata 檔；只由獨立 owner release packet 評估。 |

### 0.7 第九批：release isolation／monitor UI／quota scheduler／fallback telemetry 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `3e186e91e74aebf787a1d037ba4c95928526bb74` | 拒絕 | build workflow 改 tag trigger/release-action 條件，但 fork 的 owner release 以 immutable pin、獨立 workflow 與已驗證簽章治理為準；不能移植浮動 `@v1` release-action 的競態修補。 |
| `3a9919da370c39e57a9571c83a8f1259871e8291` | 延後 | Dashboard 卡片 theme、Monitor endpoint timeline 與 locale 同時變動；需 dark/light、endpoint timeline、keyboard/reduced-motion 與小螢幕 visual acceptance。 |
| `5a373b1b80bdf27fd4cec274bf453113d1da0c45` | 延後 | 重寫 ProxyMonitor 欄位拖曳／寬度 state；fork UI layout 契約不同。需 mouse/touch drag、keyboard resize、long-text truncation 與 persistence fixture。 |
| `21863d5a3c3c5200d4491195130ce41e349d1046` | 拒絕 | 實際僅 10 個 beta.1 version/changelog/lock/Tauri/UI metadata 檔；只由 owner release packet 評估。 |
| `b00898720e88ed1f21810e15bbc99509ddf8f739` | 拒絕 | release workflow 僅手動發布並複製空格檔名為點號；仍依 upstream 浮動 release action／asset 命名假設，不符合 fork immutable release 治理。 |
| `1ead6ff609790d3e31843a7406b2b9582f09afe0` | 延後（既有 #3666 來源） | 第一父 patch 為已審 PR #3666 scheduler/account atomicity 與 stream terminal triad；沿用既有 deferred 結論，需低 quota EDF/P2C、account/index atomicity、content/tool-after-error 與 idle-timeout fixtures。 |
| `3565de5be8c9fbdc220925deda3790279c0c5ac6` | 延後 | beta.2 除 metadata 外，改 `available_models`、tier stripping、dynamic forwarding、model specs 與 token selection；需遮罩 provider model catalog、cold-start、alias/tier mapping、quota capability 與 no-false-429 acceptance。 |
| `411f1a75183f5cb636103fdd8a0b1ead02c8b723` | 延後 | 三協定 non-stream TTFT、upstream error body 與 Monitor timeline 同步改動；需 fake transport 覆核 header/body timing、fallback、error redaction、stream/non-stream UI semantics。 |
| `5509f006c557a947ea05fafc474625d14998a054` | 拒絕 | 實際僅 10 個 beta.3 version/changelog/lock/Tauri/UI metadata 檔；只由 owner release packet 評估。 |
| `80676eafa286c19c5801591f01ecf9fa6565df8b` | 拒絕目前 patch | 修補的是未採 fallback-attempt telemetry 路徑的 Rust ownership/placeholder；fork現有 client/handler 與 error redaction 契約不同，不單獨移植。 |
| `e4924573f1f7338aafbb0a813f2271ff85d91e52` | 延後 | realtime monitor summary 開始保留完整 response headers；須先決定 response-header storage/redaction，並以 SSE live update、敏感 header 不外洩、TTFT 同步 fixture 驗收。 |
| `51b63fb4c920d7315c1f16e5b0ce47b88d658030` | 拒絕 | 實際僅 10 個 beta.4 version/changelog/lock/Tauri/UI metadata 檔；只由 owner release packet 評估。 |

### 0.8 第十批：Responses 相容、endpoint/retry、warmup 與 token stats 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `b81f08d6cc175a70b3a8032e3847a03b74f7a5f9` | 採用候選（限縮） | 修正 Responses `input` array 角色與 string input；fork 完整 converter 可保留結構化 array，但 chat-completions Responses 相容路徑仍手動將 array 壓成單一 user string，converter 又丟棄 string。另立最小 patch 使相容路徑重用完整 converter 並補 string 支援；raw schema gate 必須在轉換前，其他 `Value` 不放行。需 multi-turn role、instructions-only、string、tool output 與 Gemini role mapping fixture。 |
| `2f7641a68e90a2a45e5507903d127bac2d698d53` | 監看（相同 patch） | #3672 merge 的第一父 patch blob 與前列 `b81f08d…` 同為 `3d02ee…`；不重複採用，沿用「相容路徑重用完整 converter 加 string 支援、schema gate 前置且不放行其他 Value」的限縮候選與 multi-turn/instructions/string/tool-output/Gemini fixture。 |
| `c7293de1d1c348bc7e877d6ce2e08626731c07e4` | 延後 | beta.5 除版本檔外，改 Hub UA、Argon model catalog 與 `thinkingBudget` floor；fork 的 endpoint identity 與 inbound thinking policy 已不同，沒有可公開的 provider capability／400 契約可證明偽裝 UA 或 1024 clamp 正確。需遮罩 catalog、UA request、0/32/1024 budget 與 400 fallback fixture。 |
| `8459833238869d58d5f0ecc1ff9de6beabd354d1` | 延後 | 將 quota/project endpoints 改 Daily→Prod→Sandbox、request-level 429 擴為全池輪替，並改 index/entity state；fork維持 Sandbox-first、二輪 grace retry 與 on-disk disabled safety checks。需 fake transport 的地域 400、429 pool exhaustion、cooldown、index/entity drift 與跨帳號隔離 fixture。 |
| `56f14ba0f0ec30de4dab3a76699fbcf83f5bbfa8` | 延後 | 再把同一 endpoint 順序與 client retry 改為 Prod-first；與 fork Sandbox-first 健康／fallback 契約相衝，不能以標題或上游測試取代實際 provider timing 證據。需同前列 endpoint ordering、timeout、region 與 recovery fixture。 |
| `f397b2cedeab1dd650666f2e5dd53b6c9b4f4070` | 延後 | 移除 Claude 模型 `[1m]` suffix、重寫 Fable alias，並合併 client `anthropic-beta` headers；fork mapper/Claude header policy 含自訂 alias 與 provider identity。需 alias matrix、malformed bracket、allowlisted beta header、1M context request fixture，才可拆件採用。 |
| `d66bb273db9e101a4dfc68e1a5a9a1c4117e7f20` | 延後（需設計） | patch 不只同步 UI index，還移除 `summary.disabled/proxy_disabled` 的強制停用、以 entity 覆寫 index，並吞掉 `save_index` 錯誤；fork不能照抄，否則 entity/index 矛盾或寫入失敗可能放行手動停用帳號。需先定義 fail-closed entity/index 權威與原子寫入契約，再以 disabled/proxy-disabled、reload、並行 manual disable、讀寫失敗 fixture 驗收。 |
| `00eb21f462731901be4c066d320ad42f2da9fb41` | 延後 | 將 endpoints 改回 Daily→Sandbox→Prod，並在首輪全池 429 後 fail-fast；fork目前保留 second-round grace retry，這是不同 client latency／公平性取捨。需 pool-size 1/N、explicit Retry-After、request-level 429、取消與 user-visible error fixture。 |
| `1a4cb5ddfae382052690707bfc21d5473b70fcd2` | 監看（合併彙整） | #3680 merge 的第一父 patch 彙整本批 endpoint、429、account-state 與 Claude changes；上述每個實際來源均已逐筆判讀，merge 不產生獨立採用結論。 |
| `386fa87e54323a9ff6b33b79ce3b9b0df4494b3c` | 拒絕 | 實際僅 10 個 beta.6 version/changelog/lock/Tauri/UI metadata 檔；只由獨立 owner release packet 評估。 |
| `ad3b25c23f6e51ccf618a57839b611e9e5d17851` | 延後（安全邊界） | 新增 Tauri command 以 loopback POST 將 access token 送入 `/internal/warmup`，並把結果／帳號資訊寫入 Monitor/UI。fork的既定 internal warmup 方向要求 capability credential、header stripping 與 CORS/authOff/stopped 拒絕；不能引入只信任 `127.0.0.1` 的直接 token path。需 credentialed local client、wrong/missing secret、redirect/header non-leak、retired-model 與 log-redaction fixture。 |
| `51b4069fb9b18e86ab38cf1c6c53731385928d0c` | 延後 | 新增 user-token usage SQL aggregation、IP exposure、ranking command 與 dashboard/modal；fork沒有相同 token analytics 的資料保留、授權與 migration 契約。需 scoped authorization、retention/redaction、SQLite migration/rollback、time-range/empty data 與 a11y UI fixture。 |

### 0.9 第十一批：Hub、schema、tool 名稱、模型、配額、sanitizer、monitor、CLI 與帳號 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `250559d94e97af51840da5d92ffd3a40be24a625` | 拒絕目前 patch | 將 Hub 指紋版本／client label 改為可由設定或遠端 YAML 動態覆寫；fork 既有 UA floor，但沒有簽章來源、allowlist 或隱私 egress 契約。不得以任意 custom version／CL 偽裝 provider fingerprint。僅在固定 trust origin、簽章／版本與 label 綁定、opt-in、失敗不出網與快取 fixture 的獨立設計重開。 |
| `a01fc35f51c39aa4edaf1cbe7fc19c20f1d5fcf3` | 拒絕目前 patch | 將 schema recursion 10 提至 64，並在超限時改成 string／以無界 heap walk 補裸 array；這會靜默改工具語意，且與 fork strict schema 候選的 typed depth/cycle/resource error 相衝。需 fail-closed、depth-32/ref-cycle/source-unchanged fixture 才可另案。 |
| `96edbf5ab6af228b7504b06ffca23ec4685de7cc` | 拒絕目前 patch | 跨三協定將 unknown tool 名稱以單一宣告工具或十輪歷史「自癒」，並把 orphan response 降成文字；fork tool turn／signature store 以 name/id 關聯，不能合成 invocation 或掩蓋 receipt 錯誤。僅有可驗證 call-id mapping 的 typed local error 設計，及 unknown/duplicate/collision/truncated-history fixture 才重開。 |
| `ec5f7cb843bbbfd9993285dadd9fc99d4488ef21` | 延後 | `/v1/models` 擴出 quota、tier 派生裸模型與 custom mapping metadata；會改 API 相容、路由與帳號能力曝露。fork已有 model specs／server route，但沒有 merge precedence 或 metadata privacy 契約。需 catalog snapshot、alias/tier conflict、multi-account、legacy `/v1/models` fixture。 |
| `95d84869b036ae2f63d9bfac5bd8157f0f16e346` | 延後 | 將 Gemini 4 Argon 獨立 quota、selection/warmup 熔斷與多處 UI/locales 一起變更；fork沒有已證實的 Argon capability／quota group 契約。需遮罩 catalog、Argon/normal 混合 quota、400/429 不誤鎖、帳號隔離與 UI/locales acceptance。 |
| `d223bc46f329cdf2ad0ed9abb210b88285d38045` | 拒絕目前 patch | prompt sanitizer 將 known/custom identity 直接改成官方 Antigravity 並重寫第三方工具敘述；fork既有 sanitizer 不能證明 provider 要求 impersonation，這會改 user/system/tool semantic context。僅以最小、經核准的 redaction（不替換 identity）及 user/system/tool-preservation fixture 重開。 |
| `3a48a6ffd6a8712b13ec3854188a89b82adad1be` | 拒絕目前 patch | monitor 重構新增 request/upstream headers/body、response body、endpoint URL 的雙 HAR 匯出；fork沒有敏感資料保存、export consent、redaction/retention 契約。需 secret/header/query/body redaction、權限與加密／保留期、large/stream payload fixture 才可另案。 |
| `81d1d2f7f11f25ebeca9627eb59ce26ee3cc0400` | 延後 | CLI sync 擴大 agy discovery/config 同步、telemetry 設定與動態模型選擇；fork已有 CLI sync，但 native config 寫入與跨平台掃描契約不同。需 allowlisted filesystem/no-shell-exec、TOML/JSON migration backup、telemetry opt-in 與 model config fixture。 |
| `eccedfcb357ad453c2d73b536c88f9cc11905024` | 拒絕 | 實際僅 beta.8 version/changelog/lock/Tauri/UI metadata 10 檔；只由獨立 owner release packet 評估。 |
| `ceb2555698775b7e1fba1bb69ff71b7f7c3fcf32` | 延後（整併既有候選） | beta.9 除 metadata 外，加入 validation URL／403 分類、10 分鐘 block 與 quota 成功時 auto-enable proxy；與既有 `6e562…` validation-URL、`d697…` expiry/403 候選重疊，但不能直接採用會覆寫手動停用的整包行為。需 fake-clock load/reload/refresh/selection、malformed URL、403/validation 差異與 manual-disable invariant。 |
| `8037d80fd8a754c6237bd2ea5cc0895c0e6d3105` | 拒絕目前 patch | 重寫 Classic/IDE/CLI 本機 credential scanner 並移除 V1 backup/import；fork有既有 account/migration 路徑，不能以 release 標題刪除備份或改 credential precedence。僅在 owner migration 批次具 source precedence、fake filesystem/keyring、備份保留與失敗 rollback fixture 時重開。 |
| `0ffd1f942e5807ee7461f915ebfca032c9ea1ee3` | 延後 | 多 client active-account tracking 讀 IDE DB/keyring/CLI credentials、對 refresh token 匹配並改 UI/index；fork帳號 state/selection 契約不同且涉及 credential privacy。需 missing/corrupt keyring/DB、client collision、refresh-token collision、無 secret UI/log fixture。 |

> 本批沒有採用上游產品 patch；`b81f`／`2f764` 的 Responses 限縮候選也仍未實作。release 基線保持 `v4.9.6`／`b601f5e…`。

### 0.10 第十二批：auto-switch 翻譯與 endpoint/watchdog 註解 4 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `76c55d75471dd4b6aebe9e304c1f229d43aa49e5` | 拒絕目前 patch | 僅在韓文加入 Classic-only badge 與 IDE/CLI 不支援說明，依賴尚未採用的 classic auto-switch 功能；fork無對應設定／行為，不能先加入會誤導使用者的 UI copy。父功能有 owner 設計與跨 client acceptance 後才一併重開。 |
| `007e65babfb7b4523f4f35db7ed431347c4777a7` | 拒絕目前 patch | 與前列相同的日文 Classic-only 說明，沒有獨立產品行為或 locale key 對應；條件同 `76c55…`。 |
| `5c239f536d87a30dcb8675e6abb686a1dfaf3062` | 拒絕目前 patch | 與前列相同的俄文 Classic-only 說明，沒有獨立產品行為或 locale key 對應；條件同 `76c55…`。 |
| `685778fa4964af440d30550a5a45dd637bcead39` | 拒絕目前 patch | 表面為註解，但實際斷言 Prod endpoint「全流量必報 429」且把 stream watchdog 改寫為 120 秒契約；fork仍保留 `Daily → Sandbox → Prod` fallback。沒有受控 endpoint/TTFT 證據，不能將未驗證 provider 行為寫成維護依據。需 fake transport 覆蓋 Daily/Sandbox/Prod、400/429/5xx、first-byte timeout 與 cancellation 後才可採用任何行為或文件變更。 |

> 本批四筆都不構成獨立產品採用；沒有改動 release 水位。

### 0.11 第十三批：帳號來源、dashboard、Responses、endpoint、auto-switch 與 release 12 commits

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `959402fd5d6c9b5772f832fc4c2e88b44e9ee975` | 延後 | protobuf 改為掃描多個 Topic/Row、優先 oauth sentinel，並加入 5 秒 active-client polling；fork帳號/credential 來源與 UI polling 契約不同。需 malformed/多 entry protobuf、舊格式 migration、focus/unmount polling、無 secret UI/log fixture。 |
| `8c8354c62f27a2e3620714bfc39231ffb0772018` | 延後 | 改 Classic DB 與 CLI keyring 的偵測先後；實際會改 OS credential source precedence。需 Classic/IDE/CLI 同時存在、keyring 缺失/錯誤、refresh-token collision 與 source-isolation fixture。 |
| `e74b8769a2167e4e765d4f852946c5173ea0f34c` | 延後 | 依 Classic version 選 keyring 或 SQLite，並以 target/index 選 CLI credential；fork沒有已驗證 version/path/keyring precedence 契約。需 v1/v2、CLI target、缺失/損毀來源、交錯帳號與無 secret log fixture。 |
| `f8fe887dd19902c9ca80d3f1a183835e5ab78d10` | 延後 | Dashboard 加 Classic/IDE/agy target tabs 並可切換最佳帳號；依 active-client/account index，會改帳號切換與 UI 可及性行為。需 no-active-client、各 target disabled/proxy-disabled、切換原子性、keyboard/responsive fixture。 |
| `6d45a5057c381f063ac16acd1a0698aebb8d88cb` | 延後 | 為前列 target tabs 補 12 語系 key 與 title；fork尚未採 target-switch UI，不能獨立導入 locale。前列 dashboard 行為與 zh-TW mapping/locale key-parity 驗收後才重開。 |
| `6a4806ecf0b368b908eb7a7b51cf75186f23890f` | 延後 | 實際移除 Prod fallback 並把 stream first-byte watchdog 拉至 120 秒；fork保留 Daily→Sandbox→Prod，且無受控 provider timing 證據。需 Daily/Sandbox/Prod 的 400/429/5xx、TTFT、timeout/cancellation fake transport fixture。 |
| `aa9f103df9d6ec5914375627dd475fcc8fab35ee` | 拒絕目前 patch | chat compatibility 改用完整 Responses converter 並補 string input，但 converter 仍將任意其他 `Value` 包成 user content，且最早 raw schema gate 要求尚未保證。沿用 `b81f/2f764` 的限縮候選：僅 array/string、轉換前 raw schema gate、multi-turn/instructions/tool-output/Gemini fixture。 |
| `4d1e1d1e4166bd146731f9bc1bba71343268e12e` | 監看（合併彙整） | #3691 merge 的第一父 patch 彙整移除 Prod fallback 與 120 秒 watchdog；無獨立可採結論，條件同 `6a4806…`。 |
| `50686b58baf74836fa62c41784b881294a24c713` | 延後 | 25 檔新增 Classic quota auto-switch：選帳號、終止 language server、寫 keyring、重啟、120 秒冷卻、設定/UI/locales。fork無對等 process/keyring safety 契約；需 fake process/keyring、quota threshold/tier/LRU、manual-disable、write/restart failure、跨 client/帳號 isolation fixture。 |
| `06e8eda775cb1f333927afadae2e986cb56dff4f` | 拒絕（拆分） | 十個 version/changelog/lock/Tauri/UI release metadata 檔僅可由獨立 owner release packet 重開；`ar/es/my/pt/tr/vi` 六個 locale 則新增 Classic-only badge／宣稱 IDE、CLI 不支援的 auto-switch 文案，依賴未採用的 `50686b…` auto-switch，須先有其 client/process/keyring 安全設計及 locale key-parity acceptance，不能以 release metadata 視之。 |
| `23956b00bf245837907c385eb303c9f20e4e48da` | 監看（beta merge aggregate） | 第一父對第二父 beta aggregate 為 161 檔、52961/+26541；實際涵蓋 schema、endpoint、Responses、帳號、auto-switch、HAR、CLI、quota、UI 等已在本 ledger 逐筆拆開。merge 對第二父另有 `AGENTS.md` toolchain-skip 指引與 `README.md`／`README_ZH.md` v4.9.4→v4.9.6 release claims（3 檔 +11/-7），同樣不建立產品採用。所有 constituent decision 完成前僅保留 aggregate／resolution 追溯。 |
| `9f8e4f24d61b33fa80b8078d104c942752a5fa0b` | 拒絕 | v4.9.8 final release 實際為 version/changelog/README/lock/Tauri/UI metadata；release notes 含尚未採用的 auto-switch、schema、endpoint、monitor 等宣稱。只由 owner release packet 評估。 |

> 第十三批不採用產品 patch；第十四至第十六批完成三筆 component 判讀，162 筆 release commits 都已有實際決定；release 基線仍為 `v4.9.6`／`b601f5e…`。

### 0.12 第十四批：beta.7 integration component 補覆核

| SHA | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `224040fa23f9f9101510477bfe91d7b027dcb29c` | 延後 | `integration.rs` 將僅 IDE 的熱切換倒轉為僅 Standalone：上游熱切寫 Keyring→終止 language_server→補寫，IDE 一律 full restart；fork 為 IDE 寫 SQLite→終止子程序→補寫，Classic/full restart 為 close→write→start。採用前需 fake process/keyring/SQLite matrix：IDE／Standalone 各 running／not-running、language_server absent／kill failure、pre-write failure（不得中斷既有 process）、post-kill reassert failure、target 衝突與 active args snapshot；另驗證 full-restart close→write→launch 順序、supervisor respawn 與無 secret log。此 commit 的 metadata 不構成 release 採用。 |

> 第十四批只完成 `224040fa` 的 component 判讀；未採產品 patch，也未變更 release 水位。

### 0.13 第十五批：beta.8 reasoning／thinking／TokenStats components

| SHA／元件 | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `15f4bc938ecfa0286097ec6246a3d1583bc683a7` release metadata（10 檔） | 拒絕 | changelog、Cask、npm/Cargo/Tauri version 與 MiniView／Settings 僅將 beta.7 升 beta.8，且 release notes 宣稱未採用的 quota、compaction、HTTP API、reasoning 與 UI 工作；僅能由獨立 owner release packet 重開。 |
| `15f4bc…` commands/device/http/token-manager（4 檔） | 監看 | 本 commit 對這四檔只有 rustfmt／換行層級重寫；token-manager 的 quota/EDF 邏輯來自已另行判讀而未採用的來源，不能由格式 hunk 建立採用結論。若未來採用來源 patch，重用其既有 quota、deadline、account-isolation acceptance。 |
| `15f4bc…` reasoning 資料鏈：`proxy_db`、`token_stats`、monitor middleware／warmup（5 檔） | 延後 | 新增 `reasoning_tokens` migration、位置式 row mapping、raw/hourly aggregation 與 SSE／non-SSE usage 擷取；fork 現有 TokenStats API 沒有該欄位。需舊 DB 升級與既有 row backfill/NULL、insert/query ordinal、流式 cumulative-vs-delta、缺 usage、計數失敗不影響 request、帳號/模型隔離及無敏感 log fixture。 |
| `15f4bc…` `thinkingBudget=0`：model specs／inbound／alignment test／設定提示（4 檔） | 延後 | `>0` 改為 `>=0`，並在 generation config 注入 `includeThoughts:false`；改變 0 從 fallback/default 為 hard disable。fork 沒有已驗證的 provider thinking 契約，須以實際協定 fixture 確認 0、-1、default、low/medium/high、stream/non-stream 的輸出與 thought 欄位行為後才可採。 |
| `15f4bc…` AppleBento／CyberCockpit／PricingModal／TokenStats（4 檔） | 延後 | upstream 新增兩個 1,900+ 行 dashboard 元件、localStorage mode/pricing persistence、硬編碼模型價表與模型名稱 fallback；fork沒有三個對等元件，既有 TokenStats API 亦未帶 reasoning 欄位。需價格來源/更新責任、model match、儲存失敗/upgrade、a11y/鍵盤、小螢幕、空/大量資料與帳號資料隔離 acceptance。 |
| `15f4bc…` 12 locale（1,452 僅新增 key） | 延後 | `en/zh` 各 86、其餘十語各 128 新 key，集中於 TokenStats dashboard/pricing 與 thinking soft-budget copy；不是「非繁中」即拒絕，而是依賴未採的 UI／provider 契約。待上述元件設計確定後，做全 locale key-parity、zh-TW 文案與 fallback fixture。 |

> 第十五批只完成 `15f4bc` 的逐元件判讀；沒有採用產品 patch，也未變更 release 水位。

### 0.14 第十六批：beta.10 quota／monitor／account-dashboard components

| SHA／元件 | 判定 | 實際 patch／fork 差異與重新開案條件 |
|---|---|---|
| `1ecfa28d36c53aa90e335e75ae60a232c962e47c` release metadata（9 檔） | 拒絕 | changelog、Cask、npm/Cargo/Tauri 與 MiniView 僅從 beta.9 升 beta.10，release notes 又宣稱未採用的帳號、quota、monitor 與 dashboard 行為；只能由獨立 owner release packet 重開。 |
| `1ecfa…` `proxy_db`＋`ProxyMonitor`（2 檔） | 延後 | summary/filter query 由 `NULL as response_headers` 改為回傳原始 response headers，並用 `x-timing-ttft*` 顯示首字延遲、保留欄寬至 localStorage，export 亦帶出 headers。fork 需先定義 header redaction/retention、UI/匯出權限、malformed header 與 TTFT 缺失/偽造值、跨帳號/使用者資料隔離 acceptance，才可採。 |
| `1ecfa…` quota authority／帳號 surfaces（10 檔） | 延後 | `quotaDisplay` 以 bucket-id/window regex 和 models fallback 導出 5H／weekly、把 exhausted weekly 壓過 5H，並讓 AccountCard/Table、CurrentAccount、Details、Accounts、Settings 同時移除 5H/weekly toggle、show-all 與 pinned-model 控制。fork 尚無可驗證的 quota_groups 權威、fallback precedence、manual/proxy disable 與重置時間契約；需遮罩實際 quota payload、missing/dual/衝突 bucket、timezone/reset、disabled/protected account、鍵盤/螢幕閱讀器/小螢幕與 control-state persistence fixture。 |
| `1ecfa…` dashboard／樣式（6 檔） | 延後 | 新增 Donut/MultiRing，重構 Dashboard/CurrentAccount/BestAccounts，並改 App/Tailwind；其可用帳號、proxy-pool、abnormal 與 weekly average 聚合會直接受上述 authority 影響。先決條件是同一原始 quota snapshot 的加總一致、空/大量資料、非同步 refresh、顏色對比/焦點順序與 responsive acceptance。 |
| `1ecfa…` 12 locale（新增 39/46 key、各 1 個變更） | 延後 | keys 具體覆蓋 accounts quota、dashboard 健康/資產和 monitor TTFT，不是因非繁中拒絕；待各 UI／privacy 契約採用後，驗證所有 locale 的 key parity、zh-TW 文字、fallback 與 placeholder interpolation。 |

> 第十六批只完成 `1ecfa` 的逐元件判讀；沒有採用產品 patch，也未變更 release 水位。

### 0.15 2026-10-21 新增 tickets：PR #3708／#3714、Issue #3709–#3718（固定 10 項）

- **來源與身份**：上游只讀。PR #3708：base `6b75dcb…`、head `c8c800831ef062ce7974eca88c51517fb0651d70`、完整 diff blob `b6cb44616e315d0a18630407712862e3d3243105`；PR #3714：同一 base、head `8a496c6c8531c1d3ca9e57e3d4fc628f6b21c7c4`、完整 diff blob `92724d3d02b227c6cfae8f77c3e7d953fecf37b2`。Issue 本文與必要留言均於本輪實讀；#3713 的 maintainer 留言只稱 Google 故障，不能建立本地產品根因。
- **不變水位**：這些是「已審、未採用」的候選記錄；`reviewed_pr_through`、`reviewed_issue_through`、release 與 commit baseline 均不在此批上調，待治理完整 diff review 後才另行決定。
- **精準建議（未寫入 baseline）**：若 root 驗收此固定清單且 strict checker 沒有未審插入項，再獨立批准 `reviewed_pr_through` 至 `3714`、`reviewed_issue_through` 至 `3718`；本節本身不構成水位更新。

| 項目 | 判定 | 實際差異、fork 對照與可驗證後續條件 |
|---|---|---|
| PR #3708 | 採用候選（限縮，未實作） | 3 檔新增 `/props`、`/v1/props`、`/version`、`/v1/version` 與 OpenAI model context/max-token 欄位；fork `server.rs`／`openai.rs` 尚無這些 probe handlers。上游以未知 model 預設 1M/64K，且僅把路徑列為 log health-check，不足以證明每個欄位或 auth/log 契約。另立只讀 probe batch，需 known/unknown/alias model、URL query、無帳號選擇／無上游 HTTP、auth/log privacy、`/v1/models` 與 retrieve parity fixture。 |
| PR #3714 | 延後 | 5 檔把任意 handler error 都標為 30 秒 transient account cooldown／解除 sticky session，將 stream response-header watchdog 收斂至 30/45 秒、全鏈路 55–70 秒，並以 20 秒成功延遲降級 endpoint。fork endpoint fallback、帳號 lockout 與 session 語意不同；#3713 沒有可驗證因果，PR 留言也無完成證據。需 fake transport 區分 header/首字/完整 stream、Daily/Sandbox/Prod 400/429/503/timeout、body-size budget、slow-success recovery、不可因本地/4xx 錯誤誤冷卻、sticky migration/account isolation fixture。 |
| Issue #3709 | 拒絕 | 本文僅為貢獻者致意，無行為、程式路徑或可驗證故障；不建立產品工作。 |
| Issue #3710 | 採用候選（限縮，未實作） | 實讀 `/v1/responses` handler 只以 `instructions`／`input` 轉換，mapper 僅處理既有 `response_format`；issue 指出的 `text.format` 沒有現成映射。既有 strict tool-schema 候選不等同 Responses Structured Outputs 修復。另立 protocol batch：`text.format=json_schema` mapping、tools 共存的上游契約、裸 JSON／錯誤回傳及 `reasoning.effort` 尊重需各自以安全 payload fixture 證明；不得用提示詞注入或不加條件地剝除 code fence 假裝相容。 |
| Issue #3711 | 延後 | 本文為空，只要求新增 AI Studio channel；沒有 upstream protocol、帳號/credential 來源、授權條件或錯誤樣本。需先有 provider onboarding、credential storage/redaction、rate-limit／fallback、Windows/headless parity 與 ToS/使用者同意設計，才可建立採用批次。 |
| Issue #3713 | 監看 | 僅報 beta 後 503 截圖；唯一 maintainer 留言稱 Google 故障，沒有 request ID、sanitized response、endpoint 或時間線，不能套用 PR #3714 的 watchdog/冷卻。待有去敏 transport trace 能分出上游 503、網路 timeout 與本地 retry 後才重開。 |
| Issue #3715 | 延後 | 回報 `gemini-3.8-flash-high` 404，但留言同時出現 bare/tiered/high 不同 model；fork已有 `variant_mapping.rs` 的 high 解析測試、`model_mapping.rs` 的 bare→tiered 路由，且 `constants.rs` 記錄舊 UA 對 segmented model 的 404 gate。需去敏 model、實際 URL/404 body、UA/version、warmup 與正常 request 的矩陣，才能判斷 catalog/UA/route 修正，不能依 screenshot 改 mapping。 |
| Issue #3716 | 延後 | 回報 Gemini stream 缺 `finish_reason` 後 client 續步導致 400；fork `openai/streaming.rs` 已在無 `finishReason` 時輸出 `upstream_interrupted`，而非偽造 `stop`。須保存最小 SSE sequence，驗證 text/tool/empty 結尾在 Chat 與 Responses 的 terminal event、client continuation 與不重複上游 request；不得因 issue 直接把未知結尾標為成功。 |
| Issue #3717 | 監看 | 多帳號、官方 desktop 與不同模型的觀察指向上游容量或帳號路由，但沒有可安全取得的 provider 契約；不把「換帳號」或「切出口」寫成產品修復。若採 #3714 類 timeout 工作，仍須先通過獨立 fake-transport 與帳號隔離 acceptance。 |
| Issue #3718 | 監看（既有較強防護） | Claude 第二輪的 `thinking` signature 400 未附完整去敏 payload；fork已在 Claude handler 前置 `filter_invalid_thinking_blocks_with_family`，並針對 signature error 清除／重試，另有 retry-strategy fixture。需針對目標 provider family 的去敏雙輪 thinking/tool transcript，驗證保留有效 signature、移除 foreign/corrupt signature、無跨 session leakage 與 bounded retry，才判斷是否仍有缺口。 |

> 本節沒有採用產品 patch；#3708／#3710 僅是明確限縮的未實作候選。清單外新項目（目前 PR #3708/#3714 與 Issue #3709/#3710/#3711/#3713/#3715/#3716/#3717/#3718 以外）留給下一個授權批次，不在此追逐。

### 0.16 2026-10-21 新增 Issue #3719／#3720（本輪候選）

- **來源與界線**：上游僅讀。#3719 本文請求 system tray 顯示剩餘 quota percentage，無留言；#3720 為 rate-limiting investigation，主張直接 provider 測量、短暫 cooldown、prompt identity 字樣與 client backoff，無留言。這些觀察不是已驗證的 provider 契約，未作 live provider 測試，亦未採用任何產品 patch。
- **fork 對照**：`src-tauri/src/modules/tray.rs` 已以 active account 的既有 quota 資料顯示 `Gemini High`、`Gemini Image`、`Claude 4.5` 三個具名百分比；尚無「所有 quota」的單一權威定義。`prompt_sanitizer.rs` 只以泛用 identity marker 正規化、保護 code block；`token_manager.rs` 已有 account/model rate-limit circuit breaker、retry reason 與 zero-quota 同步，沒有可證實採用上游 1M–1.5M／2–3 分鐘門檻的 provider 契約。

| 項目 | 判定 | 實際差異、風險與重新開案條件 |
|---|---|---|
| Issue #3719 | 已存在（限縮）／監看 | 現有 tray 已顯示三個具名模型的 active-account 百分比，故本文沒有證明新的通用功能缺口；也不能把不同模型／5h／weekly bucket 假定為同一「remaining quota」。若回報者指定目標 bucket、active-account 權威、Windows tray refresh／文字可及性與 reset 顯示，並補 stale／missing／forbidden quota fixture 與不洩漏帳號資料的驗收，再獨立評估 UI batch。 |
| Issue #3720 | 延後（拆分） | report 的 endpoint、TTFT、identity 分數與 cooldown 是未驗證的外部觀察，不足以設定自動輪替或 provider backoff。沒有證據就不建立 identity 重寫工作；若未來有獨立需求，僅能評估不冒充 provider、也不繞限速的 redaction／語意保持設計，且須保留 user／system／tool 內容與 code block。token sliding window、auto-rotation 與 backoff 則需 fake-clock、429/503/timeout、manual/proxy-disabled、cross-account/session isolation、quota reset、persisted retry 與 no-secret-log fixture；不得以報告中的門檻或「換帳號」直接改行為。 |

> 這兩項完成後，候選水位僅表示上述 release 與 12 tickets 已審；它**不採用** #3708／#3710／#3720 的產品工作，也不消除 #3703／#3706、403 expiry、validation URL、Responses 雙入口或其他已列未實作缺口。

### 0.17 2026-10-21 新增 Issue #3721／#3722（本輪候選）

- **來源與界線**：上游僅讀。已保存兩份 issue 本文與各一則留言於 `checkpoint-tailwind4-braces/upstream-3722-review/issue-{3721,3722}.json`、`issue-{3721,3722}-comments.json`。本輪沒有採用上游 beta 程式碼、執行 provider 請求或變更產品。
- **fork 對照**：#3721 是三個互不等價的主張，不能以同一個「已修」結論覆蓋：fork `modules/process.rs` 只以 `current_exe().canonicalize()` 排除完全相同路徑，AppImage mount path／Linux `comm` 截斷沒有對等驗收；fork 有 `--headless` 啟動路徑，但 Desktop GUI 與 proxy 仍同一 process；`pipeline/inbound.rs` 會保留 `toolConfig.functionCallingConfig.mode=ANY`，現無 Claude forced-tool 與 `thinkingConfig` 的明確相容性證據。

| 項目 | 判定 | 實際差異、風險與重新開案條件 |
|---|---|---|
| Issue #3721 | 分拆延後：Linux process identity、Wayland lifecycle、Claude tool-choice/thinking 各自獨立 | **自身誤識別**：AppImage 報告的 `/tmp/.mount_*` 與使用者安裝路徑可使 fork 現行 canonical-path 排除失效；上游 beta `cae8fb9b` 的集中 name blacklist／Linux 15-byte `comm` 規則有關聯，但不能未驗證地套到 Windows／macOS process discovery。需可注入的 Linux process snapshot，覆蓋 manager 名稱、多版本 AppImage path、截斷 `comm`、真正 IDE 與 restart path，並證明不 kill/spawn 自身。**Wayland**：fork 既有 `--headless` 可作明確替代啟動模式；上游以 GDK log hook 攔截 `_exit`／`pthread_exit` 讓 GUI 主執行緒退出，涉及 Linux signal/thread lifecycle，不能聲稱保住既有 stream 或直接移植。若 owner 要求 Linux desktop resilience，應先評估可監督的 proxy/GUI process isolation、display-loss／SIGTERM／stream-drain fixture，而非攔截 process termination。**forced tool + thinking**：fork 正規化 `ANY` 但目前沒有剝除 `thinkingConfig`；此為有限採用候選，僅在有 Claude 目標模型、OpenAI required/specific tool-choice→Google `ANY`、thinking on/off、一般 tool mode、上游 400 分類與 bounded no-duplicate-retry fixture 時，於獨立 pipeline batch 評估。 |
| Issue #3722 | 監看／拒絕自動化恢復 | 本文明確是單一使用者觀察、未確認因果；留言更稱節點／IP 無關並猜測 Google 機房。沒有帳號、provider、rate-limit 或 identity 契約可支持反覆短 prompt、換身份、切出口、自動 warmup／rotation 或繞限速。fork 既有 account/model circuit breaker、manual/proxy-disabled 與 quota 保護不得因個例放寬。只有取得授權且去敏的 transport timeline（DNS/connect/TTFT/response/429/503）、帳號狀態與 fake-clock no-extra-request／fail-closed fixture 後，才可評估診斷，不建立 recovery 自動化產品工作。 |

> 兩項均為已讀治理結論，**不表示上游 beta 修復已採用或 fork 已修復**。在 strict 重跑未顯示額外未審項目後，候選 issue watermark 才由 `#3720` 推進至 `#3722`；release／commit／PR 水位不變。

### 0.18 2026-10-23 新增 PR #3723／#3725 與 Issue #3724、#3726–#3733（本輪候選）

- **來源與完整範圍**：上游僅讀。PR #3723：base `2560efcd7a1bdf8a132bfd6c8a7b7def4dc4b561`、head `4599e67fe6e03cbf0c93b2e59a7104f2617205ad`、3 檔完整 diff `+263/-9`（SHA-256 `1528149483EBDDAB6986DE948CE9615D3CDE7F09BB96E14F7B20EC81B735E2BC`）。PR #3725：同一 base、head `b917ff01261a517ac1edceedd1b138c7a988feb7`、29 檔完整 diff `+1387/-13`（SHA-256 `C224C1BD511FAB0CC483EA0DF6ADF5B228757D1CE65226DDBC4018377A19F76E`）。九份 issue 原文與必要留言已實讀；來源 URL、metadata 與 fork 對照／命令記錄見 `checkpoint-tailwind4-braces/upstream-3733-20261023/evidence.json`。
- **水位語意**：以下的「採用候選」只排入獨立、可驗證產品批次，**不是**產品已採用、合併或已修復；release／commit 基線仍為 `v4.9.8`／`9f8e4f24…`。

| 項目 | 判定 | 實際差異、fork 對照與重新開案條件 |
|---|---|---|
| PR #3723 | 延後／整併既有 #3710 Responses 候選 | 3 檔新增 `text.format`→`response_format`、Gemini `responseSchema` 時直接移除 tools，並對任何可解析 JSON 的 Markdown fence 改寫輸出。fork `mappers/openai/models.rs` 尚無 `text`，`request.rs:1198` 只處理既有 `response_format`；但無條件刪 tools 與改寫文字都會改客戶端語意，不能直接採。需獨立驗證 `text.format` json_schema/json_object、tools 共存時的實際 provider 契約、raw schema gate、stream/non-stream 與僅在確定 Structured Output 合約下的 fence 行為。 |
| PR #3725 | 延後／不採本 patch | 29 檔加入 QoSGuardian、TTFT 同儕基線、2–30 分鐘階梯、synthetic「你好」探針、session 強制解綁、排程過濾及 UI／12 locale；其中探針會真實出站，且選擇／sticky／cooldown 契約均與 fork 不同。#3720 的調查沒有可驗證 provider 因果，不能以慢回應自動換帳號、重試或繞過限速。若 owner 要求 QoS 診斷，先以 fake clock／transport 證明 TTFT、全域慢、thinking、429/5xx、手動停用、session isolation、無額外 provider 請求與明確 opt-in。 |
| Issue #3724 | 監看／拒絕自動化規避 | 僅多位使用者的封號／申訴經驗，沒有去敏 provider response、帳號狀態或可重現因果。不得因而加換身份、切出口、重試、暖機或申訴流程；僅在有遮罩 403 契約與 fail-closed fixture 時評估帳號狀態分類。 |
| Issue #3726 | 採用候選（限縮效能批次，未實作） | fork `modules/account.rs:2429-2492` 仍以 semaphore 包住但 `join_all(tasks)` 全量建 future，`BackgroundTaskRunner.tsx:21-31` 仍在 auto-refresh 啟用時立即刷新；這支持檢查受控並發／寫入放大，但不證明回報的 7.2GB。需獨立以假 storage／IPC 量測有界 in-flight、index coalescing、取消／失敗與既有帳號讀寫，禁止藉此增加 provider 重試或改身份。 |
| Issue #3727 | 拒絕（產品範圍外） | RFC 是跨 Claude/Pi/Codex 的前置意圖重寫／動態 skills router；上游 maintainer 也明示不在本 gateway 實作。fork 維持協定代理界線，不注入或重寫使用者意圖／權限。 |
| Issue #3728 | 延後／需 Linux 重現 | Ubuntu 22 回報 GUI 被置前／自行啟動及 App/CLI 帳號互相影響，沒有 target、process tree、Wayland/X11 或 config。fork `integration.rs:257-392` 對 Windows Classic/IDE 有明確 target、hot-switch fallback 與 restart 順序；不可用 Linux 個案改其 process lifecycle。需 Linux process/display trace 與 Classic/IDE/CLI target matrix 後才獨立評估。 |
| Issue #3729 | 採用候選（安全限縮，未實作） | fork `middleware/auth.rs:120-137` 僅讀 Authorization／x-api-key／x-goog-api-key，故 Gemini `?key=` 相容性缺口存在。query secret 會擴大 URI、monitor、redirect 與 duplicate-key 風險；需 header 優先序、嚴格 percent decoding／重複 key 拒絕、全路徑 redaction、auth-off UserToken 不誤記錄與 Gemini 成功／錯誤 fixture，才可加入最小 extractor。 |
| Issue #3730 | 採用候選（最小熱更新，未實作） | `server.rs:1779-1860` 的 `admin_save_config` 更新多個 state 卻未寫 `only_raw_quota_models`；同檔 `AxumServer::update_only_raw_quota_models` 已存在（:427-432）。獨立小批需驗證 Web save false→true→false 立即影響所有 model-list 路徑、失敗不留下 disk/memory 分歧，且不重啟或改動帳號選擇。 |
| Issue #3731 | 監看 | 只有未證實模型／帳號可用性與圖片，沒有 model ID、API response 或 provider capability。不得硬編 catalog、UA 或路由；有授權的遮罩 discovery payload、tier／quota／404 矩陣後才重開。 |
| Issue #3732 | 拒絕／已澄清 | 需求在留言中已澄清為 Gemini 本來支援多帳號、不同 Client ID 要另設 refresh token，issue 已關閉。沒有可採的 fork 行為差異；不把跨 client credentials 合併或自動同步。 |
| Issue #3733 | 監看／fork 已有 Claude 隔離 | 回報主張 Claude parallel tool call 被注入 Gemini sentinel；fork `pipeline/inbound.rs:240-304` 僅對 Gemini 填／替換 functionCall signature，Claude 分支反而移除它，`thinking_store.rs:886-931` 也明定 Claude tool call 不帶 sentinel。沒有去敏雙工具 transcript 證明繞過該路徑；需最小 Claude parallel function-call/retry transcript，驗證首簽名、無 sentinel、session isolation 與 bounded retry 後才考慮修補。 |

> 本輪 11 項均已逐項讀取；在 strict 重跑僅顯示這些固定項目後，候選 PR／issue watermark 才分別由 `#3714`／`#3722` 推進至 `#3725`／`#3733`。此水位只表示覆核完成，絕不表示上述採用候選或上游 beta patch 已進入 fork。

### 0.19 2026-10-23 新增 Issue #3734／#3735（本輪候選）

- **來源與界線**：上游僅讀；兩份完整 issue API 原文與留言快照保存於 `checkpoint-tailwind4-braces/upstream-3735-20261023/issue-{3734,3735}.json`、`issue-{3734,3735}-comments.json`。本輪未呼叫 provider、未寫入模型目錄，也未採用上游 development branch 的未定版修復。

| 項目 | 判定 | 實際差異、風險與重新開案條件 |
|---|---|---|
| Issue #3734 | 監看／不硬編模型淘汰 | 上游 maintainer 公告未來會淘汰 `<3.8` 舊模型、`gemini-3.1-pro` 與部分 Gemini/GPT 型號，但沒有 release SHA、正式 catalog、截止日期或 provider response。fork `modules/quota.rs:431-434` 已明定 `fetchAvailableModels` 對 free/Pro 都回傳同一全量目錄，不能把它當帳號 capability；`model_specs.rs` 仍有 3.x alias／tier 規則。未經授權的名稱刪除會破壞 custom alias、既有設定及 provider 實際仍可用的模型。只有取得版本化、去敏的官方 catalog／404 contract，以及 custom alias precedence、冷啟動／stale entry、model-list 與未知模型 fallback fixture 後，才以獨立 catalog 批次評估。 |
| Issue #3735 | 延後／需可驗證的 Argon provider 契約 | issue 原文把可見 thought 輸出歸因於 `thinkingBudget`／`thinkingLevel` 組合；留言稱 upstream development branch 已改為 level-only，但沒有可採的 commit SHA 或可公開 provider 契約。fork 現行 `pipeline/inbound.rs:467-534` 在 Client mode 已互斥輸出正數 budget 或 level、`551-583` 在 Gateway mode 依既有 specs 注入；對 `src-tauri/src` 與 `src-tauri/resources` 的 `argon` 搜尋無命中，故不能宣稱已支援其模型或把 `-1`／`HIGH`／`10001` 硬編成通用規則。須先取得授權且遮罩的 Argon capability snapshot（model ID、tier、允許欄位與 400 contract），再以三協定的 high/medium/low/base、budget/level mutual exclusion、stream/non-stream visible-thought 與未支援模型 fallback fixture 評估；LOW/MEDIUM 靜默行為不可被擅自升級為 HIGH。 |

> 兩項均為已讀治理結論，**不表示模型淘汰、Gemini 4 Argon 支援或 thinking 修復已進入 fork**。strict 重跑只確認這兩項且未出現額外未審項目後，候選 issue watermark 才由 `#3733` 推進至 `#3735`；release／commit／PR 水位不變。

### 0.20 2026-10-23 新增 Issue #3736（本輪候選）

- **來源與界線**：上游僅讀；完整 issue API 原文與空留言陣列保存於 `checkpoint-tailwind4-braces/upstream-3736-20261023/issue-3736.json`、`issue-3736-comments.json`。本文只有截圖與「Only Ultra will appear」，沒有版本、平台、入口、帳號／quota payload、預期清單或復現步驟；本輪未猜測圖片中的頁面或修改 model／tier 邏輯。

| 項目 | 判定 | 實際差異、風險與重新開案條件 |
|---|---|---|
| Issue #3736 | 監看／資料不足 | fork 至少有兩個不等價的相關層：`pages/Accounts.tsx:36,258-263,888-906` 是帳號 tier（all/pro/ultra/free）篩選；`components/accounts/AccountCard.tsx:102-131` 則依 `showAllQuotas`、pinned selection、預設模型與過期模型 filter 決定卡片 quota 顯示。單張截圖無法辨識是 tier filter、quota refresh、`only_raw_quota_models`、pinned selector、帳號狀態或前端 stale state，直接調整其中任一路徑可能使正常 Pro/Free／模型顯示倒退。需提供 fork/upstream 版本、OS、具體頁面／filter、至少兩個不同 tier 的遮罩 account quota payload、refresh 前後 state、預期與實際 model/tier list；之後以該最小 fixture 驗證 all/pro/ultra/free、pinned／raw quota toggles 與 reload，才可拆出產品批次。 |

> 本項為已讀治理結論，**不表示「Only Ultra」已重現、已定位或已修復**。strict 重跑只確認 #3736 且未出現額外未審項目後，候選 issue watermark 才由 `#3735` 推進至 `#3736`；release／commit／PR 水位不變。

### 0.21 2026-10-23 新增 PR #3738／#3740 與 Issue #3737／#3739（本輪候選）

- **來源與完整範圍**：上游僅讀。PR #3738：base `08fc86e71c28fbac0acb8b8f1a2a7b73973ee5ff`、head `304f445b1f09dac359e29d2d4156da3263c8ff56`、32 檔完整 diff `+1904/-563`；PR #3740：base `fe2ace7dc1d02332785a3d612cec2aa020be7136`、head `447490f930841f27b0b04cffa98a07ac99064aa3`、1 檔 `+51/-10`。兩份 PR metadata／固定-head diff、兩份 issue 與全部留言 API 快照已保存於 `checkpoint-tailwind4-braces/upstream-3736-20261023/`；本輪不執行 provider、Tauri UI 或 Rust runtime。

| 項目 | 判定 | 實際差異、fork 對照與重新開案條件 |
|---|---|---|
| PR #3738 | 逐元件延後／不採 bundle | **Mini/macOS/tray**：`Cargo.toml` 開啟 `macos-private-api`，另改 `tauri.conf`、capability、`lib.rs`、tray、commands、MiniView、stores 與跨窗 theme/language event；其中 `exit_app` 會停止 cloudflared/admin/proxy 後 `process::exit(0)`，tray focus/blur/pin 又改所有平台點擊行為，不能以 UI 功能名採用。需 macOS/Windows tray click、focus/blur、pin、main/mini restore、child shutdown/credential-write ordering、theme/language persisted sync 與 permission acceptance。**Claude pipeline**：上游清 sentinel、丟無簽名 thought、clamp budget；fork `thinking_store.rs:891-895,984-986,1569-1688` 已按 Claude signature 隔離，不能以新 clamp 覆寫既有模型／provider budget 契約。需跨 Gemini→Claude、valid/invalid signature、512/1024/disabled thinking、400 與無重複 retry fixture。**Quota/UI/locales**：GroupQuotaCard 細線、847 行 MiniView 與 12 locale 是完整 UI 設計變更；fork僅保留 `zh-TW`，上游同時改 `zh.json` 不可直接併入。各元件均待獨立 visual/a11y/localization acceptance，未採任何產品碼。 |
| Issue #3737 | 採用候選（限縮 quota fallback，未實作） | fork `modules/quota.rs:387-413` 只有 `quotaInfo` 存在時才加入 `quota.models`，後續 `443-511` 的 group bucket 融合只迭代既有模型，故「模型存在但缺模型級 quotaInfo」確有被丟棄的路徑。可獨立修為：保留公開可用模型、僅在缺模型級資料時依已驗證 group bucket 產生未知／受限 quota；**明確 0 的模型額度必須優先**，兩種資料都缺時不得虛構正額度或繞過 disabled/forbidden。需遮罩 `fetchAvailableModels` fixture 覆蓋 Gemini/3P、5h/weekly 最嚴格 bucket、模型不存在、explicit zero、group 缺失、落盤/reload、selection 503→真 upstream 429 與 UI raw-model list。 |
| Issue #3739／PR #3740 | 延後／stream lifecycle 獨立批次 | #3740 唯一改動把 `stream_lifecycle.rs` 的一次 Transition 寬限從「Thinking 且無 content」擴成任何 Thinking/Streaming 靜默；可避免 preamble 後巨型 tool JSON 被 120s 截斷，但也讓所有已輸出正文的協定多等一個 transition window，可能延後真斷線、取消或錯誤回報。不可只以 synthetic 文字 chunk 採用。需 fake-clock/transport 覆蓋 Claude/OpenAI/Gemini 的 thinking、preamble+巨型 function call、普通正文 idle、upstream 4xx/5xx、client cancel、heartbeat、單次 finalize、無 synthetic `end_turn` 與 bounded overall deadline。 |

> 四項均為已讀治理結論，**不表示 quota fallback、mini 視窗、Claude budget/sentinel 或 streaming 寬限已採用或已修復**。strict 重跑僅在固定項目無新增時，候選 PR／issue watermark 才分別由 `#3725`／`#3736` 推進至 `#3740`／`#3739`；release／commit 水位不變。

### 0.22 2026-10-24 新增 PR #3741（本輪候選）

- **來源與完整範圍**：上游僅讀。PR #3741：base `fe2ace7dc1d02332785a3d612cec2aa020be7136`（`beta`）、head `b0fa557c0c1a2e4a0876eabc852dc599b68721d8`，1 檔 `src/App.tsx`、`+22/-2`；PR API 原文、空 comments API 與固定-head diff 已保存於 `checkpoint-tailwind4-braces/upstream-3736-20261023/pr-3741-api.json`、`pr-3741-comments.json`、`pr-3741.diff`。

| 項目 | 判定 | 實際差異、fork 對照與重新開案條件 |
|---|---|---|
| PR #3741 | 延後／依賴未採用的 Mini 視窗架構 | 上游在 `App.tsx` 為 `index.html?mini=1` 與 `__TAURI_INTERNALS__` 加入首幀 Mini 判定，並將 router error、`index.html` 和所有 `*` path redirect 至 `/`。fork 的 `src/App.tsx:25-169` 僅有主 application router，搜尋不到 `isMini`、`MiniView`、`__TAURI_INTERNALS__` 或 `getCurrentWindow`；`src-tauri/tauri.conf.json` 也沒有對應 Mini 視窗入口。因此不能只搬 redirect：它會把既有未知路徑與 route error 隱藏為 Dashboard，且沒有修復不存在的 multi-window 載入鏈。若日後獨立採 Mini 視窗，先定義 `index.html?mini=1`／window label 的 Tauri contract，再以初次載入、重載、主／Mini 視窗切換、未知 path、真 route error 與非 Tauri browser fallback 測試決定是否採用同步判定或 error boundary；本輪不採產品碼。 |

> 本項為已讀治理結論，**不表示 Mini 404 已在 fork 重現、已採用或已修復**。strict 重跑只在固定 PR 且無新增項目時，候選 PR watermark 才由 `#3740` 推進至 `#3741`；issue／release／commit 水位不變。

### 0.23 2026-10-24 新增 PR #3742（本輪候選）

- **來源與完整範圍**：上游僅讀。PR #3742：base `95ba737d097f1ef0080c86aa17ba38584f0b8546`（`beta`）、head `91c342c68f147968182fd7e80a2b6af0bd015b1c`，1 檔 `src-tauri/src/proxy/model_specs.rs`、`+1/-10`；完整 PR API、空 comments API 與固定-head diff 已保存於 `checkpoint-tailwind4-braces/upstream-3742-20261024/pr-3742.json`、`pr-3742-comments.json`、`pr-3742.diff`。

| 項目 | 判定 | 實際差異、fork 對照與重新開案條件 |
|---|---|---|
| PR #3742 | 延後／不採用 beta `-1` 語意 | 上游把 Claude low/medium/high 與未分級模型的**負數**設定由 `Some(-1)` 改為 `None`，讓其 beta pipeline 從官方 model struct 取預設（宣稱如 1024）；0 本來已回 `None`，不由此 hunk 改寫。fork 的 `model_specs.rs:421-470` 已有不同的 `claude_mode`、effort／suffix 分級與未分級 `claude_budget`／`claude_high` precedence；`config.rs:557-564,611-621` 的本地預設又是 budget 16384、low 1024、medium 4096、high 16384，與上游聲稱的單一 fallback 不同。fork 的 `pipeline/inbound.rs:543-583` 只有 resolver 回 `None` 時才依 tiered 規則不注入數值，因此整合負數 fallback 前仍須連同未分級 Claude 與 client/gateway authority 驗證 request body；0 與正數則是 config migration 的不回歸邊界，不是本 patch 的直接改動。#3735 也仍缺 Gemini 4 Argon／provider 400 capability contract，不能以 beta 單測把未公開的官方模型預設當跨協定事實。需遮罩 model-spec snapshot 與 config migration fixture 覆蓋 low/medium/high/未分級、`-1`/0/正數、client/gateway、三協定、maxOutputTokens guard、unsupported model 400/no-retry；確認本 fork 應否保留 16384 等既有值後，才以獨立產品批次評估。 |

> 本項為已讀治理結論，**不表示 Claude／Argon default budget 已在 fork 採用、回退、重現或修復**。strict 重跑只在固定 PR 且無新增項目時，候選 PR watermark 才由 `#3741` 推進至 `#3742`；issue／release／commit 水位不變。

### 附錄 A. 2026-10-19 歷史停點：coverage 與第一批 12 commits（保留歷史證據，計數已過期）

- **範圍**：`b601f5e7375343343922ccab5e50653bc3450c3b..9f8e4f24d61b33fa80b8078d104c942752a5fa0b`；162 commits、161 files、53,124 additions／26,597 deletions。完整 SHA coverage、每筆檔案／patch 的可重現 Git 指令、既有 exact-patch 證據映射，保存於 `checkpoint-tailwind4-braces/upstream-v498-release-review/coverage.json`。
- **誠實水位**：只建立 release coverage；`reviewed_release` 仍為 `v4.9.6`、`reviewed_through` 仍為 `b601f5e7375343343922ccab5e50653bc3450c3b`。本節不表示 `v4.9.8` 已完整覆核，更不表示產品採用。
- **覆蓋計數**：12 筆本批實讀、15 筆由既存保存完整 PR diff 精確映射、135 筆待覆核（合計 162）。三個 merge commit 亦列入 coverage；不以 merge 標題取代實際父 patch 審查。

| SHA | 判定 | 實際 diff／fork 差異與重啟條件 |
|---|---|---|
| `5ec12dbc70fab00347a7dc4d0d715d23994644bd` | 延後 | Claude SSE 斷流統一為 `overloaded_error`；fork 有輸出型別但無 malformed-SSE／自動重試 parity。需該 transport fixture。 |
| `7838fc07a1c802fe412bb792411b1b2394ae1398` | 監看 | #3613 merge metadata；實際內容由前一 SHA 覆核，不能以 merge 標題當採用證明。 |
| `29694adc1f96f227b072aa24171e775aa587b758` | 延後 | Linux 透明視窗／DMABUF 調整；fork 仍保留 Linux override 與 transparent 設定。需 Linux/Wayland 圖形驗收。 |
| `4638fe20df9fc95bc5346857a9af1abe92a2e2e4` | 拒絕 | 上游 beta release/version metadata，不適用 fork release channel；僅另行 release packet 才評估。 |
| `f73c290b1f5869d70063cb585b2bd3109a1f6ec2` | 延後 | Cowork estimator／180 秒 first-byte 與 compaction；需遮罩 50-turn、tool schema、thought signature、timeout/rotation fixture。 |
| `365be17fb660c8162b2678af6c55ac026a36dfb2` | 延後 | Cowork/Code header、system blocks、model tier；需 desktop host＋multi-block routing fixture。 |
| `e779db902c70eb885dbfb4a7817c5d8f074be1c0` | 延後 | Cowork/Code 與 continuation state 分離；需 compaction continuation state-machine fixture。 |
| `c8dc07cdd0523acba6929db1dd88c9a476c672b1` | 延後 | generic CLI header classifier；需 sanitized desktop/wrapper header fixture。 |
| `d1fb5839fb3c53420ef9a91ad09796a9b775afe8` | 拒絕 | 僅為未採 Cowork patch 的 format follow-up；若父功能採用才重開。 |
| `9206be9e53b05ed71d87aa4b7fc58304ac3b65f2` | 延後 | quota-400 bisection trimming 耦合 compaction；需可重現 upstream 400＋trim/rotation fixture。 |
| `7da339e6acf7343dbf2576fbfba4dadebaf70d42` | 延後 | signature/cache invalidation；fork 的 session/cache 隔離尚未驗證。需 tenant/session deletion、continuation、400 fixture。 |
| `e6c09fc54f3f5496b6e71b1201dc6bed1ad34aba` | 延後 | Claude Desktop 3p header 調整；需 sanitized 3p header classification／bypass-regression fixture。 |

> 本批沒有採用上游產品 patch。既有 exact-patch 映射僅證明先前已讀相同上游 diff；它不等於 fork 已採用。特別是未發布 schema 候選與 #3703/#3706 proxy 風險均仍未修復。

## 0. 2026-10-18 上游增量覆核：PR #3685–#3706／Issue #3684–#3707

- **範圍與證據**：逐筆閱讀 `lbjlaq/Antigravity-Manager` 的保存 metadata、完整 PR diff、issue 本文與當時留言：`checkpoint-tailwind4-braces/upstream-3684-3707/{pr-*.json,pr-*.diff,issue-*.json,issue-*-comments.jsonl}`；並對照本 fork `140d663a` 的實際程式路徑。此節的「採用候選」僅表示應另立有限批次評估，**不是已採用或已修復**。
- **不變基線**：release `v4.9.6`、commit `b601f5e7375343343922ccab5e50653bc3450c3b` 不變；本輪只在完成每筆覆核後更新 PR／issue review watermark。

### 0.1 Pull requests

| 項目 | 判定 | 實際差異、fork 對照與可驗證後續條件 |
|---|---|---|
| PR #3685 | 延後：獨立 retry/backoff 候選 | diff 改 `modules/quota.rs`、`handlers/common.rs`、`token_manager.rs`、`upstream/client.rs` 等的零秒 reset／冷卻策略；fork 仍有 `proxy/rate_limit.rs` 的 optimistic reset 與 `token_manager.rs` 呼叫。Issue #3684 後續留言已撤回「新二進位直接造成 429」的因果判定，不能以此直接移植。需先有 fake-clock、多帳號、429 分類與 endpoint health fixture。 |
| PR #3687 | 延後：小型相容候選 | diff 只改 `handlers/common.rs`，使 `extract_retry_after_seconds` 接受小寫 `wait`；fork 現有同函式及測試僅覆蓋 `Wait`。需有實際小寫訊息 fixture 與輸出 `Retry-After` header regression，才可採用。 |
| PR #3689 | 延後 | diff 跨 Claude request/handler/stream、collector 與 `prompt_sanitizer.rs`，含 Claude Code 2.1.301 協定及 WAF 推論；fork 已有 `mappers/claude/request.rs`／`prompt_sanitizer.rs` 的 `CLAUDE_CODE_CLI_IDENTITY`。未保存可公開、無敏感資訊的 2.1.301 wire fixture 前，不移植多點協定與識別變更。 |
| PR #3691 | 拒絕目前 patch；持續觀察 | diff 移除 Prod endpoint 並調整 120 秒 stream watchdog；fork `upstream/client.rs` 仍有 `Prod` 與 `Sandbox → Daily → Prod` fallback。未重現 endpoint 問題，且 #3684 的 429 因果已被更正；須以 endpoint 400/429/5xx 與 first-byte 60/120 秒 fake transport fixture 證明。 |
| PR #3694 | 部分已有獨立候選，其餘延後 | diff 混合 schema、rate/retry、prompt、token 多項變更。`security/tool-schema-main-port` 有未合併的 strict schema 候選，採 typed depth/resource error；刻意**不**採 upstream `active_refs` 循環降級為 terminal object 的語意變更。該候選仍待獨立 review/owner CI；其餘 rate/retry/prompt 需逐項另立驗證。 |
| PR #3696 | 延後 | open diff 跨 auto-switch、DB、pipeline、stream lifecycle、token manager，涉及角色輪替與工具順序。需協定順序／失敗回復 fixture，不能由靜態 diff 直接採用。 |
| PR #3698 | 延後 | open diff 變更多長 context、quota/retry、signature 與多個 handler/collector。需提供 provider timing、quota 與 stream signature 的可重現 fixture，才評估。 |
| PR #3699 | 延後 | `handlers/openai.rs` 的 in-flight accounting／response path diff；PR 明示只是 static audit，沒有 production observation。需測試 stream/non-stream 並發與實際 route canonicalization。 |
| PR #3700 | 延後 | `session_manager.rs` 大幅變更 session fingerprint／multimodal；無可公開的穩定 fingerprint fixture，且不得以原始敏感 payload 作測試輸入。 |
| PR #3701 | 延後 | `mappers/claude/request.rs` 與 `upstream/client.rs` 處理 orphan tool result／最後 endpoint watchdog；需 malformed tool-result 與最終 endpoint hang 的 fake transport fixture。 |
| PR #3702 | 延後 | `model_specs`、`pipeline/inbound.rs`、`upstream/client.rs` 增加 countTokens／thinking config；公開 Gemini 文件不等於 fork 的 internal provider contract。需經核准的內部 request-shape/capability fixture。 |
| PR #3705 | 延後 | `pipeline/estimator.rs` 改 Cowork token estimator；需精確 Cowork compression/token-threshold fixture，現無契約證據。 |
| PR #3706 | 採用候選：高優先安全後續 | `proxy_pool.rs` diff 處理 authenticated proxy health、credentials 與 direct fallback；fork `proxy_pool.rs:133,195` 明確記錄路由至 `Direct`。須另立安全批次，以 fake authenticated HTTP/SOCKS proxy 驗證 health failure 不外洩至 direct、credentials 不可讀／不記錄、專用帳號不跨路由；未完成前不宣稱已修。 |

### 0.2 Issues

| 項目 | 判定 | 實際內容、fork 對照與可驗證後續條件 |
|---|---|---|
| Issue #3684 | 拒絕原因果主張；監看 | 本文稱 harness／identity 觸發 429，但後續留言確認舊 process 在 PR #3685 編譯／重啟前已成功 11 分鐘，不能把 binary/harness 當因果。fork identity sanitizer 僅證明轉換存在，不證明 provider enforcement；若重現，需遮罩 response 的受控比較，不能 spoof/bypass。 |
| Issue #3686 | 拒絕 | 是外部 agy shell wrapper 內硬編碼 `GEMINI_API_KEY="sk-"`／localhost 的提議，非 app 整合。除非 owner 要求該 CLI 功能且先定義安全 credential storage、Windows 路徑與 parity，否則不納入。 |
| Issue #3688 | 延後／監看 | thinking_budget RFC 與「已支援 custom budget」留言；fork `pipeline/inbound.rs` 有 client-direct 設定，但公開文件不足以證明 internal provider contract。需 capability/request fixture。 |
| Issue #3690 | 採用候選：獨立 schema 批次待驗收 | Docker OOM 報告的重複 schema flatten 是 strict schema 候選的風險來源；候選仍未合併，採 typed error 而非 silent fallback。需 fresh review、owner CI 與 in-memory depth/cycle/resource tests 成功後才可稱修復。 |
| Issue #3692 | 延後 | Gemini 4 Argon 的 reverse-engineered root-cause／beta 建議，缺 model discovery 與 request/response 契約；不得以猜測改 production。 |
| Issue #3693 | 拒絕作為整包修復依據 | 38 項 static audit 應逐項拆分；本輪只對已具體的 #3690 等個案建候選，不接受 blanket patch。 |
| Issue #3695 | 延後 | 六項 audit 對應 PR #3696；以該 PR 的協定與失敗回復 fixture 為採用門檻。 |
| Issue #3697 | 延後 | 十二項 audit 對應 PR #3698–#3705；各項須分別有可驗證契約，不能整包接受。 |
| Issue #3703 | 採用候選：高優先安全後續 | authenticated proxy health／direct fallback 風險與 fork `proxy_pool.rs:133,195` 的 `Direct` 路徑相符。需 isolation fixture 證明 proxy 失敗不改走 direct、credentials 不寫 log；未實作。 |
| Issue #3704 | 拒絕／不適用 | 地理定位錯誤已在留言中標示修正，屬多 NIC／soft-router 情境，沒有本 fork 可採用程式差異；僅在有目前 provider response 細節時再查。 |
| Issue #3707 | 拒絕：規格不足 | 本文僅「request adapt dsh」，沒有問題、契約、重現步驟或可驗收範圍；需完整 specification 後才重新開案。 |

**watermark 結論**：以上 24 項均已逐筆實讀，僅將 `reviewed_pr_through` 推進至 `3706`、`reviewed_issue_through` 推進至 `3707`；這些水位表示已覆核，絕不表示產品已採用。未實作的候選（特別是 #3690／#3694 schema 與 #3703／#3706 proxy）保留為獨立、可測試的後續工作。

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

移除決定與理由已記錄於 `FORK.md`「相依套件偏移」一節（依維護者指示，所有偏離上游 `package.json`/`package-lock.json` 的地方都要留痕）。

### 2.4 Dependabot alerts（`gh api repos/SanHsien/Antigravity-Manager/dependabot/alerts`）

- Dependabot security alerts 與 automated security fixes 已由維護者／主 session 開啟（本次覆核僅用 `gh api` 讀取驗證，未重新切換設定）：
  - `GET .../vulnerability-alerts` → `204 No Content`（已啟用）
  - `GET .../automated-security-fixes` → `200 OK`（已啟用）
- 目前列出的 alerts 全部是 **npm 生態系**（`colord`、`browserslist`、`mermaid`、`dompurify`、`vite`、`js-cookie`、`lodash`/`lodash-es`、`uuid`、`rollup`、`picomatch`、`yaml`、`esbuild` 等，多筆 `state: open`，少數 `auto_dismissed`）。這批是 Dependabot 對現有 `package-lock.json` 的历史掃描結果，與本次 `npm audit` 抓到的 24 筆有重疊但編號體系不同（Dependabot 以 GHSA 為單位、按套件版本區間列出，不會因為某個套件被移除就自動清空舊 alert，需要等下次掃描或手動 dismiss）。**這些 Dependabot alert 條目不在本次「待維護者決策」範圍內處理**，因為 `npm audit` 這條路徑（實際 lockfile 狀態）已驗證歸零；是否要手動到 GitHub UI 把 Dependabot 上對應已解決套件的 alert 標記為 resolved，留給維護者決定。
- **Cargo 生態系 alerts：目前 0 筆**（`gh api .../dependabot/alerts --jq '[.[] | select(.dependency.package.ecosystem=="cargo")] | length'` → `0`）。可能是還在建立索引（維護者在同一時段剛啟用），也可能 Rust 依賴目前沒有已知漏洞。本機沒裝 `cargo`，依指示不動 `Cargo.lock`/`Cargo.toml`。**列為待觀察的 open item**，之後重新查一次確認是否跑出新項目。

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
- 已確認 repo 內（README、`docs/`、`web_site/`、`package.json`、`src-tauri` 設定、`index.html`）沒有任何 `github.io` / Pages 相關連結需要改寫——`lbjlaq.github.io` 只出現在 GitHub repo 設定的 homepage 欄位，不在任何檔案內。該欄位已由維護者清空（`gh api repos/SanHsien/Antigravity-Manager --jq '.homepage'` 現在回傳空字串），本次覆核未變更任何 repo 設定，只是驗證這件事已完成。
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

### 24. Full gate 新增 upstream Issue #3683（2026-10-16）

本輪只讀取 [Issue #3683](https://github.com/lbjlaq/Antigravity-Manager/issues/3683) 原文與留言（metadata `comments=0`）。建議在 OpenAI tools parameters 轉 Gemini parameters 時，遞迴將 `{"const": x}` 轉換替換為 `{"enum": [x]}` 以相容 MCP 生態工具。

| 項目 | 判定 | 實際內容、fork 對照與後續 |
| --- | --- | --- |
| [Issue #3683](https://github.com/lbjlaq/Antigravity-Manager/issues/3683) | already-implemented／本 fork 已支援 | 經對照代碼庫，本 fork 於 `src-tauri/src/proxy/common/json_schema.rs:179-210` 早已完整實作 `const` 關鍵字規範化（`#3327`）：在 `clean_json_schema_recursive` 進入時即主動判斷 `map.remove("const")`，推導其型別（string/integer/number/boolean/array/object）並插入 `type`，同時將 `const` 值注入 `enum` 陣列（去重處理）。此功能已有完整單元測試覆蓋（`test_clean_json_schema_normalizes_const_to_enum`）。上游 issue 為社群建議提案，本 fork 無需重複變更產品代碼。 |

issue 水位提升至 `#3683`；PR 水位仍為 `#3680`，release／commit 基線不變。水位只表示 #3683 已讀取並比對 fork 行為，**確認本 fork 已具備該遞迴 const 轉換能力。**

### 25. 安全分支整併與 PR #41 驗收落地（2026-10-16）

完成對剩餘分支 `security/internal-warmup-loopback-20261015` 與 [PR #41](https://github.com/SanHsien/Antigravity-Manager/pull/41)（`fix(security): require process-local capability for internal warmup`）之排查與驗收：

1. **根因定位與測試修復**：
   - 失敗測試 `internal_namespace_boundary_and_noninternal_health_and_options_are_unchanged` 原先設定為 `ProxyAuthMode::Strict`，但在該嚴格模式下，非管理端點之 `/health` 必須攜帶 API Key，導致未帶 token 的探活請求被 `auth_middleware` 判定為 401 Unauthorized，使測試預期與實作者意圖不一致。
   - 將測試環境改為 `ProxyAuthMode::AllExceptHealth`，精確測試「非 internal 的 health 探活豁免與 OPTIONS 預檢不受 warmup 私有 capability 限制」，且計數器恰好自增 2 次，完全符合設計規範。
2. **CI 驗證**：
   - 推送至遠端後，GitHub Actions CI（Run ID: 38875911592 與 38875911620）共 10 項檢查全數綠燈 PASS（涵蓋 macOS、Ubuntu、Windows-2025 的 Rust 測試與 Tauri 打包，以及 CodeQL 安全掃描）。
3. **分支整併與清理**：
   - PR #41 於 2026-10-16 順利合併入 `main`。
   - 遠端分支 `security/internal-warmup-loopback-20261015` 與其餘 7 個已整併分支全數清理完畢，`origin` 僅保留單一主分支 `main` 與標籤/發布。

