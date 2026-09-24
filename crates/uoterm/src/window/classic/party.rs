//! The party manifest of the classic client, the box of a party invite
//! (when the General page asks for it), and the box a Tell button opens. The manifest lists the ten places
//! of the party with a Tell button each and, for the leader, a Kick button;
//! below them it sends the party a message, lets the party loot the
//! character or not, leaves or disbands the party, and adds a member with
//! the target cursor. Okay keeps the loot choice; Cancel forgets it. The
//! acts are those of the Modern party tab. The classic client puts the
//! message prefix in its chat line; here a small box takes the words.

use super::canvas::{ButtonArt, Canvas};
use super::manager::GumpManager;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::WatchFrame;
use crate::window::control::{Act, Channel};
use crate::window::keys::chat::channel_hue;
use crate::window::model::party::{
    invite_words, inviter_name, leads, leave_words, ACCEPT_COMMAND, DECLINE_COMMAND,
    INVITE_COMMAND, PARTY_PLACES,
};
use crate::window::settings::Profile;
use uoterm_nav::TextAlign;

pub const PARTY: GumpKind = GumpKind {
    id: well_known::PARTY,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(PartyGump::default()),
};

/// The box of a party invite, which the leader's serial names.
pub const PARTY_INVITE: GumpKind = GumpKind {
    id: PARTY_INVITE_ID,
    rules: GumpRules {
        movable: false,
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(InviteGump::new(serial.unwrap_or_default())),
};

/// The box of words to one member, by his serial, or to the whole party
/// (serial zero).
pub const PARTY_TELL: GumpKind = GumpKind {
    id: PARTY_TELL_ID,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(TellGump::new(serial.unwrap_or_default())),
};

const PARTY_INVITE_ID: &str = "party_invite";
const PARTY_TELL_ID: &str = "party_tell";
/// The whole party, as the serial of a tell box.
const WHOLE_PARTY: u32 = 0;
// The manifest.
const BACKGROUND: u16 = 0x0A28;
const SIZE: (i32, i32) = (450, 480);
const FIRST_ROW: i32 = 48;
const ROW_STEP: i32 = 25;
const BUTTON_DOWN: i32 = 2;
const TELL: ButtonArt = ButtonArt::new(0x0FAB, 0x0FAD, 0x0FAC);
const KICK: ButtonArt = ButtonArt::new(0x0FB1, 0x0FB3, 0x0FB2);
const TELL_X: i32 = 40;
const KICK_X: i32 = 80;
const NAME_BAR: u16 = 0x0475;
const NAME_BAR_X: i32 = 130;
const NAME_X: i32 = 140;
const NAME_WIDTH: u32 = 250;
const INK: u16 = 0x0386;
const HEADER_FONT: u8 = 1;
const BODY_FONT: u8 = 2;
const TELL_WORDS: (i32, i32, &str) = (40, 30, "Tell");
const KICK_WORDS: (i32, i32, &str) = (80, 30, "Kick");
const TITLE: (i32, i32, &str) = (153, 20, "Party Manifest");
const ACTIONS_X: i32 = 70;
const ACTION_WORDS_X: i32 = 110;
const MESSAGE_Y: i32 = 307;
const MESSAGE_WORDS: &str = "Send the party a message";
const LOOT_Y: i32 = 334;
const LOOT_ON: ButtonArt = ButtonArt::new(0x0FA2, 0x0FA2, 0x0FA2);
const LOOT_OFF: ButtonArt = ButtonArt::new(0x0FA9, 0x0FA9, 0x0FA9);
const LOOT_ON_WORDS: &str = "Party can loot me";
const LOOT_OFF_WORDS: &str = "Party CANNOT loot me";
const LEAVE_Y: i32 = 360;
const LEAVE: ButtonArt = ButtonArt::new(0x0FAE, 0x0FB0, 0x0FAF);
const ADD_Y: i32 = 385;
const ADD: ButtonArt = ButtonArt::new(0x0FA8, 0x0FAA, 0x0FA9);
const ADD_WORDS: &str = "Add New Member";
const OKAY: ButtonArt = ButtonArt::new(0x00F9, 0x00F8, 0x00F7);
const CANCEL: ButtonArt = ButtonArt::new(0x00F3, 0x00F1, 0x00F2);
const OKAY_AT: (i32, i32) = (130, 430);
const CANCEL_AT: (i32, i32) = (236, 430);
// The invite box.
const INVITE_WIDTH: i32 = 270;
const INVITE_HEIGHT: i32 = 80;
const INVITE_OPACITY: f32 = 0.8;
/// A long name makes the box wider by this much for each char past ten.
const LONG_NAME: usize = 10;
const WIDTH_PER_CHAR: i32 = 5;
/// The box stands this far left of the middle of the screen, this far down.
const INVITE_LEFT_OF_MIDDLE: f32 = 125.0;
const INVITE_TOP: f32 = 150.0;
const INVITE_WORDS_AT: (i32, i32) = (10, 15);
const INVITE_WORDS_HUE: u16 = 15;
const INVITE_FONT: u8 = 1;
const ACCEPT_AT: (i32, i32) = (224, 55);
const DECLINE_AT: (i32, i32) = (164, 55);
const CHOICE_SIZE: (i32, i32) = (45, 25);
const CHOICE_HUE: u16 = 0xFFFF;
const ACCEPT_WORDS: &str = "Accept";
const DECLINE_WORDS: &str = "Decline";
// The tell box.
const TELL_SIZE: (i32, i32) = (300, 70);
const TELL_TITLE_AT: (i32, i32) = (15, 12);
const TELL_FIELD: (i32, i32, i32, i32) = (15, 35, 270, 20);
const TELL_FIELD_BACK: u16 = 0x0BB8;
const TELL_TO_PARTY: &str = "Tell the party:";
/// The most chars of one line of speech.
const SAY_MAX_CHARS: usize = 128;

/// The width of the invite box for a name.
fn invite_width(name: &str) -> i32 {
    let chars = name.chars().count();
    if chars < LONG_NAME {
        INVITE_WIDTH
    } else {
        INVITE_WIDTH + chars as i32 * WIDTH_PER_CHAR
    }
}

/// The party manifest.
#[derive(Default)]
pub struct PartyGump {
    /// The loot choice the player made here, until Okay sends it.
    can_loot: Option<bool>,
}

impl PartyGump {
    fn label(g: &mut Canvas<'_>, (x, y, words): (i32, i32, &str), font: u8) {
        g.label(x, y, words, &TextLook::ascii(font, INK));
    }

    fn rows(&self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let leads = leads(cx.frame);
        for place in 0..PARTY_PLACES {
            let y = FIRST_ROW + place as i32 * ROW_STEP;
            let member = cx.frame.party_members.get(place);
            if g.button(("tell", place), TELL_X, y + BUTTON_DOWN, TELL) {
                if let Some(member) = member {
                    cx.open(GumpId::of(PARTY_TELL_ID, member.serial));
                }
            }
            if leads && g.button(("kick", place), KICK_X, y + BUTTON_DOWN, KICK) {
                if let Some(member) = member {
                    cx.act(Act::PartyKick(member.serial));
                }
            }
            g.pic(NAME_BAR_X, y, NAME_BAR, 0);
            let name = member.map_or("", |member| member.name.as_str());
            let look = TextLook::ascii(BODY_FONT, INK)
                .aligned(TextAlign::Center)
                .wrap(NAME_WIDTH);
            g.label(NAME_X, y + 1, name, &look);
        }
    }
}

impl GumpBody for PartyGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let frame = cx.frame;
        let in_party = !frame.party_members.is_empty();
        g.frame(0, 0, SIZE.0, SIZE.1, BACKGROUND);
        Self::label(g, TELL_WORDS, HEADER_FONT);
        Self::label(g, KICK_WORDS, HEADER_FONT);
        Self::label(g, TITLE, BODY_FONT);
        self.rows(g, cx);
        if g.button("message", ACTIONS_X, MESSAGE_Y, TELL) && in_party {
            cx.open(GumpId::of(PARTY_TELL_ID, WHOLE_PARTY));
        }
        Self::label(g, (ACTION_WORDS_X, MESSAGE_Y, MESSAGE_WORDS), BODY_FONT);
        let can_loot = self.can_loot.unwrap_or(frame.party_can_loot);
        let (art, words) = if can_loot {
            (LOOT_ON, LOOT_ON_WORDS)
        } else {
            (LOOT_OFF, LOOT_OFF_WORDS)
        };
        if g.button("loot", ACTIONS_X, LOOT_Y, art) {
            self.can_loot = Some(!can_loot);
        }
        Self::label(g, (ACTION_WORDS_X, LOOT_Y, words), BODY_FONT);
        if g.button("leave", ACTIONS_X, LEAVE_Y, LEAVE) && in_party {
            cx.act(Act::PartyLeave);
        }
        Self::label(g, (ACTION_WORDS_X, LEAVE_Y, leave_words(frame)), BODY_FONT);
        if leads(frame) {
            if g.button("add", ACTIONS_X, ADD_Y, ADD) {
                cx.act(Act::Command(INVITE_COMMAND.into()));
            }
            Self::label(g, (ACTION_WORDS_X, ADD_Y, ADD_WORDS), BODY_FONT);
        }
        if g.button("okay", OKAY_AT.0, OKAY_AT.1, OKAY) {
            let chosen = self.can_loot.take();
            if let Some(loot) = chosen.filter(|loot| *loot != frame.party_can_loot && in_party) {
                cx.act(Act::PartyLoot(loot));
            }
            cx.close(cx.me);
        }
        if g.button("cancel", CANCEL_AT.0, CANCEL_AT.1, CANCEL) {
            self.can_loot = None;
            cx.close(cx.me);
        }
    }
}

