# 本機資源相容性驗證報告

日期：2026-09-13（Asia/Taipei）。基準：`6924c114797a4545dc1735d82d4415a256ffa3f5`，`chore: project init`。開始時 `xglib` 工作樹乾淨，無 README、repository 專屬 AGENTS 或技術文件。

本次新增文件及獨立驗證工具，**未更動 `src/`、Cargo 依賴或 TypeScript 介面**。所有結果是這份基準解析器對指定樣本的觀察。診斷原文見 [validation-2026-09-13.txt](validation-2026-09-13.txt)。

## 結論

研究成果可用於圖像/RLE、動畫與地圖的結構解析，但目前不能直接當成完整且正確的資源載入方案。動畫與地圖全部通過現有解析器；圖像絕大多數符合像素長度，仍有長度異常、負尺寸與重複 ID。外部調色盤是明確不相容點，尚需研究 708-byte 變體及修正 API 整合。

本次沒有啟動原作、輸出或目視比對畫面，也沒有實作播放、碰撞或完整地圖素材映射；不宣稱已重現原作呈現與玩法。

## 輸入與指紋

相對根目錄為 `../CGoriginmood/Assets`，所有檔案均以唯讀方式開啟。

| 路徑 | Bytes | SHA-256 |
| --- | ---: | --- |
| `bin/GraphicInfo_66.bin` | 10,112,960 | `fbb97bc1dbda348333603a690bd4af0a0ddb912bc55fed361ba9121bbbcdfd0b` |
| `bin/Graphic_66.bin` | 636,994,487 | `318237a7a7208c13761f95e51f9b97010a6fe3124761e3c3e880e4d7209cc0a1` |
| `bin/AnimeInfo_4.bin` | 38,232 | `6211d843537e374677210b3de56e7b6f4bb0ff449cf6c61069d894c45d554d0c` |
| `bin/Anime_4.bin` | 27,867,440 | `b95873a2299f84af723330e6b2ac80f201a42b70190b1b679d94c5abad8dd653` |
| `bin/pal/palet_00.cgp` | 708 | `28822eb7767a714d4d8045c3ed1916ab05d1c4c0eba25c460f1429b7fb921255` |

`bin/pal/` 共 35 個 CGP，24,780 bytes。`map/` 共 605 個 DAT 與 1 個 `.DS_Store`，31,339,182 bytes；DAT 合計 31,328,938 bytes。`.DS_Store` 納入完整性清單，但不交給地圖解析器。

共 645 個輸入檔案、706,377,081 bytes。完整清單以相對路徑排序，每項為 `path`、`size`、`sha256`，採 Python `json.dumps(..., ensure_ascii=False, sort_keys=True, separators=(",", ":"))` 的 UTF-8 bytes 再做 SHA-256：

```text
manifest_sha256=a4e58343a50c4603cdd6c535839844fa6e78d78aa5844a6032829d39cbe077d8
```

個別檔案清單會由驗證腳本輸出至本機 `target/resource-audit.txt`；文件只保留統計與指紋，不包含資源內容。執行前後的檔案清單、大小、雜湊完全一致。完整性檢查範圍是指定輸入，未對整個遊戲目錄做快照。

## 方法與重現

環境：macOS ARM64，`rustc 1.98.1 (48a229cea 2026-09-01)`、`cargo 1.98.1 (797e8a9bc 2026-08-05)`；依賴使用 `Cargo.lock` 與本機快取。僅安裝原生 `aarch64-apple-darwin` target。

從 `xglib/` 執行：

```sh
cargo build --locked --offline --release --example verify_resources
python3 scripts/verify_resources.py ../CGoriginmood/Assets > target/resource-audit.txt
```

此 wrapper 執行前後各算一次 SHA-256，內部以 `cargo run --locked --offline --release --example verify_resources -- <Assets>` 呼叫實際 library。已知輸入出現問題時 Rust 工具在完成統計後回傳 1，wrapper 保留退出碼；若輸入完整性不符則回傳 2。I/O / 編譯 / 索引容器錯誤也可能非零退出，必須查看完整日誌。

本次末端為：

```text
audit_complete=true
inputs_unchanged=true
audit_exit_code=1
```

因此 exit 1 是**掃描完成但有已記錄相容性發現**，不等於全部通過。若缺少 `audit_complete=true`，不能引用這份全量結果。未列出的計數鍵表示沒有發生該事件；有錯誤的分類只保留總數與第一個案例，並未列出每個異常記錄。

