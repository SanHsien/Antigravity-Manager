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
