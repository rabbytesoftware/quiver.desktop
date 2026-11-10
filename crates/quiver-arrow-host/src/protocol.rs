//! Answers the helper window's protocol requests with the ported handler, so
//! the helper process stays dumb.

use std::sync::Arc;

use crate::handler::handle_request;
use crate::hosts::ArrowHosts;
use crate::ipc::{ProtocolRequest, ProtocolResponse};
use crate::message::ArrowRequest;
use crate::upstream::ArrowUpstream;

#[derive(Clone)]
pub struct ProtocolService {
    hosts: Arc<ArrowHosts>,
    upstream: Arc<dyn ArrowUpstream>,
}

impl ProtocolService {
    pub fn new(
        hosts: Arc<ArrowHosts>,
        upstream: Arc<dyn ArrowUpstream>,
    ) -> Self {
        Self { hosts, upstream }
    }

    /// Always produces a response (errors are 4xx/5xx pages with the CSP) with
    /// the request's `id`.
    pub async fn answer(
        &self,
        req: ProtocolRequest,
    ) -> ProtocolResponse {
        let id = req.id;
        let request = ArrowRequest {
            method: req.method,
            uri: req.uri,
            headers: req.headers,
            body: req.body,
        };
        let resp = handle_request(
            std::future::ready(self.upstream.clone()),
            &self.hosts,
            request,
        )
        .await;
        ProtocolResponse {
            id,
            status: resp.status,
            headers: resp.headers,
            body: resp.body,
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::message::ArrowResponse;
    use crate::upstream::WsRelay;
    use crate::{Error, Result};

    struct Echo;

    #[async_trait]
    impl ArrowUpstream for Echo {
        async fn request(
            &self,
            req: ArrowRequest,
        ) -> Result<ArrowResponse> {
            Ok(ArrowResponse::new(200, req.uri.into_bytes()))
        }
        async fn open_ws(
            &self,
            _: &str,
        ) -> Result<Box<dyn WsRelay>> {
            Err(Error::Upstream("not used".into()))
        }
    }

    fn service() -> (ProtocolService, String) {
        let hosts = Arc::new(ArrowHosts::new());
        let host = hosts.register("github.com/user/chat");
        (ProtocolService::new(hosts, Arc::new(Echo)), host)
    }

    fn request(
        id: u64,
        uri: String,
    ) -> ProtocolRequest {
        ProtocolRequest {
            id,
            method: "GET".into(),
            uri,
            headers: Default::default(),
            body: Vec::new(),
        }
    }

    #[tokio::test]
    async fn answers_through_the_handler_and_keeps_the_correlation_id() {
        let (svc, host) = service();
        let resp = svc
            .answer(request(41, format!("arrow-app://{host}/a?b=1")))
            .await;
        assert_eq!(resp.id, 41);
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, b"/v0/ui/github.com%2Fuser%2Fchat/a?b=1");
        assert!(resp.headers.contains("content-security-policy"));
    }

    #[tokio::test]
    async fn unknown_hosts_and_garbage_uris_are_answered_not_dropped() {
        let (svc, _) = service();
        let unknown = svc.answer(request(1, "arrow-app://deadbeef/".into())).await;
        assert_eq!((unknown.id, unknown.status), (1, 404));
        let garbage = svc.answer(request(2, "::not a uri".into())).await;
        assert_eq!((garbage.id, garbage.status), (2, 400));
    }
}
