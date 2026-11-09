//! Platform keys (`os/arch`) in words.

use super::header::PlatformBadge;
use crate::i18n::en::design;

/// "darwin/arm64" -> "Mac" (for "Not available for this Mac").
pub fn os_label(platform: &str) -> &'static str {
    match platform.split('/').next().unwrap_or("") {
        "darwin" => "Mac",
        "windows" => "PC",
        _ => "computer",
    }
}

/// "darwin/arm64" -> "macOS" (for the "Runs on" line).
pub fn os_family(platform: &str) -> Option<&'static str> {
    match platform.split('/').next()? {
        "darwin" => Some("macOS"),
        "linux" => Some("Linux"),
        "windows" => Some("Windows"),
        _ => None,
    }
}

/// "Linux · macOS · Windows": the platforms the arrow publishes a build for, deduplicated by OS family
/// (the architectures collapse), alphabetical.
pub fn runs_on(platforms: &[String]) -> String {
    let mut families: Vec<&str> = platforms.iter().filter_map(|p| os_family(p)).collect();
    families.sort_unstable();
    families.dedup();
    families.join(" · ")
}

pub(super) fn platform_supported_notice(platform: &str) -> PlatformBadge {
    PlatformBadge {
        label: design::PLATFORM_UNAVAILABLE_HERE,
        os: os_label(platform),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_os_in_words() {
        assert_eq!(os_label("darwin/arm64"), "Mac");
        assert_eq!(os_label("windows/amd64"), "PC");
        assert_eq!(os_label("linux/amd64"), "computer");
    }

    #[test]
    fn runs_on_collapses_architectures_and_sorts() {
        let platforms = [
            "windows/amd64",
            "linux/amd64",
            "linux/arm64",
            "freebsd/amd64",
        ]
        .map(String::from);
        assert_eq!(runs_on(&platforms), "Linux · Windows");
        assert_eq!(runs_on(&[]), "");
    }
}
