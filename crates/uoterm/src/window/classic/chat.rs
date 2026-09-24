//! The chat of the shard, as the reference client draws it: the list of
//! channels, the channel the character is in, and the Join, Leave and
//! Create buttons. A click picks a channel and a double click joins it. A
//! channel with a password asks for it in a small box, and Create asks for
//! the name of the new channel in the same box. Below the classic part the
//! gump shows the lines said in the channel and a box to talk in, since the
//! journal of this client does not show them.
//!
//! Before the chat opens, the shard may ask for a chat name: a box of
//! the reference client's takes it.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::{WatchChat, WatchFrame};
use crate::window::control::Act;
use crate::window::model::chat::{
    chat_door, chat_name_act, joining, lines_back, Asking, ChatDoor, Joining, CHAT_NAME_MAX_CHARS,
    WORDS_CHOOSE_NAME,
};
use eframe::egui::{Color32, Pos2};
use uoterm_nav::TextAlign;

pub const CHAT: GumpKind = GumpKind {
    id: well_known::CHAT,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(ChatGump::default()),
};

/// The box that asks for the chat name. It does not move.
pub const CHAT_NAME: GumpKind = GumpKind {
    id: well_known::CHAT_NAME,
    rules: GumpRules {
        movable: false,
        kept: false,
        first_place: Pos2::new(250.0, 100.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(ChatName::default()),
};

// The frame lines of the reference client's bordered boxes.
const BORDER_ACROSS: u16 = 0x0A8C;
const BORDER_DOWN: u16 = 0x0A8D;
const HALF: i32 = 2;
const NO_HUE: u16 = 0;
const OPAQUE: f32 = 1.0;
// The chat gump.
const WIDTH: i32 = 345;
/// The reference client's chat gump ends here; the lines and the box to talk in go
/// below it.
const CLASSIC_HEIGHT: i32 = 390;
const HEIGHT: i32 = 530;
const BACKGROUND: u16 = 0x0A28;
const HEADING_FONT: u8 = 2;
const HEADING_HUE: u16 = 0x0386;
const CHANNELS_TITLE_Y: i32 = 25;
const LIST_BORDER: Place = (61, 62, 228, 206);
const LIST: Place = (64, 65, 220, 200);
const LIST_BORDER_SIZE: i32 = 3;
const CHANNEL_WIDTH: i32 = 195;
const CHANNEL_X: i32 = 3;
const CHANNEL_FONT: u8 = 3;
const CHANNEL_HUE: u16 = 0x0049;
const CHANNEL_PICKED_HUE: u16 = 0x0022;
const CHANNEL_HOVER: Color32 = Color32::from_rgb(0, 0xFF, 0xFF);
const CURRENT_TITLE_Y: i32 = 275;
const CURRENT_Y: i32 = 300;
const BUTTON_ROW_Y: i32 = 337;
/// The buttons sit this much lower than their words.
const BUTTON_DROP: i32 = 5;
const BUTTON: ButtonArt = ButtonArt::new(0x0845, 0x0846, 0x0845);
/// The button and the words of Join, Leave and Create, across.
const JOIN_AT: (i32, i32) = (48, 65);
const LEAVE_AT: (i32, i32) = (123, 140);
const CREATE_AT: (i32, i32) = (216, 233);
// The lines and the box to talk in.
const LINES_BORDER: Place = (21, 367, 305, 126);
const LINES: Place = (24, 370, 297, 120);
const LINE_PAD: i32 = 3;
/// The room at the right of the lines for their scroll bar.
const LINES_BAR_ROOM: i32 = 16;
const LINE_FONT: u8 = 1;
const SAY_BORDER: Place = (21, 496, 305, 27);
const SAY_BOX: Place = (27, 500, 293, 19);
const FIELD_FONT: u8 = 1;
const FIELD_HUE: u16 = 0x0481;
// The small box that asks for a channel name or a password.
const BOX_WIDTH: i32 = 200;
const BOX_HEIGHT: i32 = 60;
const BOX_ROW: i32 = 25;
const BOX_BORDER_SIZE: i32 = 3;
const BOX_TEXT_X: i32 = 6;
const BOX_FIELD_GAP: i32 = 2;
const BOX_FIELD_ROOM: i32 = 5;
const BOX_TITLE_ROOM: i32 = 4;
const BOX_FONT: u8 = 1;
const BOX_HUE: u16 = 0x0023;
/// The button art that closes a small box, and the one that says yes.
const CLOSE_BUTTON: ButtonArt = ButtonArt::new(0x0A94, 0x0A95, 0x0A94);
const OKAY_BUTTON: ButtonArt = ButtonArt::new(0x0A9A, 0x0A9B, 0x0A9A);
const BOX_BUTTON_SIDE: i32 = 19;
// The box that asks for the chat name.
const NAME_WIDTH: i32 = 210;
const NAME_HEIGHT: i32 = 330;
const NAME_BORDER_SIZE: i32 = 4;
const NAME_ROW: i32 = 27;
const NAME_TEXT_AT: (i32, i32) = (6, 6);
const NAME_TEXT_ROOM: i32 = 17;
const NAME_FONT: u8 = 3;
const NAME_HUE: u16 = 23;
const NAME_LABEL_HUE: u16 = 0x0033;
const NAME_LABEL_DROP: i32 = 2;

const WORDS_CHANNELS: &str = "Channels";
const WORDS_CURRENT: &str = "Your current channel:";
const WORDS_JOIN: &str = "Join";
const WORDS_LEAVE: &str = "Leave";
const WORDS_CREATE: &str = "Create";
const WORDS_CREATE_TITLE: &str = "Create a channel:";
const WORDS_NAME: &str = "Name:";
const WORDS_PASSWORD: &str = "Password:";
/// A box of a gump: x, y, width and height.
type Place = (i32, i32, i32, i32);

/// The frame of four lines of the reference client's bordered boxes, `size`
/// thick.
fn border(g: &mut Canvas<'_>, (x, y, w, h): Place, size: i32) {
    g.pic_tiled(x, y, w, size, BORDER_ACROSS, NO_HUE);
    g.pic_tiled(x, y + h - size, w, size, BORDER_ACROSS, NO_HUE);
    g.pic_tiled(x, y, size, h, BORDER_DOWN, NO_HUE);
    // The right line starts lower by half the width of its art, as the
    // classic client draws it.
    let drop = g
        .gump_size(BORDER_DOWN)
        .map_or(0, |art| art.x as i32 / HALF);
    g.pic_tiled(x + w - size, y + drop, size, h - size, BORDER_DOWN, NO_HUE);
}

fn heading() -> TextLook {
    TextLook::ascii(HEADING_FONT, HEADING_HUE)
}

fn field_look() -> TextLook {
    TextLook::unicode(FIELD_FONT, FIELD_HUE)
}

/// What the Chat button of the classic client opens: the chat gump when
/// the chat is on, the box for the chat name when the shard asks for one,
/// and otherwise the act that asks the shard to turn the chat on.
pub fn chat_button(frame: &WatchFrame) -> Result<GumpId, Act> {
    match chat_door(frame) {
        ChatDoor::Chat => Ok(GumpId::one(well_known::CHAT)),
        ChatDoor::AskName => Ok(GumpId::one(well_known::CHAT_NAME)),
        ChatDoor::TurnOn(act) => Err(act),
    }
}

/// The small box over the chat gump, as the reference client draws it.
struct ChannelBox {
    asking: Asking,
    field: TextField,
}

impl ChannelBox {
    fn new(asking: Asking) -> Self {
        let mut field = TextField::default();
        field.password = asking.hides_words();
        Self { asking, field }
    }

    /// Draws the box at its place over the gump. True when it is done.
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) -> bool {
        let x = (WIDTH - BOX_WIDTH) / HALF;
        let y = (CLASSIC_HEIGHT - BOX_HEIGHT) / HALF;
        let look = TextLook::unicode(BOX_FONT, BOX_HUE);
        g.shade(x, y, BOX_WIDTH, BOX_HEIGHT, NO_HUE, OPAQUE);
        border(g, (x, y, BOX_WIDTH, BOX_ROW), BOX_BORDER_SIZE);
        let title = match &self.asking {
            Asking::NewChannel => WORDS_CREATE_TITLE.to_string(),
            Asking::Password(channel) => format!("{WORDS_JOIN} {channel}:"),
        };
        let title_look = look.cropped((BOX_WIDTH - BOX_TITLE_ROOM) as u32);
        g.label(x + BOX_TEXT_X, y + BOX_BORDER_SIZE, &title, &title_look);
        let row_y = y + BOX_ROW - BOX_BORDER_SIZE;
        border(g, (x, row_y, BOX_WIDTH, BOX_ROW), BOX_BORDER_SIZE);
        let label = match self.asking {
            Asking::NewChannel => WORDS_NAME,
            Asking::Password(_) => WORDS_PASSWORD,
        };
        let label_size = g.label(x + BOX_TEXT_X, y + BOX_ROW, label, &look);
        let field_x = BOX_TEXT_X + label_size.x as i32 + BOX_FIELD_GAP;
        let typed = g.text_box(
            "channel-box",
            x + field_x,
            y + BOX_ROW,
            BOX_WIDTH - field_x - BOX_FIELD_ROOM,
            BOX_ROW - BOX_BORDER_SIZE * HALF,
            &mut self.field,
            &field_look(),
        );
        let last_y = y + BOX_ROW * HALF - BOX_BORDER_SIZE * HALF;
        border(g, (x, last_y, BOX_WIDTH, BOX_ROW), BOX_BORDER_SIZE);
        let buttons_y = y + BOX_HEIGHT - BOX_BUTTON_SIDE + BOX_BORDER_SIZE * HALF;
        let close_x = x + BOX_WIDTH - BOX_BUTTON_SIDE - BOX_BORDER_SIZE;
        let closed = g.button("channel-box-close", close_x, buttons_y, CLOSE_BUTTON);
        let okay_x = close_x - BOX_BUTTON_SIDE;
        let okay = g.button("channel-box-okay", okay_x, buttons_y, OKAY_BUTTON);
        if okay || typed.submitted {
            if let Some(act) = self.asking.act(self.field.text()) {
                cx.act(act);
            }
        }
        okay || closed || typed.submitted
    }
}

/// The chat gump.
#[derive(Default)]
pub struct ChatGump {
    picked: Option<String>,
    /// The newest lines hidden under the view, and how many lines there
    /// were when the gump last looked.
    back: usize,
    seen: usize,
    say: TextField,
    channel_box: Option<ChannelBox>,
}

impl ChatGump {
    /// The list of channels. Gives the channel to join when the player
    /// double-clicked one.
    fn channels(&mut self, g: &mut Canvas<'_>, chat: &WatchChat) -> Option<String> {
        let (x, y, w, h) = LIST;
        border(g, LIST_BORDER, LIST_BORDER_SIZE);
        g.shade(x, y, w, h, NO_HUE, OPAQUE);
        let picked = &mut self.picked;
        let mut join = None;
        g.scroll_area("channels", x, y, w, h, |g| {
            let mut row_y = 0;
            for (at, (name, _)) in chat.channels.iter().enumerate() {
                let hue = if picked.as_deref() == Some(name.as_str()) {
                    CHANNEL_PICKED_HUE
                } else {
                    CHANNEL_HUE
                };
                let look = TextLook::ascii(CHANNEL_FONT, hue).cropped(CHANNEL_WIDTH as u32);
                let height = g.measure(name, &look).y as i32;
                if g.hovered(0, row_y, CHANNEL_WIDTH, height) {
                    g.fill(0, row_y, CHANNEL_WIDTH, height, CHANNEL_HOVER);
                }
                g.label(CHANNEL_X, row_y, name, &look);
                let row = g.click_area(("channel", at), 0, row_y, CHANNEL_WIDTH, height);
                if row.clicked() {
                    *picked = Some(name.clone());
                }
                if row.double_clicked() {
                    join = Some(name.clone());
                }
                row_y += height;
            }
            row_y
        });
        join
    }

    /// The lines said in the channel, the newest at the bottom, in the
    /// chat hue of the Speech page.
    fn lines(&mut self, g: &mut Canvas<'_>, chat: &WatchChat, hue: u16) {
        let (x, y, w, h) = LINES;
        border(g, LINES_BORDER, LIST_BORDER_SIZE);
        g.shade(x, y, w, h, NO_HUE, OPAQUE);
        let count = chat.lines.len();
        self.back = lines_back(self.back, self.seen, count);
        self.seen = count;
        let most_back = count.saturating_sub(1);
        if g.hovered(x, y, w, h) {
            let wheel = g.ui().input(|i| i.raw_scroll_delta.y);
            if wheel > 0.0 {
                self.back = (self.back + 1).min(most_back);
            } else if wheel < 0.0 {
                self.back = self.back.saturating_sub(1);
            }
        }
        if most_back > 0 {
            let mut value = (most_back - self.back) as i32;
            let bar_x = x + w - LINES_BAR_ROOM;
            g.scroll_bar("lines-bar", bar_x, y, h, &mut value, most_back as i32);
            self.back = most_back - value.clamp(0, most_back as i32) as usize;
        }
        let width = w - LINES_BAR_ROOM - LINE_PAD * HALF;
        let look = TextLook::unicode(LINE_FONT, hue).wrap(width as u32);
        let mut bottom = y + h - LINE_PAD;
        for (who, words) in chat.lines.iter().rev().skip(self.back) {
            let line = format!("{who}: {words}");
            let top = bottom - g.measure(&line, &look).y as i32;
            if top < y + LINE_PAD {
                break;
            }
            g.label(x + LINE_PAD, top, &line, &look);
            bottom = top;
        }
    }

    /// The box to talk in the channel. Enter says the words.
    fn say_box(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        border(g, SAY_BORDER, LIST_BORDER_SIZE);
        let (x, y, w, h) = SAY_BOX;
        let typed = g.text_box("say", x, y, w, h, &mut self.say, &field_look());
        let words = self.say.text().trim().to_string();
        if typed.submitted && !words.is_empty() {
            cx.act(Act::ChatSay(words));
            self.say.set_text("");
        }
    }
}

impl GumpBody for ChatGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(chat) = cx.frame.chat.as_ref() else {
            return;
        };
        g.frame(0, 0, WIDTH, HEIGHT, BACKGROUND);
        let centered = heading().wrap(WIDTH as u32).aligned(TextAlign::Center);
        g.label(0, CHANNELS_TITLE_Y, WORDS_CHANNELS, &centered);
        let double_clicked = self.channels(g, chat).is_some();
        g.label(0, CURRENT_TITLE_Y, WORDS_CURRENT, &centered);
        g.label(0, CURRENT_Y, &chat.in_channel, &centered);
        let button_y = BUTTON_ROW_Y + BUTTON_DROP;
        let join = g.button("join", JOIN_AT.0, button_y, BUTTON);
        let leave = g.button("leave", LEAVE_AT.0, button_y, BUTTON);
        let create = g.button("create", CREATE_AT.0, button_y, BUTTON);
        g.label(JOIN_AT.1, BUTTON_ROW_Y, WORDS_JOIN, &heading());
        g.label(LEAVE_AT.1, BUTTON_ROW_Y, WORDS_LEAVE, &heading());
        g.label(CREATE_AT.1, BUTTON_ROW_Y, WORDS_CREATE, &heading());
        self.lines(g, chat, cx.profile.speech.chat_hue);
        self.say_box(g, cx);
        // The first click of a double click picked the channel.
        if join || double_clicked {
            match joining(chat, self.picked.as_deref()) {
                Joining::Send(act) => cx.act(act),
                Joining::AskPassword(channel) => {
                    self.channel_box = Some(ChannelBox::new(Asking::Password(channel)));
                }
                Joining::Nothing => {}
            }
        }
        if leave {
            cx.act(Act::ChatLeave);
        }
        if create && self.channel_box.is_none() {
            self.channel_box = Some(ChannelBox::new(Asking::NewChannel));
        }
        if let Some(channel_box) = self.channel_box.as_mut() {
            if channel_box.draw(g, cx) {
                self.channel_box = None;
            }
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.chat.is_some()
    }
}

