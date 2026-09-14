use palette::Srgba;
use serde::{Deserialize, Serialize};

use crate::BuildError;

/// Size of the 224 active BGR colors; retained for source compatibility.
pub const CGP_SIZE: usize = 672;
/// Full CGP file size observed in the original resource set.
pub const CGP_FILE_SIZE: usize = 708;
pub const EMBEDDED_COLOR_STRIDE: usize = 3;
pub const PALETTE_COLOR_COUNT: usize = 256;
pub const CGP_CUSTOM_COLOR_COUNT: usize = 224;
pub const PALETTE_FIXED_PREFIX_COUNT: usize = 16;
pub const PALETTE_FIXED_SUFFIX_COUNT: usize = 16;

/// 16 fixed prefix colors stored as BGR triples in the original format.
#[rustfmt::skip]
const PREFIX_BGR: &[(u8, u8, u8)] = &[
    (0x00, 0x00, 0x00),
    (0x00, 0x00, 0x80),
    (0x00, 0x80, 0x00),
    (0x00, 0x80, 0x80),
    (0x80, 0x00, 0x80),
    (0x80, 0x00, 0x00),
    (0x80, 0x80, 0x00),
    (0xc0, 0xc0, 0xc0),
    (0xc0, 0xdc, 0xc0),
    (0xf0, 0xca, 0xa6),
    (0x00, 0x00, 0xde),
    (0x00, 0x5f, 0xff),
    (0xa0, 0xff, 0xff),
    (0xd2, 0x5f, 0x00),
    (0xff, 0xd2, 0x50),
    (0x28, 0xe1, 0x28),
];

/// 16 fixed suffix colors stored as BGR triples.
#[rustfmt::skip]
const SUFFIX_BGR: &[(u8, u8, u8)] = &[
    (0x96, 0xc3, 0xf5),
    (0x5f, 0xa0, 0x1e),
    (0x46, 0x7d, 0xc3),
    (0x1e, 0x55, 0x9b),
    (0x37, 0x41, 0x46),
    (0x1e, 0x23, 0x28),
    (0xf0, 0xfb, 0xff),
    (0xa5, 0x6e, 0x3a),
    (0x80, 0x80, 0x80),
    (0x00, 0x00, 0xff),
    (0x00, 0xff, 0x00),
    (0x00, 0xff, 0xff),
    (0xff, 0x00, 0x00),
    (0xff, 0x80, 0xff),
    (0xff, 0xff, 0x00),
    (0xff, 0xff, 0xff),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub colors: Vec<Srgba<u8>>,
}

impl Palette {
    /// Builds 256 colors from either 672 active bytes or a complete 708-byte CGP.
    /// The last 36 bytes of the full file do not replace the fixed suffix colors.
    /// See docs/compatibility.md for evidence and the limits of this interpretation.
    pub fn build_from_cgp(bytes: &[u8]) -> Result<Self, BuildError> {
        if bytes.len() < CGP_SIZE {
            return Err(BuildError::BufferTooShort {
                context: "cgp palette",
                needed: CGP_SIZE,
                actual: bytes.len(),
            });
        }
        if bytes.len() != CGP_SIZE && bytes.len() != CGP_FILE_SIZE {
            return Err(BuildError::InvalidValue {
                context: "cgp palette",
                message: "expected 672 active bytes or a 708-byte CGP file",
            });
        }

        let mut colors = Vec::with_capacity(PALETTE_COLOR_COUNT);
        colors.extend(PREFIX_BGR.iter().enumerate().map(|(idx, &(b, g, r))| {
            let alpha = if idx == 0 { 0 } else { 255 };
            Srgba::new(r, g, b, alpha)
        }));

        colors.extend(
            bytes[..CGP_SIZE]
                .as_chunks::<EMBEDDED_COLOR_STRIDE>()
                .0
                .iter()
                .map(|chunk| Srgba::new(chunk[2], chunk[1], chunk[0], 255)),
        );

        colors.extend(SUFFIX_BGR.iter().map(|&(b, g, r)| Srgba::new(r, g, b, 255)));

        Ok(Self { colors })
    }

