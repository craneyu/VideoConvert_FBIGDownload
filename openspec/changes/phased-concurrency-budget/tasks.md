## 1. 設定項與預設值

- [x] 1.1 讓 `get_settings` 回報兩個新的並行度設定：缺鍵時回報預設值（網路 3、CPU 1），非數字或超出範圍（網路 1-8、CPU 1-2）時保留預設值且不回寫資料庫。交付 `Concurrency Setting Keys` 與 `Default Settings Values`，並落實 design 決策「預設網路 3、CPU 1，CPU 上限 2」。實作位置為 `src-tauri/src/commands/settings.rs` 的 `Settings` 結構、其 `Default` 實作與 `merge` 邏輯。驗證：新增單元測試涵蓋 settings-management spec 中「Parsing stored concurrency values」表格的全部八列輸入與其預期回報值。
- [ ] 1.2 讓設定頁可調整兩個並行度，且 CPU 欄位明確標示變更於下次啟動生效、網路欄位立即生效，CPU 上限的說明文字寫明第二個名額的用途是「讓程式保持回應」而非「更快」。交付 `CPU Concurrency Change Requires A Restart`。實作位置為 `src/lib/stores/settings.svelte.ts` 的 `Settings` 介面與 `src/routes/settings/+page.svelte`。驗證：手動開啟設定頁，確認兩個欄位可調整、輸入值受各自範圍限制、且重新啟動提示與 CPU 說明文字皆可見。

## 2. CPU 許可池

- [x] 2.1 新增一個持有 CPU 許可池的模組並註冊為 Tauri 受管理狀態，池容量在第一次需要許可時取自 CPU 並行度設定、之後於整個 process 固定（不能在 setup 階段讀，資料庫連線與 migrations 都要等前端載入才建立），對外提供非阻塞式（async）的取得許可操作並回傳釋放時歸還許可的守衛值。交付 `Shared CPU Permit Pool`，並落實 design 決策「CPU 許可池由下載後處理與轉檔共用」。實作位置為新檔 `src-tauri/src/commands/concurrency.rs`，並於 `src-tauri/src/commands/mod.rs` 與 `src-tauri/src/lib.rs` 註冊。驗證：新增單元測試斷言上限為 1 時第二次取得許可會等待，且前一個守衛釋放後該次取得才成功。
- [x] 2.2 讓取得許可失敗回傳錯誤，而不是在沒有許可的情況下開始編碼。交付 `Permit Acquisition Failure Is Surfaced`。驗證：新增單元測試對一個已關閉的許可池請求許可，斷言回傳錯誤且未執行編碼路徑。

## 3. 兩條管線改用許可池

- [ ] 3.1 讓 `download_video` 執行期間不再佔用 async runtime 的 worker：保留其 async 宣告，把逐行讀取子行程輸出與等待其結束的阻塞區段包進 `tauri::async_runtime::spawn_blocking` 並 await 結果，且阻塞區段 panic 時回傳錯誤字串而非中止 runtime。交付 `Long-Running Commands Do Not Block The Async Runtime`，並落實 design 決策「阻塞工作以 spawn_blocking 移出 async runtime」。實作位置為 `src-tauri/src/commands/download.rs`。驗證：手動在一個重新編碼進行中的同時開啟設定頁並修改設定，確認設定頁正常回應。
- [x] 3.2 讓下載的後處理階段依既有的 remux 或重新編碼判定結果決定是否取得 CPU 許可：判定為重新編碼才取得許可，判定為 remux 直接執行不排隊。交付 `Remux Is Exempt From The CPU Budget`，並落實 design 決策「容器 remux 不取得 CPU 許可」。驗證：新增單元測試以既有的後處理決策結果為輸入，斷言 remux 不需要許可、重新編碼需要許可。
- [ ] 3.3 讓下載在進入 CPU 許可等待前先發出「等待編碼」狀態，沿用既有的下載進度事件與其狀態文字欄位承載（新增一個狀態文字常數），進度值為下載階段的既有上限值，不新增事件通道。交付 `Phase-Scoped Permit Acquisition`，並落實 design 決策「分階段許可：網路階段結束即釋放，不被 CPU 等待佔用」。驗證：手動以網路 3、CPU 1 排入三支需要重新編碼的影片，確認第一支下載完成時前端收到該狀態文字。
- [ ] 3.4 [P] 讓 `transcode_video` 同樣不佔用 async runtime 的 worker，並在開始編碼前向同一個 CPU 許可池取得許可。交付 `Transcoding Shares Its Limit With Download Post-Processing`。實作位置為 `src-tauri/src/commands/transcode.rs`。驗證：手動先讓一個下載進入重新編碼階段，再於轉檔頁籤啟動一個任務，確認該轉檔任務顯示為等待而未立刻開始。

## 4. 前端佇列與狀態顯示

- [ ] 4.1 讓下載佇列的並行判斷改讀網路並行度設定並移除前端的固定常數，且**調高設定後無須任何其他事件即有更多待處理下載開始** —— 這需要一個觀察該設定值的反應式觸發（`$effect`）在設定變大時重新驅動佇列，因為現有的佇列推進只在新增任務與任務結束時被呼叫，兩者都不會因設定變更而發生。交付 `Concurrent Download Limit`，並落實 design 決策「網路並行度留在前端，Rust 只擁有 CPU 許可池」。實作位置為 `src/routes/+page.svelte` 的佇列推進邏輯。驗證：手動把設定由 2 改為 4，在 2 個下載進行中且 3 個待處理、且期間不新增也不完成任何任務的狀態下，確認另外 2 個待處理下載自行開始。
- [ ] 4.2 讓任務收到「等待編碼」狀態時不再計入下載中的並行計數，並立即重新驅動佇列，使下一個待處理下載無須等待前者編碼完成即可開始。交付 `Automatic Queue Progression`。驗證：手動以網路 3、CPU 1 排入 5 支影片，確認第 1 支下載完成的當下第 4 支立即開始下載。
- [ ] 4.3 讓等待 CPU 許可的下載任務顯示為「等待編碼」，而非停在不動的進度數字；取得許可後離開該狀態並開始回報後處理進度；判定為 remux 的任務不進入該狀態。交付 `Waiting-For-Encode Task State`，並落實 design 決策「新增「等待編碼」任務狀態」。驗證：手動以網路 3、CPU 1 排入 5 支需要重新編碼的影片，確認至多一支在編碼、其餘顯示「等待編碼」而非全部停在同一個不動的進度值。
- [ ] 4.4 讓轉檔頁籤的任務受佇列約束：同時執行數不超過 CPU 並行度設定，超出者顯示為等待而非另開 ffmpeg 行程，且有任務完成時恰好啟動一個等待中的任務。交付 `Transcoding Tasks Are Governed By A Queue`。驗證：手動在 CPU 設定為 1 時連續啟動五個轉檔任務，確認同時只有一個在執行、其餘四個顯示等待。
- [ ] 4.5 讓側邊欄的並行任務限制顯示讀取兩個設定的實際值，不再是硬寫死的文字。驗證：手動把網路設定改為 4，確認側邊欄顯示隨之變為 4。

## 5. 文件

- [x] 5.1 [P] 改寫 `README.md` 中「支援多執行緒下載與影片格式轉換（開發中）」該行，使其描述實際成立的能力：影片格式轉換已提供、多任務並行下載的數量可設定，且不再宣稱尚未實作的單檔多連線加速。驗證：內容審閱，確認該行不含「開發中」字樣，且不宣稱本 change 未交付的能力。
