# 本機帳號切換 API

讓本機腳本不用任何憑證就能列出帳號與額度、觸發切換帳號。**預設關閉**，由維護者在設定頁明確開啟。

> 這不是 8045 埠上的管理 API（`/api/*`，需要 Admin Token）。這是另一個只綁 `127.0.0.1` 的小型服務，預設埠 `19527`，沒有任何驗證。

## 風險

- 開啟後，**本機任何程式**都能列出帳號（含 email）並切換使用中的帳號，不需要憑證。
- 只綁 `127.0.0.1`，不接受區網連線。
- 瀏覽器網頁讀不到這個 API 的回應，也觸發不了切換：帶 `Origin` 標頭的請求，或 `Host` 不是 `127.0.0.1`／`localhost`／`[::1]` 的請求回 `403`（防止網頁跨站呼叫與 DNS rebinding）；不帶 `Origin` 的 no-cors GET 仍會得到回應，但網頁讀不到內容，GET 端點也不改任何狀態。`curl`、Python `requests`、PowerShell `Invoke-RestMethod` 這類腳本不受影響。
- 不需要時請關閉。

## 開啟與關閉

設定頁「進階」分頁最下方的「HTTP API 服務」：打開開關、確認埠號（1024–65535，預設 19527）、按「儲存」，然後**重新啟動應用**。設定只在應用啟動時讀取，儲存後不會立刻生效。

也可以直接編輯資料目錄（設定頁「進階」可看到路徑）裡的 `local_account_api.json`：

```json
{ "enabled": true, "port": 19527 }
```

- 檔案不存在、或沒有 `enabled` 欄位，一律視為**關閉**。
- 檔案內容無法解析時不啟動（不會退回預設值開啟）。
- 埠號被占用時伺服器啟動失敗，只在應用日誌留下 `[HTTP API] Failed to start server`；`GET /health` 連不上就是沒起來。

## 端點

Base URL：`http://127.0.0.1:19527`（埠號以設定為準）。回應皆為 JSON。

| 方法 | 路徑 | 說明 |
| --- | --- | --- |
| GET | `/health` | `{"status":"ok","version":"..."}`，確認服務有起來 |
| GET | `/accounts` | 所有帳號與額度快取 |
| GET | `/accounts/current` | 目前使用中的帳號 |
| POST | `/accounts/switch` | 切換帳號（非同步，見下） |
| POST | `/accounts/refresh` | 重新抓取所有帳號額度（非同步，回 202） |
| POST | `/accounts/:id/bind-device` | 綁定裝置指紋（與切換無關） |
| GET | `/logs` | 反代請求日誌 |

### `GET /accounts`

```json
{
  "current_account_id": "…",
  "accounts": [
    {
      "id": "…",
      "email": "…",
      "name": null,
      "is_current": true,
      "disabled": false,
      "live_limited_models": {},
      "quota": {
        "models": [{ "name": "…", "percentage": 80, "reset_time": "…" }],
        "updated_at": 1790000000,
        "subscription_tier": "PRO"
      },
      "device_bound": true,
      "last_used": 1790000000
    }
  ]
}
```

- `quota.models[].percentage` 是**剩餘**百分比（0–100），`reset_time` 是重置時間。
- `quota` 為 `null` 表示還沒抓過額度。額度是快取，要最新值先 `POST /accounts/refresh`，再輪詢 `GET /accounts` 直到 `quota.updated_at`（Unix 秒）變大。
- `disabled: true` 的帳號不要選；`live_limited_models` 非空表示該帳號有模型正被限流。

### `POST /accounts/switch`

請求：

```json
{ "account_id": "…" }
```

只有 `account_id` 一個欄位（取自 `GET /accounts` 的 `id`）。沒有 `target_ide`，多餘欄位會被忽略。

回應：

