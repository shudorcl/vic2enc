//! Byte-level "dummy Latin-1" conversion.
//!
//! Victoria 2 reads its `;`-separated localisation CSV as Windows-1252 (cp1252).
//! Chinese translations smuggle text in by encoding it as GBK (cp936) bytes and
//! letting the game treat those bytes as Latin-1 characters, paired with a custom
//! font that maps the byte sequences back to Chinese glyphs.
//!
//! This module converts between that on-disk byte soup and real Unicode.
//!
//! Unlike the original `ParadoxLocalisationAssistant` (which round-tripped through
//! a cp1252 *string* and relied on .NET's fallback for the 5 undefined cp1252
//! bytes), we operate directly on bytes. cp1252's undefined bytes
//! (0x81/0x8D/0x8F/0x90/0x9D) can legitimately appear as GBK trailing bytes, so
//! staying at the byte level keeps the round-trip lossless.

use crate::Codepage;
use encoding_rs::{Encoding, WINDOWS_1252};

/// Paradox control bytes that must never be swallowed into a multi-byte CJK
/// sequence on their own:
/// - `0xA3` `£` — icon/sprite key delimiter (e.g. `£gold£`)
/// - `0xA4` `¤` — currency marker
/// - `0xA7` `§` — colour code introducer (e.g. `§Y...§!`)
const POUND: u8 = 0xA3;
const CURRENCY: u8 = 0xA4;
const SECTION: u8 = 0xA7;

fn codepage_encoding(cp: Codepage) -> &'static Encoding {
    match cp {
        Codepage::Gbk => encoding_rs::GBK,
    }
}

/// Try to decode exactly two bytes as a single double-byte character in `enc`.
/// Returns `Some(char)` only when the pair maps cleanly to one scalar value.
fn decode_pair(enc: &'static Encoding, b0: u8, b1: u8) -> Option<char> {
    let pair = [b0, b1];
    let (cow, had_errors) = enc.decode_without_bom_handling(&pair);
    if had_errors {
        return None;
    }
    let mut chars = cow.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        // Decoded to two separate single-byte characters, not one CJK glyph.
        return None;
    }
    Some(c)
}

/// Decode a single byte through Windows-1252 (lossless for all 256 values).
fn decode_single(b: u8) -> char {
    let single = [b];
    let (cow, _) = WINDOWS_1252.decode_without_bom_handling(&single);
    cow.chars().next().unwrap_or('\u{FFFD}')
}

/// Convert game-format "dummy Latin-1" bytes into readable Unicode.
pub fn dummy_latin1_to_unicode(bytes: &[u8], cp: Codepage) -> String {
    let enc = codepage_encoding(cp);
    let mut out = String::with_capacity(bytes.len());
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let b = bytes[i];
        match b {
            // `£` may legitimately start a GBK full-width sequence, but only when
            // followed by another high byte; otherwise it is the icon delimiter.
            POUND => {
                if i + 1 < len && bytes[i + 1] >= 0x80 {
                    if let Some(c) = decode_pair(enc, b, bytes[i + 1]) {
                        out.push(c);
                        i += 2;
                        continue;
                    }
                }
                out.push('\u{00A3}');
                i += 1;
            }
            CURRENCY => {
                out.push('\u{00A4}');
                i += 1;
            }
            SECTION => {
                out.push('\u{00A7}');
                i += 1;
            }
            // Pure ASCII: identical in cp1252/GBK/Latin-1; separators (`;` `#`
            // newlines) are all < 0x40 so they never get eaten by a CJK pair.
            _ if b < 0x80 => {
                out.push(b as char);
                i += 1;
            }
            // Other high byte: try to form a CJK pair, else a lone Latin-1 char.
            _ => {
                if i + 1 < len {
                    if let Some(c) = decode_pair(enc, b, bytes[i + 1]) {
                        out.push(c);
                        i += 2;
                        continue;
                    }
                }
                out.push(decode_single(b));
                i += 1;
            }
        }
    }
    out
}

