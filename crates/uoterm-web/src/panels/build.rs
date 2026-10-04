//! The house designer and the chat of the shard, as the Modern panels of
//! the Rust window show them. The designer shows the parts of the client
//! catalog; the player picks a part and clicks the house on the map, or
//! picks one off the house with the eyedropper, and Jev picks a part for
//! plain words. The chat shows every channel and the lines that were said.
//! What each button does is `uoterm_view::ui::build`, `ui::chat_panel`,
//! `model::house_design` and `model::chat`.

use super::sheet::Choice;
use super::{FrameSpec, Framed, NoteData, PANEL_BUILD, PANEL_CHANNELS};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::{Act, Asker};
use uoterm_view::frame::WatchFrame;
use uoterm_view::model::chat::{
    chat_door, chat_name_act, ChatDoor, CHAT_NAME_MAX_CHARS, WORDS_CHOOSE_NAME,
};
use uoterm_view::model::house_design::{limits_words, styles_of, HouseDesign, ACTION_EXIT, KINDS};
use uoterm_view::ui::build::{
    build_first_place, command_rows, counts_words, limits_of, part_ask, storey_words, storeys_of,
    take_part_answer, BUILD_ID, HINT_PICK, HINT_STOREY, HINT_WISH, NOTE_SECONDS, WORDS_ASKING,
    WORDS_FIND, WORDS_FLOOR, WORDS_NO_PARTS, WORDS_PICK, WORDS_REMOVE, WORDS_TITLE,
};
use uoterm_view::ui::chat_panel::{
    asking_label, channel_ask, chat_first_place, chat_title, say_act, ChatPanel, CHAT_BUTTONS,
    CHAT_ID, HINT_CHANNEL, HINT_CHANNEL_ROW, HINT_SAY, WORDS_CANCEL, WORDS_LOCK_MARK, WORDS_OKAY,
    WORDS_TURN_ON,
};

/// What the designer and the chat keep between frames.
#[derive(Default)]
pub(crate) struct BuildState {
    /// The part the player builds with and how each storey shows.
    pub design: HouseDesign,
    pub chat: ChatPanel,
    /// Words under each panel for a while: what Jev does, or why he
    /// could not.
    build_note: Option<(String, bool, f64)>,
    chat_note: Option<(String, bool, f64)>,
}

/// A picture of a piece of the style, and whether it is the one picked.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PieceCell {
    pub picture: Option<String>,
    pub chosen: bool,
}

/// A storey's button: its words, and how it shows (0 all shown, 1 some
/// see-through, 2 some hidden), with its tip.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StoreyButton {
    pub words: String,
    pub look: usize,
    pub tip: &'static str,
}

/// A count of the design: its words, and true at its most.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CountWords {
    pub words: String,
    pub alarm: bool,
}

/// The buttons of the designer, while the human has control.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DesignButtons {
    pub remove: Choice,
    /// The steps that change the design, then the one that leaves it.
    pub changes: Vec<&'static str>,
    /// The steps that keep the design and bring it back.
    pub kept: Vec<&'static str>,
    pub pick: Choice,
    pub pick_tip: &'static str,
    pub floor_words: &'static str,
    pub floors: Vec<Choice>,
    pub storeys: Vec<StoreyButton>,
}

/// The house designer.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BuildData {
    pub kinds: Vec<Choice>,
    pub styles: Vec<Choice>,
    pub no_parts: Option<&'static str>,
    pub pieces: Vec<PieceCell>,
    pub wish_hint: &'static str,
    pub find: &'static str,
    pub buttons: Option<DesignButtons>,
    pub counts: Vec<CountWords>,
    pub counts_tip: Option<String>,
    pub note: Option<NoteData>,
}

/// A channel of the chat.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChannelRow {
    pub name: String,
    pub picked: bool,
    /// The channel the character is in.
    pub here: bool,
    pub locked: Option<&'static str>,
}

/// The small box that asks for a channel name or a password.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AskingBox {
    pub label: &'static str,
    pub hides: bool,
    pub words: String,
    pub okay: &'static str,
    pub cancel: &'static str,
}

/// One line said in the channel.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChatLineData {
    pub who: String,
    pub words: String,
}

