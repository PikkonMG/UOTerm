//! The profile of a character in the classic look: a scroll of paper with the words the shard puts at its
//! top, the words the owner wrote, and the words the shard puts at its
//! foot. The character's own profile takes typed words; closing it sends
//! them to the shard when they changed. The corner rolls the scroll up to
//! a small picture, and a double click opens it again. The paperdoll's
//! scroll asks the shard for the profile and opens this gump.

use super::canvas::Canvas;
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::WatchProfile;
use crate::window::control::Act;

pub const PROFILE: GumpKind = GumpKind {
    id: well_known::PROFILE,
    rules: GumpRules::DEFAULT,
    open: |serial| Box::new(ProfileGump::new(serial.unwrap_or_default())),
};

const SCROLL: u16 = 0x0820;
/// The scroll starts under the rolled-up picture, this far down.
const SCROLL_TOP: i32 = 22;
const SCROLL_HEIGHT: i32 = 300;
const ROLL: u16 = 0x082D;
const ROLL_AT: (i32, i32) = (143, 0);
const ROLL_BOX: (i32, i32) = (23, 24);
const ROLLED_UP: u16 = 0x09D4;
/// The words sit in a box inside the scroll: its left, its top from the
/// scroll's top, its width, and the height the scroll keeps below it.
const AREA_X: i32 = 22;
const AREA_TOP: i32 = 32;
const AREA_WIDTH: i32 = 250;
const AREA_BELOW: i32 = 96;
const FONT: u8 = 1;
const INK: u16 = 0;
const HEADER_AT: (i32, i32) = (53, 6);
const HEADER_WIDTH: u32 = 140;
/// The rule under the header starts this far above the header's foot.
const HEADER_RULE_UP: i32 = 15;
const TOP_RULE: [(u16, i32); 3] = [(0x005C, 4), (0x005D, 56), (0x005E, 194)];
const TOP_RULE_MIDDLE_WIDTH: i32 = 138;
const BODY_BELOW_RULE: i32 = 44;
const BODY_X: i32 = 4;
const BODY_WIDTH: u32 = 220;
const BODY_LEAST_HEIGHT: i32 = 20;
const BODY_ROOM: i32 = 5;
const FOOT_GAP: i32 = 3;
const FOOT_RULE: [(u16, i32, i32); 3] = [(0x005F, 8, 0), (0x0060, 17, 9), (0x0061, 214, 0)];
const FOOT_RULE_MIDDLE_WIDTH: i32 = 197;
const FOOTER_AT: (i32, i32) = (6, 26);

pub struct ProfileGump {
    serial: u32,
    field: TextField,
    /// The words the shard sent, once they came.
    sent: Option<String>,
    /// It is the character's own, and the human plays: it takes words.
    editable: bool,
    rolled_up: bool,
}

impl ProfileGump {
    pub fn new(serial: u32) -> Self {
        let mut field = TextField::new("");
        field.multiline = true;
        Self {
            serial,
            field,
            sent: None,
            editable: false,
            rolled_up: false,
        }
    }

    /// The words inside the scroll. Gives their height.
    fn words(&mut self, g: &mut Canvas<'_>, profile: &WatchProfile, own: bool) -> i32 {
        let look = TextLook::unicode(FONT, INK);
        let header_look = look.wrap(HEADER_WIDTH);
        let header = g.label(HEADER_AT.0, HEADER_AT.1, &profile.title, &header_look);
        let rule_y = header.y as i32 - HEADER_RULE_UP;
        rule(
            g,
            rule_y,
            &TOP_RULE.map(|(gump, x)| (gump, x, 0)),
            TOP_RULE_MIDDLE_WIDTH,
        );
        let body_y = rule_y + BODY_BELOW_RULE;
        let body_look = look.wrap(BODY_WIDTH);
        let body_height = if own {
            let lines = self.field.text().split('\n').count() as i32;
            let line = g.text.fonts.line_height(&look).max(1) as i32;
            let height = (lines * line + BODY_ROOM).max(BODY_LEAST_HEIGHT);
            g.text_box(
                "body",
                BODY_X,
                body_y,
                BODY_WIDTH as i32,
                height,
                &mut self.field,
                &look,
            );
            height
        } else {
            let size = g.label(BODY_X, body_y, &profile.own_words, &body_look);
            (size.y as i32 + BODY_ROOM).max(BODY_LEAST_HEIGHT)
        };
        let foot_y = body_y + body_height + FOOT_GAP;
        rule(g, foot_y, &FOOT_RULE, FOOT_RULE_MIDDLE_WIDTH);
        let footer = g.label(
            FOOTER_AT.0,
            foot_y + FOOTER_AT.1,
            &profile.shard_words,
            &look.wrap(BODY_WIDTH),
        );
        foot_y + FOOTER_AT.1 + footer.y as i32
    }
}

/// A rule of three pictures: its ends and its middle laid down.
fn rule(g: &mut Canvas<'_>, y: i32, parts: &[(u16, i32, i32); 3], middle_width: i32) {
    let [(left, left_x, left_y), (middle, middle_x, middle_y), (right, right_x, right_y)] = *parts;
    g.pic(left_x, y + left_y, left, 0);
    let height = g.gump_size(middle).map_or(0, |size| size.y as i32);
    g.pic_tiled(middle_x, y + middle_y, middle_width, height, middle, 0);
    g.pic(right_x, y + right_y, right, 0);
}

impl GumpBody for ProfileGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if self.rolled_up {
            g.pic(0, 0, ROLLED_UP, 0);
            if g.body_double_click() {
                self.rolled_up = false;
            }
            return;
        }
        let shown = g.expandable_scroll(0, SCROLL_TOP, SCROLL, SCROLL_HEIGHT);
        g.pic(ROLL_AT.0, ROLL_AT.1, ROLL, 0);
        let (w, h) = ROLL_BOX;
        if g.hit_box("roll", ROLL_AT.0, ROLL_AT.1, w, h).clicked() {
            self.rolled_up = true;
        }
        let Some(profile) = cx.frame.profiles.iter().find(|p| p.serial == self.serial) else {
            return;
        };
        if self.sent.is_none() {
            self.field.set_text(&profile.own_words);
            self.sent = Some(profile.own_words.clone());
        }
        self.editable = self.serial == cx.frame.serial && cx.live();
        let own = self.editable;
        let area_height = shown.y as i32 - AREA_BELOW;
        let top = SCROLL_TOP + AREA_TOP;
        g.scroll_area("words", AREA_X, top, AREA_WIDTH, area_height, |g| {
            self.words(g, profile, own)
        });
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        let changed = self
            .sent
            .as_deref()
            .is_some_and(|sent| sent != self.field.text());
        if changed && self.serial == cx.frame.serial {
            cx.act(Act::ProfileWrite {
                serial: self.serial,
                text: self.field.text().to_string(),
            });
        }
        Closing::Now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchFrame;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    #[test]
    fn a_profile_draws_its_words_once_the_shard_sends_them() {
        const ANN: u32 = 5;
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::of(well_known::PROFILE, ANN);
        assert!(manager.open(id, &mut profile));
        let frame = WatchFrame {
            profiles: vec![WatchProfile {
                serial: ANN,
                name: "Ann".into(),
                title: "Ann the miner".into(),
                shard_words: "Guild".into(),
                own_words: "I dig ore.".into(),
            }],
            ..WatchFrame::default()
        };
        if draw_frames(&mut manager, &mut profile, &frame) {
            assert_eq!(manager.drawn_of(well_known::PROFILE).len(), 1);
        }
    }
}
