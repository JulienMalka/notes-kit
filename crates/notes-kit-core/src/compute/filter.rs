use crate::models::Note;

pub fn filter_by_type(notes: &[Note], note_type: &str) -> Vec<Note> {
    notes
        .iter()
        .filter(|n| n.metadata.note_type.as_deref() == Some(note_type))
        .cloned()
        .collect()
}

pub fn group_by_year(
    notes: Vec<Note>,
    extract_year: impl Fn(&str) -> String,
) -> Vec<(String, Vec<Note>)> {
    let mut groups: Vec<(String, Vec<Note>)> = Vec::new();
    for note in notes {
        let year = note
            .metadata
            .date
            .as_deref()
            .map(&extract_year)
            .unwrap_or_else(|| "Unknown".to_string());
        if let Some(last) = groups.last_mut() {
            if last.0 == year {
                last.1.push(note);
                continue;
            }
        }
        groups.push((year, vec![note]));
    }
    groups
}
