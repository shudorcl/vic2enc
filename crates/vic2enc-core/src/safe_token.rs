//! "Safe token" conversion.
//!
//! Paradox control sequences (`$VAR$` variables, `§` colour codes, `£` icon
//! keys, `¤` currency) are fragile: external translation tools and spreadsheet
//! editors love to mangle them. This module rewrites them into inert
//! `<...>`-delimited ASCII tokens and back, so the text can pass through such
//! tools untouched.
//!
//! Token format (compatible with `ParadoxLocalisationAssistant`):
//! - `§x`            -> `<A7-x>`   (lone trailing `§` -> `<A7>`)
//! - `¤`             -> `<A4>`
//! - `£key£`         -> `<A3-key-A3>`
//! - `£key ` / `£key`-> `<A3-key>`  (delimiting space is restored on decode)
//! - `$VAR$`         -> `<VAR-VAR>`
//!
//! Unlike the original C# `ToUnsafeString`, the decoder here correctly handles
//! the closed `<A3-key-A3>` form.

/// Does `chars[i..]` start with `pat`?
fn matches_at(chars: &[char], i: usize, pat: &str) -> bool {
    let mut j = i;
    for pc in pat.chars() {
        if j >= chars.len() || chars[j] != pc {
            return false;
        }
        j += 1;
    }
    true
}

/// Rewrite raw Paradox control sequences into inert `<...>` tokens.
pub fn to_safe_string(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < n {
        match chars[i] {
            '£' => {
                out.push_str("<A3-");
                i += 1;
                while i < n && chars[i] != ' ' && chars[i] != '£' {
                    out.push(chars[i]);
                    i += 1;
                }
                if i < n && chars[i] == '£' {
                    out.push_str("-A3");
                    i += 1;
                } else if i < n && chars[i] == ' ' {
                    // The space delimits the icon key; it is restored on decode.
                    i += 1;
                }
                out.push('>');
            }
            '¤' => {
                out.push_str("<A4>");
                i += 1;
            }
            '§' => {
                if i + 1 < n {
                    out.push_str("<A7-");
                    out.push(chars[i + 1]);
                    out.push('>');
                    i += 2;
                } else {
                    out.push_str("<A7>");
                    i += 1;
                }
            }
            '$' => {
                // Find a non-empty `$...$` with no inner `$`.
                let mut j = i + 1;
                while j < n && chars[j] != '$' {
                    j += 1;
                }
                if j < n && j > i + 1 {
                    out.push_str("<VAR-");
                    out.extend(&chars[i + 1..j]);
                    out.push('>');
                    i = j + 1;
                } else {
                    out.push('$');
                    i += 1;
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Reverse of [`to_safe_string`].
pub fn to_unsafe_string(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < n {
        if matches_at(&chars, i, "<A3-") {
            let mut j = i + 4;
            let mut content = String::new();
            let mut closed_pound = false;
            let mut found_close = false;
            while j < n {
                if chars[j] == '>' {
                    found_close = true;
                    break;
                }
                if chars[j] == '-' && matches_at(&chars, j, "-A3>") {
                    closed_pound = true;
                    found_close = true;
                    j += 4; // consume "-A3>"
                    break;
                }
                content.push(chars[j]);
                j += 1;
            }
            if found_close {
                out.push('£');
                out.push_str(&content);
                if closed_pound {
                    out.push('£');
                    i = j; // already past '>'
                } else {
                    out.push(' '); // restore the delimiting space
                    i = j + 1; // skip '>'
                }
                continue;
            }
            // Malformed: fall through and emit '<' literally.
        } else if matches_at(&chars, i, "<A4>") {
            out.push('¤');
            i += 4;
            continue;
        } else if matches_at(&chars, i, "<A7-") && i + 5 < n && chars[i + 5] == '>' {
            out.push('§');
            out.push(chars[i + 4]);
            i += 6;
            continue;
        } else if matches_at(&chars, i, "<A7>") {
            out.push('§');
            i += 4;
            continue;
        } else if matches_at(&chars, i, "<VAR-") {
            let mut j = i + 5;
            while j < n && chars[j] != '>' {
                j += 1;
            }
            if j < n {
                out.push('$');
                out.extend(&chars[i + 5..j]);
                out.push('$');
                i = j + 1;
                continue;
            }
            // Malformed: fall through.
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(s: &str) {
        let safe = to_safe_string(s);
        let back = to_unsafe_string(&safe);
        assert_eq!(s, back, "safe-token roundtrip failed: {s:?} -> {safe:?}");
    }

    #[test]
    fn variable_token() {
        assert_eq!(to_safe_string("$COUNTRY$"), "<VAR-COUNTRY>");
        roundtrip("$COUNTRY$ owns $CAPITAL$");
    }

    #[test]
    fn color_token() {
        assert_eq!(to_safe_string("§Y黄金§!"), "<A7-Y>黄金<A7-!>");
        roundtrip("§Y黄金§! and §Gtext§!");
    }

    #[test]
    fn icon_tokens() {
        assert_eq!(to_safe_string("£gold£"), "<A3-gold-A3>");
        roundtrip("£gold£");
        roundtrip("cost £gold and more");
        roundtrip("¤ and ¤");
    }

    #[test]
    fn plain_dollar_is_kept() {
        roundtrip("price is 5$ only");
        roundtrip("$$");
    }

    #[test]
    fn mixed() {
        roundtrip("§Y$VAL$§! gold £money£ costs ¤100");
    }
}
