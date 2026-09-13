use crate::types::palette::Palette;
use crate::{BuildError, rle_decode};
use serde::de::Deserializer;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

pub const GRAPHIC_MAGIC: [u8; 2] = *b"RD";
pub const GRAPHIC_INFO_SIZE: usize = 40;
pub const GRAPHIC_HEADER_SIZE: usize = 16;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphicInfo {
    pub id: i32,
    pub addr: u32,
    pub len: i32,
    pub off_x: i32,
    pub off_y: i32,
    pub width: i32,
    pub height: i32,
    pub grid_w: u8,
    pub grid_h: u8,
    pub access: u8,
    pub padding: [u8; 5],
    pub map_id: i32,
}

impl Serialize for GraphicInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let id = self.id;
        let addr = self.addr;
        let len = self.len;
        let off_x = self.off_x;
        let off_y = self.off_y;
        let width = self.width;
        let height = self.height;
        let grid_w = self.grid_w;
        let grid_h = self.grid_h;
        let access = self.access;
        let padding = self.padding;
        let map_id = self.map_id;

        let mut state = serializer.serialize_struct("GraphicInfo", 12)?;
        state.serialize_field("id", &id)?;
        state.serialize_field("addr", &addr)?;
        state.serialize_field("len", &len)?;
        state.serialize_field("off_x", &off_x)?;
        state.serialize_field("off_y", &off_y)?;
        state.serialize_field("width", &width)?;
        state.serialize_field("height", &height)?;
        state.serialize_field("grid_w", &grid_w)?;
        state.serialize_field("grid_h", &grid_h)?;
        state.serialize_field("access", &access)?;
        state.serialize_field("padding", &padding)?;
        state.serialize_field("map_id", &map_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for GraphicInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GraphicInfoSerde {
            id: i32,
            addr: u32,
            len: i32,
            off_x: i32,
            off_y: i32,
            width: i32,
            height: i32,
            grid_w: u8,
            grid_h: u8,
            access: u8,
            padding: [u8; 5],
            map_id: i32,
        }

        let value = GraphicInfoSerde::deserialize(deserializer)?;
        Ok(Self {
            id: value.id,
            addr: value.addr,
            len: value.len,
            off_x: value.off_x,
            off_y: value.off_y,
            width: value.width,
            height: value.height,
            grid_w: value.grid_w,
            grid_h: value.grid_h,
            access: value.access,
            padding: value.padding,
            map_id: value.map_id,
        })
    }
}

