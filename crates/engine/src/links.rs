//! Which hyperlink targets may be opened. A cell's link comes from the document (often a file
//! somebody sent), so it is hostile input: only `http`, `https` and `mailto` ever leave the
//! workbook for the system; `file:`, `smb:`, UNC paths, custom URL handlers, `javascript:` and
//! the like are refused. Everything without a scheme stays an in-workbook reference.

use gridcraft_core::RangeRef;

/// What a link target is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// A cell, range, name or table of the workbook (`Sheet2!A1`, `Total`).
    Internal(String),
    /// An address to hand to the system, trimmed: `http`, `https` or `mailto` only.
    External(String),
    /// Refused, with the reason to show.
    Refused(String),
}

/// Unicode format characters (category Cf) that reorder or hide text: soft hyphen, Arabic letter
/// mark, zero-width and bidi marks/overrides/isolates, invisible operators, BOM, tag characters
/// (the full category).
fn is_format_char(c: char) -> bool {
    matches!(c, '\u{00AD}' | '\u{0600}'..='\u{0605}' | '\u{061C}' | '\u{06DD}' | '\u{070F}' | '\u{0890}'..='\u{0891}' | '\u{08E2}' | '\u{180E}'
            | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{206F}' | '\u{FEFF}' | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}' | '\u{110CD}' | '\u{13430}'..='\u{1343F}' | '\u{1BCA0}'..='\u{1BCA3}' | '\u{1D173}'..='\u{1D17A}' | '\u{E0001}' | '\u{E0020}'..='\u{E007F}')
}

const ALLOWED: [&str; 3] = ["http", "https", "mailto"];

/// Classifies a link target. Never panics; the single place that decides what may be opened.
pub fn classify(target: &str) -> LinkKind {
    let t = target.trim();
    if t.chars().any(|c| c.is_control() || is_format_char(c)) {
        return LinkKind::Refused("The link contains control characters.".into());
    }
    if t.starts_with("\\\\") || t.starts_with("//") {
        return LinkKind::Refused("Links to network locations aren't opened.".into());
    }
    if RangeRef::parse(t).is_some() {
        return LinkKind::Internal(t.to_string());
    }
    let Some((scheme, rest)) = t.split_once(':') else { return LinkKind::Internal(t.to_string()) };
    let scheme_like = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !scheme_like {
        return LinkKind::Internal(t.to_string());
    }
    let lower = scheme.to_ascii_lowercase();
    if ALLOWED.contains(&lower.as_str()) && !rest.trim().is_empty() {
        LinkKind::External(t.to_string())
    } else {
        LinkKind::Refused(format!("Links of type \"{lower}:\" aren't opened. Only http, https and mailto links are allowed."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_characters_in_a_link_are_refused() {
        for bad in [
            "https://example.com/\u{202E}gpj.exe",
            "https://exa\u{200B}mple.com",
            "https://a.example/\u{FEFF}",
            "mailto:a@b.example\u{2066}",
            "https://a.example/\u{061C}",
            "https://a.example/\u{00AD}",
            "https://a.example/\u{E0041}",
            "https://a.example/\u{0600}",
            "https://a.example/\u{206F}",
        ] {
            assert!(matches!(classify(bad), LinkKind::Refused(_)), "{bad:?}");
        }
        assert!(matches!(classify("https://example.com/gpj.exe"), LinkKind::External(_)));
    }

    #[test]
    fn only_http_https_and_mailto_leave_the_workbook() {
        for ok in ["http://a.example/x", "HTTPS://A.example", "  https://a.example  ", "MailTo:a@b.example"] {
            assert!(matches!(classify(ok), LinkKind::External(_)), "{ok}");
        }
        assert_eq!(classify(" https://a.example "), LinkKind::External("https://a.example".into()));
        for bad in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "file:///etc/passwd",
            "smb://host/share",
            "ms-excel:ofe|u|http://x",
            "\\\\host\\share\\x.exe",
            "//host/share",
            "C:\\Windows\\calc.exe",
            "\tjavascript:1",
            "java\nscript:1",
            "\u{1}http://a.example",
            "http:",
            "mailto:",
            "x-custom://a",
        ] {
            assert!(matches!(classify(bad), LinkKind::Refused(_)), "{bad:?} -> {:?}", classify(bad));
        }
    }

    #[test]
    fn references_stay_internal() {
        for r in ["Sheet2!A1", "'My Sheet'!B2:C3", "A1:B2", "#Sheet1!A1", "Total", "", "   "] {
            assert!(matches!(classify(r), LinkKind::Internal(_)), "{r:?} -> {:?}", classify(r));
        }
    }
}
