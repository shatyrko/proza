// Reading and writing text files the way CotEditor does: encoding detection in its order (BOM, encoding
// declaration, the user's priority list), the line ending, and the characters an encoding can't represent.

use gtk::glib;

/// One entry of the encoding list: CotEditor's display name and the iconv name. `None` = a separator.
pub type EncodingEntry = Option<(&'static str, &'static str)>;

/// CotEditor's factory encoding order (Settings → Format → Encoding List), with its separators.
pub const DEFAULT_ENCODINGS: &[EncodingEntry] = &[
    Some(("Unicode (UTF-8)", "UTF-8")),
    None,
    Some(("Japanese (Shift JIS)", "SHIFT_JIS")),
    Some(("Japanese (EUC)", "EUC-JP")),
    Some(("Japanese (Windows, DOS)", "CP932")),
    Some(("Japanese (Shift JIS X0213)", "SHIFT_JISX0213")),
    Some(("Japanese (ISO 2022-JP)", "ISO-2022-JP")),
    None,
    Some(("Western (Mac OS Roman)", "MACINTOSH")),
    Some(("Western (Windows Latin 1)", "WINDOWS-1252")),
    None,
    Some(("Chinese (GB 18030)", "GB18030")),
    Some(("Traditional Chinese (Big 5 HKSCS)", "BIG5-HKSCS")),
    Some(("Traditional Chinese (Big 5)", "BIG5")),
    Some(("Traditional Chinese (EUC)", "EUC-TW")),
    Some(("Simplified Chinese (GB 2312)", "GB2312")),
    Some(("Traditional Chinese (Windows, DOS)", "CP950")),
    Some(("Simplified Chinese (Windows, DOS)", "CP936")),
    None,
    Some(("Korean (EUC)", "EUC-KR")),
    Some(("Korean (Windows, DOS)", "CP949")),
    None,
    Some(("Thai (Windows, DOS)", "CP874")),
    Some(("Thai (ISO 8859-11)", "ISO-8859-11")),
    None,
    Some(("Arabic (ISO 8859-6)", "ISO-8859-6")),
    Some(("Arabic (Windows)", "WINDOWS-1256")),
    Some(("Greek (ISO 8859-7)", "ISO-8859-7")),
    Some(("Greek (Windows)", "WINDOWS-1253")),
    Some(("Hebrew (ISO 8859-8)", "ISO-8859-8")),
    Some(("Hebrew (Windows)", "WINDOWS-1255")),
    Some(("Cyrillic (Mac OS)", "MAC-CYRILLIC")),
    Some(("Cyrillic (ISO 8859-5)", "ISO-8859-5")),
    Some(("Cyrillic (Windows)", "WINDOWS-1251")),
    Some(("Russian (DOS)", "CP866")),
    Some(("Cyrillic (KOI8-R)", "KOI8-R")),
    Some(("Ukrainian (KOI8-U)", "KOI8-U")),
    Some(("Central European (Mac OS)", "MAC-CENTRALEUROPE")),
    Some(("Turkish (Mac OS)", "MAC-TURKISH")),
    Some(("Icelandic (Mac OS)", "MAC-ICELAND")),
    None,
    Some(("Western (ISO Latin 1)", "ISO-8859-1")),
    Some(("Central European (ISO Latin 2)", "ISO-8859-2")),
    Some(("Western (ISO Latin 3)", "ISO-8859-3")),
    Some(("Central European (ISO Latin 4)", "ISO-8859-4")),
    Some(("Turkish (ISO Latin 5)", "ISO-8859-9")),
    Some(("Nordic (ISO Latin 6)", "ISO-8859-10")),
    Some(("Baltic (ISO Latin 7)", "ISO-8859-13")),
    Some(("Celtic (ISO Latin 8)", "ISO-8859-14")),
    Some(("Western (ISO Latin 9)", "ISO-8859-15")),
    Some(("Romanian (ISO Latin 10)", "ISO-8859-16")),
    None,
    Some(("Latin-US (DOS)", "CP437")),
    Some(("Central European (Windows Latin 2)", "WINDOWS-1250")),
    Some(("Western (ASCII)", "ASCII")),
    None,
    Some(("Unicode (UTF-16)", "UTF-16")),
    Some(("Unicode (UTF-16BE)", "UTF-16BE")),
    Some(("Unicode (UTF-16LE)", "UTF-16LE")),
    Some(("Unicode (UTF-32)", "UTF-32")),
    Some(("Unicode (UTF-32BE)", "UTF-32BE")),
    Some(("Unicode (UTF-32LE)", "UTF-32LE")),
];

