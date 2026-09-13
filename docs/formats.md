# 二進位格式與 RLE

重建日期：2026-09-13。依據：`src/types/{graphic,anime,palette,map}.rs` 與 `src/rle.rs`。多 byte 整數在現有解析器中均為 **little-endian**，但 RLE 的延伸長度是高位在前的位元拼接，見末節。

下文的欄位名稱沿用原始碼。「實作」代表目前程式如何處理；「實測」只適用於[指定樣本](validation-2026-09-13.md)；「待驗證」代表無足夠證據的語意。不能從 `*_layout_matches_spec` 這類測試名稱推論已有外部權威規格。

## GraphicInfo：40 bytes / 筆

索引檔是連續紀錄，無全檔 header。`GraphicInfo::build_from_bytes` 只接受恰好一筆。

| Offset | 大小 | Rust 型別 | 欄位 | 處理 / 證據 |
| --- | --- | --- | --- | --- |
| 0 | 4 | `i32` | `id` | 識別欄位；實測有重複，不能視為唯一鍵或列號 |
| 4 | 4 | `u32` | `addr` | 圖像資料檔內起始 byte offset；由呼叫端使用 |
| 8 | 4 | `i32` | `len` | 實測可作為含 header 的整筆 byte 長度 |
| 12 | 4 | `i32` | `off_x` | 保存數值；原點及顯示套用方式待驗證 |
| 16 | 4 | `i32` | `off_y` | 同上 |
| 20 | 4 | `i32` | `width` | 像素數量檢查依據之一 |
| 24 | 4 | `i32` | `height` | 同上；實測有一筆 -15 |
| 28 | 1 | `u8` | `grid_w` | 名稱暗示占格寬度；解析器不解釋 |
| 29 | 1 | `u8` | `grid_h` | 名稱暗示占格高度；解析器不解釋 |
| 30 | 1 | `u8` | `access` | 旗標語意未知，不可直接視為可通行 |
| 31 | 5 | `[u8; 5]` | `padding` | 原樣保留 |
| 36 | 4 | `i32` | `map_id` | 候選地圖圖塊對應鍵；尚非已驗證的完整映射 |

實測 252,824 筆的 `[addr, addr + len)` 無重疊、無洞，完整覆蓋 `Graphic_66.bin`。這支持本樣本的切片方式，不代表所有版本都禁止共用資料。

## Graphic：RD header 與 payload

現有 header 長度固定為 16 bytes：

| Offset | 大小 | Rust 型別 | 欄位 | 處理 |
| --- | --- | --- | --- | --- |
| 0 | 2 | `[u8; 2]` | `magic` | 必須為 ASCII `RD` |
| 2 | 1 | `u8` | `version` | bit 0 決定 RLE；值 ≥ 2 決定內嵌 palette 路徑 |
| 3 | 1 | `u8` | `graphic_type` | 保存，不解釋或驗證 |
| 4 | 4 | `i32` | `width` | 保存；library 不與 info.width 比對 |
| 8 | 4 | `i32` | `height` | 保存；library 不與 info.height 比對 |
| 12 | 4 | `i32` | `data_len` | 保存；library 不依它裁切或驗證 |
| 16 | 4（僅 version ≥ 2） | `u32` | 區域變數 `palette_size` | 解碼後尾端色表的 **byte 數**，不儲存於 `GraphicHeader` |

解碼流程：

1. 解析 40-byte info 與 RD header，以 info 的 `width * height` 求預期像素數；負尺寸回傳錯誤。
2. version 0 / 1 從 offset 16 取到呼叫端傳入切片的結尾。version ≥ 2 從 offset 20 取到結尾。
3. `version & 1 == 1` 時 RLE 解碼；否則直接複製 bytes。
4. version ≥ 2 從解碼結果末尾分出 `palette_size` bytes，前段為像素；色表長度不得超過解碼結果。
5. strict 模式要求像素長度等於 `info.width * info.height`；一般模式多則截尾、少則以 0 補滿。
6. version < 2 將第三個參數當原始 BGR 色表；version ≥ 2 將剛分出的尾段當 BGR 色表。兩者皆呼叫 `Palette::build_from_bytes`。

| version | 現有實作的解讀 | 本次真實樣本 |
| --- | --- | --- |
| 0 | 不壓縮、外部原始色表 | 340 筆 |
| 1 | RLE、外部原始色表 | 252,484 筆 |
| 2 | 不壓縮、內嵌色表 | 無 |
| 3 | RLE、內嵌色表 | 無；有合成測試 |
| 4–255 | 仍依上述比較與 bit 0 規則處理 | 無；沒有版本白名單 |

