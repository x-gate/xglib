#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RleError {
    UnexpectedEof {
        position: usize,
        needed: usize,
        remaining: usize,
    },
    // Retained for source compatibility; CGTool assigns semantics to all flag bytes.
    InvalidFlag {
        position: usize,
        flag: u8,
    },
}

pub fn rle_decode(input: &[u8]) -> Result<Vec<u8>, RleError> {
    decode_impl(input, false)
}

pub fn rle_decode_simd(input: &[u8]) -> Result<Vec<u8>, RleError> {
    decode_impl(input, true)
}

fn decode_impl(input: &[u8], use_simd: bool) -> Result<Vec<u8>, RleError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();

    while cursor < input.len() {
        let flag_pos = cursor;
        let flag = input[cursor];
        cursor += 1;

        // CGTool DecompressJob: long literals occupy 0x20..=0x7f,
        // long repeats 0xa0..=0xbf, and long zero runs 0xe0..=0xff.
        // The length prefix uses five bits (literal aliases repeat every 0x20).
        let op = match flag {
            0x20..=0x7f => 0x2,
            0xa0..=0xbf => 0xa,
            0xe0..=0xff => 0xe,
            _ => flag >> 4,
        };
        let low = usize::from(
            flag & if matches!(op, 0x2 | 0xa | 0xe) {
                0x1f
            } else {
                0x0f
            },
        );

        match op {
            0x0..=0x2 => {
                let len = decode_len(op, low, input, &mut cursor, flag_pos)?;
                ensure_remaining(input, cursor, len, flag_pos)?;
                append_raw(&mut output, &input[cursor..cursor + len], use_simd);
                cursor += len;
            }
            0x8..=0xa => {
                ensure_remaining(input, cursor, 1, flag_pos)?;
                let value = input[cursor];
                cursor += 1;
                let len = decode_len(op, low, input, &mut cursor, flag_pos)?;
                append_repeat(&mut output, value, len, use_simd);
            }
            0xc..=0xe => {
                let len = decode_len(op, low, input, &mut cursor, flag_pos)?;
                append_zero(&mut output, len, use_simd);
            }
            _ => unreachable!(),
        }
    }

    Ok(output)
}

pub fn rle_encode(input: &[u8]) -> Vec<u8> {
    encode_impl(input, false)
}

pub fn rle_encode_simd(input: &[u8]) -> Vec<u8> {
    encode_impl(input, true)
}

fn encode_impl(input: &[u8], use_simd: bool) -> Vec<u8> {
    let mut output = Vec::new();
    let mut cursor = 0usize;

    while cursor < input.len() {
        // Only use zero run if it's long enough to be beneficial
        // A single zero in raw: 1 byte.
        // A zero run of length 1: 1 byte (0xc1).
        // However, starting a new command adds overhead for the *next* command's flag.
        // If we have [A, 0, B], raw is [0x03, A, 0, B] (4 bytes), split is [0x01, A, 0xc1, 0x01, B] (5 bytes).
        // Threshold 3 is safer for zero runs to avoid expansion.
        let zero_run = run_len(input, cursor, true, use_simd);
        if zero_run >= 3 {
            let chunk = zero_run.min(MAX_LEN);
            write_zero_run(chunk, &mut output);
            cursor += chunk;
            continue;
        }

        // Only use repeat run if it's long enough to be beneficial
        // Repeat run costs 2 bytes minimum (0x8n, value).
        // Raw run costs 1 byte per byte + overhead.
        // [A, X, X, B] -> raw [0x04, A, X, X, B] (5 bytes), split [0x01, A, 0x82, X, 0x01, B] (6 bytes).
        // Threshold 4 is safer for general repeats.
        let repeat_run = run_len(input, cursor, false, use_simd);
        if repeat_run >= 4 {
            let chunk = repeat_run.min(MAX_LEN);
            write_repeat_run(input[cursor], chunk, &mut output);
            cursor += chunk;
            continue;
        }

        let raw_start = cursor;
        cursor += 1;

        while cursor < input.len() && (cursor - raw_start) < MAX_LEN {
            // Check if we should break for a long enough run
            let z_run = run_len(input, cursor, true, use_simd);
            if z_run >= 3 {
                break;
            }

            let r_run = run_len(input, cursor, false, use_simd);
            if r_run >= 4 {
                break;
            }

            cursor += 1;
        }

        write_raw(&input[raw_start..cursor], &mut output);
    }

    output
}

