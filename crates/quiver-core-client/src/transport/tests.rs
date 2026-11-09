use super::*;

#[test]
fn connect_errors_name_the_cause() {
    let e = Error::Connect("no such file".into());
    assert_eq!(e.to_string(), "connect failed: no such file");
}

#[test]
fn protocol_errors_name_the_cause() {
    let e = Error::Protocol("bad status line".into());
    assert_eq!(e.to_string(), "protocol error: bad status line");
}

/// A transport that can only answer whole: it implements `request` and `open_ws`
/// and relies on the trait's default `request_stream`.
struct Whole(Result<(u16, Vec<u8>), &'static str>);

#[async_trait]
impl Transport for Whole {
    async fn request(&self, _req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
        match &self.0 {
            Ok((status, body)) => Ok(Response::builder()
                .status(*status)
                .body(body.clone())
                .unwrap()),
            Err(why) => Err(Error::Connect((*why).into())),
        }
    }

    async fn open_ws(&self, _path: &str) -> Result<WsStream, Error> {
        Err(Error::Connect("no ws".into()))
    }
}

fn get() -> Request<Vec<u8>> {
    Request::builder()
        .uri("quiver://localhost/v0/x")
        .body(Vec::new())
        .unwrap()
}

#[tokio::test]
async fn the_default_delivers_the_whole_body_as_one_chunk_then_ends() {
    let t = Whole(Ok((200, b"hello".to_vec())));
    let mut resp = t
        .request_stream(get(), Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
    assert!(resp.chunks.recv().await.is_none());
}

#[tokio::test]
async fn the_default_keeps_the_status_of_an_error_answer() {
    let t = Whole(Ok((403, b"{}".to_vec())));
    let resp = t
        .request_stream(get(), Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(resp.status, 403);
}

#[tokio::test]
async fn the_default_delivers_nothing_for_an_empty_body() {
    let t = Whole(Ok((204, Vec::new())));
    let mut resp = t
        .request_stream(get(), Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(resp.status, 204);
    assert!(resp.chunks.recv().await.is_none());
}

#[tokio::test]
async fn the_default_passes_a_transport_failure_through() {
    let t = Whole(Err("no socket"));
    let err = t
        .request_stream(get(), Duration::from_secs(1))
        .await
        .err()
        .unwrap();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}
