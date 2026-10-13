//! Inserting the shim script into an arrow's HTML.

const TAG: &[u8] = br#"<script src="/__arrow/shim.js"></script>"#;

/// Inserts the shim `<script>` as early as possible: right after `<head ...>`,
/// else after `<html ...>`, else after the doctype, else at the start. The
/// match is on the whole tag name, so `<header>` is never taken for `<head>`.
pub fn inject_shim(html: &[u8]) -> Vec<u8> {
	let lower: Vec<u8> = html.iter().map(u8::to_ascii_lowercase).collect();
	let at = open_tag_end(&lower, b"<head")
		.or_else(|| open_tag_end(&lower, b"<html"))
		.or_else(|| doctype_end(&lower))
		.unwrap_or(0);

	let mut out = Vec::with_capacity(html.len() + TAG.len());
	out.extend_from_slice(&html[..at]);
	out.extend_from_slice(TAG);
	out.extend_from_slice(&html[at..]);
	out
}

/// Index just past the `>` of the first `name` tag whose name ends there.
fn open_tag_end(lower: &[u8], name: &[u8]) -> Option<usize> {
	let mut from = 0;
	while let Some(rel) = find(&lower[from..], name) {
		let start = from + rel;
		let after = start + name.len();
		match lower.get(after) {
			Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\n')
			| Some(b'\r') => {
				let close = lower[after..].iter().position(|&b| b == b'>')?;
				return Some(after + close + 1);
			}
			_ => from = after,
		}
	}
	None
}

fn doctype_end(lower: &[u8]) -> Option<usize> {
	let start = find(lower, b"<!doctype")?;
	let close = lower[start..].iter().position(|&b| b == b'>')?;
	Some(start + close + 1)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn s(b: Vec<u8>) -> String {
		String::from_utf8(b).unwrap()
	}

	#[test]
	fn injects_right_after_head_open_tag() {
		let out = s(inject_shim(
			b"<!doctype html><html><head><title>x</title></head><body></body></html>",
		));
		assert!(out.contains("<head><script src=\"/__arrow/shim.js\"></script><title>"));
	}

	#[test]
	fn head_match_is_case_insensitive_and_keeps_attributes() {
		let out = s(inject_shim(b"<HTML><HEAD lang=\"en\"><meta></HEAD>"));
		assert!(out.contains(
			"<HEAD lang=\"en\"><script src=\"/__arrow/shim.js\"></script><meta>"
		));
	}

	/// `<header>` starts with `<head` but is body content.
	#[test]
	fn a_header_element_is_not_a_head() {
		let out = s(inject_shim(b"<html><body><header>h</header></body></html>"));
		assert!(
			out.starts_with("<html><script src=\"/__arrow/shim.js\"></script>"),
			"{out}"
		);
	}

	#[test]
	fn falls_back_to_after_doctype_then_to_the_start() {
		let out = s(inject_shim(b"<!DOCTYPE html>plain"));
		assert!(out.starts_with(
			"<!DOCTYPE html><script src=\"/__arrow/shim.js\"></script>plain"
		));
		let out = s(inject_shim(b"plain"));
		assert!(out.starts_with("<script src=\"/__arrow/shim.js\"></script>plain"));
	}

	#[test]
	fn non_utf8_bytes_survive() {
		let mut html = b"<head>".to_vec();
		html.extend([0xff, 0xfe]);
		let out = inject_shim(&html);
		assert!(out.ends_with(&[0xff, 0xfe]));
	}
}