impl GraphicInfo {
    pub fn build_from_bytes(bytes: &[u8]) -> Result<Self, BuildError> {
        if bytes.len() < GRAPHIC_INFO_SIZE {
            return Err(BuildError::BufferTooShort {
                context: "graphic info",
                needed: GRAPHIC_INFO_SIZE,
                actual: bytes.len(),
            });
        }
        if bytes.len() > GRAPHIC_INFO_SIZE {
            return Err(BuildError::TrailingBytes {
                context: "graphic info",
                remaining: bytes.len() - GRAPHIC_INFO_SIZE,
            });
        }

        Ok(Self {
            id: read_i32_le(bytes, 0)?,
            addr: read_u32_le(bytes, 4)?,
            len: read_i32_le(bytes, 8)?,
            off_x: read_i32_le(bytes, 12)?,
            off_y: read_i32_le(bytes, 16)?,
            width: read_i32_le(bytes, 20)?,
            height: read_i32_le(bytes, 24)?,
            grid_w: bytes[28],
            grid_h: bytes[29],
            access: bytes[30],
            padding: bytes[31..36].try_into().unwrap(),
            map_id: read_i32_le(bytes, 36)?,
        })
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphicHeader {
    pub magic: [u8; 2],
    pub version: u8,
    pub graphic_type: u8,
    pub width: i32,
    pub height: i32,
    pub data_len: i32,
}

impl Serialize for GraphicHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let magic = self.magic;
        let version = self.version;
        let graphic_type = self.graphic_type;
        let width = self.width;
        let height = self.height;
        let data_len = self.data_len;

        let mut state = serializer.serialize_struct("GraphicHeader", 6)?;
        state.serialize_field("magic", &magic)?;
        state.serialize_field("version", &version)?;
        state.serialize_field("graphic_type", &graphic_type)?;
        state.serialize_field("width", &width)?;
        state.serialize_field("height", &height)?;
        state.serialize_field("data_len", &data_len)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for GraphicHeader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GraphicHeaderSerde {
            magic: [u8; 2],
            version: u8,
            graphic_type: u8,
            width: i32,
            height: i32,
            data_len: i32,
        }

        let value = GraphicHeaderSerde::deserialize(deserializer)?;
        Ok(Self {
            magic: value.magic,
            version: value.version,
            graphic_type: value.graphic_type,
            width: value.width,
            height: value.height,
            data_len: value.data_len,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Graphic {
    pub info: GraphicInfo,
    pub header: GraphicHeader,
    pub payload: Vec<u8>,
    pub palette: Palette,
}

impl Graphic {
    pub fn build_from_bytes(
        info_bytes: &[u8],
        data_bytes: &[u8],
        palette_bytes: &[u8],
    ) -> Result<Self, BuildError> {
        Self::build_from_bytes_with_mode(
            info_bytes,
            data_bytes,
            palette_bytes,
            false,
            Palette::build_from_bytes,
        )
    }

    pub fn strict_build_from_bytes(
        info_bytes: &[u8],
        data_bytes: &[u8],
        palette_bytes: &[u8],
    ) -> Result<Self, BuildError> {
        Self::build_from_bytes_with_mode(
            info_bytes,
            data_bytes,
            palette_bytes,
            true,
            Palette::build_from_bytes,
        )
    }

    /// Builds a graphic using an external CGP for versions 0/1.
    /// Embedded palettes take precedence for versions >= 2.
    /// Pixel lengths are normalized as in `build_from_bytes`; palette indices
    /// are checked. Use `strict_build_from_cgp` to reject length mismatches.
    pub fn build_from_cgp(
        info_bytes: &[u8],
        data_bytes: &[u8],
        cgp_bytes: &[u8],
    ) -> Result<Self, BuildError> {
        Self::build_from_cgp_with_mode(info_bytes, data_bytes, cgp_bytes, false)
    }

    /// Like `build_from_cgp`, but rejects decoded pixel length mismatches.
    pub fn strict_build_from_cgp(
        info_bytes: &[u8],
        data_bytes: &[u8],
        cgp_bytes: &[u8],
    ) -> Result<Self, BuildError> {
        Self::build_from_cgp_with_mode(info_bytes, data_bytes, cgp_bytes, true)
    }

    fn build_from_cgp_with_mode(
        info_bytes: &[u8],
        data_bytes: &[u8],
        cgp_bytes: &[u8],
        strict: bool,
    ) -> Result<Self, BuildError> {
        let graphic = Self::build_from_bytes_with_mode(
            info_bytes,
            data_bytes,
            cgp_bytes,
            strict,
            Palette::build_from_cgp,
        )?;
        if graphic
            .payload
            .iter()
            .any(|&index| usize::from(index) >= graphic.palette.colors.len())
        {
            return Err(BuildError::InvalidValue {
                context: "graphic palette index",
                message: "pixel index exceeds palette color count",
            });
        }
        Ok(graphic)
    }

    fn build_from_bytes_with_mode(
        info_bytes: &[u8],
        data_bytes: &[u8],
        palette_bytes: &[u8],
        strict: bool,
        external_palette_builder: fn(&[u8]) -> Result<Palette, BuildError>,
    ) -> Result<Self, BuildError> {
        let info = GraphicInfo::build_from_bytes(info_bytes)?;
        let header = parse_graphic_header(data_bytes)?;
        let header_size = GRAPHIC_HEADER_SIZE;
        let expected_len = expected_pixel_len(&info)?;

        if header.version >= 2 {
            let needed = header_size + 4;
            if data_bytes.len() < needed {
                return Err(BuildError::BufferTooShort {
                    context: "graphic extended palette size",
                    needed,
                    actual: data_bytes.len(),
                });
            }

            let palette_size = read_u32_le(data_bytes, 16)?;
            let payload_bytes = &data_bytes[20..];
            let decoded = decode_graphic_payload(header.version, payload_bytes)?;
            let palette_size_usize = palette_size as usize;

            if decoded.len() < palette_size_usize {
                return Err(BuildError::InvalidValue {
                    context: "graphic extended payload",
                    message: "embedded palette size exceeds decoded payload length",
                });
            }

            let pixel_len = decoded.len() - palette_size_usize;
            let payload = normalize_graphic_payload(&decoded[..pixel_len], expected_len, strict)?;
            let palette_bytes = &decoded[pixel_len..];
            let palette = Palette::build_from_bytes(palette_bytes)?;

            Ok(Self {
                info,
                header,
                payload,
                palette,
            })
        } else {
            let decoded = decode_graphic_payload(header.version, &data_bytes[header_size..])?;
            let payload = normalize_graphic_payload(&decoded, expected_len, strict)?;
            let palette = external_palette_builder(palette_bytes)?;
            Ok(Self {
                info,
                header,
                payload,
                palette,
            })
        }
    }
}

fn parse_graphic_header(bytes: &[u8]) -> Result<GraphicHeader, BuildError> {
    if bytes.len() < GRAPHIC_HEADER_SIZE {
        return Err(BuildError::BufferTooShort {
            context: "graphic header",
            needed: GRAPHIC_HEADER_SIZE,
            actual: bytes.len(),
        });
    }

    let magic = [bytes[0], bytes[1]];
    if magic != GRAPHIC_MAGIC {
        return Err(BuildError::InvalidMagic {
            context: "graphic header",
            expected: GRAPHIC_MAGIC.to_vec(),
            actual: magic.to_vec(),
        });
    }

    Ok(GraphicHeader {
        magic,
        version: bytes[2],
        graphic_type: bytes[3],
        width: read_i32_le(bytes, 4)?,
        height: read_i32_le(bytes, 8)?,
        data_len: read_i32_le(bytes, 12)?,
    })
}

fn decode_graphic_payload(version: u8, payload: &[u8]) -> Result<Vec<u8>, BuildError> {
    if version & 1 == 1 {
        Ok(rle_decode(payload)?)
    } else {
        Ok(payload.to_vec())
    }
}

fn expected_pixel_len(info: &GraphicInfo) -> Result<usize, BuildError> {
    let width = usize::try_from(info.width).map_err(|_| BuildError::InvalidValue {
        context: "graphic dimensions",
        message: "graphic width must be non-negative",
    })?;
    let height = usize::try_from(info.height).map_err(|_| BuildError::InvalidValue {
        context: "graphic dimensions",
        message: "graphic height must be non-negative",
    })?;
    width.checked_mul(height).ok_or(BuildError::InvalidValue {
        context: "graphic dimensions",
        message: "graphic width * height overflowed",
    })
}

fn normalize_graphic_payload(
    payload: &[u8],
    expected_len: usize,
    strict: bool,
) -> Result<Vec<u8>, BuildError> {
    if payload.len() == expected_len {
        return Ok(payload.to_vec());
    }

    if strict {
        return Err(BuildError::InvalidValue {
            context: "graphic payload",
            message: "decoded payload length does not match graphic dimensions",
        });
    }

    if payload.len() > expected_len {
        Ok(payload[..expected_len].to_vec())
    } else {
        let mut normalized = Vec::with_capacity(expected_len);
        normalized.extend_from_slice(payload);
        normalized.resize(expected_len, 0);
        Ok(normalized)
    }
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32, BuildError> {
    let end = offset + 4;
    let slice = bytes.get(offset..end).ok_or(BuildError::BufferTooShort {
        context: "i32 field",
        needed: end,
        actual: bytes.len(),
    })?;
    Ok(i32::from_le_bytes(slice.try_into().unwrap()))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, BuildError> {
    let end = offset + 4;
    let slice = bytes.get(offset..end).ok_or(BuildError::BufferTooShort {
        context: "u32 field",
        needed: end,
        actual: bytes.len(),
    })?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rle_encode;
    use core::mem::size_of;

    #[test]
    fn graphic_info_layout_matches_spec() {
        assert_eq!(size_of::<GraphicInfo>(), GRAPHIC_INFO_SIZE);
    }

    #[test]
    fn graphic_header_layout_matches_spec() {
        assert_eq!(size_of::<GraphicHeader>(), GRAPHIC_HEADER_SIZE);
    }

    #[test]
    fn build_standard_graphic_from_bytes() {
        let mut info_bytes = Vec::new();
        info_bytes.extend_from_slice(&123i32.to_le_bytes());
        info_bytes.extend_from_slice(&456u32.to_le_bytes());
        info_bytes.extend_from_slice(&3i32.to_le_bytes());
        info_bytes.extend_from_slice(&1i32.to_le_bytes());
        info_bytes.extend_from_slice(&2i32.to_le_bytes());
        info_bytes.extend_from_slice(&3i32.to_le_bytes());
        info_bytes.extend_from_slice(&1i32.to_le_bytes());
        info_bytes.push(3);
        info_bytes.push(4);
        info_bytes.push(5);
        info_bytes.extend_from_slice(&[0; 5]);
        info_bytes.extend_from_slice(&6i32.to_le_bytes());

        let info = GraphicInfo::build_from_bytes(&info_bytes).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&GRAPHIC_MAGIC);
        bytes.push(0);
        bytes.push(0xbf);
        bytes.extend_from_slice(&3i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&3i32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3]);

        let palette_bytes = vec![0; crate::types::palette::CGP_SIZE];
        let graphic = Graphic::build_from_bytes(&info_bytes, &bytes, &palette_bytes).unwrap();
        assert_eq!(
            graphic,
            Graphic {
                info,
                header: GraphicHeader {
                    magic: GRAPHIC_MAGIC,
                    version: 0,
                    graphic_type: 0xbf,
                    width: 3,
                    height: 1,
                    data_len: 3,
                },
                payload: vec![1, 2, 3],
                palette: Palette::build_from_bytes(&palette_bytes).unwrap(),
            }
        );
    }

    #[test]
    fn build_rle_extended_graphic_from_buffer() {
        let mut info_bytes = Vec::new();
        info_bytes.extend_from_slice(&1i32.to_le_bytes());
        info_bytes.extend_from_slice(&2u32.to_le_bytes());
        info_bytes.extend_from_slice(&3i32.to_le_bytes());
        info_bytes.extend_from_slice(&4i32.to_le_bytes());
        info_bytes.extend_from_slice(&5i32.to_le_bytes());
        info_bytes.extend_from_slice(&3i32.to_le_bytes());
        info_bytes.extend_from_slice(&1i32.to_le_bytes());
        info_bytes.push(6);
        info_bytes.push(7);
        info_bytes.push(8);
        info_bytes.extend_from_slice(&[0; 5]);
        info_bytes.extend_from_slice(&9i32.to_le_bytes());
        let decoded_payload = vec![7, 8, 9, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
        let encoded_payload = rle_encode(&decoded_payload);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&GRAPHIC_MAGIC);
        bytes.push(3);
        bytes.push(1);
        bytes.extend_from_slice(&3i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&(encoded_payload.len() as i32).to_le_bytes());
        bytes.extend_from_slice(&6u32.to_le_bytes());
        bytes.extend_from_slice(&encoded_payload);

        let graphic = Graphic::build_from_bytes(&info_bytes, &bytes, &[]).unwrap();
        let graphic_id = graphic.info.id;
        let version = graphic.header.version;
        assert_eq!(graphic_id, 1);
        assert_eq!(version, 3);
        assert_eq!(graphic.payload, vec![7, 8, 9]);
        assert_eq!(
            graphic.palette,
            Palette::build_from_bytes(&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60,]).unwrap()
        );
    }

    #[test]
    fn graphic_info_build_from_bytes_rejects_trailing_bytes() {
        let err = GraphicInfo::build_from_bytes(&[0; 41]).unwrap_err();
        assert_eq!(
            err,
            BuildError::TrailingBytes {
                context: "graphic info",
                remaining: 1,
            }
        );
    }

    #[test]
    fn build_from_bytes_truncates_excess_pixels_by_default() {
        let mut info_bytes = Vec::new();
        info_bytes.extend_from_slice(&12i32.to_le_bytes());
        info_bytes.extend_from_slice(&0u32.to_le_bytes());
        info_bytes.extend_from_slice(&20i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i32.to_le_bytes());
        info_bytes.extend_from_slice(&2i32.to_le_bytes());
        info_bytes.extend_from_slice(&1i32.to_le_bytes());
        info_bytes.push(1);
        info_bytes.push(1);
        info_bytes.push(0);
        info_bytes.extend_from_slice(&[0; 5]);
        info_bytes.extend_from_slice(&0i32.to_le_bytes());

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&GRAPHIC_MAGIC);
        bytes.push(0);
        bytes.push(0);
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&4i32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4]);

        let palette_bytes = vec![0; crate::types::palette::CGP_SIZE];
        let graphic = Graphic::build_from_bytes(&info_bytes, &bytes, &palette_bytes).unwrap();
        assert_eq!(graphic.payload, vec![1, 2]);
    }