const MAX_LEN: usize = 0x0f_ffff;

fn decode_len(
    op: u8,
    low: usize,
    input: &[u8],
    cursor: &mut usize,
    flag_pos: usize,
) -> Result<usize, RleError> {
    match op {
        0x0 | 0x8 | 0xc => Ok(low),
        0x1 | 0x9 | 0xd => {
            ensure_remaining(input, *cursor, 1, flag_pos)?;
            let b0 = usize::from(input[*cursor]);
            *cursor += 1;
            Ok((low << 8) | b0)
        }
        0x2 | 0xa | 0xe => {
            ensure_remaining(input, *cursor, 2, flag_pos)?;
            let b0 = usize::from(input[*cursor]);
            let b1 = usize::from(input[*cursor + 1]);
            *cursor += 2;
            Ok((low << 16) | (b0 << 8) | b1)
        }
        _ => unreachable!(),
    }
}

fn ensure_remaining(
    input: &[u8],
    cursor: usize,
    needed: usize,
    position: usize,
) -> Result<(), RleError> {
    let remaining = input.len().saturating_sub(cursor);
    if remaining < needed {
        return Err(RleError::UnexpectedEof {
            position,
            needed,
            remaining,
        });
    }

    Ok(())
}

fn run_len(input: &[u8], start: usize, zero_only: bool, use_simd: bool) -> usize {
    if use_simd {
        simd_run_len(input, start, zero_only)
    } else {
        scalar_run_len(input, start, zero_only)
    }
}

fn scalar_run_len(input: &[u8], start: usize, zero_only: bool) -> usize {
    if start >= input.len() {
        return 0;
    }

    let value = input[start];
    if zero_only && value != 0 {
        return 0;
    }

    let mut len = 1usize;
    while start + len < input.len() && input[start + len] == value {
        len += 1;
    }
    len
}

#[inline]
fn append_raw(output: &mut Vec<u8>, bytes: &[u8], use_simd: bool) {
    if use_simd {
        unsafe { append_raw_simd(output, bytes) };
    } else {
        output.extend_from_slice(bytes);
    }
}

#[inline]
fn append_repeat(output: &mut Vec<u8>, value: u8, len: usize, use_simd: bool) {
    if use_simd {
        unsafe { append_repeat_simd(output, value, len) };
    } else {
        output.extend(std::iter::repeat_n(value, len));
    }
}

#[inline]
fn append_zero(output: &mut Vec<u8>, len: usize, use_simd: bool) {
    if use_simd {
        unsafe { append_repeat_simd(output, 0, len) };
    } else {
        output.extend(std::iter::repeat_n(0, len));
    }
}

#[inline]
unsafe fn append_raw_simd(output: &mut Vec<u8>, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }

    output.reserve(bytes.len());
    let old_len = output.len();
    let dst = unsafe { output.as_mut_ptr().add(old_len) };
    let src = bytes.as_ptr();

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut offset = 0usize;
        while offset + 16 <= bytes.len() {
            let chunk = vld1q_u8(src.add(offset));
            vst1q_u8(dst.add(offset), chunk);
            offset += 16;
        }
        if offset < bytes.len() {
            core::ptr::copy_nonoverlapping(src.add(offset), dst.add(offset), bytes.len() - offset);
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut offset = 0usize;
        while offset + 16 <= bytes.len() {
            let chunk = _mm_loadu_si128(src.add(offset) as *const __m128i);
            _mm_storeu_si128(dst.add(offset) as *mut __m128i, chunk);
            offset += 16;
        }
        if offset < bytes.len() {
            core::ptr::copy_nonoverlapping(src.add(offset), dst.add(offset), bytes.len() - offset);
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, bytes.len());
    }

    unsafe { output.set_len(old_len + bytes.len()) };
}