**實測矛盾：** 所有 version 1 的 `data_len` 都與 info.len 相同，包含 header；所有 340 筆 version 0 都不同。version 0 以 16-byte header 解讀，像素數全部吻合；改成 12-byte header 反而每筆多 4 bytes，因此不能僅因 `data_len` 不符就把 header 改為 12 bytes。offset 12 在 version 0 的真正語意仍未知。

**實測異常：** 188 筆 version 1 的解碼資料比像素數多 1 byte；目前寬鬆模式截去最後 1 byte。ID 16681 的 info 與 header 均為 4 × -15、整筆長度 16 bytes，沒有 RLE 資料；兩種模式都拒絕。不能把負值自動取絕對值而宣稱還原正確。

`Graphic.payload` 是未翻轉的索引 byte 序列；程式未實作 scanline 方向、鏡射、畫布定位、透明混合或 `graphic_type` 語意。index 0 透明是色表建構器的規則，尚未以原作畫面確認。

## Palette：CGP 與原始 BGR

### `Palette::build_from_cgp`

實作只接受 **672 = 224 × 3 bytes**。不符合長度時一律回 `BufferTooShort`，即使實際輸入更長。

| 輸出 index | 來源 | 色彩 / alpha 處理 |
| --- | --- | --- |
| 0–15 | `PREFIX_BGR` 固定常數 | BGR → RGB；index 0 alpha=0，其餘 255 |
| 16–239 | 輸入 offset `3 * (index - 16)` | 每 3 bytes 為 B、G、R；alpha=255 |
| 240–255 | `SUFFIX_BGR` 固定常數 | BGR → RGB；alpha=255 |

固定色值依 `src/types/palette.rs`；本次沒有獨立原作畫面證據驗證它們。函式不會讀取檔名、辨識擴充格式或選取場景調色盤。

### `Palette::build_from_bytes`

要求長度可被 3 整除。第 n 個 triple 直接成為第 n 色，順序 B、G、R；輸出第一色 alpha=0，其餘 255。允許空色表，也未限定最多 256 色。**不會加入固定前後色。**

實測所有 35 個 CGP 都是 708 bytes：直接走 raw 入口會得到 236 色，走 CGP 入口全部失敗。僅取前 672 bytes 的診斷可得到 256 色，但剩下 36 bytes 不是可忽略 padding 的既定事實。須確認整個 708-byte 格式後才能決定偏移與截取策略。

## AnimeInfo：12 bytes / 筆

| Offset | 大小 | Rust 型別 | 欄位 | 處理 |
| --- | --- | --- | --- | --- |
| 0 | 4 | `i32` | `id` | 實測重複；不能以它決定檔案切片順序 |
| 4 | 4 | `i32` | `addr` | 呼叫端使用的 byte offset；需要檢查非負 |
| 8 | 2 | `i16` | `act_cnt` | 接續解析的動作紀錄數；負值拒絕 |
| 10 | 2 | `[u8; 2]` | `padding` | 保留；實測不全為零，不能丟棄或強制為零 |

沒有 length 欄位。單筆資料長度可由動作 header 與 frame count 走訪推得；本次工具用下一個較大地址切片，再以解析器要求「恰好耗盡切片」驗證。

## 動作 header 與 frame

每筆 Anime 包含 `act_cnt` 個 `AnimeAction { header, frames }`，每個 header 緊接對應 frames。

| Offset | 大小 | Rust 型別 | 標準 header 欄位 |
| --- | --- | --- | --- |
| 0 | 2 | `i16` | `direct` |
| 2 | 2 | `i16` | `action` |
| 4 | 4 | `i32` | `duration` |
| 8 | 4 | `i32` | `frame_cnt` |

標準長度 12 bytes。若剩餘切片至少 20 bytes 且 offset 16 的 `i32` 等於 -1，實作選擇延伸 header，追加：

| Offset | 大小 | Rust 型別 | 延伸欄位 |
| --- | --- | --- | --- |
| 12 | 2 | `[u8; 2]` | `reserved` |
| 14 | 2 | `i16` | `reversed` |
| 16 | 4 | `i32` | `sentinel`，-1 |