/// The box that asks for the chat name, before the chat opens.
pub struct ChatName {
    field: TextField,
}

impl Default for ChatName {
    fn default() -> Self {
        Self {
            field: TextField::default().with_max_chars(Some(CHAT_NAME_MAX_CHARS)),
        }
    }
}

impl GumpBody for ChatName {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        g.shade(0, 0, NAME_WIDTH, NAME_HEIGHT, NO_HUE, OPAQUE);
        border(g, (0, 0, NAME_WIDTH, NAME_HEIGHT), NAME_BORDER_SIZE);
        let words =
            TextLook::unicode(NAME_FONT, NAME_HUE).wrap((NAME_WIDTH - NAME_TEXT_ROOM) as u32);
        let (text_x, text_y) = NAME_TEXT_AT;
        let words_bottom = text_y + g.label(text_x, text_y, WORDS_CHOOSE_NAME, &words).y as i32;
        border(g, (0, words_bottom, NAME_WIDTH, NAME_ROW), NAME_BORDER_SIZE);
        let label_y = words_bottom + NAME_LABEL_DROP;
        let label_look = TextLook::unicode(NAME_FONT, NAME_LABEL_HUE);
        let label = g.label(text_x, label_y, WORDS_NAME, &label_look);
        let field_x = text_x + label.x as i32 + BOX_FIELD_GAP;
        let typed = g.text_box(
            "name",
            field_x,
            label_y,
            NAME_WIDTH - field_x - NAME_TEXT_ROOM,
            NAME_ROW - NAME_BORDER_SIZE * HALF,
            &mut self.field,
            &field_look(),
        );
        let second_y = label_y + label.y as i32;
        border(g, (0, second_y, NAME_WIDTH, NAME_ROW), NAME_BORDER_SIZE);
        let buttons_y = NAME_HEIGHT - BOX_BUTTON_SIDE - NAME_BORDER_SIZE;
        let close_x = NAME_WIDTH - BOX_BUTTON_SIDE - NAME_BORDER_SIZE;
        let closed = g.button("close", close_x, buttons_y, CLOSE_BUTTON);
        let okay = g.button("okay", close_x - BOX_BUTTON_SIDE, buttons_y, OKAY_BUTTON);
        if okay || typed.submitted {
            if let Some(act) = chat_name_act(self.field.text()) {
                cx.act(act);
            }
        }
        if okay || closed || typed.submitted {
            cx.close(cx.me);
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.chat_asks_for_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    fn chat() -> WatchChat {
        WatchChat {
            name: "Mara".into(),
            channels: vec![("General".into(), false), ("Guild".into(), true)],
            in_channel: "General".into(),
            lines: vec![
                ("Ann".into(), "hail".into()),
                ("Bob".into(), "well met".into()),
            ],
        }
    }

    #[test]
    fn the_chat_button_opens_the_chat_the_name_box_or_asks_the_shard() {
        let mut frame = WatchFrame {
            name: "Mara".into(),
            ..WatchFrame::default()
        };
        assert_eq!(chat_button(&frame), Err(Act::ChatOpen("Mara".into())));
        frame.chat_asks_for_name = true;
        assert_eq!(chat_button(&frame), Ok(GumpId::one(well_known::CHAT_NAME)));
        frame.chat = Some(chat());
        assert_eq!(chat_button(&frame), Ok(GumpId::one(well_known::CHAT)));
    }

    #[test]
    fn the_chat_name_field_takes_no_more_than_the_shard_takes() {
        let mut name = ChatName::default();
        name.field.set_text(&"x".repeat(CHAT_NAME_MAX_CHARS * 2));
        assert_eq!(name.field.text().chars().count(), CHAT_NAME_MAX_CHARS);
    }

    #[test]
    fn the_gumps_live_while_the_shard_shows_them() {
        let mut frame = WatchFrame::default();
        assert!(!ChatGump::default().alive(&frame));
        assert!(!ChatName::default().alive(&frame));
        frame.chat = Some(chat());
        frame.chat_asks_for_name = true;
        assert!(ChatGump::default().alive(&frame));
        assert!(ChatName::default().alive(&frame));
    }

    #[test]
    fn the_chat_and_the_name_box_draw_with_the_client_files() {
        let frame = WatchFrame {
            chat: Some(chat()),
            chat_asks_for_name: true,
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        let mut manager = GumpManager::default();
        let chat_id = GumpId::one(well_known::CHAT);
        let name_id = GumpId::one(well_known::CHAT_NAME);
        manager.open(chat_id, &mut profile);
        manager.open(name_id, &mut profile);
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&chat_id) && manager.is_open(&name_id));
        draw_frames(&mut manager, &mut profile, &WatchFrame::default());
        assert!(!manager.is_open(&chat_id) && !manager.is_open(&name_id));
    }
}