/// The chat on: its channels, its buttons, the lines and the fields.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChannelsData {
    pub rows: Vec<ChannelRow>,
    pub row_hint: &'static str,
    pub buttons: Vec<&'static str>,
    pub asking: Option<AskingBox>,
    pub lines: Vec<ChatLineData>,
    pub say_hint: &'static str,
    pub wish_hint: &'static str,
    pub find: &'static str,
}

/// The box that asks for the chat name before the chat opens.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NameBox {
    pub words: &'static str,
    pub most: usize,
    pub okay: &'static str,
}

/// The chat of the shard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChatPanelData {
    pub live: bool,
    pub channels: Option<ChannelsData>,
    pub name_box: Option<NameBox>,
    /// The button that asks the shard to turn the chat on.
    pub turn_on: Option<&'static str>,
    pub note: Option<NoteData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BuildAction {
    Kind(usize),
    Style(usize),
    Piece(usize),
    /// Plain words Jev picks a part for.
    Wish(String),
    Remove(bool),
    /// A step of the row that changes the design, by its place.
    Change(usize),
    /// A step of the row that keeps the design, by its place.
    Keep(usize),
    Pick(bool),
    Floor(u8),
    Storey(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ChatAction {
    /// A click on a channel.
    Pick(String),
    /// A double click on a channel.
    Join(String),
    /// A button under the channels, by its place.
    Button(usize),
    /// The words typed in the small box.
    Asking(String),
    /// Okay (true) or Cancel on the small box.
    Answer(bool),
    Say(String),
    /// Plain words Jev picks a channel for.
    Wish(String),
    /// The chat name typed for the shard.
    Name(String),
    TurnOn(bool),
}

/// The words of a note while they show at `time`.
fn note_at(note: &Option<(String, bool, f64)>, time: f64) -> Option<NoteData> {
    note.as_ref()
        .filter(|(_, _, since)| time - since <= NOTE_SECONDS)
        .map(|(words, failed, _)| NoteData {
            words: words.clone(),
            failed: *failed,
        })
}

impl WebView {
    /// The designer and the chat in one frame: the answers of Jev, the
    /// look of each storey on the map, and the chat that opens by itself.
    pub(crate) fn follow_build(&mut self, frame: &WatchFrame, time: f64) {
        for answer in self.hand.new_answers(Asker::Designer) {
            let state = &mut self.panels.build;
            match take_part_answer(&mut state.design, frame, answer) {
                Ok(true) => state.build_note = None,
                Ok(false) => {}
                Err(words) => state.build_note = Some((words, true, time)),
            }
        }
        for answer in self.hand.new_answers(Asker::Chat) {
            let state = &mut self.panels.build;
            match state.chat.take_answer(frame.chat.as_ref(), answer) {
                Ok(act) => {
                    if let Some(act) = act {
                        self.hand.act(act);
                    }
                }
                Err(words) => state.chat_note = Some((words, true, time)),
            }
        }
        self.panels.build.chat.follow(frame);
        self.scene
            .set_storey_looks(self.panels.build.design.storeys);
    }

    /// What a click on the house does while the designer is open, and the
    /// words beside the mouse.
    pub(crate) fn click_on_house(&mut self, frame: &WatchFrame, (x, y, z): (u16, u16, i8)) {
        let design = &mut self.panels.build.design;
        if let Some(act) = design.click_on_house(frame, i32::from(x), i32::from(y), i32::from(z)) {
            self.hand.act(act);
        }
    }

    pub(super) fn build_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        frame.designing?;
        let spec = FrameSpec::fixed(BUILD_ID, WORDS_TITLE, build_first_place(self.panel_room()));
        Some(spec.closable_if(frame.human_control))
    }

    pub(super) fn build_data(
        &mut self,
        frame: &WatchFrame,
        time: f64,
    ) -> Option<Framed<BuildData>> {
        let designing = frame.designing?;
        let spec = self.build_spec(frame)?;
        let styles = styles_of(&frame.house_parts, self.panels.build.design.kind);
        let style = self
            .panels
            .build
            .design
            .style
            .min(styles.len().saturating_sub(1));
        self.panels.build.design.style = style;
        let design = self.panels.build.design;
        let pieces: Vec<u16> = styles
            .get(style)
            .map(|part| part.pieces.clone())
            .unwrap_or_default();
        let pieces = pieces
            .iter()
            .enumerate()
            .map(|(at, graphic)| {
                let request = self.item_picture_request(*graphic, 0);
                PieceCell {
                    picture: Some(self.picture_key(&request)),
                    chosen: at == design.piece,
                }
            })
            .collect();
        let limits = limits_of(&designing);
        let storeys = storeys_of(limits);
        let buttons = frame.human_control.then(|| {
            let [changes, kept] = command_rows();
            DesignButtons {
                remove: Choice {
                    words: WORDS_REMOVE.to_string(),
                    chosen: design.removing,
                },
                changes: changes.iter().map(|(words, _)| *words).collect(),
                kept: kept.iter().map(|(words, _)| *words).collect(),
                pick: Choice {
                    words: WORDS_PICK.to_string(),
                    chosen: design.picking,
                },
                pick_tip: HINT_PICK,
                floor_words: WORDS_FLOOR,
                floors: (1..=storeys)
                    .map(|level| Choice {
                        words: level.to_string(),
                        chosen: level == designing.floor,
                    })
                    .collect(),
                storeys: (0..usize::from(storeys))
                    .map(|storey| StoreyButton {
                        words: storey_words(storey),
                        look: design.storeys[storey].button_look(),
                        tip: HINT_STOREY,
                    })
                    .collect(),
            }
        });
        let body = BuildData {
            kinds: KINDS
                .iter()
                .map(|(kind, _)| Choice {
                    words: kind.word().to_string(),
                    chosen: *kind == design.kind,
                })
                .collect(),
            styles: styles
                .iter()
                .enumerate()
                .map(|(at, part)| Choice {
                    words: part.name.clone(),
                    chosen: at == style,
                })
                .collect(),
            no_parts: styles.is_empty().then_some(WORDS_NO_PARTS),
            pieces,
            wish_hint: HINT_WISH,
            find: WORDS_FIND,
            buttons,
            counts: limits
                .map(|limits| {
                    counts_words(frame, limits)
                        .into_iter()
                        .map(|(words, alarm)| CountWords { words, alarm })
                        .collect()
                })
                .unwrap_or_default(),
            counts_tip: limits.map(limits_words),
            note: note_at(&self.panels.build.build_note, time),
        };
        Some(self.framed(PANEL_BUILD, &spec, body))
    }

    pub(super) fn build_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Some(designing) = frame.designing else {
            return;
        };
        let Ok(action) = serde_json::from_value::<BuildAction>(action) else {
            return;
        };
        let live = frame.human_control;
        let time = self.hand.time();
        let design = &mut self.panels.build.design;
        let act = match action {
            BuildAction::Kind(at) => {
                if let Some((kind, _)) = KINDS.get(at) {
                    design.set_kind(*kind);
                }
                None
            }
            BuildAction::Style(at) => {
                if at < styles_of(&frame.house_parts, design.kind).len() {
                    design.set_style(at);
                }
                None
            }
            BuildAction::Piece(at) => {
                let pieces = styles_of(&frame.house_parts, design.kind)
                    .get(design.style)
                    .map_or(0, |part| part.pieces.len());
                if at < pieces {
                    design.piece = at;
                }
                None
            }
            BuildAction::Wish(words) => {
                if let Some(ask) = part_ask(&frame, &words) {
                    self.hand.ask(Asker::Designer, ask);
                    self.panels.build.build_note = Some((WORDS_ASKING.to_string(), false, time));
                }
                None
            }
            BuildAction::Remove(_) if live => {
                design.toggle_removing();
                None
            }
            BuildAction::Pick(_) if live => {
                design.toggle_picking();
                None
            }
            BuildAction::Change(at) if live => command_rows()[0]
                .get(at)
                .map(|(_, action)| Act::HouseCommand(action)),
            BuildAction::Keep(at) if live => command_rows()[1]
                .get(at)
                .map(|(_, action)| Act::HouseCommand(action)),
            BuildAction::Floor(level)
                if live && (1..=storeys_of(limits_of(&designing))).contains(&level) =>
            {
                Some(design.go_to_floor(level))
            }
            BuildAction::Storey(storey) if live => {
                design.turn_storey(storey);
                None
            }
            _ => None,
        };
        if let Some(act) = act {
            self.hand.act(act);
        }
    }

    /// The close mark of the designer leaves it.
    pub(super) fn close_build(&mut self) {
        if self.frame.as_ref().is_some_and(|frame| frame.human_control) {
            self.hand.act(Act::HouseCommand(ACTION_EXIT));
        }
    }

    pub(super) fn channels_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        if !self.panels.build.chat.open {
            return None;
        }
        let (default, least) = chat_first_place(self.panel_room());
        let spec = FrameSpec::fixed(CHAT_ID, &chat_title(frame), default);
        Some(spec.closable().sized(least))
    }

    pub(super) fn channels_data(
        &self,
        frame: &WatchFrame,
        time: f64,
    ) -> Option<Framed<ChatPanelData>> {
        let spec = self.channels_spec(frame)?;
        let live = frame.human_control;
        let panel = &self.panels.build.chat;
        let door = chat_door(frame);
        let channels = frame.chat.as_ref().map(|chat| ChannelsData {
            rows: chat
                .channels
                .iter()
                .map(|(name, locked)| ChannelRow {
                    name: name.clone(),
                    picked: panel.picked.as_deref() == Some(name.as_str()),
                    here: *name == chat.in_channel,
                    locked: locked.then_some(WORDS_LOCK_MARK),
                })
                .collect(),
            row_hint: HINT_CHANNEL_ROW,
            buttons: if live {
                CHAT_BUTTONS.iter().map(|button| button.words()).collect()
            } else {
                Vec::new()
            },
            asking: panel
                .asking
                .as_ref()
                .filter(|_| live)
                .map(|(asking, words)| AskingBox {
                    label: asking_label(asking),
                    hides: asking.hides_words(),
                    words: words.clone(),
                    okay: WORDS_OKAY,
                    cancel: WORDS_CANCEL,
                }),
            lines: chat
                .lines
                .iter()
                .map(|(who, words)| ChatLineData {
                    who: who.clone(),
                    words: words.clone(),
                })
                .collect(),
            say_hint: HINT_SAY,
            wish_hint: HINT_CHANNEL,
            find: WORDS_FIND,
        });
        let body = ChatPanelData {
            live,
            channels,
            name_box: (door == ChatDoor::AskName).then_some(NameBox {
                words: WORDS_CHOOSE_NAME,
                most: CHAT_NAME_MAX_CHARS,
                okay: WORDS_OKAY,
            }),
            turn_on: matches!(door, ChatDoor::TurnOn(_))
                .then_some(WORDS_TURN_ON)
                .filter(|_| live),
            note: note_at(&self.panels.build.chat_note, time),
        };
        Some(self.framed(PANEL_CHANNELS, &spec, body))
    }

    pub(super) fn channels_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<ChatAction>(action) else {
            return;
        };
        let live = frame.human_control;
        let time = self.hand.time();
        let state = &mut self.panels.build;
        let chat = frame.chat.as_ref();
        let act = match (action, chat) {
            (ChatAction::Pick(name), Some(_)) if live => {
                state.chat.picked = Some(name);
                None
            }
            (ChatAction::Join(name), Some(chat)) if live => state.chat.join(chat, name),
            (ChatAction::Button(at), Some(chat)) if live => CHAT_BUTTONS
                .get(at)
                .and_then(|button| state.chat.press(chat, *button)),
            (ChatAction::Asking(words), _) => {
                if let Some((_, typed)) = state.chat.asking.as_mut() {
                    *typed = words;
                }
                None
            }
            (ChatAction::Answer(okay), _) if live => state.chat.answer_box(okay),
            (ChatAction::Say(words), Some(_)) if live => say_act(&words),
            (ChatAction::Wish(words), Some(chat)) if live => {
                if let Some(ask) = channel_ask(chat, &words) {
                    self.hand.ask(Asker::Chat, ask);
                    state.chat_note = Some((WORDS_ASKING.to_string(), false, time));
                }
                None
            }
            (ChatAction::Name(words), None) if live && frame.chat_asks_for_name => {
                chat_name_act(&words)
            }
            (ChatAction::TurnOn(_), None) if live => match chat_door(&frame) {
                ChatDoor::TurnOn(act) => Some(act),
                _ => None,
            },
            _ => None,
        };
        if let Some(act) = act {
            self.hand.act(act);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, view_with, view_with_all};
    use super::*;
    use crate::out::OutCall;
    use crate::tests::VIEW;
    use serde_json::json;
    use uoterm_view::model::chat::Asking;
    use uoterm_view::ui::chat_panel::ChatButton;

    const HOUSE: u32 = 70;
    const ROOF: usize = 4;

    fn designing(control: bool) -> WebView {
        view_with_all(
            &[
                (
                    "designing",
                    json!({ "serial": HOUSE, "floor": 1, "plot_width": 7, "plot_depth": 8 }),
                ),
                (
                    "house_parts",
                    json!([
                        { "kind": "wall", "name": "Stone", "pieces": [20, 22] },
                        { "kind": "roof", "name": "Tile Roof", "pieces": [11314] }
                    ]),
                ),
            ],
            control,
        )
    }

    #[test]
    fn a_click_on_the_house_builds_the_part_picked_as_the_window() {
        let mut view = designing(true);
        press(&mut view, PANEL_BUILD, json!({ "piece": 1 }));
        let frame = view.frame_ref().unwrap().clone();
        view.click_on_house(&frame, (3, 4, 0));
        let mut same = HouseDesign {
            piece: 1,
            ..HouseDesign::default()
        };
        let built = same.click_on_house(&frame, 3, 4, 0).unwrap();
        assert!(matches!(built, Act::HouseEdit { graphic: 22, .. }));
        assert_eq!(out_acts(&view.take_out_native()), vec![built.for_page()]);
        press(&mut view, PANEL_BUILD, json!({ "kind": ROOF }));
        let data = view.panel_data(0.0).build.unwrap().body;
        assert!(data.kinds[ROOF].chosen);
        assert_eq!(data.styles[0].words, "Tile Roof");
        assert_eq!(data.counts.len(), 3);
    }

    #[test]
    fn the_steps_of_the_designer_are_the_windows_and_close_leaves() {
        let mut view = designing(true);
        let [changes, kept] = command_rows();
        let out = press(&mut view, PANEL_BUILD, json!({ "change": 0 }));
        assert_eq!(
            out_acts(&out),
            vec![Act::HouseCommand(changes[0].1).for_page()]
        );
        let out = press(&mut view, PANEL_BUILD, json!({ "keep": 0 }));
        assert_eq!(
            out_acts(&out),
            vec![Act::HouseCommand(kept[0].1).for_page()]
        );
        press(&mut view, PANEL_BUILD, json!({ "storey": 0 }));
        let look = view.panel_data(0.0).build.unwrap().body.buttons.unwrap();
        assert_eq!(look.storeys[0].look, 1, "see-through");
        let out = press(&mut view, PANEL_BUILD, json!({ "floor": 2 }));
        assert_eq!(out_acts(&out), vec![Act::HouseFloor(2).for_page()]);
        let out = press(&mut view, PANEL_BUILD, json!({ "close": true }));
        assert_eq!(
            out_acts(&out),
            vec![Act::HouseCommand(ACTION_EXIT).for_page()]
        );
    }

    #[test]
    fn jev_picks_a_part_for_plain_words() {
        let mut view = designing(true);
        let out = press(&mut view, PANEL_BUILD, json!({ "wish": "a roof" }));
        let [OutCall::Jev { id, .. }] = out.as_slice() else {
            panic!("{out:?}");
        };
        view.answer_native(*id, true, &json!({ "index": 1 }).to_string(), 0.2);
        view.tick_native(0.2, VIEW, None);
        let data = view.panel_data(0.2).build.unwrap().body;
        assert!(data.kinds[ROOF].chosen);
    }

    #[test]
    fn no_step_of_the_designer_goes_without_control() {
        let mut view = designing(false);
        assert!(view.panel_data(0.0).build.unwrap().body.buttons.is_none());
        for action in [
            json!({ "change": 0 }),
            json!({ "floor": 2 }),
            json!({ "close": true }),
        ] {
            assert!(out_acts(&press(&mut view, PANEL_BUILD, action)).is_empty());
        }
        assert!(click_on_map(&mut view).is_empty());
    }

    /// A click in the middle of the map, and the acts it made.
    fn click_on_map(view: &mut WebView) -> Vec<uoterm_view::act::PageAct> {
        let at = VIEW.center();
        let down = json!({ "kind": "PointerDown", "button": "Primary", "x": at.x, "y": at.y });
        let up = json!({ "kind": "PointerUp", "button": "Primary", "x": at.x, "y": at.y });
        view.input_native(&down.to_string(), 0.1);
        view.input_native(&up.to_string(), 0.1);
        view.tick_native(0.1, VIEW, Some(at));
        out_acts(&view.take_out_native())
    }

    #[test]
    fn a_click_on_the_map_builds_while_the_designer_is_open() {
        let mut view = designing(true);
        let built = click_on_map(&mut view);
        assert_eq!(built.len(), 1);
        assert_eq!(
            built[0].calls[0].tool,
            uoterm_world::tool_names::TOOL_HOUSE_EDIT
        );
    }

    fn chatting(control: bool) -> WebView {
        view_with(
            "chat",
            json!({ "name": "Mara", "in_channel": "General",
                "channels": [{ "name": "General", "has_password": false },
                             { "name": "Guild", "has_password": true }],
                "lines": [{ "who": "Ann", "words": "<b>hail</b>" }] }),
            control,
        )
    }

    #[test]
    fn the_chat_opens_by_itself_and_its_buttons_are_the_windows() {
        let mut view = chatting(true);
        let data = view.panel_data(0.0).channels.unwrap();
        assert_eq!(data.frame.title, "Chat: General");
        let channels = data.body.channels.unwrap();
        assert_eq!(channels.lines[0].words, "<b>hail</b>", "words stay words");
        assert_eq!(channels.rows[1].locked, Some(WORDS_LOCK_MARK));
        let mut same = ChatPanel::default();
        let frame = view.frame_ref().unwrap().clone();
        let chat = frame.chat.as_ref().unwrap();
        let out = press(&mut view, PANEL_CHANNELS, json!({ "join": "General" }));
        assert_eq!(
            out_acts(&out),
            vec![same.join(chat, "General".into()).unwrap().for_page()]
        );
        press(&mut view, PANEL_CHANNELS, json!({ "join": "Guild" }));
        let asking = view
            .panel_data(0.0)
            .channels
            .unwrap()
            .body
            .channels
            .unwrap();
        assert!(asking.asking.unwrap().hides);
        press(&mut view, PANEL_CHANNELS, json!({ "asking": "pw" }));
        let out = press(&mut view, PANEL_CHANNELS, json!({ "answer": true }));
        let password = Asking::Password("Guild".into()).act("pw").unwrap();
        assert_eq!(out_acts(&out), vec![password.for_page()]);
        let leave = CHAT_BUTTONS
            .iter()
            .position(|button| *button == ChatButton::Leave)
            .unwrap();
        let out = press(&mut view, PANEL_CHANNELS, json!({ "button": leave }));
        assert_eq!(out_acts(&out), vec![Act::ChatLeave.for_page()]);
        let out = press(&mut view, PANEL_CHANNELS, json!({ "say": " hi " }));
        assert_eq!(out_acts(&out), vec![Act::ChatSay("hi".into()).for_page()]);
        press(&mut view, PANEL_CHANNELS, json!({ "close": true }));
        assert!(view.panel_data(0.0).channels.is_none());
    }

    #[test]
    fn nothing_in_the_chat_acts_without_control() {
        let mut view = chatting(false);
        let data = view.panel_data(0.0).channels.unwrap().body;
        assert!(data.channels.unwrap().buttons.is_empty());
        for action in [
            json!({ "join": "General" }),
            json!({ "button": 1 }),
            json!({ "say": "hi" }),
        ] {
            assert!(out_acts(&press(&mut view, PANEL_CHANNELS, action)).is_empty());
        }
    }
}
