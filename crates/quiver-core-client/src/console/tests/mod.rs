use super::exec::MAX_ERROR_BODY_BYTES;
use super::splitter::MAX_FRAME_BYTES;
use super::*;
use crate::bridge::FrameSink;
use crate::transport::{StreamResponse, WsStream};
use crate::Error;
use crate::Transport;
use ::http::Request;
use ::http::Response;
use async_trait::async_trait;
use std::result::Result;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::mpsc;

struct Collect(Arc<StdMutex<Vec<String>>>);
impl FrameSink for Collect {
    fn send(
        &self,
        frame: String,
    ) {
        self.0.lock().unwrap().push(frame);
    }
}
fn sink() -> (Collect, Arc<StdMutex<Vec<String>>>) {
    let seen = Arc::new(StdMutex::new(Vec::new()));
    (Collect(Arc::clone(&seen)), seen)
}

/// A transport whose `request_stream` answers with a scripted status and
/// chunks, and records the request it was given.
struct Scripted {
    status: u16,
    chunks: Vec<Result<Vec<u8>, Error>>,
    seen: StdMutex<Option<(String, String, Vec<u8>)>>,
}
impl Scripted {
    fn new(
        status: u16,
        chunks: Vec<Result<Vec<u8>, Error>>,
    ) -> Self {
        Self {
            status,
            chunks,
            seen: StdMutex::new(None),
        }
    }
}
#[async_trait]
impl Transport for Scripted {
    async fn request(
        &self,
        _req: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Error> {
        unreachable!("exec streams")
    }
    async fn open_ws(
        &self,
        _path: &str,
    ) -> Result<WsStream, Error> {
        unreachable!()
    }
    async fn request_stream(
        &self,
        req: Request<Vec<u8>>,
        _deadline: Duration,
    ) -> Result<StreamResponse, Error> {
        *self.seen.lock().unwrap() = Some((
            req.method().to_string(),
            req.uri().path().to_string(),
            req.body().clone(),
        ));
        let (tx, rx) = mpsc::channel(32);
        for c in &self.chunks {
            let item = match c {
                Ok(b) => Ok(b.clone()),
                Err(e) => Err(Error::Protocol(e.to_string())),
            };
            tx.try_send(item).unwrap();
        }
        Ok(StreamResponse {
            status: self.status,
            chunks: rx,
        })
    }
}

fn ok(s: &str) -> Result<Vec<u8>, Error> {
    Ok(s.as_bytes().to_vec())
}

// --- splitter -----------------------------------------------------------

mod exec;
mod runs;
mod splitting;
