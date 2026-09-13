//! Read-only compatibility audit. No extracted resource bytes are written.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use xglib::{Anime, AnimeHeader, AnimeInfo, Graphic, GraphicInfo, Map, Palette};

#[derive(Default)]
struct Report {
    counts: BTreeMap<String, u64>,
    errors: BTreeMap<String, (u64, String)>,
    warnings: BTreeMap<String, (u64, String)>,
}

impl Report {
    fn add(&mut self, key: &str, value: usize) {
        *self.counts.entry(key.to_owned()).or_default() += value as u64;
    }

    fn error(&mut self, key: &str, item: impl std::fmt::Display, error: impl std::fmt::Debug) {
        let entry = self
            .errors
            .entry(key.to_owned())
            .or_insert_with(|| (0, format!("{item}: {error:?}")));
        entry.0 += 1;
    }

    fn warn(&mut self, key: &str, item: impl std::fmt::Display, detail: impl std::fmt::Debug) {
        let entry = self
            .warnings
            .entry(key.to_owned())
            .or_insert_with(|| (0, format!("{item}: {detail:?}")));
        entry.0 += 1;
    }
}

fn files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(std::io::Error::other("resource symlinks are not supported"));
        }
        if kind.is_dir() {
            result.extend(files(&entry.path())?);
        } else if kind.is_file() {
            result.push(entry.path());
        }
    }
    result.sort();
    Ok(result)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: verify_resources <Assets directory>")?,
    );
    let names: Vec<_> = args.collect();
    let defaults = [
        "GraphicInfo_66.bin",
        "Graphic_66.bin",
        "AnimeInfo_4.bin",
        "Anime_4.bin",
    ]
    .map(std::ffi::OsString::from);
    let names = if names.is_empty() {
        &defaults[..]
    } else {
        &names[..]
    };
    if names.len() != 4
        || names.iter().any(|name| {
            let path = Path::new(name);
            path.components().count() != 1 || path.file_name() != Some(name.as_os_str())
        })
    {
        return Err("expected Assets directory and optionally four plain filenames: GraphicInfo Graphic AnimeInfo Anime".into());
    }
    for (kind, name) in ["graphic_info", "graphic", "anime_info", "anime"]
        .iter()
        .zip(names)
    {
        println!("resource.{kind}={}", name.to_string_lossy());
    }
    let mut report = Report::default();
    let cgp = fs::read(root.join("bin/pal/palet_00.cgp"))?;
    let pal_files = files(&root.join("bin/pal"))?;
    for path in pal_files {
        if path.extension().is_none_or(|ext| ext != "cgp") {
            report.add("palette.skipped_files", 1);
            continue;
        }
        let bytes = fs::read(&path)?;
        report.add("palette.files", 1);
        report.add(&format!("palette.size.{}", bytes.len()), 1);
        match Palette::build_from_cgp(&bytes) {
            Ok(_) => report.add("palette.cgp_ok", 1),
            Err(err) => report.error("palette.cgp", path.display(), err),
        }
        if let Ok(palette) = Palette::build_from_bytes(&bytes) {
            report.add(&format!("palette.raw_colors.{}", palette.colors.len()), 1);
        }
    }

    let info = fs::read(root.join("bin").join(&names[0]))?;
    let data = fs::read(root.join("bin").join(&names[1]))?;
    if !info.len().is_multiple_of(xglib::GRAPHIC_INFO_SIZE) {
        return Err("graphic index has trailing bytes".into());
    }
    let mut ids = BTreeSet::new();
    let mut map_ids = BTreeSet::new();
    let mut intervals = Vec::new();
    for (row, bytes) in info
        .as_chunks::<{ xglib::GRAPHIC_INFO_SIZE }>()
        .0
        .iter()
        .enumerate()
    {
        let g = GraphicInfo::build_from_bytes(bytes).map_err(|e| format!("{e:?}"))?;
        let (id, addr, len, width, height, map_id) =
            (g.id, g.addr, g.len, g.width, g.height, g.map_id);
        report.add("graphic.records", 1);
        if !ids.insert(id) {
            // IDs are not unique. Every source row is still parsed below.
            report.warn("graphic.duplicate_id", row, id);
        }
        map_ids.insert(map_id);
        let span = usize::try_from(len)
            .ok()
            .and_then(|len| (addr as usize).checked_add(len));
        let Some(end) = span else {
            report.error("graphic.range", row, "negative length or overflow");
            continue;
        };
        let Some(record) = data.get(addr as usize..end) else {
            report.error("graphic.range", row, "outside Graphic file");
            continue;
        };
        intervals.push((addr as usize, end));
        // Audit guard, not a format limit; avoid large allocations on wrong layouts.
        if i64::from(width) * i64::from(height) > 16_777_216 {
            report.error("graphic.dimension_guard", row, (width, height));
            continue;
        }
        if record.len() < 16 {
            report.error("graphic.short_header", row, record.len());
            continue;
        }
        let version = record[2];
        report.add(&format!("graphic.version.{version}"), 1);
        if i32::from_le_bytes(record[4..8].try_into()?) != width
            || i32::from_le_bytes(record[8..12].try_into()?) != height
        {
            report.error("graphic.header_dimensions", row, (width, height));
        }
        let declared = i32::from_le_bytes(record[12..16].try_into()?);
        if declared != len {
            report.warn("graphic.header_length_vs_info", row, (declared, len));
        }
        if version == 0 {
            report.add(
                &format!(
                    "graphic.v0_delta_if_12_byte_header.{}",
                    record.len() as i64 - 12 - i64::from(width) * i64::from(height)
                ),
                1,
            );
        }
        let strict = Graphic::strict_build_from_cgp(bytes, record, &cgp);
        if let Err(ref err) = strict {
            report.error("graphic.strict", format!("row={row}, id={id}"), err);
            if width >= 0 && height >= 0 && version < 2 {
                let decoded_len = if version & 1 == 1 {
                    xglib::rle_decode(&record[16..])
                        .map_err(|e| format!("{e:?}"))?
                        .len()
                } else {
                    record.len() - 16
                };
                report.add(
                    &format!(
                        "graphic.strict_mismatch.version_{version}.delta_{}",
                        decoded_len as i64 - i64::from(width) * i64::from(height)
                    ),
                    1,
                );
            }
        } else {
            report.add("graphic.strict_ok", 1);
        }
        let graphic = match strict {
            Ok(graphic) => graphic,
            Err(_) => match Graphic::build_from_cgp(bytes, record, &cgp) {
                Ok(graphic) => {
                    report.add("graphic.lenient_only", 1);
                    graphic
                }
                Err(err) => {
                    report.error("graphic.lenient", row, err);
                    continue;
                }
            },
        };
        report.add("graphic.decoded_pixels", graphic.payload.len());
        if version < 2
            && graphic
                .payload
                .iter()
                .any(|&index| usize::from(index) >= cgp.len() / 3)
        {
            report.add("graphic.legacy_raw_palette_index_risk", 1);
        }
        if graphic
            .payload
            .iter()
            .any(|&index| index as usize >= graphic.palette.colors.len())
        {
            report.error("graphic.palette_index", row, graphic.palette.colors.len());
        }
        if version < 2 {
            report.add("graphic.external_palette", 1);
        }
        if version & 1 == 1 {
            let offset = if version >= 2 { 20 } else { 16 };
            let stream = &record[offset..];
            let raw = xglib::rle_decode(stream).map_err(|e| format!("{e:?}"))?;
            if xglib::rle::rle_decode_simd(stream).as_ref() != Ok(&raw) {
                report.error("rle.scalar_simd", row, "decoded bytes differ");
            }
            report.add("rle.scalar_simd_checked", 1);
            // Deterministic, documented sampling for the more expensive encoder.
            if row % 1024 == 0 {
                let encoded = xglib::rle_encode(&raw);
                if xglib::rle_decode(&encoded).as_ref() != Ok(&raw)
                    || xglib::rle::rle_encode_simd(&raw) != encoded
                {
                    report.error("rle.roundtrip", row, "encoder mismatch");
                }
                report.add("rle.roundtrip_samples", 1);
            }
        }
    }
    intervals.sort_unstable();
    let mut covered = 0;
    for (start, end) in intervals {
        if start > covered {
            report.add("graphic.unindexed_bytes", start - covered);
        } else if start < covered {
            report.add("graphic.overlapping_ranges", 1);
        }
        covered = covered.max(end);
    }
    report.add("graphic.unindexed_bytes", data.len() - covered);
    drop(data);

    let info = fs::read(root.join("bin").join(&names[2]))?;
    let data = fs::read(root.join("bin").join(&names[3]))?;
    if !info.len().is_multiple_of(xglib::ANIME_INFO_SIZE) {
        return Err("anime index has trailing bytes".into());
    }
    let rows: Vec<_> = info
        .as_chunks::<{ xglib::ANIME_INFO_SIZE }>()
        .0
        .iter()
        .map(|bytes| AnimeInfo::build_from_bytes(bytes).map_err(|e| format!("{e:?}")))
        .collect::<Result<_, _>>()?;
    let mut addresses = BTreeSet::new();
    let mut anime_ids = BTreeSet::new();
    for a in &rows {
        let (id, addr) = (a.id, a.addr);
        if !anime_ids.insert(id) {
            report.warn(
                "anime.duplicate_id",
                id,
                "duplicate; all source rows retained",
            );
        }
        if addr < 0 || addr as usize > data.len() || !addresses.insert(addr as usize) {
            return Err(format!("invalid or duplicate anime address: {addr}").into());
        }
    }
    let starts: Vec<_> = addresses.into_iter().collect();
    report.add(
        "anime.leading_unindexed_bytes",
        starts.first().copied().unwrap_or(data.len()),
    );
    let ends: BTreeMap<_, _> = starts
        .iter()
        .copied()
        .zip(starts.iter().copied().skip(1).chain([data.len()]))
        .collect();
    let mut referenced = BTreeSet::new();
    for (a, bytes) in rows
        .iter()
        .zip(info.as_chunks::<{ xglib::ANIME_INFO_SIZE }>().0)
    {
        let (id, addr) = (a.id, a.addr);
        report.add("anime.records", 1);
        match Anime::build_from_bytes(bytes, &data[addr as usize..ends[&(addr as usize)]]) {
            Ok(anime) => {
                report.add("anime.ok", 1);
                for action in anime.actions {
                    report.add("anime.actions", 1);
                    report.add(
                        match action.header {
                            AnimeHeader::Standard(_) => "anime.standard_headers",
                            AnimeHeader::Extended(_) => "anime.extended_headers",
                        },
                        1,
                    );
                    for frame in action.frames {
                        let graphic_id = frame.graphic_id;
                        referenced.insert(graphic_id);
                        report.add("anime.frames", 1);
                        if !ids.contains(&graphic_id) {
                            report.add("anime.frames_missing_in_selected_graphics", 1);
                        }
                    }
                }
            }
            Err(err) => report.error("anime.parse", id, err),
        }
    }
    report.add("anime.unique_graphic_refs", referenced.len());
    report.add(
        "anime.unique_refs_missing_in_selected_graphics",
        referenced.difference(&ids).count(),
    );

    for path in files(&root.join("map"))? {
        if path.extension().is_none_or(|ext| ext != "dat") {
            report.add("map.skipped_files", 1);
            continue;
        }
        report.add("map.files", 1);
        let bytes = fs::read(&path)?;
        match Map::build_from_bytes(&bytes) {
            Ok(map) => {
                report.add("map.ok", 1);
                report.add("map.cells", map.ground.len());
                for value in map.ground.iter().chain(&map.object) {
                    if *value != 0 && !map_ids.contains(&i32::from(*value)) {
                        report.add("map.nonzero_tiles_without_selected_map_id", 1);
                    }
                }
            }
            Err(err) => report.error("map.parse", path.display(), err),
        }
    }
    for (key, value) in report.counts {
        println!("{key}={value}");
    }
    for (key, (count, first)) in &report.errors {
        println!("ERROR {key}: count={count}; first={first}");
    }
    for (key, (count, first)) in &report.warnings {
        println!("WARN {key}: count={count}; first={first}");
    }
    println!("audit_complete=true");
    if !report.errors.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
