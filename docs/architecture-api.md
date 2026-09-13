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
| `scripts/verify_resources.py` | 檔案指紋清單與驗證前後完整性比較 |

函式庫不做檔案尋址、檔名配對、ID 解決、圖片輸出、座標轉換或播放。呼叫端依序完成：讀索引 → 切單筆 bytes → 呼叫解析 → 解決 ID / 色表 → 自行呈現。

## Rust 公開介面

| 入口 | 輸入 | 回傳 |
| --- | --- | --- |
| `GraphicInfo::build_from_bytes` | 恰好 40 bytes | `Result<GraphicInfo, BuildError>` |
| `Graphic::build_from_bytes` | 單筆 info、單筆 RD record、原始 BGR 色表 | `Result<Graphic, BuildError>`，像素長度不符時補零或截斷 |
| `Graphic::strict_build_from_bytes` | 同上 | 像素長度不符即錯誤；並不全面驗證 header 或色表索引 |
| `AnimeInfo::build_from_bytes` | 恰好 12 bytes | `Result<AnimeInfo, BuildError>` |
| `Anime::build_from_bytes` | 單筆 info、恰好單筆動畫資料 | `Result<Anime, BuildError>` |
| `Palette::build_from_cgp` | 恰好 672 bytes | 224 色加固定前後各 16 色，共 256 色 |
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

`Graphic` version < 2 一律使用 `Palette::build_from_bytes(palette_bytes)`。如果已經透過正確的 CGP 格式載入出 `Palette`，Rust 呼叫端可以將每色的 `blue, green, red` 依序串成 BGR bytes，再傳入圖像解析器；或解析後明確替換公開的 `graphic.palette`。這只解決 API 銜接，**不解決本次 708-byte CGP 的未定義布局**。

version ≥ 2 會從解碼資料尾端取內嵌色表，忽略第三個參數。此分支本次沒有真實樣本，只有合成測試。`payload` 是 palette index 的向量，不是 RGBA；使用 `palette.colors[index]` 前必須檢查界限。

## WASM / TypeScript

`src/wasm.rs` 匯出 `graphic_build_from_bytes`、`anime_build_from_bytes`、`map_build_from_bytes`、`game_palette_build_from_cgp`、`game_palette_build_from_bytes`，以及各格式大小常數的 getter（名稱見 `xglib.d.ts`）。未匯出 strict graphic builder 或 RLE；JS 呼叫 `graphic_build_from_bytes` 使用的是寬鬆模式。

輸入為 `Uint8Array`。輸出以 `serde_wasm_bindgen::to_value` 轉換；目前手寫宣告以 JS object 與 `number[]` 描述，`AnimeHeader` 為 `{ Standard: ... } | { Extended: ... }`，色彩為 `red / green / blue / alpha`。這些型別宣告未由本次 JS runtime 測試確認。

失敗時 `Result<JsValue, JsValue>` 走 JS throw 路徑，內容是 Debug / Serde 錯誤**字串**，並非有 `code` 欄位的結構化錯誤，也不保證是 `Error` instance。

repository 未含 `package.json`、JS glue、bundler 或自動產生 `.d.ts` 的流程。若要準備 WASM，以下是預備建置流程，**本次未執行**：

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

`strict` 只加強像素長度檢查，不檢查所有 metadata、版本白名單、palette index 或圖像/動畫 ID 關係。RLE 沒有解壓輸出上限；動畫依 frame count 預先配置，Map 的最後 `20 + layer_size * 3` 也不是完整 checked arithmetic。將此 library 用於不受信任資料前，需要另外強化資源上限與所有整數運算。本次是特定本機資料的相容性研究，未進行 fuzzing 或任意輸入安全性驗證。
