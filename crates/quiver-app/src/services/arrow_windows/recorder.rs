//! What the helper windows asked the host for, kept so the e2e state dump can show it. The web content lives in
//! another OS process: the host-side answer (status, CSP) is the one thing about a page that can be read here.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

use quiver_arrow_host::{ProtocolRequest, ProtocolResponse};
use serde_json::{json, Value};

/// Enough for a test to see every page load, not a log that grows with the session.
const KEEP: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Exchange {
    namespace: String,
    method: String,
    /// Path and query only: the host label is derived from the namespace and says nothing more.
    path: String,
    status: u16,
    csp: Option<String>,
    content_type: Option<String>,
    /// Names of the headers the page sent, so a test can assert the sensitive ones were never part of it.
    request_headers: Vec<String>,
}

fn path_of(uri: &str) -> String {
    let rest = uri.split_once("://").map_or(uri, |(_, rest)| rest);
    rest.find('/')
        .map_or_else(|| "/".to_string(), |at| rest[at..].to_string())
}

#[derive(Default)]
pub(super) struct ProtocolLog {
    entries: Mutex<VecDeque<Exchange>>,
}

impl ProtocolLog {
    pub(super) fn record(
        &self,
        namespace: &str,
        request: &ProtocolRequest,
        response: &ProtocolResponse,
    ) {
        let exchange = Exchange {
            namespace: namespace.to_string(),
            method: request.method.clone(),
            path: path_of(&request.uri),
            status: response.status,
            csp: response
                .headers
                .get("content-security-policy")
                .map(str::to_owned),
            content_type: response.headers.get("content-type").map(str::to_owned),
            request_headers: request
                .headers
                .iter()
                .map(|(name, _)| name.to_ascii_lowercase())
                .collect(),
        };
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if entries.len() == KEEP {
            entries.pop_front();
        }
        entries.push_back(exchange);
    }

    pub(super) fn snapshot(&self) -> Value {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        Value::Array(
            entries
                .iter()
                .map(|e| {
                    json!({
                        "namespace": e.namespace,
                        "method": e.method,
                        "path": e.path,
                        "status": e.status,
                        "csp": e.csp,
                        "contentType": e.content_type,
                        "requestHeaders": e.request_headers,
                    })
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use quiver_arrow_host::Headers;

    use super::*;

    fn exchange(uri: &str, status: u16, with_csp: bool) -> (ProtocolRequest, ProtocolResponse) {
        let request = ProtocolRequest {
            id: 1,
            method: "GET".into(),
            uri: uri.into(),
            headers: [("Accept", "*/*")].into_iter().collect(),
            body: Vec::new(),
        };
        let mut headers = Headers::new();
        if with_csp {
            headers.set("Content-Security-Policy", "default-src 'self'");
        }
        let response = ProtocolResponse {
            id: 1,
            status,
            headers,
            body: Vec::new(),
        };
        (request, response)
    }

    #[test]
    fn records_the_path_the_status_and_the_csp_without_the_host() {
        let log = ProtocolLog::default();
        let (req, resp) = exchange("arrow-app://abc.localhost/a/b?c=1", 200, true);
        log.record("github.com/u/chat", &req, &resp);
        let seen = log.snapshot();
        assert_eq!(seen[0]["path"], "/a/b?c=1");
        assert_eq!(seen[0]["status"], 200);
        assert_eq!(seen[0]["csp"], "default-src 'self'");
        assert_eq!(seen[0]["requestHeaders"][0], "accept");
    }

    #[test]
    fn a_response_without_a_csp_reads_as_null_and_the_oldest_entry_goes_first() {
        let log = ProtocolLog::default();
        for i in 0..=KEEP {
            let (req, resp) = exchange(&format!("http://abc.localhost/{i}"), 404, false);
            log.record("ns", &req, &resp);
        }
        let seen = log.snapshot();
        assert_eq!(seen.as_array().map(Vec::len), Some(KEEP));
        assert_eq!(seen[0]["path"], "/1");
        assert!(seen[0]["csp"].is_null());
    }
}
