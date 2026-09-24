//! What the client says to the shard as the character enters the world. A
//! headless session says what the Classic Client says, in the same order and
//! from the same client versions, as the reference client does it: at login
//! confirm (`0x1B`) and at login complete (`0x55`). It also answers each
//! version request of the shard, at login and in the world.
//!
//! At login confirm: the game view size (`0xBF` `0x05`) and the language
//! (`0xBF` `0x0B`) from 2.0.0, the client version (`0xBD`), a click on the
//! character (`0x09`), the skill request (`0x34`), and the public house
//! content choice (`0xFB`) from 7.0.79.6.
//!
//! At the first login complete: the status request (`0x34`), the chat
//! (`0xB5`), the skill request again, the client type (`0xBF` `0x0F`) from
//! 3.0.0e, and the view range (`0xC8`) from 3.0.5d. One second later the
//! character is double clicked, which opens his paperdoll.

use super::*;
use uoterm_world::GameView;

/// The reference client uses the character this long after its world
/// screen opens.
const SELF_OPEN_DELAY: Duration = Duration::from_secs(1);
/// The public house content choice the reference client starts with.
const SHOW_HOUSE_CONTENT_DEFAULT: bool = false;
/// The chat opens under no name: the shard takes the character's.
const CHAT_NAME_NONE: &str = "";
const ARG_WIDTH: &str = "width";
const ARG_HEIGHT: &str = "height";
const NEEDS_SIZE: &str = "needs width and height: the game view size in pixels";

/// What the session keeps about the login talk of one connection.
#[derive(Debug)]
pub(super) struct Arrival {
    /// The shard sent login complete; a later one says nothing.
    completed: bool,
    /// When the character is double clicked, once.
    open_self_at: Option<Instant>,
    /// The view range to ask for: the largest, until the shard names one.
    view_range: u8,
}

impl Default for Arrival {
    fn default() -> Self {
        Self {
            completed: false,
            open_self_at: None,
            view_range: CLIENT_VIEW_RANGE_MAX,
        }
    }
}

/// Answers the packets of the shard that the login talk follows.
pub(super) fn on_packet(inner: &mut Inner, msg: &Inbound, now: Instant) {
    match msg {
        Inbound::VersionRequest => {
            inner
                .outbound
                .push_back(encode::client_version(inner.version));
        }
        Inbound::LoginConfirm { serial, .. } => login_confirmed(inner, *serial),
        Inbound::LoginComplete if !inner.arrival.completed => login_completed(inner, now),
        Inbound::ViewRange { tiles } => inner.arrival.view_range = *tiles,
        _ => {}
    }
}

fn login_confirmed(inner: &mut Inner, me: Serial) {
    let version = inner.version;
    if version.reports_game_view() {
        let view = inner.world.read().game_view;
        inner
            .outbound
            .push_back(encode::game_window_size(view.width, view.height));
        inner.outbound.push_back(encode::language(LANGUAGE_ENU));
    }
    inner.outbound.push_back(encode::client_version(version));
    inner.outbound.push_back(encode::single_click(me));
    inner.outbound.push_back(encode::query_skills(me));
    if version.reports_house_content() {
        inner
            .outbound
            .push_back(encode::public_house_content(SHOW_HOUSE_CONTENT_DEFAULT));
    }
}

fn login_completed(inner: &mut Inner, now: Instant) {
    let me = inner.world.read().self_state.serial;
    if !me.is_valid() {
        return;
    }
    inner.arrival.completed = true;
    inner.outbound.push_back(encode::query_status(me));
    inner.outbound.push_back(encode::chat_open(CHAT_NAME_NONE));
    inner.outbound.push_back(encode::query_skills(me));
    inner.arrival.open_self_at = Some(now + SELF_OPEN_DELAY);
    if inner.version.reports_client_type() {
        inner.outbound.push_back(encode::client_type(inner.version));
    }
    if inner.version.reports_view_range() {
        inner
            .outbound
            .push_back(encode::view_range(inner.arrival.view_range));
    }
}

/// Double clicks the character once the delay after login complete is over.
pub(super) fn pump(inner: &mut Inner, now: Instant) {
    if inner.arrival.open_self_at.is_none_or(|at| now < at) {
        return;
    }
    inner.arrival.open_self_at = None;
    let me = inner.world.read().self_state.serial;
    inner
        .outbound
        .push_back(encode::double_click(Serial(me.0 | PAPERDOLL_REQUEST_BIT)));
}

