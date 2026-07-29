# 自動更新簽章金鑰輪替手冊

本文記錄 VidBridge 自動更新（Tauri updater）所用 minisign 金鑰對的產生、設定與輪替流程。

> **先讀這段**：`tauri signer generate` 產生的兩個檔案內容**本身已經是 base64**。設定檔的 `pubkey` 欄位要填的是**公鑰檔（`.pub`）的原始內容**，不可再多做一層 base64，也**絕對不可**填私鑰檔的內容。0.1.3 版就是把私鑰檔內容填進了 `pubkey` 欄位，導致自動更新自始無法運作 —— 詳見文末「歷史事故」。

---

## 1. 何時需要輪替

- 私鑰或其 passphrase 外洩、或疑似外洩（例如誤入版控、誤貼到公開場合）。
- 私鑰遺失，無法再為新版本簽章。
- 例行安全輪替。

### 輪替的代價（重要）

輪替金鑰會使**既有安裝無法透過自動更新取得新版**，因為舊版內嵌的是舊公鑰，驗不過新簽章。使用者必須手動下載一次新版，之後才會重新接上自動更新。

因此：

| 情境 | 輪替代價 |
| --- | --- |
| 自動更新從未成功運作過（0.1.3 及之前） | **無代價**，沒有任何安裝依賴舊金鑰 |
| 自動更新已正常運作 | **有代價**，所有既有安裝都會被迫手動更新一次 |

在第二種情境下，輪替前請先評估影響範圍，並在發佈說明中明確告知使用者。

---

## 2. 前置需求

- 本機已安裝專案依賴（`npm install`）。
- 具備該 GitHub repository 的 secrets 寫入權限。
- 已安裝 GitHub CLI 並完成登入（`gh auth status` 可確認）。

---

## 3. 步驟一：產生新金鑰對

金鑰檔**必須產生在 repository 之外**。建議放在 `~/.tauri/`。

先產生一組強隨機 passphrase 並存檔，**過程中不要把它印到終端機**（避免留在 shell 歷史、終端機捲動緩衝或工作階段記錄中）：

```bash
mkdir -p ~/.tauri && chmod 700 ~/.tauri
umask 077
openssl rand -base64 32 | tr -d '\n' > ~/.tauri/vidbridge-passphrase.txt
chmod 600 ~/.tauri/vidbridge-passphrase.txt
```

> `tr -d '\n'` 不可省略。passphrase 檔案若帶尾端換行，寫進 GitHub Secrets 後會與實際 passphrase 不符，發佈流程會在簽章階段失敗。

接著產生金鑰對：

```bash
npx tauri signer generate --ci \
  -p "$(cat ~/.tauri/vidbridge-passphrase.txt)" \
  -w ~/.tauri/vidbridge.key
```

產生兩個檔案：

| 檔案 | 內容 | 可否公開 |
| --- | --- | --- |
| `~/.tauri/vidbridge.key` | 私鑰（以 passphrase 加密） | **不可**，絕不進版控 |
| `~/.tauri/vidbridge.key.pub` | 公鑰 | 可以，要填進設定檔 |

---

## 4. 步驟二：設定 GitHub Secrets

發佈流程（`.github/workflows/release.yml`）需要這兩個 secret：

| Secret 名稱 | 值 |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | 私鑰檔的內容 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | passphrase |

**兩者必須同時更新。** 只換其中一個會讓下一次發佈在簽章階段失敗。

以 stdin 傳值，避免祕密出現在指令文字與 shell 歷史中：

```bash
R=craneyu/VideoConvert_FBIGDownload
printf '%s' "$(cat ~/.tauri/vidbridge.key)" \
  | gh secret set TAURI_SIGNING_PRIVATE_KEY --repo "$R"
printf '%s' "$(cat ~/.tauri/vidbridge-passphrase.txt)" \
  | gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo "$R"
```

確認更新時間已變動（secret 的值無法讀回，只能看時間戳）：

```bash
gh secret list --repo craneyu/VideoConvert_FBIGDownload
```

---

## 5. 步驟三：填寫公鑰欄位