/// A document's encoding: iconv name plus whether a UTF-8 BOM is written ("Unicode (UTF-8) with BOM").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoding {
    pub iana: String,
    pub bom: bool,
}

impl Encoding {
    pub fn utf8() -> Self {
        Encoding { iana: "UTF-8".into(), bom: false }
    }

    pub fn label(&self) -> String {
        let base = DEFAULT_ENCODINGS
            .iter()
            .flatten()
            .find(|(_, i)| i.eq_ignore_ascii_case(&self.iana))
            .map(|(n, _)| n.to_string())
            .unwrap_or_else(|| self.iana.clone());
        if self.bom {
            format!("{base} with BOM")
        } else {
            base
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Cr,
    CrLf,
    Nel,
    Ls,
    Ps,
}

impl LineEnding {
    pub const ALL: [LineEnding; 6] = [Self::Lf, Self::Cr, Self::CrLf, Self::Nel, Self::Ls, Self::Ps];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Cr => "\r",
            Self::CrLf => "\r\n",
            Self::Nel => "\u{85}",
            Self::Ls => "\u{2028}",
            Self::Ps => "\u{2029}",
        }
    }

    /// Short name (status bar): LF, CR, CRLF, NEL, LS, PS.
    pub fn short(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Cr => "CR",
            Self::CrLf => "CRLF",
            Self::Nel => "NEL",
            Self::Ls => "LS",
            Self::Ps => "PS",
        }
    }

    /// Long name (menu): "macOS / Unix (LF)" etc.
    pub fn long(self) -> &'static str {
        match self {
            Self::Lf => "macOS / Unix (LF)",
            Self::Cr => "Classic Mac OS (CR)",
            Self::CrLf => "Windows (CRLF)",
            Self::Nel => "Unix Next Line (NEL)",
            Self::Ls => "Unix Line Separator (LS)",
            Self::Ps => "Unix Paragraph Separator (PS)",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Lf => "macOS / Unix",
            Self::Cr => "Classic Mac OS",
            Self::CrLf => "Windows",
            Self::Nel => "Unix Next Line",
            Self::Ls => "Unix Line Separator",
            Self::Ps => "Unix Paragraph Separator",
        }
    }

    pub fn from_short(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.short() == s)
    }
}

/// Every line ending in `text` with its byte offset (CRLF counted once).
pub fn line_endings(text: &str) -> Vec<(usize, LineEnding)> {
    let mut out = Vec::new();
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let e = match c {
            '\r' => {
                if it.peek().map(|(_, n)| *n) == Some('\n') {
                    it.next();
                    LineEnding::CrLf
                } else {
                    LineEnding::Cr
                }
            }
            '\n' => LineEnding::Lf,
            '\u{85}' => LineEnding::Nel,
            '\u{2028}' => LineEnding::Ls,
            '\u{2029}' => LineEnding::Ps,
            _ => continue,
        };
        out.push((i, e));
    }
    out
}

/// The most common line ending (`None` if the text has none) and whether others occur too.
pub fn dominant_line_ending(text: &str) -> (Option<LineEnding>, bool) {
    let mut counts = [0usize; 6];
    for (_, e) in line_endings(text) {
        counts[LineEnding::ALL.iter().position(|x| *x == e).unwrap()] += 1;
    }
    let used = counts.iter().filter(|c| **c > 0).count();
    let best = counts.iter().enumerate().max_by_key(|(i, c)| (**c, usize::MAX - i)).filter(|(_, c)| **c > 0);
    (best.map(|(i, _)| LineEnding::ALL[i]), used > 1)
}

