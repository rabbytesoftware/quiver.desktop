//! The daemon's own command table, as `GET /v0/console/commands` reports it.
//!
//! This client has no list of commands of its own: help and Tab completion are both read from here, so
//! the grammar is versioned with the daemon that runs it.

use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConsoleCommand {
    /// `["arrow", "list"]`.
    pub path: Vec<String>,
    pub short: String,
    pub usage: String,
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Reads the response's `data`; anything malformed is skipped, never fatal.
pub fn parse_commands(data: &Value) -> Vec<ConsoleCommand> {
    let Some(list) = data.get("commands").and_then(Value::as_array) else {
        return vec![];
    };
    list.iter()
        .filter(|item| item.is_object())
        .filter_map(|item| {
            let path = strings(item.get("path"));
            if path.is_empty() {
                return None;
            }
            let text = |key: &str| {
                item.get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            let usage = match text("usage") {
                u if u.is_empty() => path.join(" "),
                u => u,
            };
            Some(ConsoleCommand {
                short: text("short"),
                usage,
                path,
            })
        })
        .collect()
}

/// Help text, one line per command, usage padded to a common width.
pub fn render_help(commands: &[ConsoleCommand]) -> Vec<String> {
    let width = commands
        .iter()
        .map(|c| c.usage.chars().count())
        .max()
        .unwrap_or(0);
    commands
        .iter()
        .map(|c| {
            if c.short.is_empty() {
                c.usage.clone()
            } else {
                format!("{:<width$}  {}", c.usage, c.short)
            }
        })
        .collect()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Completion {
    /// The line with the word under the cursor completed as far as is unambiguous.
    pub line: String,
    /// Every command word that matched; more than one means the completion stopped at a fork.
    pub candidates: Vec<String>,
}

fn common_prefix(words: &[&String]) -> String {
    let Some(first) = words.first() else {
        return String::new();
    };
    let mut prefix: String = (*first).clone();
    for word in words {
        while !word.starts_with(&prefix) {
            prefix.pop();
        }
    }
    prefix
}

/// Tab completion over command words only: it completes `ins` to `install` and `arrow li` to
/// `arrow list`. Arguments and flags belong to the daemon, so it stops completing words at the first one
/// that is not part of the command path.
pub fn complete_line(line: &str, commands: &[ConsoleCommand]) -> Completion {
    let ends_in_space = line.chars().last().is_some_and(char::is_whitespace);
    let words: Vec<&str> = line.split_whitespace().collect();
    let completing_last = !ends_in_space && !words.is_empty();
    let (before, typed) = if completing_last {
        (&words[..words.len() - 1], words[words.len() - 1])
    } else {
        (&words[..], "")
    };

    let candidates: BTreeSet<&String> = commands
        .iter()
        .filter(|c| c.path.len() > before.len() && before.iter().zip(&c.path).all(|(w, p)| w == p))
        .map(|c| &c.path[before.len()])
        .filter(|next| next.starts_with(typed))
        .collect();
    let matches: Vec<&String> = candidates.into_iter().collect();
    if matches.is_empty() {
        return Completion {
            line: line.to_owned(),
            candidates: vec![],
        };
    }

    let completed = if matches.len() == 1 {
        format!("{} ", matches[0])
    } else {
        common_prefix(&matches)
    };
    let head = if before.is_empty() {
        String::new()
    } else {
        format!("{} ", before.join(" "))
    };
    Completion {
        line: format!("{head}{completed}"),
        candidates: matches.into_iter().cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cmd(path: &[&str], short: &str) -> ConsoleCommand {
        let path: Vec<String> = path.iter().map(|s| (*s).to_owned()).collect();
        ConsoleCommand {
            usage: path.join(" "),
            short: short.into(),
            path,
        }
    }

    fn table() -> Vec<ConsoleCommand> {
        vec![
            cmd(&["install"], "Install an arrow"),
            cmd(&["init"], ""),
            cmd(&["arrow", "list"], "List arrows"),
            cmd(&["arrow", "lint"], ""),
            cmd(&["arrow", "remove"], ""),
        ]
    }

    #[test]
    fn parsing_skips_what_is_malformed_and_defaults_usage_to_the_path() {
        let parsed = parse_commands(&json!({"commands":[
            {"path":["arrow","list"],"short":"List","usage":"arrow list [--json]"},
            {"path":["stop"]}, {"path":[]}, "junk", {"short":"no path"}
        ]}));
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].usage, "arrow list [--json]");
        assert_eq!(
            (parsed[1].usage.as_str(), parsed[1].short.as_str()),
            ("stop", "")
        );
        assert!(parse_commands(&json!({"commands": 3})).is_empty());
        assert!(parse_commands(&json!(null)).is_empty());
    }

    #[test]
    fn help_pads_usage_to_a_common_width() {
        let help = render_help(&[cmd(&["a"], "short a"), cmd(&["long", "one"], "")]);
        assert_eq!(
            help,
            [
                "a".to_owned() + &" ".repeat(7) + "  short a",
                "long one".to_owned()
            ]
        );
        assert!(render_help(&[]).is_empty());
    }

    #[test]
    fn a_unique_prefix_completes_with_a_trailing_space() {
        let c = complete_line("ins", &table());
        assert_eq!((c.line.as_str(), c.candidates.len()), ("install ", 1));
        assert_eq!(complete_line("arrow rem", &table()).line, "arrow remove ");
    }

    #[test]
    fn a_fork_completes_to_the_common_prefix_and_lists_candidates() {
        let c = complete_line("arrow li", &table());
        assert_eq!(c.line, "arrow li");
        assert_eq!(c.candidates, ["lint", "list"]);
        assert_eq!(complete_line("i", &table()).candidates, ["init", "install"]);
        assert_eq!(complete_line("i", &table()).line, "in");
    }

    #[test]
    fn a_trailing_space_lists_the_next_words_and_unknown_words_stop_completion() {
        assert_eq!(
            complete_line("arrow ", &table()).candidates,
            ["lint", "list", "remove"]
        );
        assert_eq!(
            complete_line("arrow list --j", &table()).line,
            "arrow list --j"
        );
        assert!(complete_line("zzz", &table()).candidates.is_empty());
        assert_eq!(complete_line("", &table()).candidates.len(), 3);
    }
}
