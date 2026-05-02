pub fn extract_metadata_fast(
    content: &str,
) -> (Option<String>, Option<String>, Vec<String>) {
    let mut title = None;
    let mut date = None;
    let mut filetags = Vec::new();
    let mut in_drawer = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if in_drawer {
            if trimmed.eq_ignore_ascii_case(":END:") {
                in_drawer = false;
            }
            continue;
        }

        // Skip a top-of-file drawer (`:PROPERTIES:`, `:LOGBOOK:`, etc.) so the
        // following `#+TITLE:` / `#+DATE:` / `#+FILETAGS:` lines are still
        // discovered.
        if trimmed.starts_with(':')
            && trimmed.ends_with(':')
            && trimmed.len() > 2
            && !trimmed.eq_ignore_ascii_case(":END:")
        {
            let inner = &trimmed[1..trimmed.len() - 1];
            if inner.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                in_drawer = true;
                continue;
            }
        }

        if let Some(rest) = trimmed.strip_prefix("#+") {
            if let Some((key, value)) = rest.split_once(':') {
                let key_lower = key.trim().to_lowercase();
                let value = value.trim();
                match key_lower.as_str() {
                    "title" => title = Some(value.to_string()),
                    "date" => date = Some(value.to_string()),
                    "filetags" => {
                        filetags = value
                            .trim_matches(':')
                            .split(':')
                            .filter(|t| !t.is_empty())
                            .map(|t| t.to_string())
                            .collect();
                    }
                    _ => {}
                }
            }
        } else {
            break;
        }
    }

    (title, date, filetags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_only() {
        let c = "#+TITLE: Hello\n#+DATE: 2025-01-01\n#+FILETAGS: :a:b:\n";
        let (title, date, tags) = extract_metadata_fast(c);
        assert_eq!(title.as_deref(), Some("Hello"));
        assert_eq!(date.as_deref(), Some("2025-01-01"));
        assert_eq!(tags, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn skips_leading_properties_drawer() {
        let c = ":PROPERTIES:\n:AUTHOR: Alice\n:STATUS: ready\n:END:\n#+TITLE: Hello\n#+DATE: 2025-01-01\n";
        let (title, date, _) = extract_metadata_fast(c);
        assert_eq!(title.as_deref(), Some("Hello"));
        assert_eq!(date.as_deref(), Some("2025-01-01"));
    }

    #[test]
    fn keywords_before_drawer_still_work() {
        let c = "#+TITLE: x\n:PROPERTIES:\n:K: v\n:END:\n#+DATE: 2025\n";
        let (title, date, _) = extract_metadata_fast(c);
        assert_eq!(title.as_deref(), Some("x"));
        assert_eq!(date.as_deref(), Some("2025"));
    }

    #[test]
    fn stops_at_body() {
        let c = "#+TITLE: x\nbody starts here\n#+DATE: ignored\n";
        let (title, date, _) = extract_metadata_fast(c);
        assert_eq!(title.as_deref(), Some("x"));
        assert_eq!(date, None);
    }
}
