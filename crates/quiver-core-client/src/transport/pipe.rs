//! The local transport on Windows: quiver.core over its named pipe.
//!
//! Replaces the unauthenticated loopback port Windows used before quiver.core
//! grew `npipe://`: the daemon creates the pipe with a DACL granting only the
//! current user, and refuses remote clients, so nothing else on the machine or
//! network can talk to it.

use std::time::Duration;

use ::http::{Request, Response};
use async_trait::async_trait;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

use super::stream::{open_ws_over, request_over, request_stream_over};
use super::{Error, StreamResponse, Transport, WsStream};

/// `ERROR_PIPE_BUSY`: every instance of the pipe is momentarily serving
/// another client. The daemon frees one within milliseconds, so this is worth
/// retrying where any other open failure is not.
const ERROR_PIPE_BUSY: i32 = 231;
const BUSY_RETRY: Duration = Duration::from_millis(25);
const BUSY_ATTEMPTS: u32 = 40;

/// The path Windows knows a pipe by.
pub fn pipe_path(name: &str) -> String {
    format!(r"\\.\pipe\{name}")
}

pub struct PipeTransport {
    path: String,
}

impl PipeTransport {
    pub fn new(name: impl AsRef<str>) -> Self {
        Self {
            path: pipe_path(name.as_ref()),
        }
    }

    async fn connect(&self) -> Result<NamedPipeClient, Error> {
        let mut attempts = 0;
        loop {
            match ClientOptions::new().open(&self.path) {
                Ok(client) => return Ok(client),
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempts < BUSY_ATTEMPTS => {
                    attempts += 1;
                    tokio::time::sleep(BUSY_RETRY).await;
                }
                Err(e) => return Err(Error::Connect(e.to_string())),
            }
        }
    }
}

#[async_trait]
impl Transport for PipeTransport {
    async fn request(&self, req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
        request_over(self.connect().await?, req).await
    }

    async fn open_ws(&self, path: &str) -> Result<WsStream, Error> {
        open_ws_over(self.connect().await?, path).await
    }

    async fn request_stream(
        &self,
        req: Request<Vec<u8>>,
        deadline: Duration,
    ) -> Result<StreamResponse, Error> {
        request_stream_over(self.connect().await?, req, deadline).await
    }
}

#[cfg(test)]
mod tests;
