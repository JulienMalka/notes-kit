pub fn strip_org_markup(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let indexed: Vec<(usize, char)> = input.char_indices().collect();
    let len = indexed.len();
    let mut i = 0;

    while i < len {
        let (byte_i, ch) = indexed[i];

        if ch == '[' && i + 1 < len && indexed[i + 1].1 == '[' {
            if let Some(close) = input[byte_i..].find("]]") {
                let inner = &input[byte_i + 2..byte_i + close];
                if let Some(sep) = inner.find("][") {
                    out.push_str(&inner[sep + 2..]);
                } else {
                    out.push_str(inner);
                }
                let end_byte = byte_i + close + 2;
                while i < len && indexed[i].0 < end_byte {
                    i += 1;
                }
                continue;
            }
        }

        if matches!(ch, '*' | '/' | '_' | '+' | '~' | '=') {
            let marker = ch;
            if let Some(end) = indexed[i + 1..].iter().position(|&(_, c)| c == marker) {
                let inner_start = i + 1;
                let inner_end = i + 1 + end;
                if inner_end > inner_start
                    && indexed[inner_start].1 != ' '
                    && indexed[inner_end - 1].1 != ' '
                {
                    for &(_, c) in &indexed[inner_start..inner_end] {
                        out.push(c);
                    }
                    i = inner_end + 1;
                    continue;
                }
            }
        }

        out.push(ch);
        i += 1;
    }

    out
}

fn strip_headline_prefix(line: &str) -> Option<&str> {
    let star_count = line.as_bytes().iter().take_while(|&&b| b == b'*').count();
    if star_count > 0 && line.as_bytes().get(star_count) == Some(&b' ') {
        Some(line[star_count + 1..].trim())
    } else {
        None
    }
}

pub fn extract_excerpt(content: &str, max_chars: usize) -> String {
    let mut buf = String::new();
    let mut in_drawer = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#+") {
            continue;
        }
        if trimmed.starts_with(':') && trimmed.ends_with(':') && trimmed.len() > 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            if inner == "PROPERTIES"
                || inner == "LOGBOOK"
                || inner.chars().all(|c| c.is_ascii_uppercase() || c == '_')
            {
                in_drawer = true;
                continue;
            }
            if inner == "END" {
                in_drawer = false;
                continue;
            }
        }
        if in_drawer {
            continue;
        }
        if trimmed.starts_with(':') && trimmed.contains(": ") {
            continue;
        }
        if trimmed.starts_with("SCHEDULED:")
            || trimmed.starts_with("DEADLINE:")
            || trimmed.starts_with("CLOSED:")
            || trimmed.starts_with("CLOCK:")
        {
            continue;
        }
        if trimmed.is_empty() {
            if !buf.is_empty() {
                buf.push(' ');
            }
            continue;
        }
        let text = strip_headline_prefix(trimmed)
            .or_else(|| trimmed.strip_prefix("- "))
            .unwrap_or(trimmed);
        if text.is_empty() {
            continue;
        }
        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(text);
        if buf.len() >= max_chars {
            break;
        }
    }

    let buf = strip_org_markup(&buf);
    // Org-mode special strings (matches org-export-with-special-strings)
    let buf = buf.replace("---", "\u{2014}").replace("--", "\u{2013}");

    if buf.len() > max_chars {
        let mut truncated = buf;
        let mut end = max_chars;
        while end > 0 && !truncated.is_char_boundary(end) {
            end -= 1;
        }
        if let Some(pos) = truncated[..end].rfind(' ') {
            truncated.truncate(pos);
        } else {
            truncated.truncate(end);
        }
        truncated.push_str("...");
        truncated
    } else {
        buf
    }
}

pub fn parse_property<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let mut in_drawer = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if !in_drawer {
            if trimmed.is_empty() || trimmed.starts_with("#+") {
                continue;
            }
            if trimmed == ":PROPERTIES:" {
                in_drawer = true;
                continue;
            }
            return None;
        }
        if trimmed == ":END:" {
            return None;
        }
        let Some(after_first_colon) = line.trim_start().strip_prefix(':') else { continue };
        let Some((k, after_key)) = after_first_colon.split_once(':') else { continue };
        if k.eq_ignore_ascii_case(key) {
            return Some(after_key.trim());
        }
    }
    None
}

