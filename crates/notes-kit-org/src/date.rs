pub fn clean_org_date(date: &str) -> &str {
    let clean = date
        .trim()
        .trim_start_matches(|c| c == '[' || c == '<')
        .trim_end_matches(|c| c == ']' || c == '>');
    clean.split_whitespace().next().unwrap_or(clean)
}

pub fn format_date_iso(date: &str) -> String {
    clean_org_date(date).to_string()
}

pub fn format_date_human(date: &str) -> String {
    let d = clean_org_date(date);
    if d.len() >= 10 && d.as_bytes().get(4) == Some(&b'-') {
        if let (Ok(month), Ok(day)) = (d[5..7].parse::<u32>(), d[8..10].parse::<u32>()) {
            let m = match month {
                1 => "January",
                2 => "February",
                3 => "March",
                4 => "April",
                5 => "May",
                6 => "June",
                7 => "July",
                8 => "August",
                9 => "September",
                10 => "October",
                11 => "November",
                12 => "December",
                _ => return date.to_string(),
            };
            return format!("{m} {day}, {}", &d[0..4]);
        }
    }
    date.to_string()
}

pub fn format_date_month(date: &str) -> String {
    let d = clean_org_date(date);
    if d.len() >= 7 && d.as_bytes().get(4) == Some(&b'-') {
        if let Ok(month) = d[5..7].parse::<u32>() {
            let m = match month {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => return date.to_string(),
            };
            return format!("{m} {}", &d[0..4]);
        }
    }
    date.to_string()
}

pub fn extract_year(date: &str) -> String {
    let d = clean_org_date(date);
    d.get(..4).unwrap_or(d).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_bracketed() {
        assert_eq!(clean_org_date("[2025-01-07 Tue]"), "2025-01-07");
    }

    #[test]
    fn clean_angled() {
        assert_eq!(clean_org_date("<2025-01-07>"), "2025-01-07");
    }

    #[test]
    fn iso() {
        assert_eq!(format_date_iso("[2025-01-07 Tue]"), "2025-01-07");
    }

    #[test]
    fn human() {
        assert_eq!(format_date_human("[2025-01-07 Tue]"), "January 7, 2025");
    }

    #[test]
    fn month() {
        assert_eq!(format_date_month("[2025-01-07]"), "Jan 2025");
    }

    #[test]
    fn year() {
        assert_eq!(extract_year("<2025-03-15>"), "2025");
    }

    #[test]
    fn plain_date() {
        assert_eq!(format_date_human("2025-12-25"), "December 25, 2025");
    }
}