這是啟發式辨識，不是讀取動畫全檔 version。標準格式第一個 frame 若在同一 byte 位置恰好出現 `FF FF FF FF`，有被誤判風險；本次樣本未觀察到這種失敗。所有 311,365 個動作都走標準 header，延伸 header 僅有合成測試支持，不能據此宣稱任何特定遊戲版本都使用它。

每個 frame 固定 10 bytes：

| Offset | 大小 | Rust 型別 | 欄位 |
| --- | --- | --- | --- |
| 0 | 4 | `i32` | `graphic_id` |
| 4 | 2 | `i16` | `off_x` |
| 6 | 2 | `i16` | `off_y` |
| 8 | 2 | `i16` | `flag` |

`frame_cnt` 負值拒絕，0 允許。解析完所有動作仍有 bytes 時回 `TrailingBytes`。`direct` 的方向映射、`action` 列舉、`duration` 的單位及是否為總週期、offset 的合成方式、`flag` 的事件位元、`reversed` 的用途均待驗證。程式只保存欄位，沒有動畫播放語意。

## Map：20-byte header + 三個平面

| Offset | 大小 | Rust 型別 | 欄位 |
| --- | --- | --- | --- |
| 0 | 3 | `[u8; 3]` | `magic`，ASCII `MAP` |
| 3 | 9 | `[u8; 9]` | `reserved`，原樣保留 |
| 12 | 4 | `u32` | `width` |
| 16 | 4 | `u32` | `height` |

令 `N = width * height`：

| 資料範圍（右端不含） | 現有欄位 | 編碼 |
| --- | --- | --- |
| `[20, 20 + 2N)` | `ground` | N 個 little-endian `u16` |
| `[20 + 2N, 20 + 4N)` | `object` | 同上 |
| `[20 + 4N, 20 + 6N)` | `meta` | 同上 |

整檔大小必須恰好是 `20 + 6N`；過短或有尾端資料均拒絕。605 個樣本全部符合。解析器保留各層線性順序，未指定 `(x,y)` 映射、原點或投影。`meta` 的位元與碰撞規則、`object` 是否直接對應 `GraphicInfo.map_id` 尚無完整證據；不可把名稱當成已驗證遊戲規則。不同子目錄有同名 `.dat`，載入器須保存完整相對路徑。

## RLE 命令格式

每個命令的第一 byte 為 flag：高 4 bits 是 opcode，低 4 bits 記為 `n`。`b0`、`b1` 是後續長度 byte。

| opcode | 操作 | 長度 | flag 後的 bytes 順序 |
| --- | --- | --- | --- |
| `0x0` | literal 原樣拷貝 | `n` | N bytes 資料 |
| `0x1` | literal | `(n << 8) \| b0` | b0、N bytes 資料 |
| `0x2` | literal | `(n << 16) \| (b0 << 8) \| b1` | b0、b1、N bytes 資料 |
| `0x8` | repeat 重複 byte | `n` | value |
| `0x9` | repeat | `(n << 8) \| b0` | **value、b0** |
| `0xA` | repeat | `(n << 16) \| (b0 << 8) \| b1` | **value、b0、b1** |
| `0xC` | 填 0 | `n` | 無 |
| `0xD` | 填 0 | `(n << 8) \| b0` | b0 |
| `0xE` | 填 0 | `(n << 16) \| (b0 << 8) \| b1` | b0、b1 |

其餘 opcode（3–7、B、F）回 `InvalidFlag { position, flag }`；缺少所需 bytes 回 `UnexpectedEof { position, needed, remaining }`。解析至輸入結尾，沒有終止標記。長度是直接值，不加 1；零長度命令會被接受，但 encoder 不主動產生它。

自行設計的範例：`03 41 42 43 84 5A C2` → `ABCZZZZ` 後接兩個零 byte。延伸 repeat 的 value 在長度低位 **之前**，例如 `90 58 10` → 16 個 `0x58`。

encoder 單一命令上限 `0x0F_FFFF`，更長的 run 分段。長度 ≤ 15 用短格式、≤ 4095 用中格式，其餘長格式。啟發式選擇至少連續 3 個零才使用 zero-run、至少連續 4 個相同 byte 才使用 repeat，否則累積 literal。這不保證最短編碼，也不保證與原檔壓縮 bytes 完全相同；round-trip 驗證比較的是解碼後資料。

一般入口使用 scalar。顯式 `*_simd` 在 aarch64 使用 NEON、x86_64 使用 SSE2，其餘架構有 fallback；WASM 沒有專用 SIMD 分支。本次只在 Apple ARM64 執行，未量測效能，也未驗證 x86_64 執行結果。
