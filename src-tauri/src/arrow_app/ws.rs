//! Scoping the WebSocket commands arrow pages use (through the shell).
//!
//! The generic `ws_open` takes any daemon path and the main window may call
//! it. Arrow sockets must only ever reach their own surface, so they use these
//! helpers: the path is built from the registered host, never from the page.

use super::hosts::ArrowHosts;
use super::uri::{daemon_path, is_safe_ws_path};

/// The daemon path an arrow's WebSocket may open: always `/v0/ui/<its ns>/...`.
pub fn ws_target(hosts: &ArrowHosts, host: &str, path: &str) -> Result<String, String> {
	let namespace = hosts
		.namespace_for(host)
		.ok_or_else(|| format!("unknown arrow host {host}"))?;
	if !is_safe_ws_path(path) {
		return Err("unsafe websocket path".to_string());
	}
	Ok(daemon_path(&namespace, path))
}

/// Bridge key for a connection, namespaced by host so ids can not collide.
pub fn ws_key(host: &str, conn_id: &str) -> String {
	format!("arrow:{host}:{conn_id}")
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The shell passes the host of the iframe a message came from. A host
	/// nobody registered must not turn into a daemon path.
	#[test]
	fn target_requires_a_registered_host_and_a_safe_path() {
		let hosts = ArrowHosts::new();
		let host = hosts.register("github.com/user/chat");

		assert_eq!(
			ws_target(&hosts, &host, "/ws?username=bob").unwrap(),
			"/v0/ui/github.com%2Fuser%2Fchat/ws?username=bob"
		);
		assert!(ws_target(&hosts, "deadbeef", "/ws").is_err());
		for bad in ["/../v0/health", "//evil", "/a%2e%2e/b", "ws"] {
			assert!(ws_target(&hosts, &host, bad).is_err(), "{bad}");
		}
	}

	/// Two arrows using the same client-chosen id must not collide.
	#[test]
	fn connection_keys_are_scoped_by_host() {
		assert_ne!(ws_key("aaaa", "1"), ws_key("bbbb", "1"));
		assert_eq!(ws_key("aaaa", "1"), ws_key("aaaa", "1"));
	}
}
