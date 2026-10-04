//! The login link: a WebSocket a web page logs in with.
//!
//! The page sends the login first. The link starts the session with a
//! [`LoginPicker`], and sends each question of the login to the page as a
//! [`LoginAsk`](uoterm_world::login::LoginAsk), with the client version the
//! login speaks, which a new character follows. The page answers each with
//! a [`LoginReply`]. At the end the link sends the id of the new session,
//! which the page then opens the live link of, or the words of the fault.
//!
//! The password is in the first message only. The link hands it to the
//! session and keeps it nowhere.
//!
//! A page that closes before the end plays no character: the open question
//! of the character list is answered with [`CharacterRequest::Leave`]. A
//! shard pick is never guessed for the page: its question is let go, and
//! the login ends. A session the login still makes is stopped.
//!
//! The words of a message the link cannot read name only the kind of the
//! fault and where it is, never the text, which may hold the password.

use super::live::LIVE_SEND_TIMEOUT;
use super::ApiState;
use crate::config::{ConnectOptions, LoginPicker, LoginQuestion, ScreenLogin};
use crate::error::Result;
use crate::manager::Runtime;
use crate::session::SessionHandle;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures_util::SinkExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uoterm_world::login::{CharacterRequest, LoginReply};

/// Where a page opens the login link.
pub(super) const LOGIN_PATH: &str = "/v1/login/live";
/// The largest message a page may send on the login link. A new character
/// fits many times over.
const LOGIN_MAX_MESSAGE_BYTES: usize = 64 * 1024;
const KIND_ASK: &str = "ask";
const KIND_READY: &str = "ready";
const KIND_FAILED: &str = "failed";
const LOGIN_FIRST: &str = "the first message must be a login";
/// How long the link waits for the login of a page that opened it.
const LOGIN_FIRST_WAIT: Duration = Duration::from_secs(10);
const LOGIN_LATE: &str = "no login came in time";

/// What the page sends.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FromPage {
    /// Where to log in and as whom. Only the first message may be one.
    Login(ScreenLogin),
    /// The answer to the question the link sent last.
    Reply { reply: LoginReply },
}

pub(super) async fn login(ws: WebSocketUpgrade, State(st): State<Arc<ApiState>>) -> Response {
    ws.max_message_size(LOGIN_MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| run_login(socket, st))
}

/// Reads the login, then carries the questions of the login to the page
/// and its replies back, until the login ends.
async fn run_login(socket: WebSocket, st: Arc<ApiState>) {
    let mut page = PageLogin {
        socket,
        here: true,
        open: None,
        version: String::new(),
    };
    let first = tokio::time::timeout(LOGIN_FIRST_WAIT, next_from_page(&mut page.socket)).await;
    let login = match first {
        Ok(Some(Ok(FromPage::Login(login)))) => login,
        Ok(Some(Ok(FromPage::Reply { .. }))) => return page.fail(LOGIN_FIRST.into()).await,
        Ok(Some(Err(error))) => {
            return page
                .fail(format!("{LOGIN_FIRST}: {}", fault_place(&error)))
                .await
        }
        Ok(None) => return,
        Err(_) => return page.fail(LOGIN_LATE.into()).await,
    };
    let (asks, mut questions) = mpsc::unbounded_channel();
    let opts = ConnectOptions::for_screen(login, &st.config, LoginPicker(asks));
    page.version = opts.version.to_string();
    let connecting = st.runtime.connect(opts);
    tokio::pin!(connecting);
    let ended = loop {
        tokio::select! {
            ended = &mut connecting => break ended,
            Some(question) = questions.recv() => page.ask(question).await,
            message = next_from_page(&mut page.socket), if page.here => page.read(message).await,
        }
    };
    page.end(ended, &st.runtime).await;
}

/// The next message of the page. None when the page is gone.
async fn next_from_page(
    socket: &mut WebSocket,
) -> Option<std::result::Result<FromPage, serde_json::Error>> {
    loop {
        match socket.recv().await? {
            Ok(Message::Text(text)) => return Some(serde_json::from_str(&text)),
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_)) => {}
        }
    }
}

