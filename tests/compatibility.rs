use xglib::{BuildError, CGP_FILE_SIZE, CGP_SIZE, Graphic, Palette, rle_encode};

// All fixtures are synthetic. No game bytes are embedded in the repository.
fn cgp() -> Vec<u8> {
    let mut bytes = vec![0x7b; CGP_FILE_SIZE];
    for (i, color) in bytes[..CGP_SIZE]
        .as_chunks_mut::<3>()
        .0
        .iter_mut()
        .enumerate()
    {
        color.copy_from_slice(&[
            i as u8,
            (i as u8).wrapping_add(1),
            (i as u8).wrapping_add(2),
        ]);
    }
    bytes
}

fn graphic(
    version: u8,
    width: i32,
    height: i32,
    pixels: &[u8],
    palette: &[u8],
) -> (Vec<u8>, Vec<u8>) {
    let mut decoded = pixels.to_vec();
    if version >= 2 {
        decoded.extend_from_slice(palette);
    }
    let encoded = if version & 1 == 1 {
        rle_encode(&decoded)
    } else {
        decoded
    };
    let len = 16 + if version >= 2 { 4 } else { 0 } + encoded.len();
    let mut info = vec![0; 40];
    info[8..12].copy_from_slice(&(len as i32).to_le_bytes());
    info[20..24].copy_from_slice(&width.to_le_bytes());
    info[24..28].copy_from_slice(&height.to_le_bytes());
    let mut data = b"RD".to_vec();
    data.extend_from_slice(&[version, 0]);
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.extend_from_slice(&(len as i32).to_le_bytes());
    if version >= 2 {
        data.extend_from_slice(&(palette.len() as u32).to_le_bytes());
    }
    data.extend_from_slice(&encoded);
    (info, data)
}

#[test]
fn full_cgp_maps_active_colors_and_ignores_only_the_known_tail() {
    let mut bytes = cgp();
    let palette = Palette::build_from_cgp(&bytes).unwrap();
    assert_eq!(palette.colors.len(), 256);
    for i in 0..224 {
        let color = palette.colors[i + 16];
        assert_eq!(
            (color.blue, color.green, color.red),
            (i as u8, i as u8 + 1, i as u8 + 2)
        );
    }
    assert_eq!(palette.colors[0].alpha, 0);
    assert!(palette.colors[1..].iter().all(|color| color.alpha == 255));
    assert_eq!(
        Palette::build_from_cgp(&bytes[..CGP_SIZE]).unwrap(),
        palette
    );
    bytes[CGP_SIZE..].fill(0xe9);
    assert_eq!(Palette::build_from_cgp(&bytes).unwrap(), palette);
    assert_eq!(CGP_SIZE, 672); // Existing callers retain their constant's meaning.
}

#[test]
fn cgp_rejects_truncated_and_unknown_lengths() {
    for size in [0, 671] {
        assert!(matches!(
            Palette::build_from_cgp(&vec![0; size]),
            Err(BuildError::BufferTooShort { .. })
        ));
    }
    for size in [673, 675, 705, 707, 709, 711, 768] {
        assert!(matches!(
            Palette::build_from_cgp(&vec![0; size]),
            Err(BuildError::InvalidValue {
                context: "cgp palette",
                ..
            })
        ));
    }
}

#[test]
fn raw_bgr_is_not_auto_detected_as_cgp() {
    assert_eq!(Palette::build_from_bytes(&cgp()).unwrap().colors.len(), 236);
    let (info, data) = graphic(0, 1, 1, &[16], &[]);
    let raw = Graphic::strict_build_from_bytes(&info, &data, &cgp()).unwrap();
    let full = Graphic::strict_build_from_cgp(&info, &data, &cgp()).unwrap();
    assert_eq!(raw.palette.colors.len(), 236);
    assert_eq!(full.palette.colors.len(), 256);
    assert_ne!(raw.palette.colors[16], full.palette.colors[16]);
}

#[test]
fn external_cgp_resolves_high_pixel_indices_for_raw_and_rle_graphics() {
    for version in [0, 1] {
        let (info, data) = graphic(version, 4, 1, &[0, 16, 239, 255], &[]);
        for bytes in [&cgp()[..CGP_SIZE], &cgp()[..]] {
            let value = Graphic::strict_build_from_cgp(&info, &data, bytes).unwrap();
            assert_eq!(value.payload, [0, 16, 239, 255]);
            assert_eq!(value.palette, Palette::build_from_cgp(bytes).unwrap());
        }
    }
}

#[test]
fn embedded_palette_takes_precedence_even_with_invalid_external_cgp() {
    for version in [2, 3] {
        let (info, data) = graphic(version, 2, 1, &[0, 1], &[0, 0, 0, 11, 22, 33]);
        let value = Graphic::strict_build_from_cgp(&info, &data, &[99]).unwrap();
        assert_eq!(value.palette.colors.len(), 2);
        assert_eq!(value.palette.colors[1].red, 33);
        assert_eq!(value.payload, [0, 1]);
        let (info, data) = graphic(version, 1, 1, &[2], &[0, 0, 0, 11, 22, 33]);
        assert!(matches!(
            Graphic::build_from_cgp(&info, &data, &cgp()),
            Err(BuildError::InvalidValue {
                context: "graphic palette index",
                ..
            })
        ));
    }
}

