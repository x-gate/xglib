# 相容性修復：CGP 與圖像載入

日期：2026-09-13。分支：`fix/compatibility`。解析器原始基準為 `6924c11`；文件與初次驗證基準已提交為 `e8a1481`。保留[修復前報告](validation-2026-09-13.md)及其診斷，不覆寫歷史結果。

## 修復內容與依據

### 完整 CGP 檔案

原先 `Palette::build_from_cgp` 將 224 個有效色彩的長度（672 bytes）視為唯一允許的檔案長度，拒絕本機全部 35 個 708-byte CGP。

交叉研究依據：[CGTool 的 Palet.cs](https://github.com/HonorLee-cn/CGTool/blob/master/CrossgateToolkit/Palet.cs) 在檔案起始連續讀取 224 組 BGR，再接上固定色；其餘檔案 bytes 未參與這個色表的建構。此處只以公開解析器的讀取範圍作格式證據，未移入其程式碼或資產。此行為與 xglib 既有的 16 + 224 + 16 色模型一致。

因此現在接受兩種明確長度：672-byte 有效色彩資料，以及 708-byte 完整 CGP。兩者使用前 672 bytes 作為索引 16–239 的色彩，保留既有固定前後色。708-byte 尾端的 36 bytes 不覆蓋固定後綴；這不表示已證實這些 bytes 在原作所有用途都無意義。其他長度仍拒絕，未改成任意截取。短資料為 `BufferTooShort`，非支援的較長資料改回 `InvalidValue`，不再誤報「太短」。

`CGP_SIZE = 672` 保持相容，新增 `CGP_FILE_SIZE = 708`；WASM getter 相應為原有 `cgp_size()` 與新增 `cgp_file_size()`。

### 明確的圖像 CGP 入口

原本 `Graphic::*build_from_bytes` 接收 raw BGR，因此即使放寬 CGP 長度，直接傳入 `.cgp` 仍然會得到 236 色與錯誤索引位置。新增：

| Rust | WASM / TypeScript | 行為 |
| --- | --- | --- |
| `Graphic::build_from_cgp` | `graphic_build_from_cgp` | CGP 色表；保留原有補零 / 截尾模式 |
| `Graphic::strict_build_from_cgp` | `graphic_strict_build_from_cgp` | CGP 色表；像素長度不符即錯誤 |
| 既有 `Graphic::strict_build_from_bytes` | 新增 `graphic_strict_build_from_bytes` | raw BGR 的嚴格模式，讓 JS 使用端也能取得長度錯誤 |

CGP 圖像入口對 version 0/1 使用外部 CGP，version ≥ 2 則維持內嵌色表優先並忽略外部參數。新增入口會檢查回傳的像素索引是否超出色表，含內嵌色表；越界回 `InvalidValue`，context 為 `graphic palette index`。

既有 `Graphic` 資料結構、`build_from_bytes` 及 `strict_build_from_bytes` 的 raw BGR 契約保持不變，不使用長度自動辨識。新的 API 沒有改寫原始資源或新增 binary writer。

外部 CGP 呼叫端的遷移方式：

```rust
// info 與 data 仍須先由呼叫端切出恰好一筆。
let graphic = xglib::Graphic::strict_build_from_cgp(info, data, cgp_bytes)?;
```

上例 `?` 所在函式須回 `Result<_, xglib::BuildError>`。已有 raw BGR 色表的呼叫端不需要遷移。使用 strict 時應處理圖像異常，不要無條件忽略錯誤。

### 驗證工具的分類

工具現在使用 CGP 圖像入口實測。`graphic.legacy_raw_palette_index_risk` 仍保留舊 raw 入口的界限風險數量，方便前後對照；它不是修復後色表越界數。

重複 ID 與 header.data_len 不一致改列 `WARN`：前者不是解析器拒絕的資料，後者本來就不作為切片依據。所有索引列仍逐筆解析，沒有套用 first-wins / last-wins。strict 與一般模式的解析失敗仍列 `ERROR`，不藉重新分類掩蓋失敗。

## 未擅自改變的行為

- **188 筆多 1 byte**：補充唯讀 RLE 走訪確認皆以 `C2`（兩個零 byte）結尾，跨越圖像邊界 1 byte。尚無證據將 C2 改作結束符或從一般解碼器扣 1。strict 仍拒絕；一般模式沿用已記錄的截尾，並有此形狀的合成回歸測試。
- **1 筆 4 × -15**：資料為 16-byte header、payload 為空；無依據判定特殊圖片或佔位紀錄，仍回錯誤，不取絕對值、不產生虛構透明圖。
- **340 筆 version 0 的 data_len**：保留原始欄位並使用 info 切片；不把不可靠欄位變成硬性拒絕條件。
- **圖像 / 動畫重複 ID**：函式庫仍是單筆解析 API，沒有新增未經驗證的覆蓋優先序。驗證保留全部記錄。
- **地圖缺少圖塊對應**：僅指定 Graphic 檔不足以驗證完整映射；不重寫 map_id 或清空圖塊。

## 修復後實測

執行命令（repository 根目錄）：

```sh
python3 scripts/verify_resources.py ../CGoriginmood/Assets > target/resource-audit-compatibility.txt
```

診斷摘要保存在 [compatibility-validation.txt](compatibility-validation.txt)。輸入清單、大小、SHA-256 與前次相同，manifest 為 `a4e58343a50c4603cdd6c535839844fa6e78d78aa5844a6032829d39cbe077d8`。執行前後 645 個指定檔案皆未改變。

| 指標 | 修復前 | 修復後 |
| --- | ---: | ---: |
| CGP 成功 | 0 / 35 | 35 / 35 |
| 可建構圖像的色表索引越界 | 212,300 | 0（使用 CGP 入口） |
| 圖像 strict 成功 | 252,635 | 252,635 |
| 圖像只有一般模式成功 | 188 | 188 |
| 圖像兩模式都失敗 | 1 | 1 |
| 動畫成功 / 地圖成功 | 3,186 / 605 | 3,186 / 605 |
| 不同動畫圖像引用中缺少 ID | 0 | 0 |
| RLE scalar / SIMD 相同 | 252,483 筆 | 252,483 筆 |
| RLE encoder 抽樣 round-trip 相同 | 246 筆 | 246 筆 |

最終仍為 `audit_complete=true`、`inputs_unchanged=true`、`audit_exit_code=1`，因為尚有 189 筆 strict 失敗（含 1 筆一般模式也失敗）。這次修復的是 CGP 相容性，不宣稱所有資料都已正確呈現。

## 自動驗證與界限

- `cargo test --locked --offline`：34 個既有單元測試 + 8 個新增整合回歸測試通過。
- `cargo fmt --all -- --check`、`cargo clippy --locked --offline --all-targets -- -D warnings`、`cargo doc --locked --offline --no-deps`：通過。既有 RLE / map 的機械性 lint 清理另以 `f539460` 提交，不改解碼規則。
- `cargo build --locked --offline --release --target wasm32-unknown-unknown`：通過；本次新增安裝此 Rust target。
- JS glue 與 JS runtime 測試尚未執行，瀏覽器呈現、原作畫面、動畫事件與地圖碰撞語意仍未驗證。WASM 編譯不等同 runtime 驗證。

回歸資料全為自行產生：CGP 首末有效色、非零尾段、未知長度、raw 契約不自動變更、0/1 外部色表、2/3 內嵌色表優先、越界索引、C2 超出一像素、負高度、version 0 不可靠長度欄位。未提交原作資料、解碼圖像或色表內容。