pub fn extract_section(content: &str, heading: &str) -> Option<String> {
    let mut buf = String::new();
    let mut in_target = false;
    let mut found = false;
    let mut in_drawer = false;

    for line in content.lines() {
        let trimmed = line.trim();

        let star_count = trimmed.bytes().take_while(|&b| b == b'*').count();
        let is_heading =
            star_count > 0 && trimmed.as_bytes().get(star_count) == Some(&b' ');

        if is_heading {
            if in_target {
                break;
            }
            if star_count == 1 && trimmed[star_count + 1..].trim() == heading {
                in_target = true;
                found = true;
                in_drawer = false;
            }
            continue;
        }

        if !in_target {
            continue;
        }

        if trimmed.starts_with(':') && trimmed.ends_with(':') && trimmed.len() > 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            if inner == "END" {
                in_drawer = false;
                continue;
            }
            if inner.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                in_drawer = true;
                continue;
            }
        }
        if in_drawer {
            continue;
        }

        if trimmed.starts_with("#+") || trimmed.is_empty() {
            continue;
        }

        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(trimmed);
    }

    if found { Some(buf) } else { None }
}

pub fn parse_org_link(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();
    let inner = trimmed.strip_prefix("[[")?.strip_suffix("]]")?;
    if let Some((url, text)) = inner.split_once("][") {
        Some((url.to_string(), text.to_string()))
    } else {
        Some((inner.to_string(), inner.to_string()))
    }
}

pub fn parse_field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    for line in content.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("- ") else { continue };
        let rest = rest.trim_start_matches('*');
        let Some(after_key) = rest.strip_prefix(key) else { continue };
        let after_key = after_key.trim_start_matches('*');
        let Some(after_colon) = after_key.strip_prefix(':') else { continue };
        let after_colon = after_colon.trim_start_matches('*');
        let Some(value) = after_colon.strip_prefix(' ') else { continue };
        return Some(value.trim());
    }
    None
}

pub fn reading_time(content: &str) -> usize {
    let words = content.len() / 5;
    std::cmp::max(1, words / 200)
}

