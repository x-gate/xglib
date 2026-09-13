pub mod anime;
pub mod graphic;
pub mod map;
pub mod palette;

pub use anime::{
    ANIME_FRAME_SIZE, ANIME_HEADER_EXTENDED_SIZE, ANIME_HEADER_STANDARD_SIZE, ANIME_INFO_SIZE,
    Anime, AnimeAction, AnimeFrame, AnimeHeader, AnimeHeaderExtended, AnimeHeaderStandard,
    AnimeInfo,
};
pub use graphic::{
    GRAPHIC_HEADER_SIZE, GRAPHIC_INFO_SIZE, GRAPHIC_MAGIC, Graphic, GraphicHeader, GraphicInfo,
};
pub use map::{MAP_HEADER_SIZE, MAP_MAGIC, Map, MapHeader};
pub use palette::{
    CGP_CUSTOM_COLOR_COUNT, CGP_FILE_SIZE, CGP_SIZE, EMBEDDED_COLOR_STRIDE, PALETTE_COLOR_COUNT,
    PALETTE_FIXED_PREFIX_COUNT, PALETTE_FIXED_SUFFIX_COUNT, Palette,
};
