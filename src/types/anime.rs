use crate::BuildError;
use serde::de::Deserializer;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

pub const ANIME_INFO_SIZE: usize = 12;
pub const ANIME_HEADER_STANDARD_SIZE: usize = 12;
pub const ANIME_HEADER_EXTENDED_SIZE: usize = 20;
pub const ANIME_FRAME_SIZE: usize = 10;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimeInfo {
    pub id: i32,
    pub addr: i32,
    pub act_cnt: i16,
    pub padding: [u8; 2],
}

impl Serialize for AnimeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let id = self.id;
        let addr = self.addr;
        let act_cnt = self.act_cnt;
        let padding = self.padding;

        let mut state = serializer.serialize_struct("AnimeInfo", 4)?;
        state.serialize_field("id", &id)?;
        state.serialize_field("addr", &addr)?;
        state.serialize_field("act_cnt", &act_cnt)?;
        state.serialize_field("padding", &padding)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AnimeInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AnimeInfoSerde {
            id: i32,
            addr: i32,
            act_cnt: i16,
            padding: [u8; 2],
        }

        let value = AnimeInfoSerde::deserialize(deserializer)?;
        Ok(Self {
            id: value.id,
            addr: value.addr,
            act_cnt: value.act_cnt,
            padding: value.padding,
        })
    }
}

