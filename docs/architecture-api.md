# 架構與 API

重建日期：2026-09-13。依據為本 repository 原始碼；實際樣本覆蓋見[驗證報告](validation-2026-09-13.md)。

## 模組與資料流

| 檔案 | 責任 |
| --- | --- |
| `src/lib.rs` | 模組公開與主要型別、常數重匯出 |
| `src/build_error.rs` | 統一二進位解析錯誤 |
| `src/rle.rs` | 純 bytes 編解碼；scalar 與 SIMD 版本 |
| `src/types/graphic.rs` | 40-byte 索引、RD header、像素索引與色表 |
| `src/types/anime.rs` | 12-byte 索引、動作 header 與 frames |
| `src/types/palette.rs` | 原始 BGR / 外部 CGP → `Vec<Srgba<u8>>` |
| `src/types/map.rs` | MAP header 與三個 `u16` 平面 |
| `src/wasm.rs` | bytes 參數 → Rust 解析 → Serde → `JsValue` |
| `xglib.d.ts` | 手寫 TypeScript 介面，不是已發布套件或載入器 |
| `examples/verify_resources.rs` | 選用的本機唯讀資源掃描器 |
| `scripts/verify_resources.py` | base / ex 資源集選擇、檔案指紋與驗證前後完整性比較 |

函式庫不做檔案尋址、檔名配對、ID 解決、圖片輸出、座標轉換或播放。呼叫端依序完成：讀索引 → 切單筆 bytes → 呼叫解析 → 解決 ID / 色表 → 自行呈現。

## Rust 公開介面

| 入口 | 輸入 | 回傳 |
| --- | --- | --- |
| `GraphicInfo::build_from_bytes` | 恰好 40 bytes | `Result<GraphicInfo, BuildError>` |
| `Graphic::build_from_bytes` | 單筆 info、單筆 RD record、原始 BGR 色表 | `Result<Graphic, BuildError>`，像素長度不符時補零或截斷 |
| `Graphic::strict_build_from_bytes` | 同上 | 像素長度不符即錯誤；並不全面驗證 header 或色表索引 |
| `Graphic::build_from_cgp` / `strict_build_from_cgp` | 單筆 info、單筆 RD record、外部 CGP | 一般 / 嚴格像素長度模式；皆檢查色表索引，內嵌色表優先 |
| `AnimeInfo::build_from_bytes` | 恰好 12 bytes | `Result<AnimeInfo, BuildError>` |
| `Anime::build_from_bytes` | 單筆 info、恰好單筆動畫資料 | `Result<Anime, BuildError>` |
| `Palette::build_from_cgp` | 672 有效 bytes 或完整 708-byte CGP | 224 色加固定前後各 16 色，共 256 色 |
| `Palette::build_from_bytes` | 長度為 3 的倍數的原始 BGR bytes | 原樣數量的色表，可為空，不補固定色 |
| `Map::build_from_bytes` | 恰好一個 MAP 檔案 | header 與 `ground` / `object` / `meta` |
| `rle_decode` / `rle_encode` | `&[u8]` | `Result<Vec<u8>, RleError>` / `Vec<u8>` |
| `rle::rle_decode_simd` / `rle::rle_encode_simd` | 同上 | 同上；SIMD 版本未於 crate root 重匯出 |

結構多數有 `Debug`、`Clone`、相等比較及 Serde 支援；packed header / info / frame 的 Serde 實作先複製欄位再序列化。Serde 不是原版 binary writer，也不會重新執行 `build_from_bytes` 的驗證。除 RLE 外，尚無資源檔案編碼 / 回寫 API。

### 單筆圖像的切片方式

此範例函式可放入使用端；`raw_bgr` 應由呼叫端提供已確認的完整色表，不可直接把尚未確認格式的 `.cgp` 當成它：

```rust
fn parse_graphic_at(
    index: &[u8], data: &[u8], row: usize, raw_bgr: &[u8],
) -> Result<xglib::Graphic, String> {
    let start = row.checked_mul(xglib::GRAPHIC_INFO_SIZE).ok_or("row overflow")?;
    let end = start.checked_add(xglib::GRAPHIC_INFO_SIZE).ok_or("row overflow")?;
    let info_bytes = index.get(start..end).ok_or("index range")?;
    let info = xglib::GraphicInfo::build_from_bytes(info_bytes)
        .map_err(|e| format!("{e:?}"))?;
    let addr = usize::try_from(info.addr).map_err(|_| "address overflow")?;
    let len = usize::try_from(info.len).map_err(|_| "negative length")?;
    let end = addr.checked_add(len).ok_or("data range overflow")?;
    let record = data.get(addr..end).ok_or("data range")?;
    xglib::Graphic::strict_build_from_bytes(info_bytes, record, raw_bgr)
        .map_err(|e| format!("{e:?}"))
}
```

索引檔本身還要驗證長度可整除 40。`Graphic::build_from_bytes` 不檢查傳入切片是否與 `info.addr` / `info.len` 一致；地址只作為欄位保存。

動畫索引沒有資料長度欄位。本次驗證將所有非負、唯一的 `addr` 排序，以「下一個較大的地址」作為終點，最後一筆使用檔案長度，再按原索引列解析。這是本次樣本支持的容器切片策略，非 library 自動行為；若未來資料有共用地址、洞或不同容器結構，需要重新研究。勿依 `id` 排序切片：樣本 ID 大量重複。

### 調色盤整合

既有 `Graphic::*build_from_bytes` 在 version < 2 將第三個參數解讀為原始 BGR；不變更此契約、不根據長度猜 CGP，以免 672/708-byte raw 色表被誤判。

