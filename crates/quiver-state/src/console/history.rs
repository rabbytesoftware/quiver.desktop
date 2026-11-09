//! Shell-style command history: Up walks back through what was run, Down walks forward and finally
//! returns to whatever was being typed.

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct HistoryCursor {
    /// Position in the history being shown, or `None` when the user's own draft is.
    pub index: Option<usize>,
    /// What the user had typed before they first pressed Up.
    pub draft: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Up,
    Down,
}

/// Adds a line to the history: not blank, and not a repeat of the most recent one.
pub fn push_history(history: &[String], line: &str, cap: usize) -> Vec<String> {
    let trimmed = line.trim();
    let mut next = history.to_vec();
    if trimmed.is_empty() || history.last().map(String::as_str) == Some(trimmed) {
        return next;
    }
    next.push(trimmed.to_owned());
    if next.len() > cap {
        next.drain(..next.len() - cap);
    }
    next
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HistoryStep {
    pub cursor: HistoryCursor,
    /// The text the input should now show.
    pub text: String,
}

pub fn step_history(
    history: &[String],
    cursor: &HistoryCursor,
    current: &str,
    direction: Direction,
) -> HistoryStep {
    let stay = || HistoryStep {
        cursor: cursor.clone(),
        text: current.to_owned(),
    };
    if history.is_empty() {
        return stay();
    }
    match (direction, cursor.index) {
        (Direction::Up, at) => {
            let index = at.map_or(history.len() - 1, |i| i.saturating_sub(1));
            let draft = if at.is_none() {
                current.to_owned()
            } else {
                cursor.draft.clone()
            };
            HistoryStep {
                cursor: HistoryCursor {
                    index: Some(index),
                    draft,
                },
                text: history[index].clone(),
            }
        }
        (Direction::Down, None) => stay(),
        (Direction::Down, Some(i)) if i >= history.len() - 1 => HistoryStep {
            cursor: HistoryCursor::default(),
            text: cursor.draft.clone(),
        },
        (Direction::Down, Some(i)) => HistoryStep {
            cursor: HistoryCursor {
                index: Some(i + 1),
                draft: cursor.draft.clone(),
            },
            text: history[i + 1].clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hist(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn blank_lines_and_immediate_repeats_are_not_recorded_and_lines_are_trimmed() {
        let h = push_history(&[], "  arrow list ", 10);
        assert_eq!(h, ["arrow list"]);
        assert_eq!(push_history(&h, "arrow list", 10), h);
        assert_eq!(push_history(&h, "   ", 10), h);
        assert_eq!(push_history(&hist(&["a", "b"]), "a", 10), ["a", "b", "a"]);
    }

    #[test]
    fn the_history_keeps_only_the_newest_entries() {
        assert_eq!(push_history(&hist(&["a", "b"]), "c", 2), ["b", "c"]);
    }

    #[test]
    fn up_walks_back_remembering_the_draft_and_stops_at_the_oldest() {
        let h = hist(&["a", "b", "c"]);
        let s1 = step_history(&h, &HistoryCursor::default(), "typing", Direction::Up);
        assert_eq!(
            (s1.text.as_str(), s1.cursor.index, s1.cursor.draft.as_str()),
            ("c", Some(2), "typing")
        );
        let s2 = step_history(&h, &s1.cursor, "c", Direction::Up);
        assert_eq!(s2.text, "b");
        let s3 = step_history(
            &h,
            &HistoryCursor {
                index: Some(0),
                draft: "typing".into(),
            },
            "a",
            Direction::Up,
        );
        assert_eq!((s3.text.as_str(), s3.cursor.index), ("a", Some(0)));
    }

    #[test]
    fn down_walks_forward_and_finally_returns_to_the_draft() {
        let h = hist(&["a", "b"]);
        let at_a = HistoryCursor {
            index: Some(0),
            draft: "typing".into(),
        };
        let s1 = step_history(&h, &at_a, "a", Direction::Down);
        assert_eq!((s1.text.as_str(), s1.cursor.index), ("b", Some(1)));
        let s2 = step_history(&h, &s1.cursor, "b", Direction::Down);
        assert_eq!(
            (s2.text.as_str(), s2.cursor),
            ("typing", HistoryCursor::default())
        );
    }

    #[test]
    fn with_nothing_to_walk_or_no_cursor_the_input_is_left_alone() {
        let s = step_history(&[], &HistoryCursor::default(), "x", Direction::Up);
        assert_eq!((s.text.as_str(), s.cursor), ("x", HistoryCursor::default()));
        let s = step_history(
            &hist(&["a"]),
            &HistoryCursor::default(),
            "x",
            Direction::Down,
        );
        assert_eq!(s.text, "x");
    }
}
