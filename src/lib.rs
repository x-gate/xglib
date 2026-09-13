pub mod build_error;
pub mod rle;
pub mod types;
pub mod wasm;

pub use build_error::BuildError;
pub use rle::{RleError, rle_decode, rle_encode};
pub use types::{
    ANIME_FRAME_SIZE, ANIME_HEADER_EXTENDED_SIZE, ANIME_HEADER_STANDARD_SIZE, ANIME_INFO_SIZE,
    Anime, AnimeAction, AnimeFrame, AnimeHeader, AnimeHeaderExtended, AnimeHeaderStandard,
    AnimeInfo, CGP_CUSTOM_COLOR_COUNT, CGP_FILE_SIZE, CGP_SIZE, EMBEDDED_COLOR_STRIDE,
    GRAPHIC_HEADER_SIZE, GRAPHIC_INFO_SIZE, GRAPHIC_MAGIC, Graphic, GraphicHeader, GraphicInfo,
    MAP_HEADER_SIZE, MAP_MAGIC, Map, MapHeader, PALETTE_COLOR_COUNT, PALETTE_FIXED_PREFIX_COUNT,
    PALETTE_FIXED_SUFFIX_COUNT, Palette,
};
