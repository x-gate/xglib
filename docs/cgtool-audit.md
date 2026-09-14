# CGTool 相容性對照

日期：2026-09-14。使用者指定 HonorLee-cn/CGTool 為本次正確行為基準，固定 commit `b4d08112524aa16b9fdb416ef865f8c196ffac20`（2024-07-12）。本次只參考公開流程並獨立實作；沒有匯入 CGTool 原始碼、MapExtra 資料表或遊戲素材。

## 解析修正

| 項目 | 結論與修正 | 參考 |
| --- | --- | --- |
| CGP 固定色 | index 4 應為 RGB(128,0,128)，index 5 為 RGB(0,0,128)；原本對調。其他固定色、224 色 BGR、自訂色不依 RGB 透明的行為一致。 | [Palet.cs](https://github.com/HonorLee-cn/CGTool/blob/b4d08112524aa16b9fdb416ef865f8c196ffac20/CrossgateToolkit/Palet.cs) |
| 空內嵌色表 | version 2/3 的 palette size = 0 時，改用呼叫者明確提供的 raw BGR 或 CGP；不再回傳空色表。非空內嵌色表仍優先。 | [GraphicData.UnpackGraphic](https://github.com/HonorLee-cn/CGTool/blob/b4d08112524aa16b9fdb416ef865f8c196ffac20/CrossgateToolkit/GraphicData.cs) |
| 內嵌色表位置 | 起點為 GraphicInfo.width × height，不能從解碼結果末端倒算。寬鬆模式先正規化完整 pixels + palette stream，再切分；strict 仍拒絕長度異常。 | 同上 |
| 壓縮資料邊界 | 若 RD DataLen 在 header 長度與索引切片長度之間，使用它界定 RLE stream，排除後續容器資料。未壓縮資料不以 DataLen 尋址。 | 同上 |
| RLE 長指令 | literal 0x20–0x7f、repeat 0xa0–0xbf、zero 0xe0–0xff 使用 5-bit 長度高位；literal 高旗標以 0x20 週期重複。補齊原先誤拒絕的 0x3?/4?/5?/6?/7?/b?/f?。 | 同上 DecompressJob.Execute（實際呼叫路徑，非 TestDecompress） |
| 動畫 header | CGTool 每個動作偵測一次 offset 16 sentinel。既有 `anime_build_from_bytes` 符合此流程；rsc-manager 改回使用它。 | [Anime.ReadAnimeData](https://github.com/HonorLee-cn/CGTool/blob/b4d08112524aa16b9fdb416ef865f8c196ffac20/CrossgateToolkit/Anime.cs) |

## 已核對且保持的契約

GraphicInfo 40 bytes、AnimeInfo 12 bytes、動畫 standard/extended 12/20 bytes、frame 10 bytes，以及 RD 16/20-byte header 與奇數 version RLE 規則相符。GraphicInfo 第一欄對應 CGTool Index，末欄 map_id 對應 Serial；frame.graphic_id 透過第一欄查找，地圖格值與隱藏色表透過末欄查找，不是直接用索引列位置。

CGTool [GraphicInfo.cs](https://github.com/HonorLee-cn/CGTool/blob/b4d08112524aa16b9fdb416ef865f8c196ffac20/CrossgateToolkit/GraphicInfo.cs) 將 byte 28/29 作 East/South 占格、byte 30 偶數作 Blocked、byte 31 等於 1 作 AsGround；xglib 保留原始 grid_w/grid_h/access/padding。延伸動畫 reserved 兩個 bytes 是 little-endian Palet、reversed 是 bit flags：1 水平鏡射、2 垂直鏡射、4 LOCK_PAL、8 LIGHT_THROUGH。CGTool 目前只把前兩者用於播放器；不據名稱自行實作後兩者或用 Palet 強制選色。

[Map.cs](https://github.com/HonorLee-cn/CGTool/blob/b4d08112524aa16b9fdb416ef865f8c196ffac20/CrossgateToolkit/Map.cs) 的 client MAP 資料起點 20、ground/object little-endian 平面與 xglib 相符。CGTool 在呈現前上下翻轉地圖列；xglib 保留原始順序，map-viewer 將相同方向轉換放在投影層，不改寫原始地圖座標。CGTool 只用兩平面；xglib 另外保存第三 meta 平面供研究。

## 有意保留的差異與範圍

- strict 模式繼續拒絕像素數不符、色表越界與截斷。CGTool 固定配置長度、越界顏色透明等寬鬆行為不作為靜默修復；viewer 顯示診斷。raw/CGP bytes 不以長度互相猜測。
- 不一致或未指定的壓縮 DataLen 保留舊 API 全切片行為；有效 DataLen 的截取才採新規則。
- 既有 signed 欄位、raw reserved bytes、Map u32 尺寸、完整三平面 strict 契約保持相容。CGTool 對 client 尺寸只讀低 u16；本次沒有資料證據認定高 16 bits 的意義，不將其丟棄。
- `rle_encode` 仍輸出既有較小的 canonical 分段（最大 0x0fffff），新 decoder 接受更廣的 CGTool 指令；`RleError::InvalidFlag` 留作 Rust source compatibility，新流程所有 flag byte 均有定義，截斷回 UnexpectedEof。
- 明確指定整段 header 的 API 保留，供 xgtool 使用端使用；不是 CGTool 預設解析規則。sentinel 啟發式遇到 standard frame 的兩個 -1 偏移可能誤判，此特性與 CGTool 相同。
- 加密容器、LS2MAP 伺服器地圖、MapExtra 場景資料、碰撞／路徑、音效播放與 Unity 圖集合批不屬三個 repository 目前宣告的檢視範圍；本次未新增。CGTool 自身也註明不支援 4.0+ 地圖。

## 整合與驗證

依賴方向為 map-viewer → xglib、rsc-manager → xglib，兩個 viewer 無執行時互相依賴。更新 xglib 後，分別在 viewer 執行 `bun run build:wasm`、`bun run test`、`bun run build`、`E2E_PREVIEW=1 bun run test:e2e`。舊 WASM 不會自動獲得色表、RLE 修正；必須重建，沒有遊戲資料遷移。

Rust 合成回歸覆蓋固定色、空色表、色表切分、壓縮邊界、擴充 RLE 長度與截斷、scalar/SIMD 相同輸出；viewer 另測真實 WASM、動畫鏡射／時間／調色盤／混合 header，以及地圖格距、尺寸來源與試放還原。未啟動 Unity、未使用原版素材比圖，也未對 x86_64 SIMD 執行測試；此次原生平台為 aarch64，兩個 viewer 均重建 wasm32 bindings。

本次 xglib 執行結果：`cargo test --locked --offline` 共 82 項通過（67 單元、3 layout、12 相容性）；rustfmt、Clippy（拒絕 warnings）、cargo doc 通過；Python 驗證工具 5 項通過。兩個 viewer 的 wasm32 release 與 JS bindings 已重建，Bun 的真實 WASM 測試通過。沒有對原版目錄執行本機資源全量掃描。
