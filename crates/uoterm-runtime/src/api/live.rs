//! The live link of one session: a WebSocket that keeps a web page up to
//! date and carries the tool calls of the page.
//!
//! A session has no signal that tells when it changed. So the link calls
//! the `watch` tool every `LIVE_POLL_MS`, the pace of the screen, and sends
//! the picture only when it differs from the last one this link sent. Each
//! link keeps its own last picture, so two pages on one session both stay
//! up to date.
//!
//! An act of more than one step runs here, not in the page, with
//! `ACT_STEP_GAP_MS` between the steps. A page that closes in the middle
//! of a lift and its drop does not leave the item in the hand. The acts of
//! one session run one at a time, whatever link sent them, so two pages
//! cannot mix the steps of their acts.

use super::{session_not_found, ApiState};
use crate::manager::Runtime;
use crate::session::SessionHandle;
use crate::tools::{ToolCall, ToolResult, TOOL_WATCH};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use uoterm_world::tool_names::{ACT_STEP_GAP_MS, ARG_SIZE};
use uoterm_world::WINDOW_RADAR_SIZE_WITH_ART;

/// The pace of the screen: how often the link reads its session.
pub const LIVE_POLL_MS: u64 = 33;
/// Messages that wait for the page. A page that reads slowly holds the next
/// picture back, so they do not pile up.
const LIVE_OUTBOX: usize = 32;
/// A page that takes longer than this to take one message is gone, and the
/// link closes.
pub(super) const LIVE_SEND_TIMEOUT: Duration = Duration::from_secs(5);
/// The largest message a page may send. A script or a book page it saves
/// fits.
const LIVE_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
/// The longest act of the human today is a lift and its drop. This leaves
/// room, and still keeps one act short.
const MAX_ACT_STEPS: usize = 4;
/// Acts of one session that may wait while another runs.
const MAX_WAITING_ACTS: usize = 4;
/// The act that runs, and the acts that wait.
const ACT_PLACES: usize = MAX_WAITING_ACTS + 1;
const KIND_FRAME: &str = "frame";
const KIND_ANSWER: &str = "answer";
const KIND_ENDED: &str = "ended";
const EMPTY_ACT: &str = "an act needs at least one call";
const ACT_TOO_LONG: &str = "an act has too many steps";
const ACTS_WAITING: &str = "too many acts wait; try again when they are done";

/// What the page sends.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FromPage {
    /// One tool call, answered with its id.
    Call {
        id: u64,
        #[serde(flatten)]
        call: PageCall,
    },
    /// Calls made in order with a gap between them, answered once.
    Act { id: u64, calls: Vec<PageCall> },
    /// The radar size the pictures are drawn with.
    Size { size: u16 },
}

#[derive(Deserialize)]
struct PageCall {
    tool: String,
    args: Value,
}

impl From<PageCall> for ToolCall {
    fn from(call: PageCall) -> Self {
        ToolCall {
            name: call.tool,
            args: call.args,
        }
    }
}

/// The acts of each session: the one that runs and the ones that wait.
#[derive(Default)]
pub struct ActLines {
    lines: Mutex<HashMap<String, Arc<ActLine>>>,
}

impl ActLines {
    /// The line of session `id`. Lines of sessions the runtime no longer
    /// has are let go.
    fn line_for(&self, runtime: &Runtime, id: &str) -> Arc<ActLine> {
        let mut lines = self.lines.lock();
        lines.retain(|session, _| runtime.get(session).is_some());
        lines.entry(id.to_string()).or_default().clone()
    }
}

struct ActLine {
    /// One place for the act that runs, and one for each that may wait.
    places: Arc<Semaphore>,
    /// Held by the act that runs.
    turn: tokio::sync::Mutex<()>,
}

impl Default for ActLine {
    fn default() -> Self {
        Self {
            places: Arc::new(Semaphore::new(ACT_PLACES)),
            turn: tokio::sync::Mutex::new(()),
        }
    }
}

