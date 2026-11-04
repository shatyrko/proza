// CotEditor's themes (data/themes/*.cottheme, Apache-2.0, see data/themes/NOTICE) as GtkSourceView colour schemes.
//
// A theme has colours for the editor (text, background, invisibles, line highlight, selection, insertion point) and
// for CotEditor's syntax categories (keywords, commands, types, attributes, variables, values, numbers, strings,
// characters, comments). Each category is mapped onto GtkSourceView's standard `def:*` styles, which every language
// definition inherits from, so highlighting looks like CotEditor's in every syntax.

use serde_json::Value;
use std::path::PathBuf;

macro_rules! themes {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_str!(concat!("../data/themes/", $name, ".cottheme")))),*]
    };
}

/// The bundled themes, in CotEditor's order.
pub const BUNDLED: &[(&str, &str)] = themes![
    "Anura", "Anura (Dark)", "Classic", "Classic (Dark)", "Dendrobates", "Dendrobates (Dark)", "Kawazu",
    "Lakritz", "Mono", "Note", "Printen", "Pulse", "Resinifictrix", "Resinifictrix (Dark)",
];

pub const DEFAULT_LIGHT: &str = "Anura";
pub const DEFAULT_DARK: &str = "Anura (Dark)";

/// GtkSourceView scheme id of a theme name ("Anura (Dark)" -> "proza-anura-dark").
pub fn scheme_id(name: &str) -> String {
    let mut id = String::from("proza-");
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else if !id.ends_with('-') {
            id.push('-');
        }
    }
    id.trim_end_matches('-').to_string()
}

/// The dark (or light) counterpart CotEditor switches to when the system appearance changes.
pub fn counterpart(name: &str, dark: bool) -> String {
    let base = name.trim_end_matches(" (Dark)");
    let candidate = if dark { format!("{base} (Dark)") } else { base.to_string() };
    if BUNDLED.iter().any(|(n, _)| *n == candidate) {
        candidate
    } else {
        name.to_string()
    }
}

fn color(theme: &Value, key: &str) -> Option<String> {
    theme.get(key)?.get("color")?.as_str().map(|c| {
        // CotEditor writes #RRGGBB or #RRGGBBAA; GtkSourceView reads both
        c.to_string()
    })
}

/// "#e3e3e3b3" over `bg` -> an opaque "#rrggbb" (GtkSourceView ignores alpha in some style properties).
fn flatten(c: &str, bg: &str) -> String {
    let hex = |s: &str, i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0) as f64;
    if c.len() != 9 || bg.len() < 7 {
        return c.to_string();
    }
    let a = hex(c, 7) / 255.0;
    let mix = |i: usize| (hex(c, i) * a + hex(bg, i) * (1.0 - a)).round() as u8;
    format!("#{:02x}{:02x}{:02x}", mix(1), mix(3), mix(5))
}

fn is_dark(bg: &str) -> bool {
    let hex = |i: usize| u8::from_str_radix(&bg[i..i + 2], 16).unwrap_or(255) as f64;
    bg.len() >= 7 && (0.299 * hex(1) + 0.587 * hex(3) + 0.114 * hex(5)) < 128.0
}

/// The colours Proza draws itself (the selection), for the window stylesheet.
pub struct EditorColors {
    pub text: String,
    /// Background of the other occurrences of a selected word: the theme's highlight colour (the system's find
    /// colour, yellow, when it uses that) at 30 % over the background.
    pub instance: String,
    /// `None`: the theme uses the system's selection colour (an accent tint).
    pub selection: Option<String>,
}

pub fn editor_colors(name: &str) -> Option<EditorColors> {
    let json = BUNDLED.iter().find(|(n, _)| *n == name)?.1;
    let t: Value = serde_json::from_str(json).ok()?;
    let bg = color(&t, "background").unwrap_or_else(|| "#ffffff".into());
    let system = t.get("selection").and_then(|s| s.get("usesSystemSetting")).and_then(Value::as_bool).unwrap_or(false);
    let highlight_system = t.get("highlight").and_then(|s| s.get("usesSystemSetting")).and_then(Value::as_bool).unwrap_or(true);
    let highlight = if highlight_system { None } else { color(&t, "highlight") }.unwrap_or_else(|| "#ffd60a".into());
    Some(EditorColors {
        instance: flatten(&format!("{}4d", &highlight[..7.min(highlight.len())]), &bg),
        text: color(&t, "text").map(|c| c[..7.min(c.len())].to_string()).unwrap_or_else(|| "#000000".into()),
        selection: if system { None } else { color(&t, "selection").map(|c| flatten(&c, &bg)) },
    })
}