/// Opens the box of a party invite that came, when the General page asks
/// for it.
#[derive(Default)]
pub struct InviteWatch {
    seen: Option<u32>,
}

impl InviteWatch {
    pub fn follow(
        &mut self,
        manager: &mut GumpManager,
        frame: &WatchFrame,
        profile: &mut Profile,
        classic: bool,
    ) {
        let invite = frame.party_invite;
        if let Some(leader) = invite.filter(|leader| Some(*leader) != self.seen) {
            if classic && profile.general.party_invite_gump {
                manager.open(GumpId::of(PARTY_INVITE_ID, leader), profile);
            }
        }
        self.seen = invite;
    }
}

/// The box of one party invite: who invites, Accept and Decline.
pub struct InviteGump {
    leader: u32,
    placed: bool,
}

impl InviteGump {
    fn new(leader: u32) -> Self {
        Self {
            leader,
            placed: false,
        }
    }
}

impl GumpBody for InviteGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if !self.placed {
            self.placed = true;
            let screen = g.ctx().screen_rect();
            let place = eframe::egui::Pos2::new(
                screen.center().x - INVITE_LEFT_OF_MIDDLE,
                screen.top() + INVITE_TOP,
            );
            g.move_to(place);
        }
        let name = inviter_name(cx.frame, self.leader);
        let width = invite_width(&name);
        let widened = width - INVITE_WIDTH;
        g.shade(0, 0, width, INVITE_HEIGHT, 0, INVITE_OPACITY);
        let look = TextLook::unicode(INVITE_FONT, INVITE_WORDS_HUE);
        g.label(
            INVITE_WORDS_AT.0,
            INVITE_WORDS_AT.1,
            &invite_words(&name),
            &look,
        );
        let choice = TextLook::unicode(INVITE_FONT, CHOICE_HUE);
        let (w, h) = CHOICE_SIZE;
        let accept_at = (ACCEPT_AT.0 + widened, ACCEPT_AT.1);
        let decline_at = (DECLINE_AT.0 + widened, DECLINE_AT.1);
        if g.nice_button(
            "accept",
            accept_at.0,
            accept_at.1,
            w,
            h,
            ACCEPT_WORDS,
            &choice,
            false,
        ) {
            cx.act(Act::Command(ACCEPT_COMMAND.into()));
            cx.close(cx.me);
        }
        if g.nice_button(
            "decline",
            decline_at.0,
            decline_at.1,
            w,
            h,
            DECLINE_WORDS,
            &choice,
            false,
        ) {
            cx.act(Act::Command(DECLINE_COMMAND.into()));
            cx.close(cx.me);
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.party_invite == Some(self.leader)
    }
}

