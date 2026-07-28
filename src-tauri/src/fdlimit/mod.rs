//! The desktop process's open-file limit.
//!
//! macOS starts a GUI app at launchd's soft limit of 256 open files. Go raises
//! its own soft limit to the hard limit at startup — which is why quiver.core
//! runs at thousands of descriptors without complaint — but Rust does no such
//! thing, so the app is left at 256 unless it asks for more.
//!
//! That is a real ceiling here, not a theoretical one. Every call the frontend
//! makes reaches quiver.core as a unix-socket connection owned by *this*
//! process (`connection::local::transport`), and the two event streams in
//! `connection::ws` hold a socket each for the life of the app, redialling
//! every 500ms whenever the daemon is not up yet. So the app's descriptor use
//! tracks the frontend's traffic and the daemon's availability, not its own
//! modest needs. Once the ceiling is reached, `connect()` fails with `EMFILE`,
//! which `HttpClient::health` cannot tell apart from a daemon that never
//! started — so the app reports the backend as down over an entirely local
//! fault.
//!
//! The decision logic lives here and is tested through [`raise_with`]. The
//! syscalls it drives live in [`sys`], which is excluded from coverage because
//! its failure paths need the kernel to actually refuse something.

#[cfg(unix)]
mod sys;

/// A process's open-file limits, as reported by `getrlimit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
	pub soft: u64,
	pub hard: u64,
}

/// The soft limit to move to, or `None` when the current one is already at
/// least as generous.
///
/// A kernel refuses a soft limit above its per-process ceiling, and macOS
/// reports the hard limit for `RLIMIT_NOFILE` as "unlimited" — so on macOS that
/// ceiling, not the hard limit, is what can actually be claimed. Where a kernel
/// imposes no such ceiling, `max_per_proc` is `u64::MAX` and the hard limit
/// binds.
pub fn target_soft_limit(soft: u64, hard: u64, max_per_proc: u64) -> Option<u64> {
	let target = hard.min(max_per_proc);
	(target > soft).then_some(target)
}

/// What [`raise`] did, so it can be reported once a logger exists. The raise
/// has to happen before anything opens a descriptor — which is before Tauri
/// installs its log plugin — so the outcome is carried out rather than logged
/// in place. Leaving it unreported would hide the one number that explains an
/// `EMFILE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
	Raised { from: u64, to: u64 },
	AlreadyGenerous(u64),
	Failed(String),
}

impl std::fmt::Display for Outcome {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Raised { from, to } => {
				write!(f, "raised the open-file soft limit from {from} to {to}")
			}
			Self::AlreadyGenerous(soft) => {
				write!(f, "open-file soft limit is already {soft}")
			}
			Self::Failed(why) => {
				write!(f, "could not raise the open-file limit: {why}")
			}
		}
	}
}

/// The decision [`raise`] makes, with the syscalls passed in so every branch —
/// including the two the kernel almost never takes — is reachable from a test.
pub fn raise_with<G, S>(get: G, set: S, max_per_proc: u64) -> Outcome
where
	G: FnOnce() -> Result<Limits, String>,
	S: FnOnce(u64) -> Result<(), String>,
{
	let limits = match get() {
		Ok(l) => l,
		Err(why) => return Outcome::Failed(format!("reading it failed: {why}")),
	};

	let Some(target) = target_soft_limit(limits.soft, limits.hard, max_per_proc) else {
		return Outcome::AlreadyGenerous(limits.soft);
	};

	match set(target) {
		Ok(()) => Outcome::Raised {
			from: limits.soft,
			to: target,
		},
		Err(why) => Outcome::Failed(format!("{} -> {target} refused: {why}", limits.soft)),
	}
}

/// Raises this process's open-file soft limit as far as the kernel allows, so
/// the app's descriptor budget reflects the traffic it proxies rather than
/// launchd's 256. Best-effort: a failure is reported, never fatal — the app
/// still runs, it is just back on the old ceiling.
#[cfg(unix)]
pub fn raise() -> Outcome {
	raise_with(sys::get_rlimit, sys::set_rlimit, sys::max_files_per_proc())
}

