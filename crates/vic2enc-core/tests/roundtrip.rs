//! Golden integration tests using hand-verified GBK byte vectors, exercising the
//! crate exactly as the CLI / desktop app do (public API only).
//!
//! The byte sequences below are real GB2312/GBK encodings, independent of our
//! implementation, so they pin down on-disk behaviour. Drop a real Victoria 2
//! mod `localisation/*.csv` into a `fixtures/` dir to extend coverage.

use vic2enc_core::{convert_bytes, Codepage, ConvertOptions, Direction};

fn opts(safe: bool) -> ConvertOptions {
    ConvertOptions {
        codepage: Codepage::Gbk,
        safe_tokens: safe,
    }
}

/// A Victoria 2 localisation row whose "English" column is Chinese stored as
/// GBK bytes reinterpreted as Latin-1. Chinese: 中国 (China), 北京 (Beijing).
const GAME_ROW: &[u8] = &[
    b'C', b'H', b'I', b'N', b'A', b';', // key
    0xD6, 0xD0, 0xB9, 0xFA, // 中国
    b';', //
    0xB1, 0xB1, 0xBE, 0xA9, // 北京
    b';', b'x', b'\n',
];

#[test]
fn decode_golden_matches_expected_unicode() {
    let text = convert_bytes(GAME_ROW, Direction::ToUnicode, &opts(false));
    assert_eq!(String::from_utf8(text).unwrap(), "CHINA;中国;北京;x\n");
}

#[test]
fn encode_golden_matches_expected_bytes() {
    let editable = "CHINA;中国;北京;x\n".as_bytes();
    let game = convert_bytes(editable, Direction::ToGame, &opts(false));
    assert_eq!(game, GAME_ROW);
}

#[test]
fn full_roundtrip_is_byte_identical() {
    let readable = convert_bytes(GAME_ROW, Direction::ToUnicode, &opts(true));
    let game = convert_bytes(&readable, Direction::ToGame, &opts(true));
    assert_eq!(game, GAME_ROW);
}

#[test]
fn utf8_script_line_encodes_to_gbk_game_format() {
    // A Vic2 event `change_region_name` dynamic-text line authored in UTF-8
    // Chinese with a §M colour code. Encoding (readable UTF-8 -> game) must
    // produce GBK bytes with the control char as a single 0xA7.
    let utf8 = "change_region_name = \"§M离开联盟§!\"";
    let game = convert_bytes(utf8.as_bytes(), Direction::ToGame, &opts(true));

    let mut expected = b"change_region_name = \"".to_vec();
    expected.extend_from_slice(&[0xA7, b'M']); // §M
    expected.extend_from_slice(&[0xC0, 0xEB]); // 离
    expected.extend_from_slice(&[0xBF, 0xAA]); // 开
    expected.extend_from_slice(&[0xC1, 0xAA]); // 联
    expected.extend_from_slice(&[0xC3, 0xCB]); // 盟
    expected.extend_from_slice(&[0xA7, b'!']); // §!
    expected.push(b'"');

    assert_eq!(game, expected);
    // No UTF-8 multi-byte sequence for 离 (E7 A6 BB) should remain.
    assert!(!game.windows(3).any(|w| w == [0xE7, 0xA6, 0xBB]));
}

#[test]
fn middle_dot_in_name_encodes_as_gbk_pair_not_lone_latin1() {
    // Regression for transliterated names like 阿拉贡二世·埃莱萨: the `·` (U+00B7)
    // must become GBK `A1 A4`, not the Windows-1252 byte 0xB7. 0xB7 is a GBK lead
    // byte, so a lone 0xB7 would swallow the next char's first byte and corrupt
    // everything after it.
    let name = "阿拉贡二世·埃莱萨";
    let game = convert_bytes(name.as_bytes(), Direction::ToGame, &opts(false));

    let mut expected = Vec::new();
    expected.extend_from_slice(&[0xB0, 0xA2]); // 阿
    expected.extend_from_slice(&[0xC0, 0xAD]); // 拉
    expected.extend_from_slice(&[0xB9, 0xB1]); // 贡
    expected.extend_from_slice(&[0xB6, 0xFE]); // 二
    expected.extend_from_slice(&[0xCA, 0xC0]); // 世
    expected.extend_from_slice(&[0xA1, 0xA4]); // ·
    expected.extend_from_slice(&[0xB0, 0xA3]); // 埃
    expected.extend_from_slice(&[0xC0, 0xB3]); // 莱
    expected.extend_from_slice(&[0xC8, 0xF8]); // 萨
    assert_eq!(game, expected);

    let back = convert_bytes(&game, Direction::ToUnicode, &opts(false));
    assert_eq!(String::from_utf8(back).unwrap(), name);
}

#[test]
fn pound_starting_gbk_pair_is_not_an_icon_key() {
    // 0xA3 followed by a high byte is a real GBK full-width sequence (０ = A3B0),
    // not a `£` icon delimiter. It must decode as the full-width digit and
    // re-encode to the same two bytes.
    let game: &[u8] = &[0xA3, 0xB0]; // full-width '0'
    let text = convert_bytes(game, Direction::ToUnicode, &opts(false));
    assert_eq!(text, "０".as_bytes());
    let back = convert_bytes(&text, Direction::ToGame, &opts(false));
    assert_eq!(back, game);
}
