use orgize::ast::{Drawer, Headline};
use orgize::rowan::ast::AstNode;

pub fn sum_clock_minutes(headline: &Headline) -> u64 {
    let Some(section) = headline.section() else {
        return 0;
    };

    section
        .syntax()
        .children()
        .filter_map(Drawer::cast)
        .filter(|d| d.name().eq_ignore_ascii_case("LOGBOOK"))
        .flat_map(|d| {
            let text = d.syntax().text().to_string();
            text.lines()
                .filter_map(|line| {
                    let after_arrow = line.rsplit("=>").next()?;
                    let trimmed = after_arrow.trim();
                    let (h, m) = trimmed.split_once(':')?;
                    Some(h.trim().parse::<u64>().ok()? * 60 + m.trim().parse::<u64>().ok()?)
                })
                .collect::<Vec<_>>()
        })
        .sum()
}

pub fn is_planning_text(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("CLOSED:")
        || trimmed.starts_with("SCHEDULED:")
        || trimmed.starts_with("DEADLINE:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use orgize::Org;

    #[test]
    fn test_sum_clock_minutes_single() {
        let content = "* DONE Task\n:LOGBOOK:\nCLOCK: [2025-12-17 mer. 11:00]--[2025-12-17 mer. 11:15] =>  0:15\n:END:\n";
        let org = Org::parse(content);
        let hdl = org.first_node::<Headline>().unwrap();
        assert_eq!(sum_clock_minutes(&hdl), 15);
    }

    #[test]
    fn test_sum_clock_minutes_multiple() {
        let content = "* DONE Task\n:LOGBOOK:\nCLOCK: [2025-12-18 jeu. 13:00]--[2025-12-18 jeu. 14:00] =>  1:00\nCLOCK: [2025-12-17 mer. 16:30]--[2025-12-17 mer. 17:15] =>  0:45\n:END:\n";
        let org = Org::parse(content);
        let hdl = org.first_node::<Headline>().unwrap();
        assert_eq!(sum_clock_minutes(&hdl), 105);
    }

    #[test]
    fn test_sum_clock_minutes_no_logbook() {
        let content = "* DONE Task\nSome text.\n";
        let org = Org::parse(content);
        let hdl = org.first_node::<Headline>().unwrap();
        assert_eq!(sum_clock_minutes(&hdl), 0);
    }

    #[test]
    fn test_is_planning_text() {
        assert!(is_planning_text("CLOSED: [2025-12-17 mer. 11:30]"));
        assert!(is_planning_text("SCHEDULED: <2025-12-18>"));
        assert!(is_planning_text("DEADLINE: <2025-12-20>"));
        assert!(!is_planning_text("Some regular paragraph."));
        assert!(!is_planning_text("CLOCK: [2025-12-17]"));
    }
}
