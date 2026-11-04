// Unicode facts for the inspector and the character inspector: name, block, general category and the version a
// character was added in. Blocks and ages are generated from the Unicode Character Database (data/unicode, via
// Perl's Unicode::UCD; Unicode 17.0).

use std::sync::OnceLock;
use unicode_general_category::{get_general_category, GeneralCategory as G};

fn table(src: &'static str) -> Vec<(u32, u32, &'static str)> {
    src.lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, ' ');
            let a = u32::from_str_radix(it.next()?, 16).ok()?;
            let b = u32::from_str_radix(it.next()?, 16).ok()?;
            Some((a, b, it.next()?))
        })
        .collect()
}

fn lookup(t: &[(u32, u32, &'static str)], c: char) -> Option<&'static str> {
    let cp = c as u32;
    let i = t.partition_point(|(_, b, _)| *b < cp);
    t.get(i).filter(|(a, _, _)| *a <= cp).map(|(_, _, v)| *v)
}

pub fn block(c: char) -> Option<&'static str> {
    static T: OnceLock<Vec<(u32, u32, &'static str)>> = OnceLock::new();
    lookup(T.get_or_init(|| table(include_str!("../data/unicode/blocks.txt"))), c)
}

/// "Unicode 6.1"
pub fn version(c: char) -> Option<String> {
    static T: OnceLock<Vec<(u32, u32, &'static str)>> = OnceLock::new();
    lookup(T.get_or_init(|| table(include_str!("../data/unicode/age.txt"))), c).map(|v| format!("Unicode {v}"))
}

/// "Uppercase Letter (Lu)"
pub fn category(c: char) -> String {
    let (name, code) = match get_general_category(c) {
        G::UppercaseLetter => ("Uppercase Letter", "Lu"),
        G::LowercaseLetter => ("Lowercase Letter", "Ll"),
        G::TitlecaseLetter => ("Titlecase Letter", "Lt"),
        G::ModifierLetter => ("Modifier Letter", "Lm"),
        G::OtherLetter => ("Other Letter", "Lo"),
        G::NonspacingMark => ("Nonspacing Mark", "Mn"),
        G::SpacingMark => ("Spacing Mark", "Mc"),
        G::EnclosingMark => ("Enclosing Mark", "Me"),
        G::DecimalNumber => ("Decimal Number", "Nd"),
        G::LetterNumber => ("Letter Number", "Nl"),
        G::OtherNumber => ("Other Number", "No"),
        G::ConnectorPunctuation => ("Connector Punctuation", "Pc"),
        G::DashPunctuation => ("Dash Punctuation", "Pd"),
        G::OpenPunctuation => ("Open Punctuation", "Ps"),
        G::ClosePunctuation => ("Close Punctuation", "Pe"),
        G::InitialPunctuation => ("Initial Punctuation", "Pi"),
        G::FinalPunctuation => ("Final Punctuation", "Pf"),
        G::OtherPunctuation => ("Other Punctuation", "Po"),
        G::MathSymbol => ("Math Symbol", "Sm"),
        G::CurrencySymbol => ("Currency Symbol", "Sc"),
        G::ModifierSymbol => ("Modifier Symbol", "Sk"),
        G::OtherSymbol => ("Other Symbol", "So"),
        G::SpaceSeparator => ("Space Separator", "Zs"),
        G::LineSeparator => ("Line Separator", "Zl"),
        G::ParagraphSeparator => ("Paragraph Separator", "Zp"),
        G::Control => ("Control", "Cc"),
        G::Format => ("Format", "Cf"),
        G::Surrogate => ("Surrogate", "Cs"),
        G::PrivateUse => ("Private Use", "Co"),
        G::Unassigned => ("Unassigned", "Cn"),
        _ => ("Unknown", "?"),
    };
    format!("{name} ({code})")
}

/// Unicode name, also for control characters ("LINE FEED" rather than nothing).
pub fn name(c: char) -> Option<String> {
    if let Some(n) = unicode_names2::name(c) {
        return Some(n.to_string());
    }
    let control = match c {
        '\0' => "NULL",
        '\t' => "CHARACTER TABULATION",
        '\n' => "LINE FEED (LF)",
        '\u{B}' => "LINE TABULATION",
        '\u{C}' => "FORM FEED (FF)",
        '\r' => "CARRIAGE RETURN (CR)",
        '\u{1B}' => "ESCAPE",
        '\u{7F}' => "DELETE",
        '\u{85}' => "NEXT LINE (NEL)",
        _ => return None,
    };
    Some(control.into())
}

/// `U+1F600`, with the UTF-16 surrogate pair for characters beyond the BMP.
pub fn code_point(c: char) -> String {
    let cp = c as u32;
    if cp > 0xFFFF {
        let mut units = [0u16; 2];
        c.encode_utf16(&mut units);
        format!("U+{cp:04X} (U+{:04X} U+{:04X})", units[0], units[1])
    } else {
        format!("U+{cp:04X}")
    }
}

/// How the character is shown in the inspectors: control characters and spaces as their control pictures.
pub fn picture(c: char) -> (String, bool) {
    match c {
        '\0'..='\u{1F}' => (char::from_u32(0x2400 + c as u32).unwrap_or(c).to_string(), true),
        '\u{7F}' => ("␡".into(), true),
        ' ' => ("␠".into(), true),
        '\u{A0}' | '\u{2000}'..='\u{200B}' | '\u{3000}' | '\u{2028}' | '\u{2029}' | '\u{FEFF}' => ("⬚".into(), true),
        c => (c.to_string(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facts() {
        assert_eq!(block('a'), Some("Basic Latin"));
        assert_eq!(block('ж'), Some("Cyrillic"));
        assert_eq!(version('😀').as_deref(), Some("Unicode 6.1"));
        assert_eq!(category('A'), "Uppercase Letter (Lu)");
        assert_eq!(code_point('😀'), "U+1F600 (U+D83D U+DE00)");
        assert_eq!(name('\n').as_deref(), Some("LINE FEED (LF)"));
        assert_eq!(picture('\t'), ("␉".into(), true));
    }
}
