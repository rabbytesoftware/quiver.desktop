//! The local transport on macOS and Linux: quiver.core over its unix socket.
//!
//! Windows reaches the same daemon over a named pipe instead (`super::pipe`);
//! both hand their connected stream to `super::stream`, so the HTTP and
//! WebSocket handling exists once.

use ::http::{Request, Response};
use async_trait::async_trait;
use tokio::net::UnixStream;

use std::time::Duration;

use super::stream::{open_ws_over, request_over, request_stream_over};
use super::{Error, StreamResponse, Transport, WsStream};

pub struct UnixTransport {
    socket_path: String,
}

impl UnixTransport {
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    async fn connect(&self) -> Result<UnixStream, Error> {
        UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| Error::Connect(e.to_string()))
    }
}

#[async_trait]
impl Transport for UnixTransport {
    async fn request(
        &self,
        req: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Error> {
        request_over(self.connect().await?, req).await
    }

    async fn open_ws(
        &self,
        path: &str,
    ) -> Result<WsStream, Error> {
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
