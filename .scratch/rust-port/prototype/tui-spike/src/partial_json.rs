//! PROTOTYPE — rough port of `ExtractPartialStringField` from
//! `packages/tui/partialjson.go`. Same best-effort semantics: find a
//! top-level `"field":` string and return the unescaped value seen so
//! far (`ok` = field found, `done` = closing quote reached).

pub fn extract_partial_string_field(raw: &str, field: &str) -> (String, bool, bool) {
    let needle = format!("\"{field}\":");
    let Some(idx) = raw.find(&needle) else {
        return (String::new(), false, false);
    };
    let rest = &raw[idx + needle.len()..];
    let mut j = 0usize;
    let bytes = rest.as_bytes();
    while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r') {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'"' {
        return (String::new(), false, false);
    }
    j += 1;

    let mut out = String::new();
    while j < bytes.len() {
        let c = bytes[j];
        if c == b'\\' {
            if j + 1 >= bytes.len() {
                return (out, true, false);
            }
            let esc = bytes[j + 1];
            match esc {
                b'"' => {
                    out.push('"');
                    j += 2;
                }
                b'\\' => {
                    out.push('\\');
                    j += 2;
                }
                b'/' => {
                    out.push('/');
                    j += 2;
                }
                b'n' => {
                    out.push('\n');
                    j += 2;
                }
                b't' => {
                    out.push('\t');
                    j += 2;
                }
                b'r' => {
                    out.push('\r');
                    j += 2;
                }
                b'b' => {
                    out.push('\u{8}');
                    j += 2;
                }
                b'f' => {
                    out.push('\u{c}');
                    j += 2;
                }
                b'u' => {
                    if j + 6 > bytes.len() {
                        return (out, true, false);
                    }
                    let hex = &rest[j + 2..j + 6];
                    match u32::from_str_radix(hex, 16) {
                        Ok(n) => {
                            if let Some(ch) = char::from_u32(n) {
                                out.push(ch);
                            }
                            j += 6;
                        }
                        Err(_) => return (out, true, false),
                    }
                }
                _ => {
                    out.push(c as char);
                    out.push(esc as char);
                    j += 2;
                }
            }
            continue;
        }
        if c == b'"' {
            return (out, true, true);
        }
        // Feed complete UTF-8 sequences through; partial trailing bytes
        // are dropped until the next chunk arrives.
        let tail = &rest[j..];
        let mut iter = tail.char_indices();
        match iter.next() {
            Some((_, ch)) => {
                out.push(ch);
                j += ch.len_utf8();
            }
            None => break,
        }
    }
    (out, true, false)
}
