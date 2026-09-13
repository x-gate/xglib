use crate::BuildError;
use serde::de::Deserializer;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

pub const MAP_MAGIC: [u8; 3] = *b"MAP";
pub const MAP_HEADER_SIZE: usize = 20;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapHeader {
    pub magic: [u8; 3],
    pub reserved: [u8; 9],
    pub width: u32,
    pub height: u32,
}

impl Serialize for MapHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let magic = self.magic;
        let reserved = self.reserved;
        let width = self.width;
        let height = self.height;

        let mut state = serializer.serialize_struct("MapHeader", 4)?;
        state.serialize_field("magic", &magic)?;
        state.serialize_field("reserved", &reserved)?;
        state.serialize_field("width", &width)?;
        state.serialize_field("height", &height)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for MapHeader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct MapHeaderSerde {
            magic: [u8; 3],
            reserved: [u8; 9],
            width: u32,
            height: u32,
        }

        let value = MapHeaderSerde::deserialize(deserializer)?;
        Ok(Self {
            magic: value.magic,
            reserved: value.reserved,
            width: value.width,
            height: value.height,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Map {
    pub header: MapHeader,
    pub ground: Vec<u16>,
    pub object: Vec<u16>,
    pub meta: Vec<u16>,
}

impl Map {
    pub fn build_from_bytes(bytes: &[u8]) -> Result<Self, BuildError> {
        let header = parse_map_header(bytes)?;
        let layer_len = (header.width as usize)
            .checked_mul(header.height as usize)
            .ok_or(BuildError::InvalidValue {
                context: "map header",
                message: "map dimensions overflow",
            })?;
        let layer_size = layer_len.checked_mul(2).ok_or(BuildError::InvalidValue {
            context: "map header",
            message: "map layer size overflow",
        })?;
        let needed = layer_size
            .checked_mul(3)
            .and_then(|size| MAP_HEADER_SIZE.checked_add(size))
            .ok_or(BuildError::InvalidValue {
                context: "map header",
                message: "map total size overflow",
            })?;
        if bytes.len() < needed {
            return Err(BuildError::BufferTooShort {
                context: "map data",
                needed,
                actual: bytes.len(),
            });
        }
        if bytes.len() > needed {
            return Err(BuildError::TrailingBytes {
                context: "map data",
                remaining: bytes.len() - needed,
            });
        }

        let ground = parse_u16_layer(&bytes[MAP_HEADER_SIZE..MAP_HEADER_SIZE + layer_size]);
        let object =
            parse_u16_layer(&bytes[MAP_HEADER_SIZE + layer_size..MAP_HEADER_SIZE + layer_size * 2]);
        let meta = parse_u16_layer(&bytes[MAP_HEADER_SIZE + layer_size * 2..needed]);

        Ok(Self {
            header,
            ground,
            object,
            meta,
        })
    }
}

fn parse_map_header(bytes: &[u8]) -> Result<MapHeader, BuildError> {
    if bytes.len() < MAP_HEADER_SIZE {
        return Err(BuildError::BufferTooShort {
            context: "map header",
            needed: MAP_HEADER_SIZE,
            actual: bytes.len(),
        });
    }

    let magic = [bytes[0], bytes[1], bytes[2]];
    if magic != MAP_MAGIC {
        return Err(BuildError::InvalidMagic {
            context: "map header",
            expected: MAP_MAGIC.to_vec(),
            actual: magic.to_vec(),
        });
    }

    Ok(MapHeader {
        magic,
        reserved: bytes[3..12].try_into().unwrap(),
        width: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        height: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
    })
}

fn parse_u16_layer(bytes: &[u8]) -> Vec<u16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| u16::from_le_bytes(*chunk))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn map_header_layout_matches_spec() {
        assert_eq!(size_of::<MapHeader>(), MAP_HEADER_SIZE);
    }

    #[test]
    fn build_map_from_bytes() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAP_MAGIC);
        bytes.extend_from_slice(&[0; 9]);
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&5u16.to_le_bytes());
        bytes.extend_from_slice(&6u16.to_le_bytes());

        let map = Map::build_from_bytes(&bytes).unwrap();
        let width = map.header.width;
        let height = map.header.height;
        assert_eq!(width, 2);
        assert_eq!(height, 1);
        assert_eq!(map.ground, vec![1, 2]);
        assert_eq!(map.object, vec![3, 4]);
        assert_eq!(map.meta, vec![5, 6]);
    }

    #[test]
    fn build_map_rejects_invalid_magic() {
        let err = Map::build_from_bytes(&[0; 20]).unwrap_err();
        assert_eq!(
            err,
            BuildError::InvalidMagic {
                context: "map header",
                expected: MAP_MAGIC.to_vec(),
                actual: vec![0, 0, 0],
            }
        );
    }

    fn header_bytes(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = MAP_MAGIC.to_vec();
        bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes
    }

    #[test]
    fn rejects_every_truncated_header_and_layer_prefix() {
        let mut bytes = header_bytes(2, 3);
        bytes.extend_from_slice(&[0; 36]);
        for len in 0..bytes.len() {
            let (context, needed) = if len < MAP_HEADER_SIZE {
                ("map header", MAP_HEADER_SIZE)
            } else {
                ("map data", bytes.len())
            };
            assert_eq!(
                Map::build_from_bytes(&bytes[..len]),
                Err(BuildError::BufferTooShort {
                    context,
                    needed,
                    actual: len,
                }),
                "prefix length {len}"
            );
        }
        assert!(Map::build_from_bytes(&bytes).is_ok());
    }

    #[test]
    fn preserves_reserved_bytes_and_little_endian_plane_order() {
        let mut bytes = header_bytes(1, 2);
        bytes.extend_from_slice(&[
            0x34, 0x12, 0xff, 0xff, 0x00, 0x80, 0xcd, 0xab, 0x01, 0x00, 0x00, 0x01,
        ]);
        let map = Map::build_from_bytes(&bytes).unwrap();
        assert_eq!(map.header.reserved, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(map.ground, [0x1234, 0xffff]);
        assert_eq!(map.object, [0x8000, 0xabcd]);
        assert_eq!(map.meta, [1, 0x100]);
    }

    #[test]
    fn zero_area_maps_have_no_layers() {
        for (width, height) in [(0, 0), (0, u32::MAX), (u32::MAX, 0)] {
            let map = Map::build_from_bytes(&header_bytes(width, height)).unwrap();
            assert!(map.ground.is_empty());
            assert!(map.object.is_empty());
            assert!(map.meta.is_empty());
        }
    }

    #[test]
    fn rejects_bytes_after_all_three_planes() {
        for (width, height) in [(0, 0), (1, 1)] {
            let mut bytes = header_bytes(width, height);
            bytes.resize(20 + (width * height * 6) as usize, 0);
            bytes.extend_from_slice(&[0xaa, 0xbb]);
            assert_eq!(
                Map::build_from_bytes(&bytes),
                Err(BuildError::TrailingBytes {
                    context: "map data",
                    remaining: 2,
                })
            );
        }
    }

    #[test]
    fn rejects_overflowing_total_size_without_panicking() {
        // Each layer fits usize, but the three-layer total does not.
        let width = if usize::BITS == 64 { u32::MAX } else { 1 };
        assert_eq!(
            Map::build_from_bytes(&header_bytes(width, 1 << 30)),
            Err(BuildError::InvalidValue {
                context: "map header",
                message: "map total size overflow",
            })
        );
    }

    #[test]
    fn rejects_overflowing_dimensions_or_layer_size() {
        let message = if usize::BITS == 64 {
            "map layer size overflow"
        } else {
            "map dimensions overflow"
        };
        assert_eq!(
            Map::build_from_bytes(&header_bytes(u32::MAX, u32::MAX)),
            Err(BuildError::InvalidValue {
                context: "map header",
                message,
            })
        );
    }
}
