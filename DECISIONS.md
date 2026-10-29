# 維護決策與上游觀察

## 2026-10-29：指定大檔的 fork 差異審查

審查起點為 `624ae2b61dff12271f21c3835a25b4d4e80faea2`；上游審查水位為 `v5.0.0 / 0067fe045ddd77f4462ea8e2c25384839d869f32`。fork 初始化快照為 `123a6ad2fe8ca7270e8c5a9bb205eac073853737`。

先保存與目前上游水位的 diff，再以 Git blob 追溯初始化快照的來源。下列檔案的初始化 blob 均能在上游歷史找到逐位元組相同的版本，因此可將初始化之後的 fork 改動與尚未採用的上游變化分開審查。

| 檔案（相對 `src-tauri/src/proxy/`） | 上游歷史來源 | 本輪審查範圍與結果 |
| --- | --- | --- |
| `token_manager.rs` | `9d69cdaa` | 逐行審核帳號目錄／ID 邊界、模型讀取與快取失效、背景持久化、驗證封鎖到期與配額分類的 fork 改動；未確認新缺陷。 |
| `handlers/claude.rs` | `d9122555` | 逐行審核移除 z.ai、工具 schema 閘門、重試錯誤狀態與圖片失敗恢復的 fork 改動；未確認新缺陷。下方另記繼承的計數功能限制。 |
| `handlers/gemini.rs` | `d9122555` | 逐行審核工具 schema 閘門、countTokens 與圖片失敗恢復；確認 snake_case 參數別名可漏過新增閘門。 |
| `handlers/openai.rs` | `f44209b8` | 逐行審核 HTTP／WebSocket／內部呼叫的 schema 閘門與圖片失敗恢復；未確認新缺陷。 |
| `pipeline/inbound.rs` | `f44209b8` | 逐行審核新增的借用驗證及清理函式；修補其 `parameters_json_schema` 遺漏。 |
| `handlers/common.rs` | `8495fce2` | 當前完整 blob `7661f193b533cf08e7a1670677c950971e3f0c0e` 與上游歷史相同，無 fork 改動。 |
| `handlers/warmup.rs` | `59072856` | 當前完整 blob `2908786c7355ea5ec87717d23088d5cbb1bad977` 與上游歷史相同，無 fork 改動。 |
| `thinking_store.rs` | `4fb0dc70` | 當前完整 blob `15201d6e9d00fac3b026dd78c73b0a15c0ed3dda` 與上游歷史相同，無 fork 改動。 |
| `handlers/mcp.rs`、`handlers/mod.rs` | fork 移除項 | 另逐行檢視 419 行 handler 刪除及 module 宣告移除；符合 README 已公告移除 z.ai 專屬 MCP 的政策，proxy 內已無其 handler 引用。 |

此範圍是 fork 差異審查；三份完整 blob 相同的檔案沒有宣稱完成整檔功能或安全驗收。

### FORK-001：Gemini 工具參數 schema 別名漏檢

- 輸入：`tools[].functionDeclarations[]` 或 `function_declarations[]` 下，以 `parameters_json_schema` 帶入超過既有深度上限的 schema；包括同時存在淺層 camelCase 別名的情況。
- 錯誤結果：新增驗證函式回傳成功；`countTokens` 會進到 state／帳號準備，而非在本機回 400。合法 schema 的該別名也漏過清理。
- 原因：fork 新增的 strict gate 只列 `parametersJsonSchema`、`parameters`。Google 的 [FunctionDeclaration proto](https://github.com/googleapis/googleapis/blob/master/google/ai/generativelanguage/v1beta/content.proto) 定義了 `parameters_json_schema`；[ProtoJSON 規則](https://protobuf.dev/programming-guides/json/#format-description) 接受原始 proto 欄位名稱。
- 決定：在原有兩個函式的欄位清單補齊 snake_case 別名，套用同一驗證與清理規則。
- 紅燈：先加三個 `tool_schema_proto_json_*` 測試，未改程式時全部失敗，包含禁止 state access 的 lazy factory 被呼叫。
- 驗證涵蓋：兩種 declaration 名稱、camelCase 與 snake_case 同時出現、拒絕後不改動原文、countTokens 提前回 400、合法別名清理。測試名稱符合現有 CI `cargo test --locked --lib tool_schema` selector。

### UPSTREAM-OBS-001：繼承的 Claude count_tokens 占位回應

- 可辨識輸入：一般 Google 路徑的 `/v1/messages/count_tokens`，例如 `{"model":"claude-sonnet-4-6","messages":[{"role":"user","content":"hello world"}]}`。
- 現況：fork handler 忽略 body，固定回 `input_tokens: 0`、`output_tokens: 0`。這會讓依該端點計算上下文用量的客戶端取得零值。
- 來源：[上游歷史 `d9122555`](https://github.com/lbjlaq/Antigravity-Manager/commit/d9122555) 的 Google 路徑已是相同占位行為；fork 移除 z.ai 分支未改變此路徑。當前上游水位 `0067fe04` 已改用 `pipeline::estimate_tokens`。
- 決定：依本輪「上游原有缺陷只記錄」的範圍保留程式，列入觀察。這是靜態確認，未使用真實帳號呼叫計數服務，也未將上游估算功能移植進 fork。
