// Outline extraction: CotEditor lists headings, types, functions and MARK comments per syntax. CotEditor uses
// tree-sitter for some syntaxes and regular expressions for the rest; Proza uses regular expressions for all,
// keyed by GtkSourceView's language ids.

use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Container,
    Value,
    Function,
    Title,
    /// A heading, with its level when the syntax has levels.
    Heading(Option<u8>),
    Mark,
    Separator,
}

impl Kind {
    /// The badge letter and colour class, after CotEditor's SF Symbols (cube, dot, f, tag, list, n, flag).
    pub fn badge(self) -> (String, &'static str) {
        match self {
            Kind::Container => ("C".into(), "container"),
            Kind::Value => ("•".into(), "value"),
            Kind::Function => ("f".into(), "function"),
            Kind::Title => ("T".into(), "title"),
            Kind::Heading(Some(n)) => (n.to_string(), "heading"),
            Kind::Heading(None) => ("≡".into(), "heading"),
            Kind::Mark => ("⚑".into(), "mark"),
            Kind::Separator => ("".into(), "separator"),
        }
    }

    /// Container and mark items are bold.
    pub fn bold(self) -> bool {
        matches!(self, Kind::Container | Kind::Mark)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Char offsets of the title in the document (the range that gets selected).
    pub start: usize,
    pub end: usize,
    pub title: String,
    pub kind: Kind,
    /// Literal leading indentation (shown in the menu), or levels as nesting depth.
    pub indent: String,
    pub level: usize,
}

struct Rule {
    langs: &'static [&'static str],
    pattern: &'static str,
    /// Kind for the match; a `k` group can override it (see `kind_of`).
    kind: Kind,
}

const C_LIKE: &[&str] = &["c", "cpp", "chdr", "cpphdr", "objc", "cuda", "glsl", "opencl"];
const JS: &[&str] = &["js", "jsx", "typescript", "typescript-jsx"];