檢查層次：

1. 圖像/動畫索引檔長度整除固定紀錄大小；圖像資料範圍、覆蓋、重疊及 ID 重複。
2. 逐筆圖像 strict 解析，失敗再試一般模式；比對 info/header 尺寸、header.data_len/info.len、palette index 界限。
3. 成功建構的壓縮圖像全部比較 scalar / SIMD 解碼；索引列號 `row % 1024 == 0` 的壓縮圖像額外進行編碼 round-trip 與 scalar / SIMD encoder 比較。
4. 動畫以排序後下一個地址切片，逐筆驗證恰好耗盡；統計動作、frames 及圖像 ID 存在性。重複 ID 不會被合併或跳過。
5. 所有 CGP 同時測試完整 CGP 入口、原始 BGR 入口；前 672 bytes 僅作明確標示的假說診斷，不當作成功相容。
6. 遞迴解析全部 `.dat`，檢查三層長度；以非零 ground/object 值是否出現在 GraphicInfo.map_id 作探索性關聯檢查。

工具一次載入圖像資料檔；使用 16,777,216 像素的單筆配置 guard，這是驗證工具限制，非格式規格。本次沒有觸發。工具適用本次已知本機資料，不是對任意不可信檔案的安全掃描器。

## 圖像與 RLE 結果

| 檢查 | 數量 / 結果 |
| --- | ---: |
| 圖像索引筆數 | 252,824 |
| version 0 / version 1 | 340 / 252,484 |
| version ≥ 2 | 0 |
| strict 成功 | 252,635 |
| strict 失敗、一般模式成功 | 188 |
| 兩種模式皆失敗 | 1 |
| 一般模式可回傳的像素索引總數 | 1,509,709,716 |
| 不合法切片、索引範圍重疊、未索引 bytes | 全部 0 |
| info/header 尺寸不一致 | 0 |
| header.data_len 與 info.len 不同 | 340（全為 version 0） |
| 重複 ID 的額外紀錄 | 36；共有 252,788 個不同 ID |
| scalar / SIMD 解碼比較 | 252,483 筆全部相同 |
| encoder round-trip / SIMD encoder 抽樣 | 246 筆全部相同 |

188 筆 strict 失敗全部是 **version 1 解碼後多 1 byte**，不是缺少像素；一般模式全部截尾。例如零起算 row 8725 / ID 8725，36 × 30 應為 1080 bytes，實際為 1081。這支持「本次相容模式可以繼續解析」，不證明截掉的 byte 一定沒有意義。

row / ID 16681：addr=58,762,562、len=16、info 與 header 尺寸都是 4 × -15。strict / 一般模式均回 `InvalidValue { context: "graphic dimensions", message: "graphic height must be non-negative" }`。這筆未進入圖像成功後的 RLE 對照；其 RLE payload 為空。無法判定是佔位紀錄、特殊規則或來源異常。

340 筆 version 0 的像素數全部符合 16-byte header 模型，但 `data_len` 不符。例如 row 7476，header 欄位值 8,617,560、info.len=468。若改用 12-byte header，全數反而多 4 bytes；本次不改解析器、不替這個欄位猜定新語意。

ID 重複不是直接的解析錯誤，工具用 `ERROR ...duplicate_id` 提醒載入器的唯一鍵假設不成立。第一個額外重複紀錄為 row 250968、ID 19230。應保留來源、列號與地址；尚未確認 first-wins / last-wins 或版本覆蓋規則。

## 調色盤結果

35 個檔案均為 708 bytes：

- `Palette::build_from_cgp`：0 成功 / 35 失敗，`needed=672, actual=708`。錯誤名稱雖是 `BufferTooShort`，實際輸入是更長。
- `Palette::build_from_bytes`：35 成功，但每個僅得到 236 色，沒有 CGP 固定前後色。
- 僅取前 672 bytes 再走 CGP：35 可回傳 256 色；這只是函式滿足長度條件，**未證實截取位置、剩下 36 bytes 或固定色值的正確性**。

圖像測試以完整 `palet_00.cgp` 作為第三參數，刻意記錄「直接傳入指定檔案」的現況。252,823 筆可建構的圖像中，212,300 筆至少包含一個 ≥ 236 的像素索引，超出回傳色表範圍。即使未越界的圖像，色表位置仍可能錯誤；不能把它們列為色彩正確。

