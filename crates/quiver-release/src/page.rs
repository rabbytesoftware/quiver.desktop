//! Reading GitHub's `expanded_assets` HTML fragment.

use crate::asset::ReleaseAsset;
use crate::{Error, Result};

/// Undoes the `%XX` escapes of a URL path segment.
pub(crate) fn percent_decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%' && i + 2 < bytes.len())
            .then(|| {
                segment
                    .get(i + 1..i + 3)
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            })
            .flatten();
        match escaped {
            Some(byte) => {
                out.push(byte);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The `sha256:<hex>` closest to the end of `text`, if any.
fn last_digest(text: &str) -> Option<String> {
    let mut found = None;
    let mut from = 0;
    while let Some(at) = text[from..].find("sha256:") {
        let start = from + at;
        let hex = text.get(start + 7..start + 71);
        if hex.is_some_and(|h| h.bytes().all(|b| b.is_ascii_hexdigit())) {
            found = Some(text[start..start + 71].to_string());
        }
        from = start + 7;
    }
    found
}

/// The assets an `expanded_assets` page lists, each with the digest shown
/// beside it. Source archives are not assets.
pub(crate) fn parse_assets(page: &str, origin: &str) -> Result<Vec<ReleaseAsset>> {
    if !page.contains("<ul") {
        return Err(Error::Malformed("not a release assets page".into()));
    }
    let mut items: Vec<&str> = Vec::new();
    let mut rest = page;
    while let Some(at) = rest.find("<li") {
        rest = &rest[at + 3..];
        let opens = rest.starts_with('>') || rest.starts_with(char::is_whitespace);
        let end = rest.find("<li").unwrap_or(rest.len());
        if opens {
            items.push(&rest[..end]);
        }
    }

    let mut assets = Vec::new();
    for item in items {
        let Some(href) = item
            .split_once("href=\"")
            .and_then(|(_, after)| after.split_once('"'))
            .map(|(href, _)| href)
        else {
            continue;
        };
        if href.contains("/archive/") {
            continue;
        }
        let name = percent_decode(href.rsplit('/').next().unwrap_or(href));
        let url = if href.starts_with('/') {
            format!("{origin}{href}")
        } else {
            href.to_string()
        };
        assets.push(ReleaseAsset {
            name,
            browser_download_url: url,
            digest: last_digest(item),
        });
    }
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUM_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SUM_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const REPO_PATH: &str = "/rabbytesoftware/quiver.desktop";

    #[test]
    fn source_archives_are_not_assets_and_names_are_unescaped() {
        let html = format!(
            "<div><ul><li class=\"Box-row\"><a href=\"{REPO_PATH}/releases/download/t/Quiver%20App.dmg\"><span>x</span></a><span>sha256:{SUM_A}</span></li>\
             <li><a href=\"{REPO_PATH}/archive/refs/tags/t.zip\">Source code (zip)</a></li></ul></div>"
        );

        let assets = parse_assets(&html, "https://github.com").unwrap();

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "Quiver App.dmg");
        assert_eq!(
            assets[0].digest.as_deref(),
            Some(format!("sha256:{SUM_A}").as_str())
        );
    }

    #[test]
    fn the_digest_nearest_the_end_of_an_item_is_the_asset_s() {
        let text = format!("sha256:{SUM_A} then sha256:{SUM_B}");
        assert_eq!(
            last_digest(&text).as_deref(),
            Some(format!("sha256:{SUM_B}").as_str())
        );
        assert_eq!(last_digest("sha256:short"), None);
    }

    #[test]
    fn a_stray_percent_sign_is_left_alone() {
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a%zzb"), "a%zzb");
        assert_eq!(percent_decode("%é"), "%é");
    }
}