/// One login a page drives.
struct PageLogin {
    socket: WebSocket,
    /// False once the page is gone.
    here: bool,
    /// The question the page has not answered yet.
    open: Option<LoginQuestion>,
    /// The client version the login speaks, as the page reads it.
    version: String,
}

impl PageLogin {
    /// Sends a question to the page, which is left open until the page
    /// answers. A page that is gone does not answer.
    async fn ask(&mut self, question: LoginQuestion) {
        if self.here
            && self
                .tell(json!({ "kind": KIND_ASK, "ask": question.ask(), "version": self.version }))
                .await
        {
            self.open = Some(question);
        } else {
            self.here = false;
            leave(question);
        }
    }

    /// Takes one message of the page. A reply that does not fit the open
    /// question is not taken, and the question goes to the page again.
    async fn read(&mut self, message: Option<std::result::Result<FromPage, serde_json::Error>>) {
        match message {
            None => {
                self.here = false;
                if let Some(question) = self.open.take() {
                    leave(question);
                }
            }
            Some(Ok(FromPage::Reply { reply })) => match self.open.take() {
                Some(question) => {
                    if let Err(question) = question.answer(reply) {
                        self.ask(question).await;
                    }
                }
                None => tracing::debug!("the login link read a reply to no question"),
            },
            Some(Ok(FromPage::Login(_))) => {
                tracing::debug!("the login link read a second login");
            }
            Some(Err(error)) => {
                let fault = fault_place(&error);
                tracing::debug!(fault, "the login link read a message it does not know");
            }
        }
    }

    /// Tells the page how the login ended, and closes the link. A session
    /// the page cannot learn of is stopped: nobody would play it.
    async fn end(mut self, ended: Result<SessionHandle>, runtime: &Runtime) {
        match ended {
            Ok(handle) => {
                let ready = json!({ "kind": KIND_READY, "session": &handle.id });
                if !(self.here && self.tell(ready).await) {
                    let _ = runtime.stop(&handle.id).await;
                }
                let _ = self.socket.close().await;
            }
            Err(error) => self.fail(error.to_string()).await,
        }
    }

    /// Tells the page the words of the fault, and closes the link.
    async fn fail(mut self, words: String) {
        if self.here {
            self.tell(json!({ "kind": KIND_FAILED, "words": words }))
                .await;
        }
        let _ = self.socket.close().await;
    }

    /// Sends one message to the page. False when the page did not take it.
    async fn tell(&mut self, letter: Value) -> bool {
        let sent = tokio::time::timeout(
            LIVE_SEND_TIMEOUT,
            self.socket.send(Message::Text(letter.to_string().into())),
        );
        matches!(sent.await, Ok(Ok(())))
    }
}

/// The kind of a fault in a message of the page and where it is. The text
/// of the fault is left out: it may quote what the page sent.
fn fault_place(error: &serde_json::Error) -> String {
    format!(
        "{:?} fault at line {}, column {}",
        error.classify(),
        error.line(),
        error.column()
    )
    .to_lowercase()
}

