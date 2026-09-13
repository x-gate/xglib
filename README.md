# xglib

用 Rust 實作的《魔力寶貝》（Cross Gate／クロスゲート）資源格式研究函式庫，提供圖像、動畫、調色盤、地圖解析，以及 RLE 編解碼。這是 `x-gate` 工作區中的獨立 repository；不含遊戲引擎、素材、完整資源載入器或渲染器。

本文件於 2026-09-13 根據匯入的原始碼重建。原始研究筆記已遺失，因此以下文件描述的是**現有實作與本次樣本的觀察**，並非官方格式規格。

## 目前可用程度

相容性修復後，指定本機樣本的全量驗證結果：

| 範圍 | 結果 |
| --- | --- |
| `GraphicInfo_66.bin` / `Graphic_66.bin` | 252,824 筆；252,635 筆通過嚴格像素長度檢查，188 筆多解出 1 byte，1 筆負高度無法解析 |
| `AnimeInfo_4.bin` / `Anime_4.bin` | 3,186 筆全部解析成功，2,413,106 個 frame 的圖像 ID 都存在於指定圖像索引 |
| `pal/*.cgp` | 35 個 708-byte CGP 全部載入成功；以明確 CGP 圖像入口解析後，色彩索引越界為 0 |
| `map/**/*.dat` | 605 個全部解析成功，共 5,219,473 格；地圖圖塊與完整素材對應仍未驗證 |

**可作為結構解析與後續研究基礎，尚不能視為完整、正確的遊戲素材載入方案。** 已修復 CGP 長度與圖像色表整合問題；ID 重複、部分圖像長度異常及畫面語意仍需保留診斷。詳見[相容性修復報告](docs/compatibility.md)，以及保留的[修復前驗證報告](docs/validation-2026-09-13.md)。

## 開發環境與安裝

- Rust / Cargo：支援 edition 2024 的工具鏈；本次實測 `rustc 1.98.1`、`cargo 1.98.1`，`aarch64-apple-darwin`。未宣告或驗證最低支援版本（MSRV）。
- Python 3.10+：僅重跑附帶的資源驗證包裝腳本時需要，無第三方 Python 套件。
- 無資料庫、網路服務或環境變數設定需求。函式庫接收 bytes；檔案位置由呼叫端決定。
- Rust 依賴以 [Cargo.toml](Cargo.toml) 與 [Cargo.lock](Cargo.lock) 為準：`palette`、`serde`、`serde-wasm-bindgen`、`wasm-bindgen`。

在此 repository 執行：

```sh
cargo fetch --locked
cargo build --locked
cargo test --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo doc --locked --no-deps
```

已有依賴快取時可加入 `--offline`（`cargo fmt` 除外）。目前 34 個單元測試與 8 個相容性回歸測試通過，rustfmt、嚴格 Clippy 與文件建置也通過。WASM target 已編譯成功；尚未執行 JS runtime 測試。

其他 Rust repository 可使用 path dependency；路徑相對於該 repository 的 `Cargo.toml`：

```toml
[dependencies]
xglib = { path = "../xglib" }
```

未設定 crates.io 發布、npm 套件或 CI 流程。輸出包含 `rlib` 與 `cdylib`；`cdylib` 不代表已有可供 C 呼叫的 ABI。

## Rust 使用範例

以下只使用自行產生的 bytes，不需要原版素材：

```rust
use xglib::{Palette, rle_decode, rle_encode};

let pixels = [0, 0, 0, 7, 7, 7, 7];
assert_eq!(rle_decode(&rle_encode(&pixels)).unwrap(), pixels);

// 原始 BGR 色表；index 0 的 alpha 由函式庫設為 0。
let palette = Palette::build_from_bytes(&[0, 0, 0, 0x10, 0x20, 0x30]).unwrap();
assert_eq!(palette.colors[1].red, 0x30);
```

`GraphicInfo` 必須傳入恰好 40 bytes，`AnimeInfo` 恰好 12 bytes。`Graphic` / `Anime` 接收單筆資料切片，不會自行根據 `addr` 尋址。研究外部 CGP 圖像時優先用 `Graphic::strict_build_from_cgp`；`Graphic::build_from_cgp` 保留原有的截尾 / 補零行為。raw BGR 使用原本的 `*_build_from_bytes` 入口。

`Graphic` 第三個參數是原始 BGR 色表，**不會自動呼叫 `Palette::build_from_cgp`**。直接傳入 `.cgp` 可能成功返回卻產生錯誤色表，應改用新增的 `Graphic::build_from_cgp` / `strict_build_from_cgp`。兩者皆接受 672 或 708 bytes，並檢查像素索引界限；version ≥ 2 仍優先使用內嵌色表。完整切片範例與 WASM 限制見 [API 與架構](docs/architecture-api.md)。

## 重跑本機資源驗證

從 `xglib` 目錄執行；參數是 `Assets` 目錄，不是遊戲根目錄：

```sh
cargo build --locked --offline --release --example verify_resources
python3 scripts/verify_resources.py ../CGoriginmood/Assets > target/resource-audit.txt
```

腳本只讀取指定四個 `.bin`、`bin/pal` 與 `map`；不啟動遊戲、不輸出解碼素材。它會列出檔案大小與 SHA-256，呼叫 Rust 範例，再次比對輸入清單、大小與雜湊。日誌寫在已忽略的 `target/`。目前樣本預期回傳 **exit 1**，日誌末端包含 `audit_complete=true`、`inputs_unchanged=true`，代表掃描完成但仍有 188 筆像素長度不符與 1 筆負高度；不會將剩餘問題隱藏成成功。

資料是選用的本機研究輸入，不隨 repository 提供；一般單元測試完全不依賴它。完整掃描一次載入約 637 MB 的圖像檔，另有索引、解碼與執行時記憶體需求。不要將輸出重導至 `CGoriginmood/`。

## 文件導覽

- [AGENTS.md](AGENTS.md)：此 repository 的修改、驗證與素材處理規則。
- [API 與架構](docs/architecture-api.md)：模組責任、Rust / WASM 介面、錯誤與整合方式。
- [二進位格式與 RLE](docs/formats.md)：欄位 offset、解碼流程、推論邊界。
- [相容性修復報告](docs/compatibility.md)：修復依據、API 遷移方式、前後對照與剩餘問題。
- [2026-09-13 驗證報告](docs/validation-2026-09-13.md)：樣本指紋、方法、成功範圍與已知問題。

## 授權與歸屬

此 repository 尚未附帶 LICENSE，`Cargo.toml` 也未宣告授權；請勿自行假定可依某個開源授權散布。原版資源不在本專案的授權範圍內，亦不得納入提交、套件或部署產物。本專案為非官方研究，未聲稱 Square Enix 授權或背書；相關遊戲名稱與商標屬各權利人。