pub(super) async fn live(
    ws: WebSocketUpgrade,
    State(st): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Response {
    let Some(handle) = st.runtime.get(&id) else {
        return session_not_found();
    };
    let line = st.acts.line_for(&st.runtime, &id);
    ws.max_message_size(LIVE_MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| run_live(socket, handle, line))
}

/// Sends pictures and reads the page until one side ends. Answers of acts
/// still running go out while the page is there to read them.
async fn run_live(socket: WebSocket, handle: SessionHandle, line: Arc<ActLine>) {
    let (sink, stream) = socket.split();
    let (outbox, letters) = mpsc::channel(LIVE_OUTBOX);
    tokio::spawn(write_letters(sink, letters));
    let size = AtomicU16::new(WINDOW_RADAR_SIZE_WITH_ART);
    let link = Link {
        handle: &handle,
        line: &line,
        size: &size,
        outbox: &outbox,
    };
    tokio::select! {
        () = send_frames(&link) => {}
        () = read_page(stream, &link) => {}
    }
}

/// What the two halves of one link share.
struct Link<'a> {
    handle: &'a SessionHandle,
    line: &'a Arc<ActLine>,
    size: &'a AtomicU16,
    outbox: &'a mpsc::Sender<String>,
}

/// The only writer of the socket. It closes the socket when nothing is
/// left to send, and gives up on a page that stopped reading.
async fn write_letters(
    mut sink: SplitSink<WebSocket, Message>,
    mut letters: mpsc::Receiver<String>,
) {
    while let Some(letter) = letters.recv().await {
        let sent = tokio::time::timeout(LIVE_SEND_TIMEOUT, sink.send(Message::Text(letter.into())));
        if !matches!(sent.await, Ok(Ok(()))) {
            return;
        }
    }
    let _ = sink.close().await;
}

async fn send_frames(link: &Link<'_>) {
    let mut last_sent: Option<Value> = None;
    loop {
        let watch = link
            .handle
            .call(ToolCall {
                name: TOOL_WATCH.into(),
                args: json!({ ARG_SIZE: link.size.load(Ordering::Relaxed) }),
            })
            .await;
        if watch.ok {
            if last_sent.as_ref() != Some(&watch.result) {
                let letter = json!({ "kind": KIND_FRAME, "watch": &watch.result });
                if link.outbox.send(letter.to_string()).await.is_err() {
                    return;
                }
                last_sent = Some(watch.result);
            }
        } else if link.handle.closed() {
            let _ = link
                .outbox
                .send(json!({ "kind": KIND_ENDED }).to_string())
                .await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(LIVE_POLL_MS)).await;
    }
}

async fn read_page(mut stream: SplitStream<WebSocket>, link: &Link<'_>) {
    while let Some(Ok(message)) = stream.next().await {
        let Message::Text(text) = message else {
            continue;
        };
        match serde_json::from_str::<FromPage>(&text) {
            Ok(FromPage::Call { id, call }) => {
                let answer = link.handle.call(call.into()).await;
                if link.outbox.send(answer_letter(id, answer)).await.is_err() {
                    return;
                }
            }
            Ok(FromPage::Act { id, calls }) => start_act(link, id, calls),
            Ok(FromPage::Size { size }) => link.size.store(size, Ordering::Relaxed),
            Err(error) => tracing::debug!(%error, "the live link read a message it does not know"),
        }
    }
}

/// Starts an act in a task of its own, which keeps running when the page
/// closes. It waits for the acts of the session before it, and is refused
/// when it is too long or too many wait.
///
/// The answer goes to the page only when there is room for it: the act has
/// already run, and a page that stopped reading must not hold it.
fn start_act(link: &Link<'_>, id: u64, calls: Vec<PageCall>) {
    let refuse = |words: &str| {
        let _ = link
            .outbox
            .try_send(answer_letter(id, ToolResult::err(words)));
    };
    if calls.len() > MAX_ACT_STEPS {
        return refuse(ACT_TOO_LONG);
    }
    let Ok(place) = link.line.places.clone().try_acquire_owned() else {
        return refuse(ACTS_WAITING);
    };
    let (handle, line, outbox) = (link.handle.clone(), link.line.clone(), link.outbox.clone());
    tokio::spawn(async move {
        let answer = {
            let _turn = line.turn.lock().await;
            run_act(&handle, calls).await
        };
        drop(place);
        let _ = outbox.try_send(answer_letter(id, answer));
    });
}

/// Makes the calls of one act in order, `ACT_STEP_GAP_MS` apart, and stops
/// at the first that fails. The answer is the one of the last call made.
async fn run_act(handle: &SessionHandle, calls: Vec<PageCall>) -> ToolResult {
    let mut answer = ToolResult::err(EMPTY_ACT);
    for (step, call) in calls.into_iter().enumerate() {
        if step > 0 {
            tokio::time::sleep(Duration::from_millis(ACT_STEP_GAP_MS)).await;
        }
        answer = handle.call(call.into()).await;
        if !answer.ok {
            break;
        }
    }
    answer
}