/// `game_view`: the window says the size of the game view it draws. The
/// shard hears a new size at once when the character is in the world, and
/// every size at the next login.
pub(super) fn game_view(inner: &mut Inner, args: &Value) -> ToolResult {
    let size = |key: &str| {
        args.get(key)
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
    };
    let (Some(width), Some(height)) = (size(ARG_WIDTH), size(ARG_HEIGHT)) else {
        return ToolResult::err(NEEDS_SIZE);
    };
    let view = GameView { width, height };
    let (changed, in_world) = {
        let mut world = inner.world.write();
        let changed = world.game_view != view;
        world.game_view = view;
        (changed, world.logged_in)
    };
    if changed && in_world && inner.version.reports_game_view() {
        inner
            .outbound
            .push_back(encode::game_window_size(width, height));
    }
    ToolResult::ok(json!({ ARG_WIDTH: width, ARG_HEIGHT: height }))
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::test_session;
    use super::*;
    use uoterm_world::{GAME_VIEW_DEFAULT_HEIGHT, GAME_VIEW_DEFAULT_WIDTH};

    const ME: Serial = Serial(0x0000_00AA);
    const SHARD_RANGE: u8 = 18;

    fn session(version: ClientVersion) -> Inner {
        let mut inner = test_session();
        inner.version = version;
        inner.outbound.clear();
        inner
    }

    fn confirm() -> Inbound {
        Inbound::LoginConfirm {
            serial: ME,
            body: 0x0190,
            x: 1,
            y: 1,
            z: 0,
            direction: 0,
            map_width: MAP_DEFAULT_WIDTH,
            map_height: MAP_DEFAULT_HEIGHT,
        }
    }

    /// Feeds the packets as the session reads them: the world takes each
    /// one, then the login talk answers it.
    fn arrive(inner: &mut Inner, packets: &[Inbound], now: Instant) -> Vec<Vec<u8>> {
        for msg in packets {
            inner.world.write().apply(msg);
            on_packet(inner, msg, now);
        }
        inner.outbound.drain(..).collect()
    }

    #[test]
    fn a_modern_client_says_everything_in_the_reference_order() {
        let mut inner = session(ClientVersion::MODERN);
        inner.world.write().game_view = GameView::default();
        let now = Instant::now();
        let range = Inbound::ViewRange { tiles: SHARD_RANGE };
        let sent = arrive(&mut inner, &[confirm(), range, Inbound::LoginComplete], now);
        assert_eq!(
            sent,
            vec![
                encode::game_window_size(GAME_VIEW_DEFAULT_WIDTH, GAME_VIEW_DEFAULT_HEIGHT),
                encode::language(LANGUAGE_ENU),
                encode::client_version(ClientVersion::MODERN),
                encode::single_click(ME),
                encode::query_skills(ME),
                encode::public_house_content(false),
                encode::query_status(ME),
                encode::chat_open(""),
                encode::query_skills(ME),
                encode::client_type(ClientVersion::MODERN),
                encode::view_range(SHARD_RANGE),
            ]
        );
        pump(&mut inner, now + SELF_OPEN_DELAY / 2);
        assert!(inner.outbound.is_empty(), "the double click waits");
        pump(&mut inner, now + SELF_OPEN_DELAY);
        assert_eq!(
            inner.outbound.drain(..).collect::<Vec<_>>(),
            vec![encode::double_click(Serial(ME.0 | PAPERDOLL_REQUEST_BIT))]
        );
        pump(&mut inner, now + SELF_OPEN_DELAY * 2);
        assert!(inner.outbound.is_empty(), "the double click goes once");
        let again = arrive(&mut inner, &[Inbound::LoginComplete], now);
        assert!(again.is_empty(), "a second login complete says nothing");
    }

    #[test]
    fn a_t2a_client_leaves_out_what_came_later() {
        let mut inner = session(ClientVersion::T2A);
        let sent = arrive(
            &mut inner,
            &[confirm(), Inbound::LoginComplete],
            Instant::now(),
        );
        let view = GameView::default();
        assert_eq!(
            sent,
            vec![
                encode::game_window_size(view.width, view.height),
                encode::language(LANGUAGE_ENU),
                encode::client_version(ClientVersion::T2A),
                encode::single_click(ME),
                encode::query_skills(ME),
                encode::query_status(ME),
                encode::chat_open(""),
                encode::query_skills(ME),
            ]
        );
    }

    #[test]
    fn a_client_before_t2a_says_no_view_size_or_language() {
        let mut inner = session(ClientVersion::new(1, 26, 4, 0));
        let sent = arrive(&mut inner, &[confirm()], Instant::now());
        assert_eq!(sent[0], encode::client_version(inner.version));
    }

    #[test]
    fn with_no_word_from_the_shard_the_largest_range_is_asked() {
        let mut inner = session(ClientVersion::MODERN);
        let sent = arrive(
            &mut inner,
            &[confirm(), Inbound::LoginComplete],
            Instant::now(),
        );
        assert_eq!(
            sent.last(),
            Some(&encode::view_range(CLIENT_VIEW_RANGE_MAX))
        );
    }

    #[test]
    fn every_version_request_is_answered() {
        let mut inner = session(ClientVersion::MODERN);
        let sent = arrive(&mut inner, &[Inbound::VersionRequest], Instant::now());
        assert_eq!(sent, vec![encode::client_version(ClientVersion::MODERN)]);
    }

    #[test]
    fn the_window_view_size_goes_to_the_shard_once_it_changes() {
        const WIDTH: u32 = 1024;
        const HEIGHT: u32 = 700;
        let mut inner = session(ClientVersion::MODERN);
        inner.world.write().logged_in = false;
        let size = json!({ ARG_WIDTH: WIDTH, ARG_HEIGHT: HEIGHT });
        assert!(game_view(&mut inner, &size).ok);
        assert!(inner.outbound.is_empty(), "out of the world it waits");
        let sent = arrive(&mut inner, &[confirm()], Instant::now());
        assert_eq!(sent[0], encode::game_window_size(WIDTH, HEIGHT));
        assert!(game_view(&mut inner, &size).ok);
        assert!(inner.outbound.is_empty(), "the same size says nothing");
        let wider = json!({ ARG_WIDTH: WIDTH + 1, ARG_HEIGHT: HEIGHT });
        assert!(game_view(&mut inner, &wider).ok);
        assert_eq!(
            inner.outbound.drain(..).collect::<Vec<_>>(),
            vec![encode::game_window_size(WIDTH + 1, HEIGHT)]
        );
        assert!(!game_view(&mut inner, &json!({ ARG_WIDTH: WIDTH })).ok);
    }
}