const RULES: &[Rule] = &[
    // MARK comments in any syntax with line comments (MARK: - = separator before the mark)
    Rule { langs: &["*"], pattern: r"(?m)^[ \t]*(?://|#|--|;|/\*)[ \t]*MARK:[ \t]*(?P<sep>-)?[ \t]*(?P<t>[^\n*]*?)[ \t]*(?:\*/)?$", kind: Kind::Mark },
    Rule { langs: &["markdown"], pattern: r"(?m)^(?P<l>#{1,6})[ \t]+(?P<t>[^\n]+?)[ \t#]*$", kind: Kind::Heading(None) },
    Rule { langs: &["html", "xml", "svg", "mallard", "docbook"], pattern: r"(?is)<title>(?P<t>[^<]*)</title>", kind: Kind::Title },
    Rule { langs: &["html"], pattern: r"(?is)<h(?P<l>[1-6])[^>]*>(?P<t>.*?)</h[1-6]>", kind: Kind::Heading(None) },
    Rule { langs: &["css", "scss", "less"], pattern: r"(?m)^[ \t]*(?P<t>@media[^{\n]*?|[^\s@{}/;][^{};\n]*?)[ \t]*\{", kind: Kind::Value },
    Rule { langs: &["python", "python3", "gdscript"], pattern: r"(?m)^(?P<i>[ \t]*)(?:async[ \t]+)?(?P<k>class|def|func)[ \t]+(?P<t>\w+)", kind: Kind::Function },
    Rule {
        langs: &["rust"],
        pattern: r"(?m)^(?P<i>[ \t]*)(?:pub(?:\([^)]*\))?[ \t]+)?(?:(?:async|const|unsafe|default)[ \t]+|extern[ \t]+\x22[^\x22]*\x22[ \t]+)*(?P<k>fn|struct|enum|trait|mod|union|type|macro_rules!)[ \t]+(?P<t>\w+)",
        kind: Kind::Function,
    },
    Rule { langs: &["rust"], pattern: r"(?m)^(?P<i>[ \t]*)(?:unsafe[ \t]+)?(?P<k>impl)(?:<[^>\n]*>)?[ \t]+(?P<t>[^{\n]+?)[ \t]*(?:where\b[^{]*)?\{", kind: Kind::Container },
    Rule { langs: JS, pattern: r"(?m)^(?P<i>[ \t]*)(?:export[ \t]+)?(?:default[ \t]+)?(?:async[ \t]+)?(?P<k>function\*?|class)[ \t]+(?P<t>[\w$]+)", kind: Kind::Function },
    Rule { langs: JS, pattern: r"(?m)^(?P<i>[ \t]*)(?:export[ \t]+)?(?:const|let|var)[ \t]+(?P<t>[\w$]+)[ \t]*=[ \t]*(?:async[ \t]*)?(?:function\b|\([^)\n]*\)[ \t]*=>|[\w$]+[ \t]*=>)", kind: Kind::Function },
    Rule { langs: &["typescript", "typescript-jsx"], pattern: r"(?m)^(?P<i>[ \t]*)(?:export[ \t]+)?(?P<k>interface|enum|type)[ \t]+(?P<t>\w+)", kind: Kind::Container },
    Rule { langs: C_LIKE, pattern: r"(?m)^(?:typedef[ \t]+)?(?P<k>struct|class|enum|union|namespace)[ \t]+(?P<t>\w+)[^;\n]*$", kind: Kind::Container },
    Rule { langs: C_LIKE, pattern: r"(?m)^[A-Za-z_][\w \t\*&:<>,]*?[ \t\*&](?P<t>[A-Za-z_~][\w:~]*)[ \t]*\([^;{}]*\)[ \t]*(?:const[ \t]*)?(?:\{|$)", kind: Kind::Function },
    Rule { langs: &["go"], pattern: r"(?m)^func[ \t]+(?:\([^)]*\)[ \t]*)?(?P<t>\w+)", kind: Kind::Function },
    Rule { langs: &["go"], pattern: r"(?m)^type[ \t]+(?P<t>\w+)", kind: Kind::Container },
    Rule {
        langs: &["java", "kotlin", "swift", "c-sharp", "scala", "dart", "vala", "groovy"],
        pattern: r"(?m)^(?P<i>[ \t]*)(?:[@\w]+[ \t]+)*?(?P<k>class|interface|enum|struct|protocol|extension|object|record)[ \t]+(?P<t>\w+)",
        kind: Kind::Container,
    },
    Rule { langs: &["swift"], pattern: r"(?m)^(?P<i>[ \t]*)(?:[@\w]+[ \t]+)*?func[ \t]+(?P<t>[^\s(<]+)", kind: Kind::Function },
    Rule { langs: &["kotlin"], pattern: r"(?m)^(?P<i>[ \t]*)(?:\w+[ \t]+)*?fun[ \t]+(?:<[^>]*>[ \t]*)?(?:\w+\.)?(?P<t>\w+)", kind: Kind::Function },
    Rule { langs: &["ruby"], pattern: r"(?m)^(?P<i>[ \t]*)(?P<k>class|module|def)[ \t]+(?P<t>[\w.:?!=]+)", kind: Kind::Function },
    Rule { langs: &["php"], pattern: r"(?m)^(?P<i>[ \t]*)(?:(?:abstract|final|public|private|protected|static)[ \t]+)*(?P<k>class|interface|trait|function)[ \t]+(?P<t>\w+)", kind: Kind::Function },
    Rule { langs: &["lua"], pattern: r"(?m)^(?P<i>[ \t]*)(?:local[ \t]+)?function[ \t]+(?P<t>[\w.:]+)", kind: Kind::Function },
    Rule { langs: &["perl"], pattern: r"(?m)^(?P<i>[ \t]*)sub[ \t]+(?P<t>\w+)", kind: Kind::Function },
    Rule { langs: &["sh", "fish"], pattern: r"(?m)^(?P<i>[ \t]*)(?:function[ \t]+(?P<t>[\w.:-]+)|(?P<t2>[\w.:-]+)[ \t]*\(\))", kind: Kind::Function },
    Rule { langs: &["latex"], pattern: r"\\(?P<k>part|chapter|section|subsection|subsubsection|paragraph)\*?\{(?P<t>[^}\n]*)\}", kind: Kind::Heading(None) },
    Rule { langs: &["ini", "toml", "desktop", "pkgconfig"], pattern: r"(?m)^[ \t]*\[\[?(?P<t>[^\]\n]+)\]\]?", kind: Kind::Heading(None) },
    Rule { langs: &["makefile"], pattern: r"(?m)^(?P<t>[\w.%/$(){}-]+)[ \t]*::?(?:[^=]|$)", kind: Kind::Function },
    Rule { langs: &["yaml"], pattern: r"(?m)^(?P<t>[\w.-]+):", kind: Kind::Value },
    Rule { langs: &["diff"], pattern: r"(?m)^diff --git a/(?P<t>\S+)", kind: Kind::Title },
    Rule { langs: &["asciidoc", "t2t"], pattern: r"(?m)^(?P<l>={1,6})[ \t]+(?P<t>[^\n]+?)[ \t]*$", kind: Kind::Heading(None) },
    Rule { langs: &["sql"], pattern: r"(?im)^[ \t]*create[ \t]+(?:or[ \t]+replace[ \t]+)?(?:table|view|function|procedure|index|trigger)[ \t]+(?:if[ \t]+not[ \t]+exists[ \t]+)?(?P<t>[\w.\x22`]+)", kind: Kind::Container },
];