原因有兩層：真實 CGP 長度與 parser 不符，以及 `Graphic` 實際呼叫 raw BGR 入口而非 CGP 入口。後續需先獨立確認 708-byte 檔案布局，再決定 API 是否接收已建構 `Palette` 或提供明確的外部 CGP builder。不可只截檔案、補色或忽略錯誤讓測試變綠。

## 動畫結果

3,186 筆皆完整耗盡各自切片，共 311,365 個標準動作 header、2,413,106 frames；沒有延伸 header。地址有效、無重複、第一筆從 0 開始，最後一筆結束於檔案 EOF。

233,458 個不同的 frame.graphic_id 全部能在 `GraphicInfo_66` 的 ID 集合中找到。此檢查只證明存在，沒有選定重複 ID 的正確紀錄，也未證明所有 frame 的圖像能正確呈現。

動畫 ID 只有 806 個不同值，額外重複紀錄 2,380 筆；例如 ID 100000 出現在 row 0、790、1580、2375，地址不同。補充唯讀欄位統計顯示 padding 有 3,176 筆 `0000`、10 筆 `4000`；意義未知，應保留。這些群組可能有版本或資料分段語意，但尚未驗證。

## 地圖結果

605 個 `.dat` 全數符合 magic、20-byte header 與 `20 + width * height * 6` 長度，解析出 5,219,473 格，每層各有同樣的元素數；無額外或不足 bytes。1 個 `.DS_Store` 被跳過。

探索性檢查中，ground / object 合計有 292,402 個**非零出現次數**找不到同值 `GraphicInfo_66.map_id`，不是 292,402 個不同 ID。這不直接代表檔案損壞：可能需要其他圖像檔、不同鍵或編碼規則。故本次只能證明地圖平面結構可解析，不能宣稱僅靠指定 Graphic 檔就能完整渲染所有地圖。

## 建置與品質檢查

| 命令 / 檢查 | 本次結果 |
| --- | --- |
| `cargo test --locked --offline` | 34 個單元測試通過；0 個 doc tests |
| `cargo build --locked --offline --release --example verify_resources` / wrapper 內部 release build | 成功，且實際完成全量掃描 |
| `cargo doc --locked --offline --no-deps` | 成功 |
| `cargo fmt --all -- --check` | 既有 `src/lib.rs`、`src/types/mod.rs`、`src/wasm.rs` 排序 / 排版差異，失敗 |
| `cargo clippy --locked --offline --all-targets -- -D warnings` | 既有 library 8 個診斷、test 額外 4 個診斷，失敗 |
| 新增驗證工具的 rustfmt / 一般 Clippy | `rustfmt --edition 2024 --check examples/verify_resources.rs` 通過；一般 Clippy 無新增 example 診斷，既有 library 警告仍存在 |
| 文件範例與連結 | README 範例編譯執行成功；API 切片函式編譯成功；文件內部連結均存在 |
| 驗證工具合成案例 | 暫存目錄內自行建立正常資料、多一個像素、重複 ID 三組輸入；退出碼分別為 0、1、1，診斷符合預期（非新增 library 單元測試） |
| WASM 編譯 / JS runtime | 未執行；缺少 WASM target 與 wasm-bindgen CLI |
| 原作畫面、動畫時間、碰撞或路徑行為比對 | 未執行 |

Clippy 類型：`manual_range_patterns`、`needless_return`、`chunks_exact_to_as_chunks`、`manual_is_multiple_of` 與測試中的 `identity_op`。本次保留匯入程式碼，未把格式整理與 lint 修正混入文件復原工作。

## 後續研究與相容性決策

本次決策是保留既有 API 行為，以文件揭露限制；無資料遷移、跨 repository 修改或部署順序需求。建議後續工作依相依關係處理：

1. 研究 708-byte CGP 的正確布局與固定色，再定義明確的圖像調色盤契約，加入合成回歸測試。
2. 查明多 1 byte、負高度與 version 0 offset 12 的語意；先保留 strict 的診斷能力，再決定相容行為。
3. 定義多來源 / 重複 ID 的查找方式，補齊地圖圖塊映射；不可默默用單一 HashMap 覆蓋。
4. 使用另行允許的比對方式驗證色彩、scanline、offset、動畫事件與地圖語意；補足 version ≥ 2 與延伸動畫真實樣本。
5. 在需要對外整合前完成 WASM target / JS 形狀測試及解析資源上限強化。
