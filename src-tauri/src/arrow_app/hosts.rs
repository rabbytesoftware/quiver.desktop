//! Which arrow owns which `arrow-app` host.
//!
//! A host is a stable hash of the namespace under `.localhost`, registered by the shell before it
//! creates an iframe. Only registered hosts resolve, so a page can not invent
//! hosts to reach arrows it was not given.

use std::collections::HashMap;
use std::sync::RwLock;

use sha2::{Digest, Sha256};

/// 32 lowercase hex characters: DNS-safe (a label is at most 63) and stable.
pub fn host_label(namespace: &str) -> String {
	let digest = Sha256::digest(namespace.as_bytes());
	digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// The host an arrow's iframe uses: its label under `.localhost`. wry rewrites
/// `arrow-app://<host>/` to `http://arrow-app.<host>/` on Windows, and only a
/// `*.localhost` host keeps that a secure context there.
pub fn arrow_host(namespace: &str) -> String {
	format!("{}.localhost", host_label(namespace))
}

#[derive(Default)]
pub struct ArrowHosts {
	by_host: RwLock<HashMap<String, String>>,
}

impl ArrowHosts {
	pub fn new() -> Self {
		Self::default()
	}

	/// Registers `namespace` and returns its host (see `arrow_host`).
	pub fn register(&self, namespace: &str) -> String {
		let host = arrow_host(namespace);
		self.by_host
			.write()
			.unwrap_or_else(|e| e.into_inner())
			.insert(host.clone(), namespace.to_string());
		host
	}

	pub fn namespace_for(&self, host: &str) -> Option<String> {
		self.by_host
			.read()
			.unwrap_or_else(|e| e.into_inner())
			.get(host)
			.cloned()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A host label must be DNS-safe and stable, or the origin of an arrow
	/// (and with it its storage) would change between launches.
	#[test]
	fn label_is_32_lowercase_hex_and_stable() {
		let a = host_label("github.com/user/chat@v1");
		assert_eq!(a.len(), 32);
		assert!(a
			.bytes()
			.all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
		assert_eq!(a, host_label("github.com/user/chat@v1"));
		assert_ne!(a, host_label("github.com/user/other"));
	}

	#[test]
	fn registered_host_maps_back_and_unknown_does_not() {
		let hosts = ArrowHosts::new();
		let host = hosts.register("github.com/user/chat");
		assert_eq!(
			hosts.namespace_for(&host).as_deref(),
			Some("github.com/user/chat")
		);
		assert_eq!(hosts.namespace_for("deadbeef"), None);
	}

	/// The registered host ends in `.localhost` so that wry's Windows rewrite
	/// (`http://arrow-app.<host>`) lands on a potentially trustworthy origin.
	/// A bare `http://arrow-app.<hex>` is not a secure context, and
	/// `crypto.randomUUID`, `crypto.subtle` and the clipboard would be missing.
	#[test]
	fn registered_host_is_the_label_under_localhost_and_the_bare_label_is_unknown() {
		let hosts = ArrowHosts::new();
		let host = hosts.register("github.com/user/chat");
		let label = host_label("github.com/user/chat");
		assert_eq!(host, format!("{label}.localhost"));
		assert_eq!(arrow_host("github.com/user/chat"), host);
		assert_eq!(hosts.namespace_for(&label), None);
	}

	#[test]
	fn registering_twice_is_idempotent() {
		let hosts = ArrowHosts::new();
		assert_eq!(hosts.register("a/b"), hosts.register("a/b"));
	}
}