/// The box of words to one member or to the whole party. Enter says them.
pub struct TellGump {
    to: u32,
    field: TextField,
    focused: bool,
}

impl TellGump {
    fn new(to: u32) -> Self {
        Self {
            to,
            field: TextField::new("").with_max_chars(Some(SAY_MAX_CHARS)),
            focused: false,
        }
    }
}

impl GumpBody for TellGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let (w, h) = TELL_SIZE;
        g.frame(0, 0, w, h, BACKGROUND);
        let title = match cx
            .frame
            .party_members
            .iter()
            .find(|member| member.serial == self.to)
        {
            Some(member) => format!("{} {}:", TELL_WORDS.2, member.name),
            None => TELL_TO_PARTY.to_string(),
        };
        let look = TextLook::ascii(BODY_FONT, INK);
        g.label(TELL_TITLE_AT.0, TELL_TITLE_AT.1, &title, &look);
        let (x, y, fw, fh) = TELL_FIELD;
        g.frame(x, y, fw, fh, TELL_FIELD_BACK);
        let key = ("words", self.to);
        let done = g.text_box(
            key,
            x,
            y,
            fw,
            fh,
            &mut self.field,
            &TextLook::unicode(INVITE_FONT, 0),
        );
        if !self.focused {
            self.focused = true;
            g.focus(key);
        }
        if done.submitted && !self.field.text().is_empty() {
            let channel = if self.to == WHOLE_PARTY {
                Channel::Party
            } else {
                Channel::PartyMember(self.to)
            };
            cx.act(Act::Speak {
                channel,
                text: self.field.text().to_string(),
                hue: channel_hue(&cx.profile.speech, Some(channel)),
            });
            cx.close(cx.me);
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        self.to == WHOLE_PARTY || frame.party_members.iter().any(|m| m.serial == self.to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPartyMember;
    use crate::window::classic::testing::draw_frames;

    const ME: u32 = 1;
    const BOB: u32 = 2;

    fn member(serial: u32) -> WatchPartyMember {
        WatchPartyMember {
            serial,
            name: format!("member {serial}"),
            hits_percent: None,
            ..WatchPartyMember::default()
        }
    }

    #[test]
    fn the_invite_box_grows_for_a_long_name() {
        assert_eq!(invite_width("Bob"), INVITE_WIDTH);
        assert_eq!(
            invite_width("Bartholomew"),
            INVITE_WIDTH + 11 * WIDTH_PER_CHAR
        );
    }

    #[test]
    fn an_invite_opens_once_by_the_option_and_closes_with_its_answer() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let mut watch = InviteWatch::default();
        let mut frame = WatchFrame {
            party_invite: Some(BOB),
            ..WatchFrame::default()
        };
        let id = GumpId::of(PARTY_INVITE_ID, BOB);
        watch.follow(&mut manager, &frame, &mut profile, true);
        assert!(!manager.is_open(&id), "the option is off");
        frame.party_invite = None;
        watch.follow(&mut manager, &frame, &mut profile, true);
        profile.general.party_invite_gump = true;
        frame.party_invite = Some(BOB);
        watch.follow(&mut manager, &frame, &mut profile, true);
        assert!(manager.is_open(&id));
        assert!(InviteGump::new(BOB).alive(&frame));
        frame.party_invite = None;
        assert!(!InviteGump::new(BOB).alive(&frame));
    }

    #[test]
    fn the_manifest_and_a_tell_box_draw() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        assert!(manager.open(GumpId::one(well_known::PARTY), &mut profile));
        assert!(manager.open(GumpId::of(PARTY_TELL_ID, BOB), &mut profile));
        let frame = WatchFrame {
            serial: ME,
            party_members: vec![member(ME), member(BOB)],
            ..WatchFrame::default()
        };
        if draw_frames(&mut manager, &mut profile, &frame) {
            assert_eq!(manager.drawn_of(well_known::PARTY).len(), 1);
            assert_eq!(manager.drawn_of(PARTY_TELL_ID).len(), 1);
        }
    }
}
