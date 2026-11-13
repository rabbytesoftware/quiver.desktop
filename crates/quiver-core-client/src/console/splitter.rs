//! Splitting a streamed response body into NDJSON lines.

/// Longest single NDJSON line accepted. A frame is a line of output wrapped in
/// a little JSON, so this is generous; anything longer is dropped with a marker
/// rather than buffered without bound.
pub(super) const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Splits a byte stream into NDJSON lines.
///
/// Chunk boundaries are arbitrary, so a line (or a multi-byte character) may
/// straddle two chunks; the splitter holds the partial tail until its newline
/// arrives. A line over [`MAX_FRAME_BYTES`] is discarded up to its newline and
/// replaced by a marker frame, so memory stays bounded whatever the peer sends.
#[derive(Default)]
pub struct NdjsonSplitter {
    pub(super) tail: Vec<u8>,
    discarding: bool,
}

/// The frame substituted for a line that was too long to keep.
fn oversize_marker() -> String {
    serde_json::json!({
        "type": "out",
        "stream": "stderr",
        "data": "[quiver: a line of output was too long to display and was dropped]\n",
    })
    .to_string()
}

impl NdjsonSplitter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one chunk; returns the lines it completed, without their newline.
    /// Blank lines are skipped, and invalid UTF-8 is replaced rather than
    /// rejected: the frontend treats a non-JSON line as raw text.
    pub fn push(
        &mut self,
        chunk: &[u8],
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let mut rest = chunk;
        while !rest.is_empty() {
            match rest.iter().position(|b| *b == b'\n') {
                Some(i) => {
                    let (head, tail) = rest.split_at(i);
                    rest = &tail[1..];
                    if self.discarding {
                        self.discarding = false;
                        self.tail.clear();
                        lines.push(oversize_marker());
                        continue;
                    }
                    self.tail.extend_from_slice(head);
                    if self.tail.len() > MAX_FRAME_BYTES {
                        self.tail.clear();
                        lines.push(oversize_marker());
                        continue;
                    }
                    if let Some(line) = Self::take(&mut self.tail) {
                        lines.push(line);
                    }
                }
                None => {
                    if !self.discarding {
                        self.tail.extend_from_slice(rest);
                        if self.tail.len() > MAX_FRAME_BYTES {
                            self.tail.clear();
                            self.discarding = true;
                        }
                    }
                    break;
                }
            }
        }
        lines
    }

    /// The unterminated remainder once the stream has ended, if any. A daemon
    /// that exits mid-line leaves one, and dropping it would hide the last
    /// words of the command.
    pub fn finish(&mut self) -> Option<String> {
        if self.discarding {
            self.discarding = false;
            self.tail.clear();
            return Some(oversize_marker());
        }
        Self::take(&mut self.tail)
    }

    fn take(buf: &mut Vec<u8>) -> Option<String> {
        let line = String::from_utf8_lossy(buf)
            .trim_end_matches('\r')
            .to_owned();
        buf.clear();
        (!line.trim().is_empty()).then_some(line)
    }
}