fn compiled() -> &'static Vec<Regex> {
    static C: OnceLock<Vec<Regex>> = OnceLock::new();
    C.get_or_init(|| RULES.iter().map(|r| Regex::new(r.pattern).expect(r.pattern)).collect())
}

fn kind_of(rule: &Rule, keyword: Option<&str>, level: Option<&str>) -> Kind {
    let heading_level = |s: &str| -> Option<u8> {
        match s {
            "part" => Some(1),
            "chapter" => Some(1),
            "section" => Some(1),
            "subsection" => Some(2),
            "subsubsection" => Some(3),
            "paragraph" => Some(4),
            _ => s.parse().ok().or_else(|| s.chars().all(|c| c == '#' || c == '=').then_some(s.len() as u8)),
        }
    };
    match rule.kind {
        Kind::Heading(_) => Kind::Heading(level.or(keyword).and_then(heading_level)),
        Kind::Function | Kind::Container => match keyword {
            Some("class" | "struct" | "enum" | "trait" | "mod" | "union" | "type" | "impl" | "interface" | "module" | "protocol" | "extension" | "object" | "record" | "namespace") => Kind::Container,
            Some("def" | "fn" | "func" | "function" | "function*" | "macro_rules!") => Kind::Function,
            _ => rule.kind,
        },
        k => k,
    }
}

/// The outline items of `text` for the GtkSourceView language `lang`, in document order.
pub fn extract(lang: &str, text: &str) -> Vec<Item> {
    if lang.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(usize, usize, Item)> = Vec::new(); // (byte start, byte end, item with byte offsets)
    for (rule, re) in RULES.iter().zip(compiled()) {
        if !(rule.langs.contains(&lang) || rule.langs == ["*"]) {
            continue;
        }
        for caps in re.captures_iter(text).take(20_000) {
            let Some(t) = caps.name("t").or_else(|| caps.name("t2")) else { continue };
            let raw = t.as_str();
            let title = raw.trim();
            let mut kind = kind_of(rule, caps.name("k").map(|m| m.as_str()), caps.name("l").map(|m| m.as_str()));
            if rule.kind == Kind::Mark {
                if title.is_empty() {
                    if caps.name("sep").is_none() {
                        continue;
                    }
                    kind = Kind::Separator;
                } else if caps.name("sep").is_some() {
                    let at = caps.get(0).map_or(t.start(), |m| m.start());
                    found.push((at, at, Item { start: 0, end: 0, title: String::new(), kind: Kind::Separator, indent: String::new(), level: 0 }));
                }
            } else if title.is_empty() {
                continue;
            }
            // a title with markup (html headings) is shown as plain text
            let title = if title.contains('<') { strip_tags(title) } else { title.split_whitespace().collect::<Vec<_>>().join(" ") };
            let lead = raw.len() - raw.trim_start().len();
            let (bs, be) = (t.start() + lead, t.start() + lead + raw.trim().len());
            let indent = caps.name("i").map(|m| m.as_str().to_string()).unwrap_or_default();
            let level = match kind {
                Kind::Heading(Some(n)) => n as usize - 1,
                _ => 0,
            };
            found.push((bs, be, Item { start: 0, end: 0, title, kind, indent, level }));
        }
    }
    found.sort_by_key(|(s, e, _)| (*s, *e));
    found.dedup_by(|a, b| a.0 == b.0 && a.2.kind != Kind::Separator && b.2.kind != Kind::Separator);
    // byte → char offsets, in one pass
    let mut items = Vec::with_capacity(found.len());
    let (mut byte, mut chars) = (0usize, 0usize);
    let mut advance = |to: usize| {
        chars += text[byte..to].chars().count();
        byte = to;
        chars
    };
    for (bs, be, mut item) in found {
        item.start = advance(bs);
        item.end = advance(be);
        items.push(item);
    }
    items
}

