use super::*;

#[tokio::test]
async fn cancel_aborts_the_run_and_forgets_it() {
    let m = ConsoleExecManager::new();
    let task = tokio::spawn(std::future::pending::<()>());
    m.register("a".into(), task.abort_handle());
    assert_eq!(m.in_flight(), 1);
    m.cancel("a");
    assert_eq!(m.in_flight(), 0);
    assert!(task.await.unwrap_err().is_cancelled());
    m.cancel("a"); // unknown id: not an error
}

#[tokio::test]
async fn cancel_all_aborts_every_run() {
    let m = ConsoleExecManager::new();
    let a = tokio::spawn(std::future::pending::<()>());
    let b = tokio::spawn(std::future::pending::<()>());
    m.register("a".into(), a.abort_handle());
    m.register("b".into(), b.abort_handle());
    m.cancel_all();
    assert_eq!(m.in_flight(), 0);
    assert!(a.await.unwrap_err().is_cancelled());
    assert!(b.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn a_reused_id_aborts_the_run_it_replaces() {
    let m = ConsoleExecManager::new();
    let first = tokio::spawn(std::future::pending::<()>());
    let second = tokio::spawn(std::future::pending::<()>());
    m.register("a".into(), first.abort_handle());
    m.register("a".into(), second.abort_handle());
    assert!(first.await.unwrap_err().is_cancelled());
    assert_eq!(m.in_flight(), 1);
    m.cancel_all();
}

// --- check_line and the oversize error body ----------------------------

#[test]
fn a_command_line_is_checked_against_the_ipc_guard() {
    assert!(check_line("").is_ok());
    assert!(check_line(&"x".repeat(MAX_LINE_BYTES)).is_ok());
    let err = check_line(&"x".repeat(MAX_LINE_BYTES + 1)).unwrap_err();
    assert!(
        err.to_string().contains(&MAX_LINE_BYTES.to_string()),
        "{err}"
    );
}

#[tokio::test]
async fn an_error_body_larger_than_the_cap_is_cut_and_the_status_still_reported() {
    let big = "x".repeat(MAX_ERROR_BODY_BYTES + 1);
    let t = Scripted::new(502, vec![ok(&big), ok("never read")]);
    let (sink, seen) = sink();
    run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
    let frames = seen.lock().unwrap().clone();
    assert_eq!(frames.len(), 1);
    let f: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
    assert_eq!(f["status"], 502);
    assert!(f["message"].as_str().unwrap().contains("502"));
}

#[tokio::test]
async fn a_transport_error_with_no_text_still_says_something_useful() {
    let t = Scripted::new(200, vec![Err(Error::Protocol(String::new()))]);
    let (sink, seen) = sink();
    run_exec(&t, "list", &sink, Duration::from_secs(1)).await;
    let frames = seen.lock().unwrap().clone();
    assert_eq!(frames.len(), 1);
    assert!(frames[0].contains("\"type\":\"error\""));
}

// --- start: the whole of console_exec minus Tauri's State -----------------

#[tokio::test]
async fn start_runs_the_command_streams_its_frames_and_forgets_the_run() {
    let m = ConsoleExecManager::new();
    let (sink, seen) = sink();
    let t: Arc<dyn Transport> = Arc::new(Scripted::new(
        200,
        vec![ok("{\"type\":\"exit\",\"code\":0,\"error\":\"\"}\n")],
    ));
    let run = m.start("a".into(), t, "list".into(), sink, Duration::from_secs(1));

    // The run has ended when its task has: nothing is waited for by the clock.
    run.await.unwrap();
    assert_eq!(m.in_flight(), 0);
    let frames = seen.lock().unwrap();
    assert_eq!(frames.len(), 1);
    assert!(frames[0].contains("exit"));
}

/// A transport whose stream never ends: a command that is still running.
struct Endless;

#[async_trait]
impl Transport for Endless {
    async fn request(
        &self,
        _r: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Error> {
        unreachable!()
    }
    async fn open_ws(
        &self,
        _p: &str,
    ) -> Result<WsStream, Error> {
        unreachable!()
    }
    async fn request_stream(
        &self,
        _r: Request<Vec<u8>>,
        _d: Duration,
    ) -> Result<StreamResponse, Error> {
        let (tx, rx) = mpsc::channel(1);
        // Hold the sender so the stream stays open until the run is aborted.
        tokio::spawn(async move {
            let _tx = tx;
            std::future::pending::<()>().await;
        });
        Ok(StreamResponse {
            status: 200,
            chunks: rx,
        })
    }
}

#[tokio::test]
async fn start_registers_a_running_command_so_it_can_be_cancelled() {
    let m = ConsoleExecManager::new();
    let (sink, seen) = sink();
    let run = m.start(
        "a".into(),
        Arc::new(Endless),
        "run x".into(),
        sink,
        Duration::from_secs(5),
    );
    assert_eq!(m.in_flight(), 1);

    m.cancel("a");
    assert_eq!(m.in_flight(), 0);
    // The task is gone once it is awaited: whatever it was going to say, it never says.
    assert!(run.await.unwrap_err().is_cancelled());
    assert!(
        seen.lock().unwrap().is_empty(),
        "a cancelled run says nothing more"
    );
}

#[tokio::test]
async fn start_replaces_a_run_that_reuses_its_id() {
    let m = ConsoleExecManager::new();
    let (first_sink, first_seen) = sink();
    let first = m.start(
        "a".into(),
        Arc::new(Endless),
        "run x".into(),
        first_sink,
        Duration::from_secs(5),
    );
    let (second_sink, _) = sink();
    let second = m.start(
        "a".into(),
        Arc::new(Endless),
        "run y".into(),
        second_sink,
        Duration::from_secs(5),
    );
    // The second took the id, so the first was ended.
    assert!(first.await.unwrap_err().is_cancelled());
    assert_eq!(m.in_flight(), 1);
    m.cancel_all();
    assert_eq!(m.in_flight(), 0);
    assert!(second.await.unwrap_err().is_cancelled());
    assert!(first_seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn start_reports_an_unreachable_daemon_and_leaves_nothing_behind() {
    struct Dead;
    #[async_trait]
    impl Transport for Dead {
        async fn request(
            &self,
            _r: Request<Vec<u8>>,
        ) -> Result<Response<Vec<u8>>, Error> {
            Err(Error::Connect("no socket".into()))
        }
        async fn open_ws(
            &self,
            _p: &str,
        ) -> Result<WsStream, Error> {
            Err(Error::Connect("no socket".into()))
        }
    }
    let m = ConsoleExecManager::new();
    let (sink, seen) = sink();
    let run = m.start(
        "a".into(),
        Arc::new(Dead),
        "list".into(),
        sink,
        Duration::from_secs(1),
    );
    run.await.unwrap();
    assert_eq!(m.in_flight(), 0);
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert!(seen.lock().unwrap()[0].contains("no socket"));
}