impl AnimeInfo {
    pub fn build_from_bytes(bytes: &[u8]) -> Result<Self, BuildError> {
        if bytes.len() < ANIME_INFO_SIZE {
            return Err(BuildError::BufferTooShort {
                context: "anime info",
                needed: ANIME_INFO_SIZE,
                actual: bytes.len(),
            });
        }
        if bytes.len() > ANIME_INFO_SIZE {
            return Err(BuildError::TrailingBytes {
                context: "anime info",
                remaining: bytes.len() - ANIME_INFO_SIZE,
            });
        }

        Ok(Self {
            id: read_i32_le(bytes, 0)?,
            addr: read_i32_le(bytes, 4)?,
            act_cnt: read_i16_le(bytes, 8)?,
            padding: bytes[10..12].try_into().unwrap(),
        })
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimeHeaderStandard {
    pub direct: i16,
    pub action: i16,
    pub duration: i32,
    pub frame_cnt: i32,
}

impl Serialize for AnimeHeaderStandard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let direct = self.direct;
        let action = self.action;
        let duration = self.duration;
        let frame_cnt = self.frame_cnt;

        let mut state = serializer.serialize_struct("AnimeHeaderStandard", 4)?;
        state.serialize_field("direct", &direct)?;
        state.serialize_field("action", &action)?;
        state.serialize_field("duration", &duration)?;
        state.serialize_field("frame_cnt", &frame_cnt)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AnimeHeaderStandard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AnimeHeaderStandardSerde {
            direct: i16,
            action: i16,
            duration: i32,
            frame_cnt: i32,
        }

        let value = AnimeHeaderStandardSerde::deserialize(deserializer)?;
        Ok(Self {
            direct: value.direct,
            action: value.action,
            duration: value.duration,
            frame_cnt: value.frame_cnt,
        })
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimeHeaderExtended {
    pub direct: i16,
    pub action: i16,
    pub duration: i32,
    pub frame_cnt: i32,
    pub reserved: [u8; 2],
    pub reversed: i16,
    pub sentinel: i32,
}

impl Serialize for AnimeHeaderExtended {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let direct = self.direct;
        let action = self.action;
        let duration = self.duration;
        let frame_cnt = self.frame_cnt;
        let reserved = self.reserved;
        let reversed = self.reversed;
        let sentinel = self.sentinel;

        let mut state = serializer.serialize_struct("AnimeHeaderExtended", 7)?;
        state.serialize_field("direct", &direct)?;
        state.serialize_field("action", &action)?;
        state.serialize_field("duration", &duration)?;
        state.serialize_field("frame_cnt", &frame_cnt)?;
        state.serialize_field("reserved", &reserved)?;
        state.serialize_field("reversed", &reversed)?;
        state.serialize_field("sentinel", &sentinel)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AnimeHeaderExtended {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AnimeHeaderExtendedSerde {
            direct: i16,
            action: i16,
            duration: i32,
            frame_cnt: i32,
            reserved: [u8; 2],
            reversed: i16,
            sentinel: i32,
        }

        let value = AnimeHeaderExtendedSerde::deserialize(deserializer)?;
        Ok(Self {
            direct: value.direct,
            action: value.action,
            duration: value.duration,
            frame_cnt: value.frame_cnt,
            reserved: value.reserved,
            reversed: value.reversed,
            sentinel: value.sentinel,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimeHeader {
    Standard(AnimeHeaderStandard),
    Extended(AnimeHeaderExtended),
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimeFrame {
    pub graphic_id: i32,
    pub off_x: i16,
    pub off_y: i16,
    pub flag: i16,
}

impl Serialize for AnimeFrame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graphic_id = self.graphic_id;
        let off_x = self.off_x;
        let off_y = self.off_y;
        let flag = self.flag;

        let mut state = serializer.serialize_struct("AnimeFrame", 4)?;
        state.serialize_field("graphic_id", &graphic_id)?;
        state.serialize_field("off_x", &off_x)?;
        state.serialize_field("off_y", &off_y)?;
        state.serialize_field("flag", &flag)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AnimeFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AnimeFrameSerde {
            graphic_id: i32,
            off_x: i16,
            off_y: i16,
            flag: i16,
        }

        let value = AnimeFrameSerde::deserialize(deserializer)?;
        Ok(Self {
            graphic_id: value.graphic_id,
            off_x: value.off_x,
            off_y: value.off_y,
            flag: value.flag,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimeAction {
    pub header: AnimeHeader,
    pub frames: Vec<AnimeFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anime {
    pub info: AnimeInfo,
    pub actions: Vec<AnimeAction>,
}

impl Anime {
    pub fn build_from_bytes(info_bytes: &[u8], data_bytes: &[u8]) -> Result<Self, BuildError> {
        Self::build_with_header_size(info_bytes, data_bytes, None)
    }

    /// Parses every action with the container's header size (12 or 20 bytes).
    /// Unlike the legacy entry point, later frame bytes cannot change the layout.
    pub fn build_from_bytes_with_header_size(
        info_bytes: &[u8],
        data_bytes: &[u8],
        header_size: usize,
    ) -> Result<Self, BuildError> {
        if ![ANIME_HEADER_STANDARD_SIZE, ANIME_HEADER_EXTENDED_SIZE].contains(&header_size) {
            return Err(BuildError::InvalidValue {
                context: "anime header size",
                message: "expected 12 or 20 bytes",
            });
        }
        Self::build_with_header_size(info_bytes, data_bytes, Some(header_size))
    }

    fn build_with_header_size(
        info_bytes: &[u8],
        data_bytes: &[u8],
        fixed_header_size: Option<usize>,
    ) -> Result<Self, BuildError> {
        let info = AnimeInfo::build_from_bytes(info_bytes)?;
        if info.act_cnt < 0 {
            return Err(BuildError::InvalidValue {
                context: "anime info",
                message: "negative action count",
            });
        }

        let mut cursor = 0usize;
        let mut actions = Vec::with_capacity(info.act_cnt as usize);

        for _ in 0..info.act_cnt as usize {
            let remaining = &data_bytes[cursor..];
            let (header, header_size) = parse_anime_header(remaining, fixed_header_size)?;
            cursor += header_size;

            let frame_count = match header {
                AnimeHeader::Standard(header) => header.frame_cnt,
                AnimeHeader::Extended(header) => header.frame_cnt,
            };

            if frame_count < 0 {
                return Err(BuildError::InvalidValue {
                    context: "anime header",
                    message: "negative frame count",
                });
            }

            let mut frames = Vec::with_capacity(frame_count as usize);
            for _ in 0..frame_count as usize {
                frames.push(parse_anime_frame(data_bytes, cursor)?);
                cursor += 10;
            }

            actions.push(AnimeAction { header, frames });
        }

        if cursor != data_bytes.len() {
            return Err(BuildError::TrailingBytes {
                context: "anime data",
                remaining: data_bytes.len() - cursor,
            });
        }

        Ok(Self { info, actions })
    }
}

fn parse_anime_header(
    bytes: &[u8],
    fixed_header_size: Option<usize>,
) -> Result<(AnimeHeader, usize), BuildError> {
    if bytes.len() < ANIME_HEADER_STANDARD_SIZE {
        return Err(BuildError::BufferTooShort {
            context: "anime header",
            needed: ANIME_HEADER_STANDARD_SIZE,
            actual: bytes.len(),
        });
    }

    let direct = read_i16_le(bytes, 0)?;
    let action = read_i16_le(bytes, 2)?;
    let duration = read_i32_le(bytes, 4)?;
    let frame_cnt = read_i32_le(bytes, 8)?;

    let extended = match fixed_header_size {
        Some(size) => size == ANIME_HEADER_EXTENDED_SIZE,
        None => bytes.len() >= ANIME_HEADER_EXTENDED_SIZE && read_i32_le(bytes, 16)? == -1,
    };
    if extended {
        if bytes.len() < ANIME_HEADER_EXTENDED_SIZE {
            return Err(BuildError::BufferTooShort {
                context: "anime extended header",
                needed: ANIME_HEADER_EXTENDED_SIZE,
                actual: bytes.len(),
            });
        }
        Ok((
            AnimeHeader::Extended(AnimeHeaderExtended {
                direct,
                action,
                duration,
                frame_cnt,
                reserved: [bytes[12], bytes[13]],
                reversed: read_i16_le(bytes, 14)?,
                sentinel: read_i32_le(bytes, 16)?,
            }),
            ANIME_HEADER_EXTENDED_SIZE,
        ))
    } else {
        Ok((
            AnimeHeader::Standard(AnimeHeaderStandard {
                direct,
                action,
                duration,
                frame_cnt,
            }),
            ANIME_HEADER_STANDARD_SIZE,
        ))
    }
}

fn parse_anime_frame(bytes: &[u8], offset: usize) -> Result<AnimeFrame, BuildError> {
    let end = offset + ANIME_FRAME_SIZE;
    if bytes.len() < end {
        return Err(BuildError::BufferTooShort {
            context: "anime frame",
            needed: end,
            actual: bytes.len(),
        });
    }

    Ok(AnimeFrame {
        graphic_id: read_i32_le(bytes, offset)?,
        off_x: read_i16_le(bytes, offset + 4)?,
        off_y: read_i16_le(bytes, offset + 6)?,
        flag: read_i16_le(bytes, offset + 8)?,
    })
}

fn read_i16_le(bytes: &[u8], offset: usize) -> Result<i16, BuildError> {
    let end = offset + 2;
    let slice = bytes.get(offset..end).ok_or(BuildError::BufferTooShort {
        context: "i16 field",
        needed: end,
        actual: bytes.len(),
    })?;
    Ok(i16::from_le_bytes(slice.try_into().unwrap()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn anime_info_layout_matches_spec() {
        assert_eq!(size_of::<AnimeInfo>(), ANIME_INFO_SIZE);
    }

    #[test]
    fn anime_header_standard_layout_matches_spec() {
        assert_eq!(size_of::<AnimeHeaderStandard>(), ANIME_HEADER_STANDARD_SIZE);
    }

    #[test]
    fn anime_header_v3_layout_matches_spec() {
        assert_eq!(size_of::<AnimeHeaderExtended>(), ANIME_HEADER_EXTENDED_SIZE);
    }

    #[test]
    fn anime_frame_layout_matches_spec() {
        assert_eq!(size_of::<AnimeFrame>(), ANIME_FRAME_SIZE);
    }

    #[test]
    fn build_anime_from_bytes() {
        let mut info_bytes = Vec::new();
        info_bytes.extend_from_slice(&7i32.to_le_bytes());
        info_bytes.extend_from_slice(&128i32.to_le_bytes());
        info_bytes.extend_from_slice(&2i16.to_le_bytes());
        info_bytes.extend_from_slice(&[0, 0]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1i16.to_le_bytes());
        bytes.extend_from_slice(&2i16.to_le_bytes());
        bytes.extend_from_slice(&100i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&10i32.to_le_bytes());
        bytes.extend_from_slice(&1i16.to_le_bytes());
        bytes.extend_from_slice(&2i16.to_le_bytes());
        bytes.extend_from_slice(&3i16.to_le_bytes());

        bytes.extend_from_slice(&3i16.to_le_bytes());
        bytes.extend_from_slice(&4i16.to_le_bytes());
        bytes.extend_from_slice(&200i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&[0xaa, 0xbb]);
        bytes.extend_from_slice(&1i16.to_le_bytes());
        bytes.extend_from_slice(&(-1i32).to_le_bytes());
        bytes.extend_from_slice(&20i32.to_le_bytes());
        bytes.extend_from_slice(&4i16.to_le_bytes());
        bytes.extend_from_slice(&5i16.to_le_bytes());
        bytes.extend_from_slice(&6i16.to_le_bytes());

        let anime = Anime::build_from_bytes(&info_bytes, &bytes).unwrap();
        let anime_id = anime.info.id;
        assert_eq!(anime_id, 7);
        assert_eq!(anime.actions.len(), 2);
        assert!(matches!(anime.actions[0].header, AnimeHeader::Standard(_)));
        assert!(matches!(anime.actions[1].header, AnimeHeader::Extended(_)));
        let first_graphic_id = anime.actions[0].frames[0].graphic_id;
        let second_graphic_id = anime.actions[1].frames[0].graphic_id;
        assert_eq!(first_graphic_id, 10);
        assert_eq!(second_graphic_id, 20);
    }

    #[test]
    fn build_anime_rejects_trailing_bytes() {
        let mut info_bytes = Vec::new();
        info_bytes.extend_from_slice(&1i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i32.to_le_bytes());
        info_bytes.extend_from_slice(&0i16.to_le_bytes());
        info_bytes.extend_from_slice(&[0, 0]);
        let err = Anime::build_from_bytes(&info_bytes, &[1]).unwrap_err();
        assert_eq!(
            err,
            BuildError::TrailingBytes {
                context: "anime data",
                remaining: 1,
            }
        );
    }

    #[test]
    fn anime_info_build_from_bytes_rejects_trailing_bytes() {
        let err = AnimeInfo::build_from_bytes(&[0; 13]).unwrap_err();
        assert_eq!(
            err,
            BuildError::TrailingBytes {
                context: "anime info",
                remaining: 1,
            }
        );
    }

    fn info_bytes(action_count: i16) -> Vec<u8> {
        let mut bytes = (-123i32).to_le_bytes().to_vec();
        bytes.extend_from_slice(&(-456i32).to_le_bytes());
        bytes.extend_from_slice(&action_count.to_le_bytes());
        bytes.extend_from_slice(&[0xab, 0xcd]);
        bytes
    }

    fn action_bytes(extended: bool, frame_count: i32) -> Vec<u8> {
        let mut bytes = (-2i16).to_le_bytes().to_vec();
        bytes.extend_from_slice(&(-3i16).to_le_bytes());
        bytes.extend_from_slice(&(-100i32).to_le_bytes());
        bytes.extend_from_slice(&frame_count.to_le_bytes());
        if extended {
            bytes.extend_from_slice(&[0xab, 0xcd]);
            bytes.extend_from_slice(&(-2i16).to_le_bytes());
            bytes.extend_from_slice(&(-1i32).to_le_bytes());
        }
        for _ in 0..frame_count.max(0) {
            bytes.extend_from_slice(&(-42i32).to_le_bytes());
            bytes.extend_from_slice(&i16::MIN.to_le_bytes());
            bytes.extend_from_slice(&i16::MAX.to_le_bytes());
            bytes.extend_from_slice(&(-4i16).to_le_bytes());
        }
        bytes
    }

    #[test]
    fn rejects_every_short_info_record() {
        let info = info_bytes(0);
        for len in 0..ANIME_INFO_SIZE {
            let error = BuildError::BufferTooShort {
                context: "anime info",
                needed: ANIME_INFO_SIZE,
                actual: len,
            };
            assert_eq!(
                AnimeInfo::build_from_bytes(&info[..len]),
                Err(error.clone())
            );
            assert_eq!(Anime::build_from_bytes(&info[..len], &[]), Err(error));
        }
    }

    #[test]
    fn preserves_signed_fields_padding_and_extended_metadata() {
        for extended in [false, true] {
            let anime =
                Anime::build_from_bytes(&info_bytes(1), &action_bytes(extended, 1)).unwrap();
            assert_eq!(
                anime.info,
                AnimeInfo {
                    id: -123,
                    addr: -456,
                    act_cnt: 1,
                    padding: [0xab, 0xcd]
                }
            );
            let expected = if extended {
                AnimeHeader::Extended(AnimeHeaderExtended {
                    direct: -2,
                    action: -3,
                    duration: -100,
                    frame_cnt: 1,
                    reserved: [0xab, 0xcd],
                    reversed: -2,
                    sentinel: -1,
                })
            } else {
                AnimeHeader::Standard(AnimeHeaderStandard {
                    direct: -2,
                    action: -3,
                    duration: -100,
                    frame_cnt: 1,
                })
            };
            assert_eq!(anime.actions[0].header, expected);
            assert_eq!(
                anime.actions[0].frames,
                [AnimeFrame {
                    graphic_id: -42,
                    off_x: i16::MIN,
                    off_y: i16::MAX,
                    flag: -4
                }]
            );
        }
    }

    #[test]
    fn rejects_negative_action_and_frame_counts() {
        assert_eq!(
            Anime::build_from_bytes(&info_bytes(-1), &[]),
            Err(BuildError::InvalidValue {
                context: "anime info",
                message: "negative action count",
            })
        );
        for extended in [false, true] {
            assert_eq!(
                Anime::build_from_bytes(&info_bytes(1), &action_bytes(extended, -1)),
                Err(BuildError::InvalidValue {
                    context: "anime header",
                    message: "negative frame count",
                })
            );
        }
    }

    #[test]
    fn rejects_every_truncated_action_prefix() {
        for extended in [false, true] {
            let bytes = action_bytes(extended, 2);
            for len in 0..bytes.len() {
                assert!(
                    matches!(
                        Anime::build_from_bytes(&info_bytes(1), &bytes[..len]),
                        Err(BuildError::BufferTooShort { .. })
                    ),
                    "extended={extended}, prefix={len}"
                );
            }
            assert!(Anime::build_from_bytes(&info_bytes(1), &bytes).is_ok());
        }
    }

    #[test]
    fn supports_empty_anime_and_zero_frame_actions() {
        assert!(
            Anime::build_from_bytes(&info_bytes(0), &[])
                .unwrap()
                .actions
                .is_empty()
        );
        let mut bytes = action_bytes(true, 0);
        bytes.extend(action_bytes(false, 0));
        let anime = Anime::build_from_bytes(&info_bytes(2), &bytes).unwrap();
        assert_eq!(anime.actions.len(), 2);
        assert!(anime.actions.iter().all(|action| action.frames.is_empty()));
        assert!(matches!(anime.actions[0].header, AnimeHeader::Extended(_)));
        assert!(matches!(anime.actions[1].header, AnimeHeader::Standard(_)));
    }

    #[test]
    fn reports_frame_truncation_at_absolute_offset_in_later_action() {
        let mut bytes = action_bytes(true, 1);
        bytes.extend(action_bytes(false, 1));
        let needed = bytes.len();
        bytes.pop();
        assert_eq!(
            Anime::build_from_bytes(&info_bytes(2), &bytes),
            Err(BuildError::BufferTooShort {
                context: "anime frame",
                needed,
                actual: needed - 1,
            })
        );
    }

    #[test]
    fn rejects_trailing_bytes_after_nonempty_action() {
        let mut bytes = action_bytes(false, 1);
        bytes.extend_from_slice(&[1, 2]);
        assert_eq!(
            Anime::build_from_bytes(&info_bytes(1), &bytes),
            Err(BuildError::TrailingBytes {
                context: "anime data",
                remaining: 2,
            })
        );
    }
}
