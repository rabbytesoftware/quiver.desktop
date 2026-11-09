//! Tiny flag parser for the xtask subcommands (`--flag value`, `--switch`, positionals).

use anyhow::{bail, Result};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Args {
    flags: HashMap<String, String>,
    switches: Vec<String>,
    pub positional: Vec<String>,
}

impl Args {
    /// `valued` lists the flags that take a value; every other `--name` is a switch.
    pub fn parse(raw: impl IntoIterator<Item = String>, valued: &[&str]) -> Result<Self> {
        let mut out = Self::default();
        let mut raw = raw.into_iter();
        while let Some(arg) = raw.next() {
            let Some(name) = arg.strip_prefix("--") else {
                out.positional.push(arg);
                continue;
            };
            if valued.contains(&name) {
                let Some(value) = raw.next() else {
                    bail!("--{name} needs a value")
                };
                out.flags.insert(name.to_string(), value);
            } else {
                out.switches.push(name.to_string());
            }
        }
        Ok(out)
    }

    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    pub fn has(&self, switch: &str) -> bool {
        self.switches.iter().any(|s| s == switch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(list: &[&str]) -> Result<Args> {
        Args::parse(list.iter().map(|s| s.to_string()), &["target", "out"])
    }

    #[test]
    fn splits_flags_switches_and_positionals() {
        let a = parse(&["macos", "--target", "x", "--universal", "extra"]).unwrap();
        assert_eq!(a.flag("target"), Some("x"));
        assert!(a.has("universal"));
        assert_eq!(a.positional, ["macos", "extra"]);
    }

    #[test]
    fn a_valued_flag_without_value_is_an_error() {
        assert!(parse(&["--out"]).is_err());
    }
}
