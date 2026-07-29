//! The raw `getrlimit`/`setrlimit`/`sysctl` calls behind [`super::raise`].
//!
//! Split out from the decision logic on purpose: nothing in here can be
//! exercised without the kernel actually refusing something, so this file is
//! listed in the tarpaulin exclusions alongside the other modules that need
//! live OS state. Everything worth asserting on lives in `super` and is tested
//! there through [`super::raise_with`], which takes these as parameters.
//!
//! Keep this file thin. A line added here is a line nothing can cover.

use super::Limits;

/// Reads the process's current open-file limits.
#[cfg(unix)]
pub fn get_rlimit() -> Result<Limits, String> {
	let mut lim = libc::rlimit {
		rlim_cur: 0,
		rlim_max: 0,
	};
	// SAFETY: `lim` is a valid, fully-initialised out-param.
	if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) } != 0 {
		return Err(std::io::Error::last_os_error().to_string());
	}
	// No cast: libc::rlim_t is u64 on every unix target this builds for, and
	// clippy rejects the redundant one under -D warnings.
	Ok(Limits {
		soft: lim.rlim_cur,
		hard: lim.rlim_max,
	})
}

/// Moves the soft limit to `target`, leaving the hard limit alone.
#[cfg(unix)]
pub fn set_rlimit(target: u64) -> Result<(), String> {
	let mut lim = libc::rlimit {
		rlim_cur: 0,
		rlim_max: 0,
	};
	// SAFETY: `lim` is a valid, fully-initialised out-param.
	if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) } != 0 {
		return Err(std::io::Error::last_os_error().to_string());
	}
	lim.rlim_cur = target as libc::rlim_t;
	// SAFETY: `lim` came from getrlimit; only its soft field changed, to a
	// value the caller has already clamped to the hard limit and the
	// per-process ceiling.
	if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &lim) } != 0 {
		return Err(std::io::Error::last_os_error().to_string());
	}
	Ok(())
}

/// The kernel's per-process descriptor ceiling, or `u64::MAX` where there is
/// none beyond the hard limit.
///
/// macOS reports an "unlimited" hard limit for `RLIMIT_NOFILE` but will not in
/// fact grant more than `kern.maxfilesperproc`, so on macOS that sysctl is the
/// number that actually binds.
#[cfg(target_os = "macos")]
pub fn max_files_per_proc() -> u64 {
	/// Fallback when the sysctl cannot be read; the stock macOS value.
	const DEFAULT: u64 = 61_440;

	let name = c"kern.maxfilesperproc";
	let mut value: libc::c_int = 0;
	let mut size = std::mem::size_of::<libc::c_int>();
	// SAFETY: `name` is NUL-terminated and `value`/`size` are a matching
	// out-param pair for an integer sysctl.
	let rc = unsafe {
		libc::sysctlbyname(
			name.as_ptr(),
			std::ptr::addr_of_mut!(value).cast::<libc::c_void>(),
			&mut size,
			std::ptr::null_mut(),
			0,
		)
	};
	if rc == 0 && value > 0 {
		value as u64
	} else {
		DEFAULT
	}
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn max_files_per_proc() -> u64 {
	u64::MAX
}