把**公鑰檔的原始內容**填進 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`：

```bash
cat ~/.tauri/vidbridge.key.pub
```

輸出是一整行 base64，**照原樣貼進欄位**：

- 不要再做一層 base64 編碼。
- 不要只貼解碼後的金鑰行。
- 不要貼 `vidbridge.key`（私鑰）的內容。

### 怎麼分辨手上拿的是公鑰還是私鑰

把欄位值 base64 解碼後觀察：

| 特徵 | 公鑰（正確） | 私鑰（**錯誤，絕不可填**） |
| --- | --- | --- |
| 註解行 | `untrusted comment: minisign public key: <KEY_ID>` | `untrusted comment: rsign encrypted secret key` |
| 金鑰行長度 | 56 字元 | 212 字元 |
| 含 KDF 欄位（salt / opsLimit / memLimit） | 無 | 有 |

一行指令即可檢查目前設定檔填的是什麼：

```bash
python3 -c "
import json,base64,pathlib
f=json.loads(pathlib.Path('src-tauri/tauri.conf.json').read_text())['plugins']['updater']['pubkey']
d=base64.b64decode(f).decode(); L=d.strip().split(chr(10))
print('註解行     :', L[0])
print('金鑰行長度 :', [len(x) for x in L[1:]])
print('是私鑰嗎？ :', 'secret key' in d.lower(), '<-- 必須是 False')
"
```

---

## 6. 步驟四：驗證

輪替後依序確認四項：

1. **公鑰欄位正確** —— 執行上一節的檢查指令，`是私鑰嗎？` 必須為 `False`、金鑰行長度為 56。

2. **欄位值與公鑰檔一致** ——

   ```bash
   python3 -c "
   import json,pathlib
   f=json.loads(pathlib.Path('src-tauri/tauri.conf.json').read_text())['plugins']['updater']['pubkey']
   p=pathlib.Path('$HOME/.tauri/vidbridge.key.pub').read_text().strip()
   print('一致:', f==p)
   "
   ```

3. **私鑰未進版控** —— 必須零命中：

   ```bash
   grep -rlF "$(cat ~/.tauri/vidbridge.key)" \
     --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=target .
   ```

4. **私鑰與 passphrase 匹配** —— 本機實際簽章一次，`exit code` 必須為 0。這一步能在發佈前就抓出 passphrase 不符的問題：

   ```bash
   echo test > /tmp/sigtest.txt
   npx tauri signer sign \
     -p "$(cat ~/.tauri/vidbridge-passphrase.txt)" \
     -f ~/.tauri/vidbridge.key /tmp/sigtest.txt
   echo "exit: $?"
   ```

最終仍須以一次真實發佈驗證：從輪替前的版本觸發自動更新並完成安裝。設定看起來正確不等於更新能運作 —— 0.1.3 的教訓正是如此。

---

## 7. 步驟五：備份

把下列兩項備份到密碼管理器：

- `~/.tauri/vidbridge.key`（私鑰）
- `~/.tauri/vidbridge-passphrase.txt` 的內容（passphrase）

備份完成後**刪除明文 passphrase 檔**：

```bash
rm ~/.tauri/vidbridge-passphrase.txt
```

明文 passphrase 與加密私鑰放在同一個目錄，等於抵銷了私鑰的加密保護。

---

## 8. 禁止事項

以下每一項都會造成安全問題或使自動更新失效：

- **私鑰不得以任何形式進入版控。** 包含但不限於：設定檔、workflow 檔、文件、註解、測試檔、範例檔。本文件本身也不含任何私鑰內容。
- **不得把私鑰檔的內容填進 `pubkey` 欄位。** 這是 0.1.3 的實際成因。
- **不得把 passphrase 寫進版控或印進 CI log。**
- **不得把金鑰檔產生在 repository 目錄內**，即使打算稍後刪除 —— 可能被誤 commit，或殘留在 git 物件中。
- **不得只更新兩個 secret 中的一個。**
- **不得為了讓建置通過而移除簽章。** 未簽章的更新無法被驗證，等同關閉更新的完整性保護。
- **不得在自動更新已正常運作後隨意輪替。** 每次輪替都會迫使所有既有安裝手動更新一次。

---

## 9. 現行啟用的金鑰

| 項目 | 值 |
| --- | --- |
| 公鑰 Key ID | `51D19E1E18280C0` |
| 產生日期 | 2026-07-28 |
| 公鑰所在 | `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey` |
| 私鑰所在 | 本機 `~/.tauri/vidbridge.key`，以及 GitHub Secrets |

Key ID 可用於確認設定檔中的公鑰與手上的金鑰檔屬於同一對。

---

## 10. 歷史事故：0.1.3 的自動更新自始未運作

**現象**：自動更新從發佈起從未對任何使用者成功過，且兩個月間沒有任何徵兆。

**三個同時存在的成因**：

1. `pubkey` 欄位填的是**私鑰**檔的內容。解碼後註解行為 `untrusted comment: rsign encrypted secret key`，且含 KDF 欄位 —— 這些只存在於私鑰檔。公鑰不可能驗證通過，因為填進去的根本不是公鑰。
2. updater endpoint 指向 GitHub raw 上一個**從未進入版控**的 manifest 檔案，因此連取得更新資訊都會失敗。
3. 前端的更新檢查把所有錯誤吞進**空的 catch**，導致上述兩個問題完全沒有徵兆。

**得到的三個教訓**：

- 公鑰與私鑰的差異必須有**可執行的檢查**，不能靠肉眼看 base64。本文第 5 節的判別表與檢查指令即為此而寫。
- 更新檢查的失敗路徑**必須留下 log**。靜默失敗會讓問題存活數個月。
- 設定正確不等於功能正常，**必須以一次真實發佈端到端驗證**。

**附帶影響**：私鑰進入公開版控屬於須輪替等級的暴露。該私鑰有 passphrase 加密（passphrase 存於 GitHub Secrets 未外流），因此屬於「應盡快輪替、可離線暴力破解」，而非「已被攻破」。因當時自動更新對所有使用者皆未運作，輪替不影響任何既有安裝，代價為零。
