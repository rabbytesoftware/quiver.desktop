//! No-op counterparts of the bridge hooks, compiled when `e2e-bridge` is off.

/// Raises a named signal for `wait.signal`. Does nothing in a release build.
#[inline(always)]
pub fn signal(_name: &str) {}

/// Call once per painted root frame. Does nothing in a release build.
#[inline(always)]
pub fn frame_painted() {}
