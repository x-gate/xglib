use wasm_bindgen::prelude::*;

use crate::{Anime, Graphic, Map, Palette};

#[wasm_bindgen]
pub fn graphic_info_size() -> u32 {
    crate::GRAPHIC_INFO_SIZE as u32
}

#[wasm_bindgen]
pub fn graphic_header_size() -> u32 {
    crate::GRAPHIC_HEADER_SIZE as u32
}

#[wasm_bindgen]
pub fn anime_info_size() -> u32 {
    crate::ANIME_INFO_SIZE as u32
}

#[wasm_bindgen]
pub fn anime_header_standard_size() -> u32 {
    crate::ANIME_HEADER_STANDARD_SIZE as u32
}

#[wasm_bindgen]
pub fn anime_header_extended_size() -> u32 {
    crate::ANIME_HEADER_EXTENDED_SIZE as u32
}

#[wasm_bindgen]
pub fn anime_frame_size() -> u32 {
    crate::ANIME_FRAME_SIZE as u32
}

#[wasm_bindgen]
pub fn map_header_size() -> u32 {
    crate::MAP_HEADER_SIZE as u32
}

#[wasm_bindgen]
pub fn cgp_size() -> u32 {
    crate::CGP_SIZE as u32
}

#[wasm_bindgen]
pub fn cgp_file_size() -> u32 {
    crate::CGP_FILE_SIZE as u32
}

#[wasm_bindgen]
pub fn palette_color_count() -> u32 {
    crate::PALETTE_COLOR_COUNT as u32
}

#[wasm_bindgen]
pub fn cgp_custom_color_count() -> u32 {
    crate::CGP_CUSTOM_COLOR_COUNT as u32
}

#[wasm_bindgen]
pub fn palette_fixed_prefix_count() -> u32 {
    crate::PALETTE_FIXED_PREFIX_COUNT as u32
}

#[wasm_bindgen]
pub fn palette_fixed_suffix_count() -> u32 {
    crate::PALETTE_FIXED_SUFFIX_COUNT as u32
}

#[wasm_bindgen]
pub fn embedded_color_stride() -> u32 {
    crate::EMBEDDED_COLOR_STRIDE as u32
}

#[wasm_bindgen]
pub fn graphic_build_from_bytes(
    info_bytes: &[u8],
    data_bytes: &[u8],
    palette_bytes: &[u8],
) -> Result<JsValue, JsValue> {
    let graphic = Graphic::build_from_bytes(info_bytes, data_bytes, palette_bytes)
        .map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&graphic).map_err(serde_error_to_js)
}

#[wasm_bindgen]
pub fn game_palette_build_from_cgp(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let palette = Palette::build_from_cgp(bytes).map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&palette).map_err(serde_error_to_js)
}

#[wasm_bindgen]
pub fn graphic_strict_build_from_bytes(
    info_bytes: &[u8],
    data_bytes: &[u8],
    palette_bytes: &[u8],
) -> Result<JsValue, JsValue> {
    let graphic = Graphic::strict_build_from_bytes(info_bytes, data_bytes, palette_bytes)
        .map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&graphic).map_err(serde_error_to_js)
}

#[wasm_bindgen]
pub fn graphic_build_from_cgp(
    info_bytes: &[u8],
    data_bytes: &[u8],
    cgp_bytes: &[u8],
) -> Result<JsValue, JsValue> {
    let graphic =
        Graphic::build_from_cgp(info_bytes, data_bytes, cgp_bytes).map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&graphic).map_err(serde_error_to_js)
}

#[wasm_bindgen]
pub fn graphic_strict_build_from_cgp(
    info_bytes: &[u8],
    data_bytes: &[u8],
    cgp_bytes: &[u8],
) -> Result<JsValue, JsValue> {
    let graphic = Graphic::strict_build_from_cgp(info_bytes, data_bytes, cgp_bytes)
        .map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&graphic).map_err(serde_error_to_js)
}

#[wasm_bindgen]
pub fn game_palette_build_from_bytes(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let palette = Palette::build_from_bytes(bytes).map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&palette).map_err(serde_error_to_js)
}

#[wasm_bindgen]
pub fn map_build_from_bytes(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let map = Map::build_from_bytes(bytes).map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&map).map_err(serde_error_to_js)
}

#[wasm_bindgen]
pub fn anime_build_from_bytes(info_bytes: &[u8], data_bytes: &[u8]) -> Result<JsValue, JsValue> {
    let anime = Anime::build_from_bytes(info_bytes, data_bytes).map_err(build_error_to_js)?;
    serde_wasm_bindgen::to_value(&anime).map_err(serde_error_to_js)
}

fn build_error_to_js(err: crate::BuildError) -> JsValue {
    JsValue::from_str(&format!("{err:?}"))
}

fn serde_error_to_js(err: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&err.to_string())
}