/// The answer to call `id`. A refused call carries the words that say why.
fn answer_letter(id: u64, answer: ToolResult) -> String {
    let mut letter = json!({
        "kind": KIND_ANSWER,
        "id": id,
        "ok": answer.ok,
        "result": answer.result,
    });
    if let Some(error) = answer.error {
        letter["error"] = json!(error);
    }
    letter.to_string()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::api::router_for;
    use crate::config::AppConfig;
    use crate::manager::Runtime;
    use crate::mock::test_login::{mock_opts, wait_for_login};
    use crate::mock::{MockServer, MOCK_BACKPACK, MOCK_HATCHET};
    use axum::http::StatusCode;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::net::SocketAddr;
    use std::time::{Duration, Instant};
    use tokio::net::TcpStream;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::http::HeaderName;
    use tokio_tungstenite::tungstenite::{self, Message as PageMessage};
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
    use uoterm_protocol::types::PKT_DROP;
    use uoterm_world::tool_names::{ARG_HUMAN, ARG_SIZE, TOOL_DROP, TOOL_LIFT, TOOL_OBSERVE};
    use uoterm_world::RADAR_MAX;

    pub(in crate::api) type WsClient = WebSocketStream<MaybeTlsStream<TcpStream>>;

    const ANY_LOCAL_PORT: &str = "127.0.0.1:0";
    const TEST_SESSIONS: usize = 1;
    const NO_SUCH_SESSION: &str = "s999";
    const CALL_ID: u64 = 7;
    const ACT_ID: u64 = 1;
    /// Two observe calls: one act, one gap.
    const TWO_STEPS: usize = 2;
    /// About ten polls of the link.
    const IDLE_QUIET_MS: u64 = 300;
    /// Steps of the act a closed page leaves behind, and the gaps to wait
    /// for them: one gap between them and two to spare.
    const ACT_WAIT_GAPS: u64 = 3;

    /// The task of a test API server. Dropping it stops the server, which
    /// dropping a `JoinHandle` does not.
    pub(in crate::api) struct ServerTask(JoinHandle<()>);

    impl Drop for ServerTask {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    /// An API server of `runtime` on a free port of this machine.
    pub(in crate::api) async fn serve_api(
        runtime: Runtime,
        token: Option<String>,
        local_only: bool,
    ) -> (SocketAddr, ServerTask) {
        let listener = tokio::net::TcpListener::bind(ANY_LOCAL_PORT).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = router_for(runtime, AppConfig::default(), token, local_only);
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (addr, ServerTask(server))
    }

    /// An API server with one session on the mock shard. Dropping it stops
    /// the server; the session and the shard stop with the runtime and the
    /// mock.
    pub(in crate::api) struct LiveTestServer {
        pub addr: SocketAddr,
        pub id: String,
        pub runtime: Runtime,
        pub shard: MockServer,
        _server: ServerTask,
    }

    pub(in crate::api) async fn serve_with_mock_session() -> LiveTestServer {
        serve_mock_session(None, true).await
    }

    /// A server that wants `token` and answers callers from any machine,
    /// so the token is all it checks.
    pub(in crate::api) async fn serve_with_mock_session_token(token: &str) -> LiveTestServer {
        serve_mock_session(Some(token.to_string()), false).await
    }

    async fn serve_mock_session(token: Option<String>, local_only: bool) -> LiveTestServer {
        let shard = MockServer::start().await.unwrap();
        let runtime = Runtime::new(TEST_SESSIONS);
        let handle = runtime.connect(mock_opts(&shard)).await.unwrap();
        wait_for_login(&handle).await;
        let (addr, server) = serve_api(runtime.clone(), token, local_only).await;
        LiveTestServer {
            addr,
            id: handle.id,
            runtime,
            shard,
            _server: server,
        }
    }

    /// The WebSocket at `path`, or the HTTP status of a refused one.
    /// `headers` go with the request, as a browser adds its cookie and
    /// origin.
    pub(in crate::api) async fn try_connect(
        addr: SocketAddr,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Result<WsClient, u16> {
        let mut request = format!("ws://{addr}{path}").into_client_request().unwrap();
        for (name, value) in headers {
            request.headers_mut().insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        match tokio_tungstenite::connect_async(request).await {
            Ok((ws, _)) => Ok(ws),
            Err(tungstenite::Error::Http(response)) => Err(response.status().as_u16()),
            Err(other) => panic!("the WebSocket at {path} failed: {other}"),
        }
    }

    /// The live link of session `id`. See [`try_connect`].
    pub(in crate::api) async fn try_connect_live(
        addr: SocketAddr,
        id: &str,
        headers: &[(&str, &str)],
    ) -> Result<WsClient, u16> {
        try_connect(addr, &format!("/v1/sessions/{id}/live"), headers).await
    }

    async fn connect_live(addr: SocketAddr, id: &str) -> WsClient {
        try_connect_live(addr, id, &[]).await.unwrap()
    }

    pub(in crate::api) async fn next_json(ws: &mut WsClient) -> Value {
        loop {
            if let PageMessage::Text(text) = ws.next().await.unwrap().unwrap() {
                return serde_json::from_str(&text).unwrap();
            }
        }
    }

    async fn next_of_kind(ws: &mut WsClient, kind: &str) -> Value {
        loop {
            let message = next_json(ws).await;
            if message["kind"] == kind {
                return message;
            }
        }
    }

    pub(in crate::api) async fn send_json(ws: &mut WsClient, message: Value) {
        ws.send(PageMessage::Text(message.to_string().into()))
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_live_link_sends_a_frame_first() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        let first = next_json(&mut ws).await;
        assert_eq!(first["kind"], KIND_FRAME);
        assert!(first["watch"].get("self_state").is_some());
    }

    /// A session where nothing happens sends its picture once.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_idle_session_sends_one_frame() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        next_of_kind(&mut ws, KIND_FRAME).await;
        let quiet = Duration::from_millis(IDLE_QUIET_MS);
        let next = tokio::time::timeout(quiet, next_json(&mut ws)).await;
        assert!(next.is_err(), "a second frame came: {next:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_call_is_answered_with_its_id() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        next_json(&mut ws).await;
        send_json(
            &mut ws,
            json!({"kind": "call", "id": CALL_ID, "tool": TOOL_OBSERVE, "args": {}}),
        )
        .await;
        let answer = next_of_kind(&mut ws, KIND_ANSWER).await;
        assert_eq!(answer["id"], CALL_ID);
        assert_eq!(answer["ok"], true);
        assert!(answer["result"].get("self_state").is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_refused_call_brings_its_words() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        send_json(
            &mut ws,
            json!({"kind": "call", "id": CALL_ID, "tool": TOOL_LIFT, "args": {}}),
        )
        .await;
        let answer = next_of_kind(&mut ws, KIND_ANSWER).await;
        assert_eq!(answer["id"], CALL_ID);
        assert_eq!(answer["ok"], false);
        assert!(answer["error"].is_string());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_links_on_one_session_both_get_frames() {
        let server = serve_with_mock_session().await;
        let mut a = connect_live(server.addr, &server.id).await;
        let mut b = connect_live(server.addr, &server.id).await;
        assert_eq!(next_json(&mut a).await["kind"], KIND_FRAME);
        assert_eq!(next_json(&mut b).await["kind"], KIND_FRAME);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_new_radar_size_brings_a_new_frame() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        let small = next_of_kind(&mut ws, KIND_FRAME).await["watch"]["radar"].clone();
        send_json(&mut ws, json!({"kind": "size", ARG_SIZE: RADAR_MAX})).await;
        loop {
            let radar = next_of_kind(&mut ws, KIND_FRAME).await["watch"]["radar"].clone();
            if radar != small {
                break;
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_act_waits_between_its_steps_and_answers_once() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        let started = Instant::now();
        send_json(
            &mut ws,
            json!({"kind": "act", "id": ACT_ID, "calls": [
                {"tool": TOOL_OBSERVE, "args": {}},
                {"tool": TOOL_OBSERVE, "args": {}}
            ]}),
        )
        .await;
        let answer = next_of_kind(&mut ws, KIND_ANSWER).await;
        assert!(started.elapsed() >= Duration::from_millis(ACT_STEP_GAP_MS));
        assert_eq!(answer["id"], ACT_ID);
        assert_eq!(answer["ok"], true);
    }

    /// An act runs in a task of its own, so the page is still read while it
    /// waits, and the end of the page does not end the act.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_act_runs_beside_the_calls_of_the_page() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        send_json(
            &mut ws,
            json!({"kind": "act", "id": ACT_ID, "calls": [
                {"tool": TOOL_OBSERVE, "args": {}},
                {"tool": TOOL_OBSERVE, "args": {}}
            ]}),
        )
        .await;
        send_json(
            &mut ws,
            json!({"kind": "call", "id": CALL_ID, "tool": TOOL_OBSERVE, "args": {}}),
        )
        .await;
        assert_eq!(next_of_kind(&mut ws, KIND_ANSWER).await["id"], CALL_ID);
        assert_eq!(next_of_kind(&mut ws, KIND_ANSWER).await["id"], ACT_ID);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_act_stops_at_the_step_that_fails() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        send_json(
            &mut ws,
            json!({"kind": "act", "id": ACT_ID, "calls": [
                {"tool": TOOL_LIFT, "args": {}},
                {"tool": TOOL_OBSERVE, "args": {}}
            ]}),
        )
        .await;
        let answer = next_of_kind(&mut ws, KIND_ANSWER).await;
        assert_eq!(answer["id"], ACT_ID);
        assert_eq!(answer["ok"], false);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_empty_act_is_refused() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        send_json(&mut ws, json!({"kind": "act", "id": ACT_ID, "calls": []})).await;
        let answer = next_of_kind(&mut ws, KIND_ANSWER).await;
        assert_eq!(answer["ok"], false);
        assert_eq!(answer["error"], EMPTY_ACT);
    }

    fn observe_act(id: u64, steps: usize) -> Value {
        let calls: Vec<Value> = (0..steps)
            .map(|_| json!({"tool": TOOL_OBSERVE, "args": {}}))
            .collect();
        json!({"kind": "act", "id": id, "calls": calls})
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_too_long_act_is_refused() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        let started = Instant::now();
        send_json(&mut ws, observe_act(ACT_ID, MAX_ACT_STEPS + 1)).await;
        let answer = next_of_kind(&mut ws, KIND_ANSWER).await;
        assert!(started.elapsed() < Duration::from_millis(ACT_STEP_GAP_MS));
        assert_eq!(answer["ok"], false);
        assert_eq!(answer["error"], ACT_TOO_LONG);
    }

    /// Two pages on one session: the second act starts its first step
    /// only after the last step of the first.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_acts_of_one_session_run_one_at_a_time() {
        let server = serve_with_mock_session().await;
        let mut a = connect_live(server.addr, &server.id).await;
        let mut b = connect_live(server.addr, &server.id).await;
        let started = Instant::now();
        send_json(&mut a, observe_act(ACT_ID, TWO_STEPS)).await;
        send_json(&mut b, observe_act(ACT_ID, TWO_STEPS)).await;
        next_of_kind(&mut a, KIND_ANSWER).await;
        next_of_kind(&mut b, KIND_ANSWER).await;
        let both_acts = Duration::from_millis(ACT_STEP_GAP_MS * TWO_STEPS as u64);
        assert!(started.elapsed() >= both_acts, "{:?}", started.elapsed());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_act_past_the_waiting_ones_is_refused() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        let running_and_waiting = ACT_PLACES as u64;
        for id in 0..running_and_waiting {
            send_json(&mut ws, observe_act(id, TWO_STEPS)).await;
        }
        send_json(&mut ws, observe_act(running_and_waiting, TWO_STEPS)).await;
        let answer = next_of_kind(&mut ws, KIND_ANSWER).await;
        assert_eq!(answer["id"], running_and_waiting);
        assert_eq!(answer["ok"], false);
        assert_eq!(answer["error"], ACTS_WAITING);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_closed_link_still_finishes_a_started_act() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        next_json(&mut ws).await;
        send_json(
            &mut ws,
            json!({"kind": "act", "id": ACT_ID, "calls": [
                {"tool": TOOL_LIFT, "args": {"serial": MOCK_HATCHET, "amount": 1, ARG_HUMAN: true}},
                {"tool": TOOL_DROP, "args": {"serial": MOCK_HATCHET, "dest": MOCK_BACKPACK, ARG_HUMAN: true}}
            ]}),
        )
        .await;
        drop(ws);
        tokio::time::sleep(Duration::from_millis(ACT_STEP_GAP_MS * ACT_WAIT_GAPS)).await;
        assert!(server
            .shard
            .heard()
            .iter()
            .any(|packet| packet.first() == Some(&PKT_DROP)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_link_ends_with_its_session() {
        let server = serve_with_mock_session().await;
        let mut ws = connect_live(server.addr, &server.id).await;
        next_json(&mut ws).await;
        server.runtime.stop(&server.id).await.unwrap();
        next_of_kind(&mut ws, KIND_ENDED).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unknown_session_has_no_live_link() {
        let server = serve_with_mock_session().await;
        let refused = try_connect_live(server.addr, NO_SUCH_SESSION, &[]).await;
        assert_eq!(refused.err(), Some(StatusCode::NOT_FOUND.as_u16()));
    }
}