#[test]
fn cgp_builders_do_not_hide_length_errors_or_invent_negative_dimensions() {
    // Observed anomaly shape: a final C2 emits two zeroes, crossing the image
    // boundary by one byte. It is not a universal RLE terminator.
    let (mut info, mut data) = graphic(1, 2, 1, &[7, 0, 0], &[]);
    data.truncate(16);
    data.extend_from_slice(&[0x01, 7, 0xc2]);
    info[8..12].copy_from_slice(&19i32.to_le_bytes());
    data[12..16].copy_from_slice(&19i32.to_le_bytes());
    assert_eq!(xglib::rle_decode(&data[16..]).unwrap(), [7, 0, 0]);
    assert!(Graphic::strict_build_from_cgp(&info, &data, &cgp()).is_err());
    assert_eq!(
        Graphic::build_from_cgp(&info, &data, &cgp())
            .unwrap()
            .payload,
        [7, 0]
    );
    for pixels in [&[1, 2, 3][..], &[1][..]] {
        let (info, data) = graphic(1, 2, 1, pixels, &[]);
        assert!(matches!(
            Graphic::strict_build_from_cgp(&info, &data, &cgp()),
            Err(BuildError::InvalidValue {
                context: "graphic payload",
                ..
            })
        ));
        let value = Graphic::build_from_cgp(&info, &data, &cgp()).unwrap();
        assert_eq!(
            value.payload,
            if pixels.len() > 2 {
                vec![1, 2]
            } else {
                vec![1, 0]
            }
        );
    }
    let (info, data) = graphic(1, 4, -15, &[], &[]);
    assert!(Graphic::build_from_cgp(&info, &data, &cgp()).is_err());
    assert!(Graphic::strict_build_from_cgp(&info, &data, &cgp()).is_err());
}

#[test]
fn version_zero_retains_unreliable_header_length_without_using_it_to_slice() {
    let (info, mut data) = graphic(0, 2, 1, &[1, 2], &[]);
    data[12..16].copy_from_slice(&(-123i32).to_le_bytes());
    let value = Graphic::strict_build_from_cgp(&info, &data, &cgp()).unwrap();
    let declared = value.header.data_len;
    assert_eq!(declared, -123);
    assert_eq!(value.payload, [1, 2]);
}

#[test]
fn external_graphic_rejects_invalid_cgp_in_both_modes() {
    let (info, data) = graphic(0, 1, 1, &[0], &[]);
    for bytes in [vec![], vec![0; 707]] {
        assert!(Graphic::build_from_cgp(&info, &data, &bytes).is_err());
        assert!(Graphic::strict_build_from_cgp(&info, &data, &bytes).is_err());
    }
}

#[test]
fn cgtool_fixed_prefix_distinguishes_magenta_and_blue() {
    let palette = Palette::build_from_cgp(&cgp()).unwrap();
    assert_eq!(palette.colors[4], palette::Srgba::new(128, 0, 128, 255));
    assert_eq!(palette.colors[5], palette::Srgba::new(0, 0, 128, 255));
}

#[test]
fn empty_embedded_palettes_inherit_the_explicit_external_palette() {
    for version in [2, 3] {
        let (info, data) = graphic(version, 2, 1, &[0, 249], &[]);
        let parsed = Graphic::strict_build_from_cgp(&info, &data, &cgp()).unwrap();
        assert_eq!(parsed.palette, Palette::build_from_cgp(&cgp()).unwrap());
        assert!(Graphic::strict_build_from_cgp(&info, &data, &[]).is_err());
        let (info, data) = graphic(version, 1, 1, &[1], &[]);
        let raw = [0, 0, 0, 3, 2, 1];
        let parsed = Graphic::strict_build_from_bytes(&info, &data, &raw).unwrap();
        assert_eq!(parsed.palette, Palette::build_from_bytes(&raw).unwrap());
    }
}

#[test]
fn lenient_embedded_palette_starts_after_the_expected_pixels_not_at_the_tail() {
    for version in [2, 3] {
        // One extra byte after the actual palette must not shift its BGR triples.
        let (info, data) = graphic(version, 1, 1, &[1, 0], &[0, 0, 3, 2, 1, 99]);
        assert!(Graphic::strict_build_from_bytes(&info, &data, &[]).is_err());
        let parsed = Graphic::build_from_bytes(&info, &data, &[]).unwrap();
        assert_eq!(parsed.payload, [1]);
        assert_eq!(parsed.palette.colors[1], palette::Srgba::new(1, 2, 3, 255));
    }
}

#[test]
fn compressed_rd_length_excludes_container_bytes_after_the_stream() {
    for version in [1, 3] {
        let (mut info, mut data) = graphic(
            version,
            1,
            1,
            &[1],
            if version == 3 {
                &[0, 0, 0, 3, 2, 1]
            } else {
                &[]
            },
        );
        data.extend_from_slice(&[0x01, 99]);
        info[8..12].copy_from_slice(&(data.len() as i32).to_le_bytes());
        let parsed = Graphic::strict_build_from_cgp(&info, &data, &cgp()).unwrap();
        assert_eq!(parsed.payload, [1]);
    }
}
