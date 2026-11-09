use super::*;

#[tokio::test]
async fn frames_are_forwarded_verbatim_and_the_request_is_the_contract() {
    let t = Scripted::new(
        200,
        vec![
            ok("{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"hi\\n\"}\n{\"type\":"),
            ok("\"exit\",\"code\":0,\"error\":\"\"}\n"),
        ],
    );
    let (sink, seen) = sink();
    run_exec(
        &t,
        "install github.com/char2cs/crowbar",
        &sink,
        Duration::from_secs(1),
    )
    .await;

    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            "{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"hi\\n\"}",
            "{\"type\":\"exit\",\"code\":0,\"error\":\"\"}",
        ]
    );
    let (method, path, body) = t.seen.lock().unwrap().clone().unwrap();
    assert_eq!(
        (method.as_str(), path.as_str()),
        ("POST", "/v0/console/exec")
    );
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"line": "install github.com/char2cs/crowbar"})
    );
}

#[tokio::test]
async fn a_command_line_with_quotes_and_unicode_is_sent_as_one_json_string() {
    let t = Scripted::new(
        200,
        vec![ok("{\"type\":\"exit\",\"code\":0,\"error\":\"\"}\n")],
    );
    let (sink, _) = sink();
    let line = "run \"a b\" 'c' \u{1F600}\n\\";
    run_exec(&t, line, &sink, Duration::from_secs(1)).await;
    let (_, _, body) = t.seen.lock().unwrap().clone().unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["line"], line);
}

#[tokio::test]
async fn an_error_status_becomes_one_error_frame_with_the_daemons_message() {
    let t = Scripted::new(
			403,
			vec![ok("{\"success\":false,\"error\":\"command \\\"daemon\\\" is not available in the console\"}")],
		);
    let (sink, seen) = sink();
    run_exec(&t, "daemon", &sink, Duration::from_secs(1)).await;
    let frames = seen.lock().unwrap().clone();
    assert_eq!(frames.len(), 1);
    let f: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
    assert_eq!(f["type"], "error");
    assert_eq!(f["status"], 403);
    assert!(f["message"].as_str().unwrap().contains("not available"));
}

#[tokio::test]
async fn an_error_status_with_an_unreadable_body_still_reports_the_status() {
    let t = Scripted::new(502, vec![ok("<html>bad gateway</html>")]);
    let (sink, seen) = sink();
    run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
    let f: serde_json::Value = serde_json::from_str(&seen.lock().unwrap()[0]).unwrap();
    assert_eq!(f["status"], 502);
    assert!(f["message"].as_str().unwrap().contains("502"));
}

#[tokio::test]
async fn a_stream_that_ends_without_an_exit_frame_is_reported() {
    let t = Scripted::new(
        200,
        vec![ok(
            "{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"x\"}\n",
        )],
    );
    let (sink, seen) = sink();
    run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
    let frames = seen.lock().unwrap().clone();
    assert_eq!(frames.len(), 2);
    let f: serde_json::Value = serde_json::from_str(&frames[1]).unwrap();
    assert_eq!(f["type"], "error");
    assert!(f["message"].as_str().unwrap().contains("ended"));
}

#[tokio::test]
async fn a_transport_error_mid_stream_is_reported_after_what_arrived() {
    let t = Scripted::new(
        200,
        vec![
            ok("{\"type\":\"out\",\"stream\":\"stdout\",\"data\":\"x\"}\n"),
            Err(Error::Protocol("connection reset".into())),
        ],
    );
    let (sink, seen) = sink();
    run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
    let frames = seen.lock().unwrap().clone();
    assert_eq!(frames.len(), 2);
    let f: serde_json::Value = serde_json::from_str(&frames[1]).unwrap();
    assert_eq!(f["type"], "error");
    assert!(f["message"].as_str().unwrap().contains("connection reset"));
}

#[tokio::test]
async fn an_exit_frame_without_a_trailing_newline_still_counts() {
    let t = Scripted::new(
        200,
        vec![ok("{\"type\":\"exit\",\"code\":1,\"error\":\"boom\"}")],
    );
    let (sink, seen) = sink();
    run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
    assert_eq!(
        seen.lock().unwrap().len(),
        1,
        "no extra error frame after a real exit"
    );
}

#[tokio::test]
async fn a_transport_that_cannot_connect_reports_one_error_frame() {
    struct Dead;
    #[async_trait]
    impl Transport for Dead {
        async fn request(&self, _r: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
            Err(Error::Connect("no socket".into()))
        }
        async fn open_ws(&self, _p: &str) -> Result<WsStream, Error> {
            Err(Error::Connect("no socket".into()))
        }
    }
    let (sink, seen) = sink();
    run_exec(&Dead, "list", &sink, Duration::from_secs(1)).await;
    let frames = seen.lock().unwrap().clone();
    assert_eq!(frames.len(), 1);
    assert!(frames[0].contains("no socket"));
}

// --- manager ------------------------------------------------------------
