# 安全政策 (Security Policy)

## 支援版本 (Supported Versions)

| 版本 | 支援狀態 |
| --- | --- |
| `main` 分支最新代碼 | ✅ 支援 |
| 最新正式發布版本 | ✅ 支援 |
| 舊版本 | ❌ 不支援 |

---

## 回報安全性弱點 (Reporting a Vulnerability)

若您發現本專案存在任何潛在的安全風險或漏洞，請**切勿**直接建立公開的 Issue。

請使用 GitHub 的私密安全漏洞回報機制進行通報：

**<https://github.com/SanHsien/Antigravity-Manager/security/advisories/new>**

您亦可前往倉庫頁面的 **Security** 標籤，點選 **Report a vulnerability** 填寫。

回報內容請盡可能包含：
- 受影響的版本與組件
- 重現步驟與概念驗證（PoC）
- 潛在影響範圍
- 建議之緩解或修復措施（如有）

維護者將在確認問題後評估影響並儘速發布安全性更新。

## 遠端自訂資料庫匯入

`POST /api/accounts/import/db-custom` 預設停用；伺服器管理者須設定
`ANTIGRAVITY_DB_IMPORT_DIR`，指向專用匯入目錄後才可使用。請把待匯入的
資料庫放在該目錄，請求 body 的 `path` 只填檔名，例如 `{"path":"state.vscdb"}`。
API 不接受絕對路徑、子目錄或 symbolic link；資料庫只以唯讀模式開啟。

目錄與檔案必須由可信任的管理者管理，不可允許其他使用者或遠端 API 寫入／替換；
匯入檔內含帳號憑證，應限制檔案權限，完成匯入後依管理者的保留政策移除。
設定此目錄即授權已登入的管理 API 匯入其中任何一般檔案，不要指向整個使用者家目錄。
桌面應用程式仍可用原有檔案選擇器直接選取資料庫，無須設定此環境變數。

The remote custom database API requires the server-owned `ANTIGRAVITY_DB_IMPORT_DIR`.
Send a filename only in `path`; absolute paths, nested paths and symlinks are rejected.
Keep this dedicated directory writable only by trusted administrators. Imports open SQLite
read-only. Desktop file selection remains available without this environment variable.
