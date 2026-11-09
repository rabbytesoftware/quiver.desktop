//! Notes for an arrow whose manifest Fletcher inferred from release downloads.

use crate::i18n::{en::arrow, Key};

/// The message for a generator warning code; `None` for a code this build does not know.
pub fn warning_key(code: &str) -> Option<Key> {
    Some(match code {
        "assumed_arch" => arrow::INFERRED_WARNING_ASSUMED_ARCH,
        "emulated" => arrow::INFERRED_WARNING_EMULATED,
        "windows_exe_unverified" => arrow::INFERRED_WARNING_WINDOWS_EXE_UNVERIFIED,
        "name_mismatch" => arrow::INFERRED_WARNING_NAME_MISMATCH,
        "unpinned_rolling_tag" => arrow::INFERRED_WARNING_UNPINNED_ROLLING_TAG,
        _ => return None,
    })
}

pub fn confidence_key(confidence: &str) -> Option<Key> {
    Some(match confidence {
        "high" => arrow::INFERRED_CONFIDENCE_HIGH,
        "medium" => arrow::INFERRED_CONFIDENCE_MEDIUM,
        "low" => arrow::INFERRED_CONFIDENCE_LOW,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes_map_and_unknown_ones_do_not() {
        assert!(warning_key("assumed_arch").is_some());
        assert!(warning_key("toString").is_none());
        assert!(confidence_key("medium").is_some());
        assert!(confidence_key("certain").is_none());
    }
}