#[inline]
unsafe fn append_repeat_simd(output: &mut Vec<u8>, value: u8, len: usize) {
    if len == 0 {
        return;
    }

    output.reserve(len);
    let old_len = output.len();
    let dst = unsafe { output.as_mut_ptr().add(old_len) };

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let block = vdupq_n_u8(value);
        let mut offset = 0usize;
        while offset + 16 <= len {
            vst1q_u8(dst.add(offset), block);
            offset += 16;
        }
        if offset < len {
            core::ptr::write_bytes(dst.add(offset), value, len - offset);
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        let block = _mm_set1_epi8(value as i8);
        let mut offset = 0usize;
        while offset + 16 <= len {
            _mm_storeu_si128(dst.add(offset) as *mut __m128i, block);
            offset += 16;
        }
        if offset < len {
            core::ptr::write_bytes(dst.add(offset), value, len - offset);
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    unsafe {
        core::ptr::write_bytes(dst, value, len);
    }

    unsafe { output.set_len(old_len + len) };
}

#[inline]
fn simd_run_len(input: &[u8], start: usize, zero_only: bool) -> usize {
    #[cfg(target_arch = "aarch64")]
    {
        unsafe { neon_run_len(input, start, zero_only) }
    }

    #[cfg(target_arch = "x86_64")]
    {
        unsafe { sse2_run_len(input, start, zero_only) }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        scalar_run_len(input, start, zero_only)
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn neon_run_len(input: &[u8], start: usize, zero_only: bool) -> usize {
    if start >= input.len() {
        return 0;
    }

    let value = input[start];
    if zero_only && value != 0 {
        return 0;
    }

    let mut len = 1usize;
    let target = unsafe { vdupq_n_u8(value) };
    let ptr = input.as_ptr();

    while start + len + 16 <= input.len() {
        let chunk = unsafe { vld1q_u8(ptr.add(start + len)) };
        let cmp = unsafe { vceqq_u8(chunk, target) };
        if unsafe { vminvq_u8(cmp) } == 0xff {
            len += 16;
            continue;
        }

        let mut bytes = [0u8; 16];
        unsafe { vst1q_u8(bytes.as_mut_ptr(), cmp) };
        let mismatch = bytes.iter().position(|&b| b != 0xff).unwrap_or(16);
        len += mismatch;
        return len;
    }

    while start + len < input.len() && input[start + len] == value {
        len += 1;
    }
    len
}

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn sse2_run_len(input: &[u8], start: usize, zero_only: bool) -> usize {
    if start >= input.len() {
        return 0;
    }

    let value = input[start];
    if zero_only && value != 0 {
        return 0;
    }

    let mut len = 1usize;
    let target = unsafe { _mm_set1_epi8(value as i8) };
    let ptr = input.as_ptr();

    while start + len + 16 <= input.len() {
        let chunk = unsafe { _mm_loadu_si128(ptr.add(start + len) as *const __m128i) };
        let cmp = unsafe { _mm_cmpeq_epi8(chunk, target) };
        let mask = unsafe { _mm_movemask_epi8(cmp) as u32 };
        if mask == 0xffff {
            len += 16;
            continue;
        }

        len += (!mask).trailing_zeros() as usize;
        return len;
    }

    while start + len < input.len() && input[start + len] == value {
        len += 1;
    }
    len
}

fn write_raw(bytes: &[u8], output: &mut Vec<u8>) {
    debug_assert!(!bytes.is_empty());
    write_len(bytes.len(), 0x0, 0x1, 0x2, output);
    output.extend_from_slice(bytes);
}

fn write_repeat_run(value: u8, len: usize, output: &mut Vec<u8>) {
    debug_assert!(len > 0);
    if len <= 0x0f {
        output.push((0x8 << 4) | (len as u8));
        output.push(value);
    } else if len <= 0x0fff {
        output.push((0x9 << 4) | (((len >> 8) & 0x0f) as u8));
        output.push(value);
        output.push((len & 0xff) as u8);
    } else {
        output.push((0xa << 4) | (((len >> 16) & 0x0f) as u8));
        output.push(value);
        output.push(((len >> 8) & 0xff) as u8);
        output.push((len & 0xff) as u8);
    }
}

fn write_zero_run(len: usize, output: &mut Vec<u8>) {
    debug_assert!(len > 0);
    write_len(len, 0xc, 0xd, 0xe, output);
}

fn write_len(len: usize, short_op: u8, medium_op: u8, long_op: u8, output: &mut Vec<u8>) {
    debug_assert!(len <= MAX_LEN);

    if len <= 0x0f {
        output.push((short_op << 4) | (len as u8));
    } else if len <= 0x0fff {
        output.push((medium_op << 4) | (((len >> 8) & 0x0f) as u8));
        output.push((len & 0xff) as u8);
    } else {
        output.push((long_op << 4) | (((len >> 16) & 0x0f) as u8));
        output.push(((len >> 8) & 0xff) as u8);
        output.push((len & 0xff) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_decode_variants(encoded: &[u8], expected: &[u8]) {
        let decoded = rle_decode(encoded).unwrap();
        assert_eq!(decoded, expected);

        let decoded_simd = rle_decode_simd(encoded).unwrap();
        assert_eq!(decoded_simd, expected);
    }

    fn assert_encode_round_trip_variants(source: &[u8]) {
        let encoded = rle_encode(source);
        let decoded = rle_decode(&encoded).unwrap();
        assert_eq!(decoded, source);

        let encoded_simd = rle_encode_simd(source);
        let decoded_simd = rle_decode_simd(&encoded_simd).unwrap();
        assert_eq!(decoded_simd, source);
    }

    #[test]
    fn decode_raw_repeat_and_zero_commands() {
        let encoded = [
            0x03, b'A', b'B', b'C', // raw len 3
            0x84, b'Z', // repeat len 4
            0xc2, // zero len 2
        ];

        assert_decode_variants(&encoded, b"ABCZZZZ\0\0");
    }

    #[test]
    fn decode_extended_length_commands() {
        let encoded = [
            0x10, 0x10, // raw len 16
            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd',
            b'e', b'f', 0x90, b'X', 0x10, // repeat len 16
            0xd0, 0x10, // zero len 16
        ];

        let decoded = rle_decode_simd(&encoded).unwrap();
        assert_eq!(&decoded[..16], b"0123456789abcdef");
        assert_eq!(&decoded[16..32], &[b'X'; 16]);
        assert_eq!(&decoded[32..48], &[0; 16]);

        let scalar_decoded = rle_decode(&encoded).unwrap();
        assert_eq!(scalar_decoded, decoded);
    }

    #[test]
    fn decode_long_length_command() {
        let mut encoded = vec![0xe0, 0x10, 0x00];
        let decoded = rle_decode_simd(&encoded).unwrap();
        assert_eq!(decoded.len(), 0x1000);
        assert!(decoded.iter().all(|&b| b == 0));

        encoded = vec![0x20, 0x10, 0x00];
        encoded.extend(std::iter::repeat_n(0x5a, 0x1000));
        let decoded = rle_decode_simd(&encoded).unwrap();
        assert_eq!(decoded, vec![0x5a; 0x1000]);
    }

    #[test]
    fn decode_rejects_truncated_long_literal() {
        let err = rle_decode(&[0x30]).unwrap_err();
        assert_eq!(
            err,
            RleError::UnexpectedEof {
                position: 0,
                needed: 2,
                remaining: 0
            }
        );
    }

    #[test]
    fn decode_rejects_truncated_stream() {
        let err = rle_decode(&[0x10]).unwrap_err();
        assert_eq!(
            err,
            RleError::UnexpectedEof {
                position: 0,
                needed: 1,
                remaining: 0,
            }
        );

        let err = rle_decode(&[0x83]).unwrap_err();
        assert_eq!(
            err,
            RleError::UnexpectedEof {
                position: 0,
                needed: 1,
                remaining: 0,
            }
        );
    }

    #[test]
    fn encode_prefers_zero_and_repeat_runs() {
        // [0, 0, 0, 0] -> zero run len 4 (0xc4) - 1 byte
        // [0xaa, 0xaa, b'Q'] -> raw len 3 - 4 bytes (including flag)
        // Total: 5 bytes.
        // If we used repeat 2 for 0xaa: [0xc4, 0x82, 0xaa, 0x01, b'Q'] - 5 bytes.
        // Our heuristic prefers raw when equal to reduce command count.
        let encoded = rle_encode(&[0, 0, 0, 0, 0xaa, 0xaa, b'Q']);
        assert_eq!(encoded, vec![0xc4, 0x03, 0xaa, 0xaa, b'Q']);

        let encoded_simd = rle_encode_simd(&[0, 0, 0, 0, 0xaa, 0xaa, b'Q']);
        assert_eq!(encoded_simd, encoded);

        // Long enough runs should still be compressed
        let encoded2 = rle_encode(&[0xaa, 0xaa, 0xaa, 0xaa, 0xaa]);
        assert_eq!(encoded2, vec![0x85, 0xaa]);
        assert_eq!(rle_encode_simd(&[0xaa, 0xaa, 0xaa, 0xaa, 0xaa]), encoded2);
    }

    #[test]
    fn encode_decode_empty_and_single_byte() {
        let empty: [u8; 0] = [];
        assert_encode_round_trip_variants(&empty);

        let single = [0x42];
        assert_encode_round_trip_variants(&single);

        let single_zero = [0];
        assert_encode_round_trip_variants(&single_zero);
    }

    #[test]
    fn encode_avoids_expansion() {
        // [1, 0, 2] -> 3 bytes
        // Old encoder: [0x01, 1, 0xc1, 0x01, 2] -> 5 bytes
        // New encoder: [0x03, 1, 0, 2] -> 4 bytes (still 1 byte overhead for flag)
        let data = [1, 0, 2];
        let encoded = rle_encode(&data);
        assert_eq!(encoded, vec![0x03, 1, 0, 2]);
        assert_eq!(rle_encode_simd(&data), encoded);
    }

    #[test]
    fn decode_matches_testcases() {
        let cases: &[(&str, Vec<u8>, Vec<u8>)] = &[
            ("read 1 bytes", vec![0x01, 0xaa], vec![0xaa]),
            (
                "read 4078 bytes",
                {
                    let mut encoded = vec![0x1f, 0xee];
                    encoded.extend(std::iter::repeat_n(0xaa, 0x0f * 0x100 + 0xee));
                    encoded
                },
                vec![0xaa; 0x0f * 0x100 + 0xee],
            ),
            (
                "read 74291 bytes",
                {
                    let mut encoded = vec![0x21, 0x22, 0x33];
                    encoded.extend(std::iter::repeat_n(0xaa, 0x10000 + 0x22 * 0x100 + 0x33));
                    encoded
                },
                vec![0xaa; 0x10000 + 0x22 * 0x100 + 0x33],
            ),
            ("repeat 1 byte 2 times", vec![0x82, 0xaa], vec![0xaa, 0xaa]),
            (
                "repeat 1 byte 4078 times",
                vec![0x9f, 0xaa, 0xee],
                vec![0xaa; 0x0f * 0x100 + 0xee],
            ),
            (
                "repeat 1 byte 74291 times",
                vec![0xa1, 0xaa, 0x22, 0x33],
                vec![0xaa; 0x10000 + 0x22 * 0x100 + 0x33],
            ),
            ("repeat 1 alpha byte", vec![0xc1], vec![0x00]),
            (
                "repeat 4078 alpha bytes",
                vec![0xdf, 0xee],
                vec![0x00; 0x0f * 0x100 + 0xee],
            ),
            (
                "repeat 74291 alpha bytes",
                vec![0xe1, 0x22, 0x33],
                vec![0x00; 0x10000 + 0x22 * 0x100 + 0x33],
            ),
        ];

        for (name, encoded, expected) in cases {
            let decoded = rle_decode(encoded).unwrap_or_else(|err| {
                panic!("{name}: unexpected error: {err:?}");
            });
            assert_eq!(&decoded, expected, "{name}");

            let decoded_simd = rle_decode_simd(encoded).unwrap_or_else(|err| {
                panic!("{name}: unexpected simd error: {err:?}");
            });
            assert_eq!(&decoded_simd, expected, "{name} (simd)");
        }
    }

    #[test]
    fn expanded_flags_still_reject_truncation() {
        let cases = [
            ("invalid flag 0x3?", vec![0x31]),
            ("invalid flag 0x4?", vec![0x42]),
            ("invalid flag 0x5?", vec![0x53]),
            ("invalid flag 0x6?", vec![0x64]),
            ("invalid flag 0x7?", vec![0x75]),
            ("invalid flag 0xb?", vec![0xb6]),
        ];

        for (name, data) in cases {
            let err = rle_decode(&data).unwrap_err();
            assert!(
                matches!(err, RleError::UnexpectedEof { .. }),
                "{name}: expected truncation error, got {err:?}"
            );
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        let source = [
            0, 0, 0, 0, 1, 2, 3, 4, 4, 4, 4, 9, 8, 7, 0, 5, 5, 6, 7, 8, 0, 0, 0, 3, 2, 1,
        ];

        assert_encode_round_trip_variants(&source);
    }

    #[test]
    fn encode_splits_large_runs() {
        let source = vec![0; MAX_LEN + 7];
        let encoded = rle_encode(&source);
        assert_eq!(encoded[..3], [0xef, 0xff, 0xff]);
        assert_eq!(encoded[3], 0xc7);

        let decoded = rle_decode(&encoded).unwrap();
        assert_eq!(decoded, source);

        let encoded_simd = rle_encode_simd(&source);
        assert_eq!(encoded_simd, encoded);
        let decoded_simd = rle_decode_simd(&encoded_simd).unwrap();
        assert_eq!(decoded_simd, source);
    }

    #[test]
    fn expanded_flags_report_truncation_at_the_command_offset_in_both_decoders() {
        for op in [0x3, 0x4, 0x5, 0x6, 0x7, 0xb, 0xf] {
            for low in 0..=15 {
                let flag = (op << 4) | low;
                let encoded = [0x01, 0x42, flag];
                let error = RleError::UnexpectedEof {
                    position: 2,
                    needed: if op == 0xb { 1 } else { 2 },
                    remaining: 0,
                };
                assert_eq!(rle_decode(&encoded), Err(error.clone()));
                assert_eq!(rle_decode_simd(&encoded), Err(error));
            }
        }
    }

    #[test]
    fn cgtool_long_runs_use_five_length_bits_and_literal_aliases() {
        for flag in [0x20u8, 0x30, 0x40, 0x50, 0x60, 0x70, 0x7f] {
            let len = (usize::from(flag % 0x20) << 16) + 3;
            let mut wire = vec![flag, 0, 3];
            wire.extend(std::iter::repeat_n(0x57, len));
            let expected = vec![0x57; len];
            assert_eq!(rle_decode(&wire).unwrap(), expected);
            assert_eq!(rle_decode_simd(&wire).unwrap(), expected);
            wire.pop();
            assert!(matches!(
                rle_decode(&wire),
                Err(RleError::UnexpectedEof { .. })
            ));
        }
        for flag in [0xb0u8, 0xbf, 0xf0, 0xff] {
            let len = (usize::from(flag & 0x1f) << 16) + 3;
            let wire = if flag < 0xc0 {
                vec![flag, 0x57, 0, 3]
            } else {
                vec![flag, 0, 3]
            };
            let expected = vec![if flag < 0xc0 { 0x57 } else { 0 }; len];
            assert_eq!(rle_decode(&wire).unwrap(), expected);
            assert_eq!(rle_decode_simd(&wire).unwrap(), expected);
        }
    }

    #[test]
    fn truncated_commands_report_exact_required_and_remaining_bytes() {
        let cases: &[(&[u8], usize, usize)] = &[
            (&[0x03, 0xaa], 3, 1),
            (&[0x10], 1, 0),
            (&[0x10, 0x10, 0xaa], 16, 1),
            (&[0x20], 2, 0),
            (&[0x20, 0x01], 2, 1),
            (&[0x20, 0x01, 0x00], 256, 0),
            (&[0x80], 1, 0),
            (&[0x90], 1, 0),
            (&[0x90, 0xaa], 1, 0),
            (&[0xa0], 1, 0),
            (&[0xa0, 0xaa], 2, 0),
            (&[0xa0, 0xaa, 0x01], 2, 1),
            (&[0xd0], 1, 0),
            (&[0xe0], 2, 0),
            (&[0xe0, 0x01], 2, 1),
        ];
        for &(command, needed, remaining) in cases {
            for prefix in [&[][..], &[0x01, 0x42][..]] {
                let input = [prefix, command].concat();
                let error = RleError::UnexpectedEof {
                    position: prefix.len(),
                    needed,
                    remaining,
                };
                assert_eq!(rle_decode(&input), Err(error.clone()), "{input:02x?}");
                assert_eq!(rle_decode_simd(&input), Err(error), "{input:02x?}");
            }
        }
    }

    #[test]
    fn zero_length_commands_consume_their_headers_and_repeat_value() {
        let encoded = [
            0x00, 0x10, 0x00, 0x20, 0x00, 0x00, 0x80, 0xff, 0x90, 0xff, 0x00, 0xa0, 0xff, 0x00,
            0x00, 0xc0, 0xd0, 0x00, 0xe0, 0x00, 0x00, 0x01, 0x42,
        ];
        assert_decode_variants(&encoded, &[0x42]);
    }

    #[test]
    fn command_length_boundaries_match_independent_wire_bytes() {
        // Explicit wire headers avoid relying on encode/decode sharing a length bug.
        for (len, literal, repeat, zero) in [
            (15, vec![0x0f], vec![0x8f, 0x55], vec![0xcf]),
            (
                16,
                vec![0x10, 0x10],
                vec![0x90, 0x55, 0x10],
                vec![0xd0, 0x10],
            ),
            (
                4095,
                vec![0x1f, 0xff],
                vec![0x9f, 0x55, 0xff],
                vec![0xdf, 0xff],
            ),
            (
                4096,
                vec![0x20, 0x10, 0],
                vec![0xa0, 0x55, 0x10, 0],
                vec![0xe0, 0x10, 0],
            ),
            (
                0xfffff,
                vec![0x2f, 0xff, 0xff],
                vec![0xaf, 0x55, 0xff, 0xff],
                vec![0xef, 0xff, 0xff],
            ),
        ] {
            let raw: Vec<_> = (0..len).map(|index| (index % 251 + 1) as u8).collect();
            let raw_encoded = [literal, raw.clone()].concat();
            for (source, encoded) in [
                (raw, raw_encoded),
                (vec![0x55; len], repeat),
                (vec![0; len], zero),
            ] {
                assert_decode_variants(&encoded, &source);
                assert_eq!(rle_encode(&source), encoded);
                assert_eq!(rle_encode_simd(&source), encoded);
            }
        }
    }

    #[test]
    fn splits_literal_and_nonzero_repeat_runs_at_maximum_length() {
        let raw: Vec<_> = (0..MAX_LEN + 7)
            .map(|index| (index % 251 + 1) as u8)
            .collect();
        let mut raw_encoded = vec![0x2f, 0xff, 0xff];
        raw_encoded.extend_from_slice(&raw[..MAX_LEN]);
        raw_encoded.push(0x07);
        raw_encoded.extend_from_slice(&raw[MAX_LEN..]);
        for (source, expected) in [
            (raw, raw_encoded),
            (
                vec![0x55; MAX_LEN + 7],
                vec![0xaf, 0x55, 0xff, 0xff, 0x87, 0x55],
            ),
        ] {
            assert_eq!(rle_encode(&source), expected);
            assert_eq!(rle_encode_simd(&source), expected);
            assert_decode_variants(&expected, &source);
        }
    }

    #[test]
    fn scalar_and_simd_agree_on_misaligned_slices_and_mixed_run_boundaries() {
        let mut state = 0x1234_5678u32;
        for offset in 0..16 {
            for len in [0, 1, 2, 3, 4, 15, 16, 17, 31, 32, 33, 255, 256, 257] {
                let mut storage = vec![0xee; offset];
                for _ in 0..len {
                    state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                    storage.push((state >> 24) as u8);
                }
                storage.extend(std::iter::repeat_n(0, len));
                storage.extend(std::iter::repeat_n(0x55, len));
                storage.extend_from_slice(&[1, 0, 0, 2, 3, 3, 3, 4]);
                let source = &storage[offset..];
                let encoded = rle_encode(source);
                assert_eq!(
                    rle_encode_simd(source),
                    encoded,
                    "offset={offset}, len={len}"
                );
                let mut encoded_storage = vec![0xff; offset];
                encoded_storage.extend(encoded);
                assert_decode_variants(&encoded_storage[offset..], source);
            }
        }
    }
}