| 狀態碼 | 意義 |
| --- | --- |
| `202` | **切換工作已開始**，不代表已完成。`{"success":true,"message":"Account switch task started: <id>"}` |
| `409` | 已有另一個切換在進行，同時只允許一個。等它結束再送 |
| `403` | 被拒絕（瀏覽器請求或 `Host` 不是回送位址） |
| `400`／`415`／`422` | JSON 語法錯誤、請求不是 `Content-Type: application/json`，或缺少 `account_id` |

`202` 之後 HTTP 不會再回報結果。**完成與否要輪詢 `GET /accounts/current`**：`account.id` 等於目標帳號才算完成（應用在整個切換流程，含重啟 Desktop，做完之後才更新目前帳號）。建議逾時 90 秒仍未變就當失敗；失敗原因只寫在應用日誌（`[HTTP API] Account switch failed: …`），例如帳號不存在、權杖更新失敗、Antigravity 關不掉。切換失敗時 `409` 的占用旗標會被釋放，可以直接重試；若切換流程異常中止而持續回 `409`，重啟應用即可。

### 範例

```powershell
$base = 'http://127.0.0.1:19527'
$list = Invoke-RestMethod "$base/accounts"
# 自行決定選哪個帳號；這裡取剩餘百分比最高的模型額度當例子
$target = $list.accounts | Where-Object { -not $_.disabled -and $_.quota } |
  Sort-Object { ($_.quota.models | Measure-Object percentage -Maximum).Maximum } -Descending |
  Select-Object -First 1
Invoke-RestMethod "$base/accounts/switch" -Method Post -ContentType 'application/json' `
  -Body (@{ account_id = $target.id } | ConvertTo-Json)   # 202
```

```sh
curl -s http://127.0.0.1:19527/accounts/current
curl -s -X POST http://127.0.0.1:19527/accounts/switch \
  -H 'Content-Type: application/json' -d '{"account_id":"<id>"}'
```

## 切換會做什麼

`POST /accounts/switch` 呼叫的是和設定頁「切換帳號」按鈕同一個流程 `account::switch_account(id, None, …)`，`target_ide` 固定為空，由應用自動判定要套用到哪個 Antigravity：

| 當下狀態 | 目標 | 做法 |
| --- | --- | --- |
| 經典桌面版（Antigravity Desktop）正在執行 | 經典版 | 關閉程序（最多等 20 秒）→ 寫入新帳號憑證 → **重新啟動 Desktop**（沿用原本的安裝路徑與啟動參數） |
| 只有 IDE 版在執行 | IDE | 熱切號：先寫憑證，只終止 `language_server` 子行程，主視窗保留，由 IDE 自行重建子行程；15 秒內沒恢復就改為完整重啟 |
| 兩者都沒在執行 | 找得到經典版執行檔就選經典版 | 寫入憑證後啟動經典版 |

重點：

- **不需要 `target_ide`。** 同時開著經典版與 IDE 時，優先判定為經典版。
- **經典桌面版會被關掉再重新開啟**，所以 Desktop 會重新載入新帳號；進行中的對話或工作會被中斷。
- 經典版的順序是「先關、再寫憑證、再開」，避免舊程序結束時把舊權杖寫回去；IDE 熱切號則是「先寫憑證、再終止子行程」。
- 與設定頁按鈕的差異：HTTP 路徑不會通知反代重新載入帳號池（設定頁按鈕會）。需要反代立刻改用新帳號時，從設定頁操作一次或重新啟動反代。

## 驗收（需要維護者實機做）

1. 設定頁開啟、儲存、重新啟動應用。
2. `curl http://127.0.0.1:19527/health` 回 `{"status":"ok",…}`。
3. 用 `GET /accounts` 看到帳號與額度。
4. 對另一個帳號送 `POST /accounts/switch`，回 `202`；等 Desktop 重新載入後 `GET /accounts/current` 回傳該帳號，Desktop 內顯示的是新帳號。

## 為什麼設定檔叫 `local_account_api.json`

上游舊版的 19527 API 預設開啟，設定存在 `http_api_settings.json`。本 fork 改用新檔名，讓舊版留下的 `{"enabled": true}` 在升級後不會自動生效；要開啟必須在設定頁重新打開。
