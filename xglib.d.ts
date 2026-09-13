export interface RgbaColor {
  red: number;
  green: number;
  blue: number;
  alpha: number;
}

export interface Palette {
  colors: RgbaColor[];
}

export interface GraphicInfo {
  id: number;
  addr: number;
  len: number;
  off_x: number;
  off_y: number;
  width: number;
  height: number;
  grid_w: number;
  grid_h: number;
  access: number;
  padding: [number, number, number, number, number];
  map_id: number;
}

export interface GraphicHeader {
  magic: [number, number];
  version: number;
  graphic_type: number;
  width: number;
  height: number;
  data_len: number;
}

export interface Graphic {
  info: GraphicInfo;
  header: GraphicHeader;
  payload: number[];
  palette: Palette;
}

export interface AnimeInfo {
  id: number;
  addr: number;
  act_cnt: number;
  padding: [number, number];
}

export interface AnimeHeaderStandard {
  direct: number;
  action: number;
  duration: number;
  frame_cnt: number;
}

export interface AnimeHeaderExtended {
  direct: number;
  action: number;
  duration: number;
  frame_cnt: number;
  reserved: [number, number];
  reversed: number;
  sentinel: number;
}

export type AnimeHeader =
  | { Standard: AnimeHeaderStandard }
  | { Extended: AnimeHeaderExtended };

export interface AnimeFrame {
  graphic_id: number;
  off_x: number;
  off_y: number;
  flag: number;
}

export interface AnimeAction {
  header: AnimeHeader;
  frames: AnimeFrame[];
}

export interface Anime {
  info: AnimeInfo;
  actions: AnimeAction[];
}

export interface MapHeader {
  magic: [number, number, number];
  reserved: [
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
  ];
  width: number;
  height: number;
}

export interface Map {
  header: MapHeader;
  ground: number[];
  object: number[];
  meta: number[];
}

export function graphic_info_size(): number;
export function graphic_header_size(): number;
export function anime_info_size(): number;
export function anime_header_standard_size(): number;
export function anime_header_extended_size(): number;
export function anime_frame_size(): number;
export function map_header_size(): number;
export function cgp_size(): number;
export function palette_color_count(): number;
export function cgp_custom_color_count(): number;
export function palette_fixed_prefix_count(): number;
export function palette_fixed_suffix_count(): number;
export function embedded_color_stride(): number;

export function graphic_build_from_bytes(
  info_bytes: Uint8Array,
  data_bytes: Uint8Array,
  palette_bytes: Uint8Array,
): Graphic;

export function game_palette_build_from_cgp(bytes: Uint8Array): Palette;

export function game_palette_build_from_bytes(bytes: Uint8Array): Palette;

export function map_build_from_bytes(bytes: Uint8Array): Map;

export function anime_build_from_bytes(
  info_bytes: Uint8Array,
  data_bytes: Uint8Array,
): Anime;
