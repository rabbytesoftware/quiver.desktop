//! Remote connections: rows, the Add-connection form, the pairing code, overlays and toasts
//! (port of `src/features/remote`). The mutations themselves are the data layer's.

mod add_form;
mod filter;
mod outcome;
mod overlays;
mod pairing_code;
mod rows;

pub use add_form::{AddConnectionForm, AddStage, AddSubmission};
pub use filter::{clamp_index, filter_connections};
pub use outcome::{toast_for, ConnectionOutcome, Toast};
pub use overlays::{RemoteOverlays, ToastStack, TOAST_DURATION_MS};
pub use pairing_code::{PairingCode, PAIRING_CODE_LENGTH};
pub use rows::{connection_rows, ConnectionRowView};