/// Ends the login at the character list, for a page that is gone. A
/// shard question is let go unanswered, which ends the login as well.
fn leave(question: LoginQuestion) {
    let _ = question.answer(LoginReply::Request {
        request: CharacterRequest::Leave,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::live::tests::{
        next_json, send_json, serve_api, try_connect, ServerTask, WsClient,
    };
    use crate::manager::Runtime;
    use crate::mock::test_login::SECOND_CHAR;
    use crate::mock::{MockServer, MOCK_CHAR};
    use serde_json::{json, Value};
    use std::net::SocketAddr;
    use std::time::Duration;
    use uoterm_protocol::types::{ClientVersion, PKT_PLAY_CHARACTER};

    const KIND_LOGIN: &str = "login";
    const KIND_REPLY: &str = "reply";
    const TEST_SESSIONS: usize = 1;
    const ANY_LOCAL_PORT: &str = "127.0.0.1:0";
    /// The first slot of the account, which a login that guesses plays.
    const FIRST_SLOT: usize = 0;
    /// A free slot of the mock account for a new character.
    const FREE_SLOT: u16 = 1;
    /// Long enough for a login the page left to reach the world, if it
    /// went on.
    const LEFT_LOGIN_WAIT_MS: u64 = 1500;

    async fn serve_runtime() -> (SocketAddr, ServerTask) {
        serve_api(Runtime::new(TEST_SESSIONS), None, true).await
    }

    async fn connect(addr: SocketAddr, path: &str) -> WsClient {
        try_connect(addr, path, &[]).await.unwrap()
    }

    fn login_to(shard: SocketAddr) -> Value {
        json!({"kind": KIND_LOGIN, "host": shard.ip().to_string(), "port": shard.port(),
            "account": "test", "password": "test", "shard": null, "character": null,
            "era": "t2a", "version": null})
    }

    /// The login link, its login sent to `shard`, and the first answer.
    async fn begin_login(addr: SocketAddr, shard: SocketAddr) -> (WsClient, Value) {
        let mut ws = connect(addr, LOGIN_PATH).await;
        send_json(&mut ws, login_to(shard)).await;
        let first = next_json(&mut ws).await;
        (ws, first)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_login_asks_for_the_character_and_ends_ready() {
        let shard = MockServer::start().await.unwrap();
        let (addr, _server) = serve_runtime().await;
        let (mut ws, ask) = begin_login(addr, shard.addr).await;
        assert_eq!(ask["kind"], KIND_ASK);
        assert_eq!(ask["ask"]["kind"], "Characters");
        // A new character follows the client version the login speaks.
        assert_eq!(ask["version"], ClientVersion::T2A.to_string());
        let slot = ask["ask"]["names"]
            .as_array()
            .unwrap()
            .iter()
            .position(|name| name == MOCK_CHAR)
            .unwrap();
        send_json(
            &mut ws,
            json!({"kind": KIND_REPLY, "reply": {"kind": "Request", "request": {"Play": slot}}}),
        )
        .await;
        let end = next_json(&mut ws).await;
        assert_eq!(end["kind"], KIND_READY);
        assert!(end["session"].as_str().unwrap().starts_with('s'));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_taken_name_comes_back_as_a_refusal() {
        let shard = MockServer::start().await.unwrap();
        let (addr, _server) = serve_runtime().await;
        let (mut ws, _) = begin_login(addr, shard.addr).await;
        let wish = json!({"name": MOCK_CHAR, "female": false, "race": 1, "strength": 60,
            "dexterity": 10, "intelligence": 10, "skills": [[0, 50], [1, 50], [2, 0]],
            "skin_hue": 1002, "hair": 0, "hair_hue": 0, "beard": 0, "beard_hue": 0,
            "shirt_hue": 0, "pants_hue": 0, "profession": 0, "start_city": 0, "slot": FREE_SLOT});
        send_json(
            &mut ws,
            json!({"kind": KIND_REPLY, "reply": {"kind": "Request", "request": {"Make": wish}}}),
        )
        .await;
        let again = next_json(&mut ws).await;
        assert_eq!(again["ask"]["kind"], "Characters");
        assert!(again["ask"]["refused"].is_string());
    }

    /// A reply that does not fit the question is not taken: the link asks
    /// again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_reply_of_the_wrong_kind_brings_the_question_again() {
        let shard = MockServer::start().await.unwrap();
        let (addr, _server) = serve_runtime().await;
        let (mut ws, ask) = begin_login(addr, shard.addr).await;
        send_json(
            &mut ws,
            json!({"kind": KIND_REPLY, "reply": {"kind": "Pick", "index": FIRST_SLOT}}),
        )
        .await;
        assert_eq!(next_json(&mut ws).await, ask);
    }

    /// A page that closes at the character list plays no character, not
    /// even the first of an account of two: the login leaves the list, and
    /// no session is left behind.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_closed_page_ends_the_login_at_the_character_list() {
        let shard = MockServer::start_with_characters(&[MOCK_CHAR, SECOND_CHAR])
            .await
            .unwrap();
        let runtime = Runtime::new(TEST_SESSIONS);
        let (addr, _server) = serve_api(runtime.clone(), None, true).await;
        let (ws, ask) = begin_login(addr, shard.addr).await;
        assert_eq!(ask["ask"]["kind"], "Characters");
        drop(ws);
        tokio::time::sleep(Duration::from_millis(LEFT_LOGIN_WAIT_MS)).await;
        assert!(runtime.list().is_empty());
        assert!(!shard
            .heard()
            .iter()
            .any(|packet| packet.first() == Some(&PKT_PLAY_CHARACTER)));
    }

    /// One answer plays the slot it names, on an account of two: the page
    /// is asked nothing more.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_play_answer_plays_its_slot() {
        let shard = MockServer::start_with_characters(&[MOCK_CHAR, SECOND_CHAR])
            .await
            .unwrap();
        let runtime = Runtime::new(TEST_SESSIONS);
        let (addr, _server) = serve_api(runtime.clone(), None, true).await;
        let (mut ws, ask) = begin_login(addr, shard.addr).await;
        let names = ask["ask"]["names"].as_array().unwrap();
        let slot = names.iter().position(|name| name == SECOND_CHAR).unwrap();
        send_json(
            &mut ws,
            json!({"kind": KIND_REPLY, "reply": {"kind": "Request", "request": {"Play": slot}}}),
        )
        .await;
        let end = next_json(&mut ws).await;
        assert_eq!(end["kind"], KIND_READY);
        let session = runtime.get(end["session"].as_str().unwrap()).unwrap();
        assert_eq!(session.world.read().self_state.name, SECOND_CHAR);
    }

    /// A slot with no character in it is no answer: the link asks again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_slot_with_no_character_brings_the_question_again() {
        let shard = MockServer::start().await.unwrap();
        let (addr, _server) = serve_runtime().await;
        let (mut ws, ask) = begin_login(addr, shard.addr).await;
        let past_the_list = ask["ask"]["names"].as_array().unwrap().len();
        send_json(
            &mut ws,
            json!({"kind": KIND_REPLY, "reply": {"kind": "Request", "request": {"Play": past_the_list}}}),
        )
        .await;
        assert_eq!(next_json(&mut ws).await, ask);
    }

    /// The words of a login the link cannot read never quote it: a bad
    /// value may be the password.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unreadable_login_is_not_quoted_back() {
        const SECRET: u64 = 9_876_543_210;
        let (addr, _server) = serve_runtime().await;
        let mut ws = connect(addr, LOGIN_PATH).await;
        let mut login = login_to(addr);
        login["password"] = json!(SECRET);
        send_json(&mut ws, login).await;
        let end = next_json(&mut ws).await;
        assert_eq!(end["kind"], KIND_FAILED);
        let words = end["words"].as_str().unwrap();
        assert!(words.starts_with(LOGIN_FIRST), "{words}");
        assert!(!words.contains(&SECRET.to_string()), "{words}");
    }

    /// A page that opens the link and sends no login is let go.
    #[tokio::test(start_paused = true)]
    async fn a_page_that_sends_no_login_is_let_go() {
        let (addr, _server) = serve_runtime().await;
        let mut ws = connect(addr, LOGIN_PATH).await;
        let end = next_json(&mut ws).await;
        assert_eq!(end["kind"], KIND_FAILED);
        assert_eq!(end["words"], LOGIN_LATE);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_shard_that_does_not_answer_ends_the_login_failed() {
        let gone = tokio::net::TcpListener::bind(ANY_LOCAL_PORT)
            .await
            .unwrap()
            .local_addr()
            .unwrap();
        let (addr, _server) = serve_runtime().await;
        let (_ws, end) = begin_login(addr, gone).await;
        assert_eq!(end["kind"], KIND_FAILED);
        assert!(end["words"].is_string());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_first_message_must_be_a_login() {
        let (addr, _server) = serve_runtime().await;
        let mut ws = connect(addr, LOGIN_PATH).await;
        send_json(
            &mut ws,
            json!({"kind": KIND_REPLY, "reply": {"kind": "Pick", "index": FIRST_SLOT}}),
        )
        .await;
        let end = next_json(&mut ws).await;
        assert_eq!(end["kind"], KIND_FAILED);
        assert_eq!(end["words"], LOGIN_FIRST);
    }
}