    /// Builds a palette directly from raw BGR palette bytes.
    pub fn build_from_bytes(bytes: &[u8]) -> Result<Self, BuildError> {
        if !bytes.len().is_multiple_of(EMBEDDED_COLOR_STRIDE) {
            return Err(BuildError::InvalidValue {
                context: "embedded palette",
                message: "embedded palette size must be divisible by 3",
            });
        }

        let colors = bytes
            .as_chunks::<EMBEDDED_COLOR_STRIDE>()
            .0
            .iter()
            .enumerate()
            .map(|(idx, chunk)| {
                let alpha = if idx == 0 { 0 } else { 255 };
                Srgba::new(chunk[2], chunk[1], chunk[0], alpha)
            })
            .collect();

        Ok(Self { colors })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_embedded_palette() {
        let palette = Palette::build_from_bytes(&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60]).unwrap();

        assert_eq!(
            palette.colors,
            vec![
                Srgba::new(0x30, 0x20, 0x10, 0),
                Srgba::new(0x60, 0x50, 0x40, 255),
            ]
        );
    }

    #[test]
    fn build_embedded_palette_rejects_invalid_size() {
        let err = Palette::build_from_bytes(&[1, 2]).unwrap_err();
        assert_eq!(
            err,
            BuildError::InvalidValue {
                context: "embedded palette",
                message: "embedded palette size must be divisible by 3",
            }
        );
    }

    #[test]
    fn build_cgp_palette_requires_static_prefix_suffix_data() {
        let palette = Palette::build_from_cgp(&[0; CGP_SIZE]).unwrap();
        assert_eq!(palette.colors.len(), PALETTE_COLOR_COUNT);
        assert_eq!(palette.colors[0], Srgba::new(0x00, 0x00, 0x00, 0));
        assert_eq!(palette.colors[1], Srgba::new(0x80, 0x00, 0x00, 255));
        assert_eq!(palette.colors[16], Srgba::new(0x00, 0x00, 0x00, 255));
        assert_eq!(palette.colors[239], Srgba::new(0x00, 0x00, 0x00, 255));
        assert_eq!(palette.colors[240], Srgba::new(0xf5, 0xc3, 0x96, 255));
        assert_eq!(palette.colors[255], Srgba::new(0xff, 0xff, 0xff, 255));
    }

    #[test]
    fn build_cgp_palette_rejects_invalid_size() {
        let err = Palette::build_from_cgp(&[0; CGP_SIZE - 1]).unwrap_err();
        assert_eq!(
            err,
            BuildError::BufferTooShort {
                context: "cgp palette",
                needed: CGP_SIZE,
                actual: CGP_SIZE - 1,
            }
        );
    }

    #[test]
    fn empty_raw_palette_remains_empty() {
        assert!(Palette::build_from_bytes(&[]).unwrap().colors.is_empty());
    }

    #[test]
    fn raw_palette_transparency_depends_on_index_not_color() {
        let palette = Palette::build_from_bytes(&[1, 2, 3, 0, 0, 0, 1, 2, 3]).unwrap();
        assert_eq!(
            palette.colors,
            [
                Srgba::new(3, 2, 1, 0),
                Srgba::new(0, 0, 0, 255),
                Srgba::new(3, 2, 1, 255)
            ]
        );
    }

    #[test]
    fn raw_palette_preserves_all_256_bgr_entries_without_fixed_colors() {
        let bytes: Vec<_> = (0..=255u8)
            .flat_map(|index| [index, index ^ 0x55, 255 - index])
            .collect();
        let palette = Palette::build_from_bytes(&bytes).unwrap();
        assert_eq!(palette.colors.len(), 256);
        for index in 0..=255u8 {
            assert_eq!(
                palette.colors[usize::from(index)],
                Srgba::new(
                    255 - index,
                    index ^ 0x55,
                    index,
                    if index == 0 { 0 } else { 255 }
                )
            );
        }
    }

    #[test]
    fn rejects_partial_bgr_triples_at_every_palette_length() {
        for colors in 0..=256 {
            for remainder in [1, 2] {
                assert_eq!(
                    Palette::build_from_bytes(&vec![0; colors * 3 + remainder]),
                    Err(BuildError::InvalidValue {
                        context: "embedded palette",
                        message: "embedded palette size must be divisible by 3",
                    })
                );
            }
        }
    }

    #[test]
    fn cgp_custom_colors_cannot_override_fixed_colors() {
        let black = Palette::build_from_cgp(&[0; CGP_SIZE]).unwrap();
        let white = Palette::build_from_cgp(&[255; CGP_SIZE]).unwrap();
        assert_eq!(black.colors[..16], white.colors[..16]);
        assert_eq!(black.colors[240..], white.colors[240..]);
        assert!(
            black.colors[16..240]
                .iter()
                .all(|color| *color == Srgba::new(0, 0, 0, 255))
        );
        assert!(
            white.colors[16..240]
                .iter()
                .all(|color| *color == Srgba::new(255, 255, 255, 255))
        );
    }
}
