use xglib::{Anime, AnimeHeader};

fn info() -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[8..10].copy_from_slice(&2i16.to_le_bytes());
    bytes
}
fn action(extended: bool, offset: i16, sentinel: i32) -> Vec<u8> {
    let size = if extended { 20 } else { 12 };
    let mut bytes = vec![0; size + 10];
    bytes[4..8].copy_from_slice(&400i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&1i32.to_le_bytes());
    if extended {
        bytes[16..20].copy_from_slice(&sentinel.to_le_bytes());
    }
    bytes[size..size + 4].copy_from_slice(&7i32.to_le_bytes());
    bytes[size + 4..size + 6].copy_from_slice(&offset.to_le_bytes());
    bytes[size + 6..size + 8].copy_from_slice(&offset.to_le_bytes());
    bytes
}
#[test]
fn fixed_standard_layout_does_not_treat_later_negative_offsets_as_a_sentinel() {
    let mut bytes = action(false, 0, 0);
    bytes.extend(action(false, -1, 0));
    assert!(Anime::build_from_bytes(&info(), &bytes).is_err());
    let parsed = Anime::build_from_bytes_with_header_size(&info(), &bytes, 12).unwrap();
    assert!(matches!(parsed.actions[1].header, AnimeHeader::Standard(_)));
    let offset = parsed.actions[1].frames[0].off_x;
    assert_eq!(offset, -1);
}
#[test]
fn fixed_extended_layout_preserves_later_sentinels_instead_of_redetecting() {
    let mut bytes = action(true, 0, -1);
    bytes.extend(action(true, 4, 123));
    let parsed = Anime::build_from_bytes_with_header_size(&info(), &bytes, 20).unwrap();
    let AnimeHeader::Extended(header) = parsed.actions[1].header else {
        panic!("expected extended");
    };
    let sentinel = header.sentinel;
    assert_eq!(sentinel, 123);
    assert!(
        Anime::build_from_bytes_with_header_size(&info(), &bytes[..bytes.len() - 1], 20).is_err()
    );
}
#[test]
fn explicit_layout_rejects_invalid_sizes_and_trailing_bytes() {
    for size in [0, 11, 13, 19, 21, usize::MAX] {
        assert!(Anime::build_from_bytes_with_header_size(&info(), &[], size).is_err());
    }
    let mut bytes = action(false, 0, 0);
    bytes.extend(action(false, 0, 0));
    bytes.push(0);
    assert!(Anime::build_from_bytes_with_header_size(&info(), &bytes, 12).is_err());
    assert!(Anime::build_from_bytes_with_header_size(&info(), &[0; 12], 20).is_err());
}