/// `text` with every line ending replaced by `ending`.
pub fn convert_line_endings(text: &str, ending: LineEnding) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (i, e) in line_endings(text) {
        out.push_str(&text[last..i]);
        out.push_str(ending.as_str());
        last = i + e.as_str().len();
    }
    out.push_str(&text[last..]);
    out
}

pub struct Decoded {
    pub text: String,
    pub encoding: Encoding,
}

fn decode_with(bytes: &[u8], iana: &str) -> Option<String> {
    if iana.eq_ignore_ascii_case("UTF-8") {
        return std::str::from_utf8(bytes).ok().map(str::to_string);
    }
    let (out, read) = glib::convert(bytes, "UTF-8", iana).ok()?;
    (read == bytes.len()).then(|| String::from_utf8_lossy(&out).into_owned())
}

/// An encoding named in the first 1024 bytes (CSS @charset, coding:/encoding:/fileencoding= in the first two
/// lines, HTML/XML charset= / encoding= after ASCII only), as an iconv name.
fn declared_encoding(bytes: &[u8]) -> Option<String> {
    let head = &bytes[..bytes.len().min(1024)];
    let ascii_end = head.iter().position(|b| !b.is_ascii()).unwrap_or(head.len());
    let text = String::from_utf8_lossy(&head[..ascii_end]);
    let re = regex::Regex::new(
        r#"(?ix)\A@charset\s+"([\w.:-]+)"|(?:coding[:=]|fileencoding=)\s*["']?([\w.:-]+)|(?:charset|encoding)\s*=\s*["']?([\w.:-]+)"#,
    )
    .ok()?;
    let first_two: usize = text.match_indices('\n').nth(1).map(|(i, _)| i).unwrap_or(text.len());
    let mut best: Option<(usize, String)> = None;
    for c in re.captures_iter(&text) {
        let (m, name) = if let Some(m) = c.get(1) {
            (m, m.as_str())
        } else if let Some(m) = c.get(2) {
            if m.start() > first_two {
                continue;
            }
            (m, m.as_str())
        } else if let Some(m) = c.get(3) {
            (m, m.as_str())
        } else {
            continue;
        };
        if best.as_ref().map(|(s, _)| m.start() < *s).unwrap_or(true) {
            best = Some((m.start(), name.to_string()));
        }
    }
    best.map(|(_, n)| n.to_uppercase())
}

/// Decodes `bytes` like CotEditor: a forced encoding only; else BOM, declaration, then `candidates` in order.
pub fn decode(bytes: &[u8], candidates: &[String], forced: Option<&Encoding>, use_declaration: bool) -> Option<Decoded> {
    if let Some(f) = forced {
        let body = if f.bom && bytes.starts_with(&[0xEF, 0xBB, 0xBF]) { &bytes[3..] } else { bytes };
        return decode_with(body, &f.iana).map(|text| Decoded { text, encoding: f.clone() });
    }
    let boms: [(&[u8], &str); 5] = [
        (&[0xEF, 0xBB, 0xBF], "UTF-8"),
        (&[0x00, 0x00, 0xFE, 0xFF], "UTF-32BE"),
        (&[0xFF, 0xFE, 0x00, 0x00], "UTF-32LE"),
        (&[0xFE, 0xFF], "UTF-16BE"),
        (&[0xFF, 0xFE], "UTF-16LE"),
    ];
    for (bom, iana) in boms {
        if bytes.starts_with(bom) {
            if iana == "UTF-8" {
                if let Some(text) = decode_with(&bytes[3..], "UTF-8") {
                    return Some(Decoded { text, encoding: Encoding { iana: "UTF-8".into(), bom: true } });
                }
            } else if let Some(text) = decode_with(bytes, if iana.starts_with("UTF-16") { "UTF-16" } else { "UTF-32" }) {
                let fam = if iana.starts_with("UTF-16") { "UTF-16" } else { "UTF-32" };
                return Some(Decoded { text, encoding: Encoding { iana: fam.into(), bom: false } });
            }
        }
    }
    if use_declaration {
        if let Some(name) = declared_encoding(bytes) {
            let known = candidates.iter().find(|c| c.eq_ignore_ascii_case(&name)
                || (name == "SHIFT-JIS" && c.as_str() == "SHIFT_JIS"));
            if let Some(iana) = known {
                if let Some(text) = decode_with(bytes, iana) {
                    return Some(Decoded { text, encoding: Encoding { iana: iana.clone(), bom: false } });
                }
            }
        }
    }
    for iana in candidates {
        if let Some(text) = decode_with(bytes, iana) {
            return Some(Decoded { text, encoding: Encoding { iana: iana.clone(), bom: false } });
        }
    }
    None
}