fn strip_tags(s: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let plain = RE.get_or_init(|| Regex::new(r"<[^>]*>").unwrap()).replace_all(s, "");
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The item containing `offset`: the last non-separator item that starts at or before it.
pub fn current(items: &[Item], offset: usize) -> Option<usize> {
    items.iter().rposition(|i| i.kind != Kind::Separator && i.start <= offset)
}

/// Previous: the item before the one containing the selection start. Next: the first item if the selection ends
/// before it, else the first item after the one containing the selection end.
pub fn previous(items: &[Item], sel_start: usize) -> Option<usize> {
    let cur = current(items, sel_start)?;
    items[..cur].iter().rposition(|i| i.kind != Kind::Separator)
}

pub fn next(items: &[Item], sel_end: usize) -> Option<usize> {
    match current(items, sel_end) {
        None => items.iter().position(|i| i.kind != Kind::Separator),
        Some(cur) => items.iter().enumerate().skip(cur + 1).find(|(_, i)| i.kind != Kind::Separator).map(|(n, _)| n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_and_svg() {
        let md = "# One\ntext\n## Two ##\n\n### Three\n";
        let items = extract("markdown", md);
        assert_eq!(items.iter().map(|i| (i.title.as_str(), i.kind)).collect::<Vec<_>>(), vec![
            ("One", Kind::Heading(Some(1))),
            ("Two", Kind::Heading(Some(2))),
            ("Three", Kind::Heading(Some(3)))
        ]);
        assert_eq!((items[0].start, items[0].end), (2, 5));
        let svg = "<svg>\n<title>besides editor</title>\n</svg>";
        assert_eq!(extract("svg", svg)[0].title, "besides editor");
    }

    #[test]
    fn code_and_marks() {
        let py = "class A:\n    def run(self):\n        pass\n# MARK: - Helpers\ndef f():\n    pass\n";
        let items = extract("python", py);
        let got: Vec<_> = items.iter().map(|i| (i.title.as_str(), i.kind, i.indent.as_str())).collect();
        assert_eq!(got, vec![
            ("A", Kind::Container, ""),
            ("run", Kind::Function, "    "),
            ("", Kind::Separator, ""),
            ("Helpers", Kind::Mark, ""),
            ("f", Kind::Function, "")
        ]);
        let rs = "pub struct S;\nimpl Display for S {\n}\npub(crate) async fn go() {}\n";
        let titles: Vec<_> = extract("rust", rs).into_iter().map(|i| i.title).collect();
        assert_eq!(titles, vec!["S", "Display for S", "go"]);
        assert_eq!(next(&items, 0), Some(0));
        assert_eq!(next(&items, 6), Some(1));
        assert_eq!(previous(&items, 30), Some(0));
        assert_eq!(current(&items, 0), None);
        assert_eq!(current(&items, 6), Some(0));
    }
}
