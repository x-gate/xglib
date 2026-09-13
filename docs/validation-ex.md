# Ex 資源相容性驗證

日期：2026-09-13。分支：`fix/compatibility`。本輪開始的基準 commit：`022cfc5`。工作樹原先乾淨；沿用已修復的 CGP 入口，對本次指定 Ex 資料做全量唯讀驗證。

## 結論與變更

既有解析器可讀取全部 343,875 筆 Ex 圖像（一般模式）與全部 827 筆 Ex 動畫。調色盤索引沒有越界；所有動畫圖像 ID 都存在於選定的 Ex 圖像索引。沒有發現需要新增二進位格式分支的證據。

strict 圖像解析仍有 102 筆多 1 byte；補充逐命令走訪確認全數是最後的 `C2` 零填充命令跨越像素邊界。這與 base 組既有異常形狀相同，沿用一般模式截尾、strict 拒絕的既定契約，沒有把 `C2` 改作終止符，也未放寬 strict 來消除診斷。

本輪修復集中在驗證工具：原先硬編碼 base 檔名與 `graphic_66` 統計標籤，無法直接正確選擇與識別 Ex。現在提供 `--set base|ex`，雜湊清單與 Rust 呼叫使用同一組檔名，保留大小寫；Rust 輸出選定資源名稱，引用統計改為 `selected_graphics` / `selected_map_id`。新增 5 個合成測試覆蓋選擇、退出碼與輸入完整性。

**本輪未修改 `src/` 的解析器、WASM 介面、TypeScript 宣告或依賴。** 這是對既有相容性修復的跨資源集驗證，沒有宣稱 102 筆 strict 異常已修正為原作規則。

## 輸入與指紋

根目錄：`../CGoriginmood/Assets`。沿用此前指定的 `bin/pal/` 與 `map/` 作共享輸入，不合併 base 圖像索引。四個選定檔案為：

| 相對路徑 | Bytes | SHA-256 |
| --- | ---: | --- |
| `bin/GraphicInfoEx_5.bin` | 13,755,000 | `9eebd38a850b004e8b5eb6998f006637d8c16a5380cbf761f9051fcfc3a186aa` |
| `bin/GraphicEx_5.bin` | 941,800,110 | `6090ddcf827c6090441266df18114bd0c70b98148c4e7a10faa55b23661a7b9d` |
| `bin/AnimeInfoEx_1.Bin` | 9,924 | `f8ad8aa0618d6db11073c1646c74648f0139ab7ef2fda1dd08c1cb3e3f3739ea` |
| `bin/AnimeEx_1.Bin` | 10,088,314 | `edd02e625607465b15093e3dedfea5ca59aa0589956fc1d62968adabf18fe40c` |

四個檔案合計 965,653,348 bytes。含共享的 35 個 CGP、605 個 DAT 及只做雜湊的 1 個 `.DS_Store`，共 645 個檔案、997,017,310 bytes。manifest 算法沿用[初次報告](validation-2026-09-13.md)：

```text
manifest_sha256=e98ec8a6f4193698b774dee4067ed1e257065b64de3af55c5a7ba385bc5fbe51
```

所有指定輸入在執行前後的檔名、大小與 SHA-256 一致。日誌僅含指紋與診斷；未輸出或提交解碼素材、動畫或地圖內容。

## 重現

從 `xglib` 執行：

```sh
python3 scripts/verify_resources.py ../CGoriginmood/Assets --set ex > target/resource-audit-ex.txt
```

此命令內部建置並執行 release 範例。Rust 直接入口等價於：

```sh
cargo run --locked --offline --release --example verify_resources -- \
  ../CGoriginmood/Assets GraphicInfoEx_5.bin GraphicEx_5.bin AnimeInfoEx_1.Bin AnimeEx_1.Bin
```

直接 Rust 入口不含前後雜湊比較，正式研究驗證應使用 Python wrapper。不需建立暫存資源副本、改名或符號連結。Rust 只接受四個純檔名，依序為 GraphicInfo、Graphic、AnimeInfo、Anime；不完整參數組或包含路徑會在讀取前拒絕。

完整診斷摘要見 [validation-ex.txt](validation-ex.txt)。末尾如下，exit 1 是 strict 異常仍然存在，不代表掃描未完成：

```text
audit_complete=true
inputs_unchanged=true
audit_exit_code=1
```

## 全量結果