/// The characters of `text` that `enc` can't represent: (char offset, character), in order, deduplicated per
/// position. Empty when all of it converts.
pub fn incompatible_chars(text: &str, enc: &Encoding) -> Vec<(usize, char)> {
    if enc.iana.starts_with("UTF") || glib::convert(text.as_bytes(), &enc.iana, "UTF-8").is_ok() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut buf = [0u8; 4];
    for (i, c) in text.chars().enumerate() {
        if c.is_ascii() {
            continue;
        }
        if glib::convert(c.encode_utf8(&mut buf).as_bytes(), &enc.iana, "UTF-8").is_err() {
            out.push((i, c));
        }
    }
    out
}

/// `text` in `enc` (characters it can't hold become '?'), with a BOM if asked.
pub fn encode(text: &str, enc: &Encoding) -> Vec<u8> {
    let mut out = Vec::new();
    if enc.iana.eq_ignore_ascii_case("UTF-8") {
        if enc.bom {
            out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        out.extend_from_slice(text.as_bytes());
        return out;
    }
    if let Ok((bytes, _)) = glib::convert(text.as_bytes(), &enc.iana, "UTF-8") {
        out.extend_from_slice(&bytes);
        return out;
    }
    let mut buf = [0u8; 4];
    for c in text.chars() {
        match glib::convert(c.encode_utf8(&mut buf).as_bytes(), &enc.iana, "UTF-8") {
            Ok((b, _)) => out.extend_from_slice(&b),
            Err(_) => out.push(b'?'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cands() -> Vec<String> {
        DEFAULT_ENCODINGS.iter().flatten().map(|(_, i)| i.to_string()).collect()
    }

    #[test]
    fn utf8_bom_and_plain() {
        let d = decode(b"\xEF\xBB\xBFhi", &cands(), None, true).unwrap();
        assert_eq!((d.text.as_str(), d.encoding.bom), ("hi", true));
        assert_eq!(encode("hi", &d.encoding), b"\xEF\xBB\xBFhi");
        let d = decode("привіт".as_bytes(), &cands(), None, true).unwrap();
        assert_eq!(d.encoding, Encoding::utf8());
    }

    #[test]
    fn declaration_wins_over_the_list() {
        let bytes = encode("# -*- coding: windows-1251 -*-\nпривет\n", &Encoding { iana: "WINDOWS-1251".into(), bom: false });
        let d = decode(&bytes, &cands(), None, true).unwrap();
        assert_eq!(d.encoding.iana, "WINDOWS-1251");
        assert!(d.text.contains("привет"));
    }

    #[test]
    fn line_endings_and_incompatibles() {
        assert_eq!(dominant_line_ending("a\r\nb\r\nc\n"), (Some(LineEnding::CrLf), true));
        assert_eq!(dominant_line_ending("abc"), (None, false));
        assert_eq!(convert_line_endings("a\r\nb\rc\n", LineEnding::Lf), "a\nb\nc\n");
        let latin1 = Encoding { iana: "ISO-8859-1".into(), bom: false };
        assert_eq!(incompatible_chars("café ☕", &latin1), vec![(5, '☕')]);
        assert_eq!(encode("café ☕", &latin1), b"caf\xE9 ?");
    }
}