fn scheme_xml(name: &str, json: &str) -> Option<String> {
    let t: Value = serde_json::from_str(json).ok()?;
    let bg = color(&t, "background").unwrap_or_else(|| "#ffffff".into());
    let fg = color(&t, "text").unwrap_or_else(|| "#000000".into());
    let inv = color(&t, "invisibles").unwrap_or_else(|| "#bfbfbf".into());
    let line = flatten(&color(&t, "lineHighlight").unwrap_or_else(|| "#e3e3e3".into()), &bg);
    let sel = flatten(&color(&t, "selection").unwrap_or_else(|| "#a7caff".into()), &bg);
    let caret = color(&t, "insertionPoint").unwrap_or_else(|| fg.clone());
    let dark = is_dark(&bg);
    // line numbers: the text colour at 40 % (CotEditor draws them that way), current line at 100 %
    let numbers = flatten(&format!("{}66", &fg[..7.min(fg.len())]), &bg);
    let guide = flatten(&format!("{}33", &fg[..7.min(fg.len())]), &bg);
    let cat = |k: &str| color(&t, k).map(|c| flatten(&c, &bg));
    let mut styles = String::new();
    let mut style = |id: &str, fgc: Option<String>, extra: &str| {
        if let Some(c) = fgc {
            styles.push_str(&format!("  <style name=\"{id}\" foreground=\"{c}\"{extra}/>\n"));
        }
    };
    let (kw, cmd, ty, attr, var, val, num, st, ch, com) = (
        cat("keywords"), cat("commands"), cat("types"), cat("attributes"), cat("variables"),
        cat("values"), cat("numbers"), cat("strings"), cat("characters"), cat("comments"),
    );
    for id in ["def:keyword", "def:statement", "def:reserved", "def:operator"] {
        style(id, if id == "def:operator" { None } else { kw.clone() }, "");
    }
    for id in ["def:builtin", "def:function", "def:preprocessor", "def:net-address", "def:link-destination"] {
        style(id, cmd.clone(), "");
    }
    for id in ["def:type", "def:heading", "def:shebang"] {
        style(id, ty.clone(), if id == "def:heading" { " bold=\"true\"" } else { "" });
    }
    for id in ["def:identifier", "def:variable", "def:link-text", "xml:attribute-name", "html:attrib-name"] {
        style(id, if id.contains("attrib") { attr.clone() } else { var.clone() }, "");
    }
    for id in ["def:constant", "def:boolean", "def:special-constant", "def:special-char", "def:emphasis", "def:strong-emphasis"] {
        let extra = match id {
            "def:emphasis" => " italic=\"true\"",
            "def:strong-emphasis" => " bold=\"true\"",
            _ => "",
        };
        style(id, val.clone(), extra);
    }
    for id in ["def:number", "def:decimal", "def:base-n-integer", "def:floating-point"] {
        style(id, num.clone(), "");
    }
    for id in ["def:string", "def:inline-code", "def:preformatted-section", "def:link-symbol"] {
        style(id, st.clone(), "");
    }
    for id in ["def:character", "def:special-character"] {
        style(id, ch.clone(), "");
    }
    for id in ["def:comment", "def:doc-comment", "def:doc-comment-element", "def:note", "def:shebang-comment"] {
        style(id, com.clone(), "");
    }
    style("xml:element-name", kw.clone(), "");
    style("def:error", None, "");
    let id = scheme_id(name);
    let esc = name.replace('&', "&amp;").replace('"', "&quot;");
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<style-scheme id=\"{id}\" name=\"{esc}\" version=\"1.0\" kind=\"{}\">\n\
  <description>CotEditor theme {esc} (Apache-2.0)</description>\n\
  <style name=\"text\" foreground=\"{fg}\" background=\"{bg}\"/>\n\
  <style name=\"selection\" background=\"{sel}\"/>\n\
  <style name=\"selection-unfocused\" background=\"{sel}\"/>\n\
  <style name=\"cursor\" foreground=\"{caret}\"/>\n\
  <style name=\"current-line\" background=\"{line}\"/>\n\
  <style name=\"current-line-number\" foreground=\"{fg}\" background=\"{bg}\" bold=\"true\"/>\n\
  <style name=\"line-numbers\" foreground=\"{numbers}\" background=\"{bg}\"/>\n\
  <style name=\"right-margin\" foreground=\"{guide}\"/>\n\
  <style name=\"draw-spaces\" foreground=\"{inv}\"/>\n\
  <style name=\"background-pattern\" background=\"{bg}\"/>\n\
  <style name=\"bracket-match\" background=\"#f7e26b\" foreground=\"#000000\"/>\n\
  <style name=\"search-match\" background=\"#f7e26b80\"/>\n\
{styles}</style-scheme>\n",
        if dark { "dark" } else { "light" }
    ))
}

/// Writes all bundled themes as schemes into the user's data folder and registers that folder. Returns its path.
pub fn install() -> PathBuf {
    let dir = gtk::glib::user_data_dir().join("proza").join("styles");
    let _ = std::fs::create_dir_all(&dir);
    for (name, json) in BUNDLED {
        if let Some(xml) = scheme_xml(name, json) {
            let path = dir.join(format!("{}.xml", scheme_id(name)));
            if std::fs::read_to_string(&path).ok().as_deref() != Some(xml.as_str()) {
                let _ = std::fs::write(&path, xml);
            }
        }
    }
    let sm = sourceview5::StyleSchemeManager::default();
    sm.append_search_path(dir.to_str().unwrap_or_default());
    sm.force_rescan();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_counterparts() {
        assert_eq!(scheme_id("Anura (Dark)"), "proza-anura-dark");
        assert_eq!(counterpart("Anura", true), "Anura (Dark)");
        assert_eq!(counterpart("Anura (Dark)", false), "Anura");
        assert_eq!(counterpart("Mono", true), "Mono");
    }

    #[test]
    fn every_bundled_theme_converts() {
        for (name, json) in BUNDLED {
            let xml = scheme_xml(name, json).expect(name);
            assert!(xml.contains("<style name=\"text\""), "{name}");
        }
        assert_eq!(flatten("#e3e3e3b3", "#fafafa"), "#eaeaea");
    }
}