/// Convert readable Unicode back into game-format "dummy Latin-1" bytes.
///
/// Each character is emitted as its single Windows-1252 byte when possible
/// (covers ASCII, accented Latin-1, smart quotes, and the `£`/`¤`/`§` control
/// characters); otherwise it is encoded as GBK bytes (Chinese and other CJK).
pub fn unicode_to_dummy_latin1(s: &str, cp: Codepage) -> Vec<u8> {
    let enc = codepage_encoding(cp);
    let mut out = Vec::with_capacity(s.len());
    let mut buf = [0u8; 4];
    for c in s.chars() {
        let cs = c.encode_utf8(&mut buf);
        // Prefer a single Latin-1 byte so control chars stay one byte and never
        // get turned into GBK's own multi-byte form of `£`/`¤`/`§`.
        let (bytes, _, unmappable) = WINDOWS_1252.encode(cs);
        if !unmappable {
            out.extend_from_slice(&bytes);
            continue;
        }
        // Windows-1252's 5 undefined byte slots (0x81/0x8D/0x8F/0x90/0x9D) decode
        // to the C1 control of the same value but cannot be *encoded* by
        // encoding_rs. Any unmappable char that still fits in one byte is one of
        // those, so emit it directly to keep the round-trip lossless.
        if (c as u32) <= 0xFF {
            out.push(c as u8);
            continue;
        }
        let (bytes, _, _) = enc.encode(cs);
        out.extend_from_slice(&bytes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(s: &str) {
        let bytes = unicode_to_dummy_latin1(s, Codepage::Gbk);
        let back = dummy_latin1_to_unicode(&bytes, Codepage::Gbk);
        assert_eq!(s, back, "roundtrip mismatch for {s:?} (bytes={bytes:02X?})");
    }

    #[test]
    fn ascii_is_identity() {
        let s = "KEY_NAME;Hello, World!;x";
        assert_eq!(unicode_to_dummy_latin1(s, Codepage::Gbk), s.as_bytes());
        roundtrip(s);
    }

    #[test]
    fn chinese_roundtrips() {
        roundtrip("维多利亚二号");
        roundtrip("大英帝国");
        roundtrip("普鲁士王国与北德意志邦联");
    }

    #[test]
    fn mixed_content_roundtrips() {
        roundtrip("PROV123;北京;x");
        roundtrip("$COUNTRY$ 的首都是 $CAPITAL$");
    }

    #[test]
    fn control_chars_stay_single_byte() {
        // £ ¤ § must survive as their single cp1252 bytes.
        let s = "§Y黄金§! costs ¤ and £gold£";
        let bytes = unicode_to_dummy_latin1(s, Codepage::Gbk);
        assert!(bytes.contains(&POUND));
        assert!(bytes.contains(&CURRENCY));
        assert!(bytes.contains(&SECTION));
        roundtrip(s);
    }

    #[test]
    fn every_single_byte_roundtrips_losslessly() {
        // A lone (non-CJK-pair) byte must survive decode->encode, including the
        // 5 Windows-1252 undefined slots.
        for b in 0u16..=255 {
            let b = b as u8;
            let s = dummy_latin1_to_unicode(&[b], Codepage::Gbk);
            let back = unicode_to_dummy_latin1(&s, Codepage::Gbk);
            assert_eq!(back, vec![b], "byte {b:#04X} did not round-trip (got {s:?})");
        }
    }

    #[test]
    fn gbk_trail_byte_in_cp1252_gap_is_lossless() {
        // Find a Chinese char whose GBK encoding uses an undefined cp1252 byte
        // as its trailing byte, and confirm it still round-trips.
        roundtrip("们"); // GBK 0xC3 0xC7 area; broad sweep below
        for cp in 0x4E00u32..0x4F00 {
            if let Some(c) = char::from_u32(cp) {
                roundtrip(&c.to_string());
            }
        }
    }
}
