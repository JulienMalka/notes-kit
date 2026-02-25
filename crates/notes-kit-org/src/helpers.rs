pub fn extract_metadata_fast(
    content: &str,
) -> (Option<String>, Option<String>, Vec<String>) {
    let mut title = None;
    let mut date = None;
    let mut filetags = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
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