pub fn growth_stage(content_len: usize) -> (&'static str, &'static str, &'static str) {
    if content_len >= 2000 {
        ("evergreen", "\u{1F332}", "Evergreen")
    } else if content_len >= 500 {
        ("budding", "\u{1F33F}", "Budding")
    } else {
        ("seedling", "\u{1F331}", "Seedling")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_org_link_with_desc() {
        assert_eq!(strip_org_markup("see [[https://x.com][example]]"), "see example");
    }

    #[test]
    fn strip_org_link_without_desc() {
        assert_eq!(strip_org_markup("[[https://x.com]]"), "https://x.com");
    }

    #[test]
    fn strip_bold() {
        assert_eq!(strip_org_markup("hello *world*"), "hello world");
    }

    #[test]
    fn strip_italic() {
        assert_eq!(strip_org_markup("/emphasis/"), "emphasis");
    }

    #[test]
    fn extract_excerpt_skips_keywords() {
        let content = "#+TITLE: My Note\n#+DATE: 2025-01-01\nHello world.";
        assert_eq!(extract_excerpt(content, 100), "Hello world.");
    }

    #[test]
    fn extract_excerpt_truncates() {
        let content = "This is a long sentence that should be truncated at a word boundary.";
        let excerpt = extract_excerpt(content, 30);
        assert!(excerpt.ends_with("..."));
        assert!(excerpt.len() <= 40);
    }

    #[test]
    fn parse_field_found() {
        let content = "#+TITLE: Test\n- Authors: Alice, Bob\n- Venue: ICSE 2025\n";
        assert_eq!(parse_field(content, "Authors"), Some("Alice, Bob"));
        assert_eq!(parse_field(content, "Venue"), Some("ICSE 2025"));
    }

    #[test]
    fn parse_field_not_found() {
        assert_eq!(parse_field("no fields here", "Key"), None);
    }

    #[test]
    fn parse_field_bold_label() {
        // Colon inside bold: *Key:* value
        let c1 = "- *Authors:* Alice, Bob\n- *Venue:* ICSE 2025\n";
        assert_eq!(parse_field(c1, "Authors"), Some("Alice, Bob"));
        assert_eq!(parse_field(c1, "Venue"), Some("ICSE 2025"));
        // Colon outside bold: *Key*: value
        let c2 = "- *Authors*: Alice, Bob\n";
        assert_eq!(parse_field(c2, "Authors"), Some("Alice, Bob"));
    }

    #[test]
    fn parse_property_basic() {
        let c = ":PROPERTIES:\n:AUTHORS: Alice, Bob\n:VENUE: ICSE 2025\n:END:\n#+TITLE: x\n";
        assert_eq!(parse_property(c, "AUTHORS"), Some("Alice, Bob"));
        assert_eq!(parse_property(c, "VENUE"), Some("ICSE 2025"));
    }

    #[test]
    fn parse_property_case_insensitive_key() {
        let c = ":PROPERTIES:\n:Authors: Alice\n:END:\n";
        assert_eq!(parse_property(c, "AUTHORS"), Some("Alice"));
        assert_eq!(parse_property(c, "authors"), Some("Alice"));
    }

    #[test]
    fn parse_property_after_keywords() {
        let c = "#+TITLE: x\n#+DATE: 2025\n:PROPERTIES:\n:AUTHORS: Alice\n:END:\n";
        assert_eq!(parse_property(c, "AUTHORS"), Some("Alice"));
    }

    #[test]
    fn parse_property_missing_drawer() {
        let c = "#+TITLE: x\n- Authors: Alice\n";
        assert_eq!(parse_property(c, "AUTHORS"), None);
    }

    #[test]
    fn parse_property_missing_key() {
        let c = ":PROPERTIES:\n:AUTHORS: Alice\n:END:\n";
        assert_eq!(parse_property(c, "VENUE"), None);
    }

    #[test]
    fn parse_property_link_value() {
        let c = ":PROPERTIES:\n:VENUE: [[https://x][ICSE]]\n:END:\n";
        assert_eq!(parse_property(c, "VENUE"), Some("[[https://x][ICSE]]"));
    }

    #[test]
    fn extract_section_basic() {
        let c = "#+TITLE: x\n* Abstract\n\nWe present a thing.\n\n* See also\nfoo\n";
        assert_eq!(extract_section(c, "Abstract"), Some("We present a thing.".to_string()));
    }

    #[test]
    fn extract_section_missing() {
        let c = "#+TITLE: x\n* Other\nfoo\n";
        assert_eq!(extract_section(c, "Abstract"), None);
    }

    #[test]
    fn extract_section_inline_links() {
        let c = "* Abstract\nSee [[https://x][here]] for details.\n";
        assert_eq!(
            extract_section(c, "Abstract"),
            Some("See [[https://x][here]] for details.".to_string())
        );
    }

    #[test]
    fn extract_section_multiline_joined() {
        let c = "* Abstract\nFirst line.\nSecond line.\n";
        assert_eq!(
            extract_section(c, "Abstract"),
            Some("First line. Second line.".to_string())
        );
    }

    #[test]
    fn extract_section_stops_at_subheading() {
        let c = "* Abstract\nBody.\n** Sub\nSubbody.\n";
        assert_eq!(extract_section(c, "Abstract"), Some("Body.".to_string()));
    }

    #[test]
    fn parse_org_link_with_desc() {
        assert_eq!(
            parse_org_link("[[https://x][example]]"),
            Some(("https://x".to_string(), "example".to_string()))
        );
    }

    #[test]
    fn parse_org_link_without_desc() {
        assert_eq!(
            parse_org_link("[[https://x]]"),
            Some(("https://x".to_string(), "https://x".to_string()))
        );
    }

    #[test]
    fn parse_org_link_plain_text() {
        assert_eq!(parse_org_link("https://x"), None);
        assert_eq!(parse_org_link("just text"), None);
    }

    #[test]
    fn reading_time_short() {
        assert_eq!(reading_time("hello"), 1);
    }

    #[test]
    fn reading_time_long() {
        let content = "a ".repeat(1000);
        assert!(reading_time(&content) >= 1);
    }

    #[test]
    fn growth_stages() {
        assert_eq!(growth_stage(3000).0, "evergreen");
        assert_eq!(growth_stage(800).0, "budding");
        assert_eq!(growth_stage(100).0, "seedling");
    }
}