#[cfg(not(unix))]
pub fn raise() -> Outcome {
	Outcome::AlreadyGenerous(0)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The case the app actually ships in: launchd's 256, an "unlimited" hard
	/// limit, and a kernel that will not in fact go past kern.maxfilesperproc.
	#[test]
	fn claims_the_per_process_ceiling_when_the_hard_limit_is_unlimited() {
		assert_eq!(
			target_soft_limit(256, u64::MAX, 61_440),
			Some(61_440),
			"a GUI app inherits 256 and must climb to the kernel's real ceiling"
		);
	}

	#[test]
	fn never_asks_for_more_than_the_hard_limit() {
		assert_eq!(
			target_soft_limit(256, 1024, 61_440),
			Some(1024),
			"a hard limit below the kernel's ceiling is the binding one"
		);
	}

	#[test]
	fn leaves_an_already_generous_limit_alone() {
		assert_eq!(target_soft_limit(61_440, u64::MAX, 61_440), None);
		assert_eq!(
			target_soft_limit(70_000, u64::MAX, 61_440),
			None,
			"a soft limit above the ceiling is someone else's deliberate choice"
		);
	}

	/// Every arm gets rendered somewhere in a real run — `raise()` returns one
	/// of the three and lib.rs logs it — so a broken arm would only ever show
	/// up as a confusing log line at startup, which is the moment it is least
	/// likely to be noticed.
	#[test]
	fn every_outcome_renders_its_numbers() {
		assert_eq!(
			Outcome::Raised {
				from: 256,
				to: 61_440
			}
			.to_string(),
			"raised the open-file soft limit from 256 to 61440"
		);
		assert_eq!(
			Outcome::AlreadyGenerous(61_440).to_string(),
			"open-file soft limit is already 61440"
		);
		assert_eq!(
			Outcome::Failed("nope".into()).to_string(),
			"could not raise the open-file limit: nope"
		);
	}

	#[test]
	fn raises_the_soft_limit_to_the_binding_ceiling() {
		let outcome = raise_with(
			|| {
				Ok(Limits {
					soft: 256,
					hard: u64::MAX,
				})
			},
			|target| {
				assert_eq!(
					target, 61_440,
					"must claim the ceiling, not the hard limit"
				);
				Ok(())
			},
			61_440,
		);
		assert_eq!(
			outcome,
			Outcome::Raised {
				from: 256,
				to: 61_440
			}
		);
	}

	#[test]
	fn reports_an_already_generous_limit_without_setting_anything() {
		let outcome = raise_with(
			|| {
				Ok(Limits {
					soft: 61_440,
					hard: u64::MAX,
				})
			},
			|_| panic!("must not call setrlimit when nothing needs raising"),
			61_440,
		);
		assert_eq!(outcome, Outcome::AlreadyGenerous(61_440));
	}

	/// The branch that made this worth extracting: if the app cannot even read
	/// its own limit, the reason has to survive into the startup log rather
	/// than being swallowed.
	#[test]
	fn surfaces_a_failure_to_read_the_current_limit() {
		let outcome = raise_with(
			|| Err("EPERM".into()),
			|_| panic!("must not set a limit it failed to read"),
			61_440,
		);
		assert_eq!(outcome, Outcome::Failed("reading it failed: EPERM".into()));
	}

	#[test]
	fn surfaces_a_refused_raise_with_both_numbers() {
		let outcome = raise_with(
			|| {
				Ok(Limits {
					soft: 256,
					hard: 1024,
				})
			},
			|_| Err("EINVAL".into()),
			61_440,
		);
		assert_eq!(
			outcome,
			Outcome::Failed("256 -> 1024 refused: EINVAL".into()),
			"the message has to name the limit it tried to claim"
		);
	}

	/// The real syscalls, end to end. The point is that a Rust binary does not
	/// get this for free the way quiver.core does — run under `ulimit -Sn 256`
	/// to exercise the launchd condition itself.
	#[cfg(unix)]
	#[test]
	fn raise_lifts_this_process_clear_of_the_default_ceiling() {
		let outcome = raise();
		eprintln!("fdlimit: {outcome}");
		assert!(
			!matches!(outcome, Outcome::Failed(_)),
			"the raise must not fail: {outcome}"
		);

		let limits = sys::get_rlimit().expect("reading the limit back must work");
		assert!(
			limits.soft > 256,
			"soft limit is still {}; the app would hit EMFILE under ordinary \
			 use and would report quiver.core as unreachable because of it",
			limits.soft
		);
	}
}