    #[test]
    fn build_from_bytes_pads_missing_pixels_with_zero_by_default() {
        let mut info_bytes = Vec::new();
        info_bytes.extend_from_slice(&13i32.to_le_bytes());
        info_bytes.extend_from_slice(&0u32.to_le_bytes());
        info_bytes.extend_from_slice(&17i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i32.to_le_bytes());
        info_bytes.extend_from_slice(&2i32.to_le_bytes());
        info_bytes.extend_from_slice(&2i32.to_le_bytes());
        info_bytes.push(1);
        info_bytes.push(1);
        info_bytes.push(0);
        info_bytes.extend_from_slice(&[0; 5]);
        info_bytes.extend_from_slice(&0i32.to_le_bytes());

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&GRAPHIC_MAGIC);
        bytes.push(0);
        bytes.push(0);
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&[9]);

        let palette_bytes = vec![0; crate::types::palette::CGP_SIZE];
        let graphic = Graphic::build_from_bytes(&info_bytes, &bytes, &palette_bytes).unwrap();
        assert_eq!(graphic.payload, vec![9, 0, 0, 0]);
    }

    #[test]
    fn strict_build_from_bytes_rejects_mismatched_pixel_length() {
        let mut info_bytes = Vec::new();
        info_bytes.extend_from_slice(&14i32.to_le_bytes());
        info_bytes.extend_from_slice(&0u32.to_le_bytes());
        info_bytes.extend_from_slice(&17i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i32.to_le_bytes());
        info_bytes.extend_from_slice(&2i32.to_le_bytes());
        info_bytes.extend_from_slice(&2i32.to_le_bytes());
        info_bytes.push(1);
        info_bytes.push(1);
        info_bytes.push(0);
        info_bytes.extend_from_slice(&[0; 5]);
        info_bytes.extend_from_slice(&0i32.to_le_bytes());

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&GRAPHIC_MAGIC);
        bytes.push(0);
        bytes.push(0);
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&[9]);

        let palette_bytes = vec![0; crate::types::palette::CGP_SIZE];
        let err =
            Graphic::strict_build_from_bytes(&info_bytes, &bytes, &palette_bytes).unwrap_err();
        assert_eq!(
            err,
            BuildError::InvalidValue {
                context: "graphic payload",
                message: "decoded payload length does not match graphic dimensions",
            }
        );
    }

    fn info_bytes(width: i32, height: i32) -> [u8; GRAPHIC_INFO_SIZE] {
        let mut bytes = [0; GRAPHIC_INFO_SIZE];
        bytes[20..24].copy_from_slice(&width.to_le_bytes());
        bytes[24..28].copy_from_slice(&height.to_le_bytes());
        bytes
    }

    fn record_bytes(version: u8, payload: &[u8], palette_size: u32) -> Vec<u8> {
        let mut bytes = GRAPHIC_MAGIC.to_vec();
        bytes.extend_from_slice(&[version, 0xab]);
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        if version >= 2 {
            bytes.extend_from_slice(&palette_size.to_le_bytes());
        }
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn rejects_every_short_info_record_and_header() {
        let info = info_bytes(2, 1);
        let bytes = record_bytes(0, &[0, 1], 0);
        for len in 0..GRAPHIC_INFO_SIZE {
            let error = BuildError::BufferTooShort {
                context: "graphic info",
                needed: GRAPHIC_INFO_SIZE,
                actual: len,
            };
            assert_eq!(
                GraphicInfo::build_from_bytes(&info[..len]),
                Err(error.clone())
            );
            assert_eq!(
                Graphic::build_from_bytes(&info[..len], &bytes, &[]),
                Err(error)
            );
        }
        for len in 0..GRAPHIC_HEADER_SIZE {
            assert_eq!(
                Graphic::build_from_bytes(&info, &bytes[..len], &[]),
                Err(BuildError::BufferTooShort {
                    context: "graphic header",
                    needed: GRAPHIC_HEADER_SIZE,
                    actual: len,
                })
            );
        }
    }

    #[test]
    fn rejects_invalid_magic_before_decoding_payload() {
        let mut bytes = record_bytes(1, &[0xff], 0);
        bytes[..2].copy_from_slice(b"DR");
        assert_eq!(
            Graphic::build_from_bytes(&info_bytes(2, 1), &bytes, &[]),
            Err(BuildError::InvalidMagic {
                context: "graphic header",
                expected: b"RD".to_vec(),
                actual: b"DR".to_vec(),
            })
        );
    }

    #[test]
    fn rejects_truncated_extended_palette_size() {
        for version in [2, 3] {
            let bytes = record_bytes(version, &[], 0);
            for len in 16..20 {
                assert_eq!(
                    Graphic::build_from_bytes(&info_bytes(0, 0), &bytes[..len], &[]),
                    Err(BuildError::BufferTooShort {
                        context: "graphic extended palette size",
                        needed: 20,
                        actual: len,
                    })
                );
            }
        }
    }

    #[test]
    fn rejects_embedded_palette_larger_than_decoded_payload() {
        for (version, payload) in [(2, vec![0, 1]), (3, vec![0x02, 0, 1])] {
            let bytes = record_bytes(version, &payload, 3);
            assert_eq!(
                Graphic::build_from_bytes(&info_bytes(2, 1), &bytes, &[]),
                Err(BuildError::InvalidValue {
                    context: "graphic extended payload",
                    message: "embedded palette size exceeds decoded payload length",
                })
            );
        }
    }

    #[test]
    fn propagates_rle_errors_in_both_pixel_length_modes() {
        for version in [1, 3] {
            let bytes = record_bytes(version, &[0xc2, 0x90, 0x55], 0);
            let error = BuildError::Rle(crate::rle::RleError::UnexpectedEof {
                position: 1,
                needed: 1,
                remaining: 0,
            });
            let info = info_bytes(2, 1);
            assert_eq!(
                Graphic::build_from_bytes(&info, &bytes, &[]),
                Err(error.clone())
            );
            assert_eq!(
                Graphic::strict_build_from_bytes(&info, &bytes, &[]),
                Err(error)
            );
        }
    }

    #[test]
    fn propagates_incomplete_bgr_palette_errors() {
        for version in 0..=3 {
            let decoded = if version < 2 {
                vec![0, 1]
            } else {
                vec![0, 1, 0xab, 0xcd]
            };
            let payload = if version & 1 == 1 {
                rle_encode(&decoded)
            } else {
                decoded
            };
            let bytes = record_bytes(version, &payload, 2);
            assert_eq!(
                Graphic::strict_build_from_bytes(&info_bytes(2, 1), &bytes, &[0xab, 0xcd]),
                Err(BuildError::InvalidValue {
                    context: "embedded palette",
                    message: "embedded palette size must be divisible by 3",
                })
            );
        }
    }

    #[test]
    fn normalizes_only_pixels_preserving_embedded_palette() {
        let palette = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
        for version in [2, 3] {
            for pixels in [&[1][..], &[1, 0, 1][..]] {
                let decoded = [pixels, &palette].concat();
                let payload = if version == 3 {
                    rle_encode(&decoded)
                } else {
                    decoded
                };
                let bytes = record_bytes(version, &payload, 6);
                let info = info_bytes(2, 1);
                let graphic = Graphic::build_from_bytes(&info, &bytes, &[]).unwrap();
                assert_eq!(graphic.payload, [1, 0]);
                assert_eq!(
                    graphic.palette,
                    Palette::build_from_bytes(&palette).unwrap()
                );
                assert_eq!(
                    Graphic::strict_build_from_bytes(&info, &bytes, &[]),
                    Err(BuildError::InvalidValue {
                        context: "graphic payload",
                        message: "decoded payload length does not match graphic dimensions",
                    })
                );
            }
        }
    }

    #[test]
    fn zero_area_graphics_accept_empty_pixels_in_all_versions() {
        for version in 0..=3 {
            for (width, height) in [(0, 0), (0, 2), (2, 0)] {
                let graphic = Graphic::strict_build_from_bytes(
                    &info_bytes(width, height),
                    &record_bytes(version, &[], 0),
                    &[],
                )
                .unwrap();
                assert!(graphic.payload.is_empty());
                assert!(graphic.palette.colors.is_empty());
            }
        }
    }
}