| 指標 | Ex 結果 |
| --- | ---: |
| GraphicInfo 紀錄 | 343,875 |
| 圖像 version 0 / 1 | 77 / 343,798 |
| 圖像 version ≥ 2 | 0 |
| strict 圖像成功 | 343,773 |
| 只有一般模式成功 | 102 |
| 一般模式失敗、負尺寸 | 0 / 0 |
| 正規化後像素索引總數 | 2,177,879,992 |
| 色表索引越界 | 0 |
| 圖像索引範圍越界、重疊、未索引 bytes | 全部 0 |
| info/header 尺寸不同 | 0 |
| header.data_len 與 info.len 不同 | 77（全為 version 0） |
| 額外重複圖像 ID 紀錄 | 6；共有 343,869 個不同 ID |
| scalar / SIMD 解碼對照 | 343,798 筆全部相同 |
| encoder round-trip / SIMD encoder 對照 | 336 筆抽樣全部相同 |
| 動畫成功 | 827 / 827 |
| 動作 / frame | 119,397 / 865,555 |
| 標準 / 延伸動作 header | 119,397 / 0 |
| 不同 frame.graphic_id | 333,779；全部存在於 Ex 圖像索引 |
| 重複動畫 ID / 重複動畫地址 | 0 / 0 |
| CGP 成功 | 35 / 35 |
| 地圖成功 / 格數 | 605 / 5,219,473 |

圖像 RLE 對照涵蓋所有壓縮紀錄，包括 strict 失敗但一般模式成功者。encoder 抽樣規則保持零起算索引列號 `row % 1024 == 0`，不是將原始壓縮 bytes 當成唯一正確編碼。

102 筆異常全是 version 1，多出 1 個零 byte，最後一個命令為 `C2`（填 2 個零）。例如 row / ID 3462：480 × 402 應為 192,960 bytes，實際 192,961；row 3989 / 3990 為 896 × 720，實際 645,121。`C2` 在其他位置仍是正常命令，因此不修改通用解碼器，也未證實尾端多出的零在原作有特殊語意。

77 筆 version 0 與 base 一樣：16-byte header 後的像素量正確，offset 12 的欄位不可靠。第一個案例 row 7063，header.data_len=47,427,224、info.len=307,216。改用 12-byte header 會每筆多 4 bytes，不採用這項假設。

重複圖像 ID 仍保留所有紀錄，第一筆額外重複為 row 343831、ID 5492。沒有根據 ID 去重或覆蓋，也沒有建立尚未驗證的查找優先序。動畫按地址切片，全部恰好耗盡，首筆始於 0、末筆結束於 EOF。

`graphic.legacy_raw_palette_index_risk=333139` 描述將完整 CGP 當 raw BGR 時會遇到的舊入口風險；修復後 CGP 入口的實際越界為 0，不應將此計數誤解為本輪解析失敗。

地圖 ground / object 共有 2,504,942 個非零值的出現次數，無法在**僅選定的 Ex** `map_id` 集合中找到。這不是不同 ID 的數目，也不代表地圖檔解析失敗。Ex 為獨立資源集，不能據此假定它含有所有 base 圖塊；本輪未研究多套資源的合併順序。

## 回歸與未驗證範圍

同時以 `--set base` 重跑完整驗證，將三個更名統計鍵正規化、排除新增 `resource.*` 說明後，結果與 `docs/compatibility-validation.txt` 完全一致，包含原本的 strict 異常、警告與 manifest；沒有把 Ex 參數誤套到 base。

本輪已執行並通過：

- `cargo test --locked --offline`：34 個單元測試 + 8 個整合回歸測試。
- `python3 -B -m unittest discover -s tests -p 'test_*.py'`：5 個驗證工具測試，使用暫存的合成資料；驗證 Ex 大小寫、雜湊與實際參數一致、base 預設行為、資料變更回傳 2、無效資源集及早拒絕。
- `cargo fmt --all -- --check`、`cargo clippy --locked --offline --all-targets -- -D warnings`、`cargo doc --locked --offline --no-deps`。

本輪無核心或 WASM 變更，因此未重新執行 WASM target 編譯；前輪已編譯成功。JS runtime、原作畫面、色彩目視、動畫時序與事件、地圖碰撞和跨套資源優先序均未驗證。檔名 Ex 不等於動畫 Extended header；本次樣本仍全為標準 header。