新使用端直接傳入 `.cgp` bytes 至 `Graphic::build_from_cgp` 或 `Graphic::strict_build_from_cgp`。支援 672 有效 bytes 與完整 708-byte 檔案。`CGP_SIZE` / `cgp_size()` 保持 672；`CGP_FILE_SIZE` / `cgp_file_size()` 提供 708。

version ≥ 2 在所有入口都使用內嵌色表，忽略外部參數；因此 CGP builder 在此情況下不要求有效外部 CGP。新入口會檢查輸出像素索引界限，包括內嵌色表。原有 raw BGR 入口仍保持原始行為。

`payload` 是 palette index 的向量，不是 RGBA。CGP 一般入口仍會截尾 / 補零；研究或驗證用途選 strict，明確處理錯誤，不以補零掩蓋來源異常。

## WASM / TypeScript

`src/wasm.rs` 匯出 `graphic_build_from_bytes`、`anime_build_from_bytes`、`map_build_from_bytes`、`game_palette_build_from_cgp`、`game_palette_build_from_bytes`，以及各格式大小常數的 getter（名稱見 `xglib.d.ts`）。新增 `graphic_build_from_cgp`、`graphic_strict_build_from_cgp`、`graphic_strict_build_from_bytes` 與 `cgp_file_size`。原有 `graphic_build_from_bytes` 仍是 raw BGR 寬鬆模式；RLE 尚未匯出。

輸入為 `Uint8Array`。輸出以 `serde_wasm_bindgen::to_value` 轉換；目前手寫宣告以 JS object 與 `number[]` 描述，`AnimeHeader` 為 `{ Standard: ... } | { Extended: ... }`，色彩為 `red / green / blue / alpha`。這些型別宣告未由本次 JS runtime 測試確認。

失敗時 `Result<JsValue, JsValue>` 走 JS throw 路徑，內容是 Debug / Serde 錯誤**字串**，並非有 `code` 欄位的結構化錯誤，也不保證是 `Error` instance。

repository 未含 `package.json`、JS glue、bundler 或自動產生 `.d.ts` 的流程。若要準備 WASM，本次已安裝 WASM target 並完成以下 `cargo build`；CLI 安裝、JS glue 產生與 JS runtime 驗證仍未執行：

```sh
rustup target add wasm32-unknown-unknown
cargo build --locked --release --target wasm32-unknown-unknown
# CLI 版本需與 Cargo.lock 的 wasm-bindgen 相同；本次 lock 為 0.2.118。
cargo install wasm-bindgen-cli --version 0.2.118 --locked
wasm-bindgen target/wasm32-unknown-unknown/release/xglib.wasm --target web --out-dir target/wasm-web
```

之後仍需載入產生的 JS module 並初始化 WASM，補上瀏覽器或 Node 測試。`xglib.d.ts` 不會自動被上述 CLI 合併；產生的綁定可能仍將 `JsValue` 標成 `any`。不要宣稱目前已可直接 `npm install xglib`。

## 錯誤與限制

`BuildError` 包含 `BufferTooShort`、`InvalidMagic`、`InvalidValue`、`Unsupported`、`TrailingBytes`、`Rle`；`Unsupported` 目前沒有使用的解析分支。錯誤有 context，但缺少檔名與整體索引列位置，應由上層補上。`BuildError` / `RleError` 尚未實作 `Display` 或 `std::error::Error`，範例因此使用 `format!("{e:?}")`。

原有 raw BGR `strict` 只加強像素長度檢查。新增的 CGP 圖像入口也驗證 palette index；所有入口均未全面檢查 metadata、版本白名單或圖像/動畫 ID 關係。RLE 沒有解壓輸出上限；動畫依 frame count 預先配置。將此 library 用於不受信任資料前，需要另外強化資源上限與檢查其餘整數運算。本次是特定本機資料的相容性研究，未進行 fuzzing 或任意輸入安全性驗證。

2026-09-13 補測時，合成的超大地圖尺寸重現了 `20 + layer_size * 3` 的整數溢位 panic。Map 現在對三個平面總長度的乘法及 header 加法都使用 checked arithmetic；溢位回傳 `BuildError::InvalidValue { context: "map header", message: "map total size overflow" }`。有效地圖的解析方式不變；此修正不等於新增記憶體配置上限。

## 驗證工具的資源集

Python 入口支援 `--set base`（預設）與 `--set ex`，四個檔名只由 `RESOURCE_SETS` 選定，再同時用於檔案雜湊與 Rust 執行參數。共享調色盤與地圖仍取自同一 Assets。未另外載入或合併另一組圖像索引。

Rust 範例維持單一 Assets 參數的 base 預設行為；亦可指定四個 bin 目錄下的純檔名，順序為 GraphicInfo、Graphic、AnimeInfo、Anime。路徑或不完整參數組會拒絕。輸出 `resource.graphic_info`、`resource.graphic`、`resource.anime_info`、`resource.anime` 供核對。

引用統計現改為 `anime.frames_missing_in_selected_graphics`、`anime.unique_refs_missing_in_selected_graphics`、`map.nonzero_tiles_without_selected_map_id`，取代先前硬編碼的 `graphic_66` 字樣。舊驗證報告保留舊名稱；消費診斷文字的腳本須同步調整。這不是函式庫 API 或二進位格式變更。

## 2026-09-14：明確指定整段動畫 header 長度

新增 `Anime::build_from_bytes_with_header_size(info_bytes, data_bytes, header_size)` 與 WASM 同名 snake-case 入口 `anime_build_from_bytes_with_header_size`；`header_size` 只接受 12 或 20。用於按照容器起點判定一次 layout 的使用端，避免後續 frame offsets 被誤認為 sentinel。舊入口保持逐動作自動判斷；輸出 Anime 結構與 strict 完整切片要求不變。來源、合成回歸案例與 rsc-manager 整合方式見 [明確動畫 layout](animation-layout.md)。
