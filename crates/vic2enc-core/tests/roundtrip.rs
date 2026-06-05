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
