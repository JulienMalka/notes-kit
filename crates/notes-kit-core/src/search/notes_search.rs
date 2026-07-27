use crate::models::{Note, SearchResult};

pub fn search_notes(notes: &[Note], query: &str) -> Vec<SearchResult> {
    if query.trim().len() < 2 {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();

    let mut results: Vec<(bool, SearchResult)> = notes
        .iter()
        .filter_map(|note| {
            let title_match = note
                .metadata
                .title
                .as_ref()
                .is_some_and(|t| t.to_lowercase().contains(&query_lower));
            // Body when present; precomputed excerpt as the degraded
            // pre-backfill fallback so search stays useful on summaries.
            let content_match = note.content_contains_lowercase(&query_lower)
                || (note.content.is_none()
                    && note
                        .excerpt
                        .as_ref()
                        .is_some_and(|e| e.to_lowercase().contains(&query_lower)));

            if !title_match && !content_match {
                return None;
            }

            let snippet = if note.content.is_some() {
                note.snippet_around(query)
            } else {
                note.excerpt.clone().unwrap_or_default()
            };

            Some((title_match, SearchResult {
                path: note.path.clone(),
                title: note.metadata.title.clone(),
                snippet,
            }))
        })
        .collect();

    results.sort_by(|a, b| {
        match (a.0, b.0) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.1.path.cmp(&a.1.path),
        }
    });

    results.truncate(20);
    results.into_iter().map(|(_, r)| r).collect()
}
