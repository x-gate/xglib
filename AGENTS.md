# xglib 維護指引

本檔適用於 `xglib/`。同時遵守工作區上層 `AGENTS.md`；更深層指令及使用者當前明確要求優先。

## 範圍與基線

- 此處是獨立 Rust library repository，使用 edition 2024、Cargo 與已提交的 `Cargo.lock`；不是遊戲 client，也沒有資源檔案發布流程。
- 開始前閱讀 `README.md`、`Cargo.toml`、`docs/architecture-api.md`，並在本 repository 確認 `git status --short`。
- 格式變更先讀 `docs/formats.md`、`docs/compatibility.md`、`docs/validation-ex.md` 與 `docs/validation-2026-09-13.md`。匯入程式碼是研究證據，不等同完整或權威規格。
- 目前未訂定 MSRV、正式授權與發布方案；不得將臨時選擇寫成既定標準。

## 程式碼慣例

- 公開入口在 `src/lib.rs`；二進位解析放 `src/types/`，RLE 放 `src/rle.rs`，JS 轉接放 `src/wasm.rs`。保持核心解析與檔案 I/O、渲染、平台整合分離。
- 明確讀取 little-endian 欄位；不要用記憶體轉型取代 bytes 邊界檢查。`#[repr(C, packed)]` 欄位先複製到 local 再借用，避免未對齊參照。
- 保留 signed 欄位、padding / reserved 與未知旗標；不要僅依名稱推定碰撞、方向、時間單位或顯示規則。
- 保留嚴格與寬鬆圖像解析差異。不得為了讓樣本「通過」而默默補零、截尾、取絕對值或丟棄重複 ID。
- `Graphic` 的 palette bytes 與 CGP 檔案不是相同契約。若修正調色盤或改變公開結構，同步更新 Rust、WASM、`xglib.d.ts`、文件與合成回歸測試。
- 修改 SIMD / unsafe 時，必須檢查指標範圍、容量與 `set_len` 條件，並驗證 scalar / SIMD 結果相同。未執行的平台不得宣稱已驗證。
- 未信任輸入的尺寸、解碼輸出與配置量需要明確上限；目前 API 尚非完整強化的任意檔案解析器。

## 原始資源與研究證據

- `../CGoriginmood/` 為嚴格唯讀參考。不得在其中寫入、產生快取、建立連結、啟動遊戲、執行會寫入的轉換器或工具。
- 不得複製或提交原版素材、原始碼、解碼圖像、地圖或動畫內容。單元測試使用自行產生的最小 bytes；真實檔案驗證只接受外部路徑，輸出統計、雜湊與必要診斷。
- `examples/verify_resources.rs` 直接呼叫 library；`scripts/verify_resources.py` 比較輸入檔案的執行前後 SHA-256。研究日誌放 `target/` 或明確允許的暫存目錄。使用 `--set base` / `--set ex` 明確選擇資源集；報告須核對輸出的四個 `resource.*` 檔名。
- 報告須區分「程式碼行為」、「樣本實測」、「待驗證推論」與「自行設計」。記錄日期、基準 commit、檔案指紋、命令、數量與失敗案例；解析成功不等於畫面、動畫時序或玩法語意正確。
- ID 已知會重複。新增載入器時保留來源檔案、索引列與地址；任何覆蓋優先序都須另有依據。

## 驗證與交付

```sh
cargo test --locked --offline
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo doc --locked --offline --no-deps
python3 -B -m unittest discover -s tests -p 'test_*.py'
```

依賴未快取時先用 `cargo fetch --locked`。匯入基準曾有 rustfmt / Clippy 問題，相容性分支已清理；後續變更維持檢查通過，不要藉此大幅重構或升級套件。

修改解析行為時新增能重現問題的合成回歸測試；修改 RLE 時涵蓋邊界長度、截斷、非法 flag 與 scalar / SIMD 對照。需要真實資料時，依 README 重跑唯讀驗證，檢查結尾的完整性標記與輸入未改變標記；exit 1 可能是已記錄的相容性問題，不能直接當成驗證成功。

交付列出變更範圍、已執行與未執行的檢查、相容性影響。涉及 WASM 必須區分 Rust host 編譯、WASM target 編譯與 JS runtime 測試；本機原生測試不涵蓋後兩者。
