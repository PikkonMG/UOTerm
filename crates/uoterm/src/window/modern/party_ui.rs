//! The party tab of the sheet, with what the classic party manifest does:
//! the ten places of the party with each member's hits, mana and stamina,
//! Tell and, for the leader, Kick; whether the party loots the character;
//! leave or disband; a new member by the target cursor or from the people
//! near; and words to the party or to one member. An invite the shard sent asks Accept or
//! Decline at the top of the tab, and in a small panel of its own while
//! the tab is closed.

use super::super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::super::control::{Act, Channel};
use super::super::deck_ui::{ROW, TAB_GAP};
use super::super::keys::chat::channel_hue;
use super::super::model::party::{
    invite_words, inviter_name, leads, leave_words, ACCEPT_COMMAND, DECLINE_COMMAND,
    INVITE_COMMAND, PARTY_PLACES,
};
use super::super::settings::{Profile, SpeechOptions};
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, PanelSpec};
use super::layout::{self, Spot};
use crate::view::{WatchFrame, WatchPartyMember};
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use uoterm_assist::mobiles::is_humanoid;

pub const INVITE_ID: &str = "modern:party_invite";
const INVITE_WIDTH: f32 = 320.0;
const CHOICE_WIDTH: f32 = 80.0;
const BUTTON_WIDTH: f32 = 64.0;
const LOOT_WIDTH: f32 = 128.0;
const ADD_WIDTH: f32 = 104.0;
const LEAVE_WIDTH: f32 = 124.0;
const SAY_WIDTH: f32 = 56.0;
const NUMBER_WIDTH: f32 = 22.0;
const PERCENT: f32 = 100.0;
/// The hits, mana and stamina of a member stand side by side, this far
/// apart.
const POOL_GAP: f32 = 4.0;
const POOLS: f32 = 3.0;
/// People this near can be invited from the party tab.
const INVITE_TILES: u16 = 12;
const WORDS_INVITE_TITLE: &str = "Party invite";
const WORDS_ACCEPT: &str = "Accept";
const WORDS_DECLINE: &str = "Decline";
const WORDS_LOOT_ON: &str = "Party loots: yes";
const WORDS_LOOT_OFF: &str = "Party loots: no";
const WORDS_ADD: &str = "Add member";
const WORDS_TELL: &str = "Tell";
const WORDS_KICK: &str = "Kick";
const WORDS_INVITE: &str = "Invite";
const WORDS_EMPTY: &str = "Empty";
const WORDS_NEAR: &str = "Invite someone near:";
const WORDS_SAY: &str = "Say";
const HINT_TELL_PARTY: &str = "Tell the party";
const HINT_MEMBER: &str = "Click: look, or target while the shard asks for one.";

/// One row of the list of the tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Place(usize),
    NearTitle,
    Near(u32),
}

/// The people near the character a player may invite: humans in sight who
/// are not in the party.
fn near(frame: &WatchFrame) -> Vec<u32> {
    frame
        .mobiles
        .iter()
        .filter(|mobile| {
            mobile.serial != frame.serial
                && mobile.dist <= INVITE_TILES
                && is_humanoid(mobile.look.body)
                && !frame
                    .party_members
                    .iter()
                    .any(|member| member.serial == mobile.serial)
        })
        .map(|mobile| mobile.serial)
        .collect()
}

/// The rows of the list: the ten places, then the people near to invite
/// when the character may add.
fn entries(frame: &WatchFrame) -> Vec<Entry> {
    let mut rows: Vec<Entry> = (0..PARTY_PLACES).map(Entry::Place).collect();
    let near = near(frame);
    if leads(frame) && !near.is_empty() {
        rows.push(Entry::NearTitle);
        rows.extend(near.into_iter().map(Entry::Near));
    }
    rows
}

/// The Accept and Decline of an invite, at the right of `row`, with its
/// words at the left.
fn invite_band(ui: &egui::Ui, row: Rect, frame: &WatchFrame, tools: &Tools<'_>, leader: u32) {
    ui.painter().text(
        row.left_center(),
        Align2::LEFT_CENTER,
        invite_words(&inviter_name(frame, leader)),
        text_font(theme::SIZE_SMALL),
        theme::WAITING,
    );
    if !frame.human_control {
        return;
    }
    let decline = Rect::from_min_size(
        Pos2::new(row.right() - CHOICE_WIDTH, row.top()),
        Vec2::new(CHOICE_WIDTH, row.height() - TAB_GAP),
    );
    let accept = decline.translate(Vec2::new(-(CHOICE_WIDTH + TAB_GAP), 0.0));
    if theme::segment_keyed(
        ui,
        accept,
        Id::new(("party-accept", leader)),
        WORDS_ACCEPT,
        theme::GOAL,
    ) {
        tools.hand.act(Act::Command(ACCEPT_COMMAND.into()));
    }
    if theme::segment_keyed(
        ui,
        decline,
        Id::new(("party-decline", leader)),
        WORDS_DECLINE,
        theme::ALARM,
    ) {
        tools.hand.act(Act::Command(DECLINE_COMMAND.into()));
    }
}

/// The small panel of an invite, while the party tab is closed. Gives its
/// place.
pub fn invite_panel(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> Option<Rect> {
    let leader = frame.party_invite?;
    let height = frame::TITLE_ROW + ROW * 2.0 + theme::PANEL_PAD * 2.0;
    let spec = PanelSpec {
        id: INVITE_ID,
        title: WORDS_INVITE_TITLE,
        default: layout::first_place(rect, Spot::LeftColumn(0), Vec2::new(INVITE_WIDTH, height)),
        min_size: None,
        closable: false,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, WORDS_INVITE_TITLE);
    ui.painter().text(
        body.left_top(),
        Align2::LEFT_TOP,
        invite_words(&inviter_name(frame, leader)),
        text_font(theme::SIZE_SMALL),
        theme::TEXT,
    );
    let row = Rect::from_min_size(
        body.left_top() + Vec2::new(0.0, ROW),
        Vec2::new(body.width(), ROW),
    );
    if frame.human_control {
        let accept = Rect::from_min_size(row.min, Vec2::new(CHOICE_WIDTH, ROW - TAB_GAP));
        let decline = accept.translate(Vec2::new(CHOICE_WIDTH + TAB_GAP, 0.0));
        if theme::segment_keyed(
            ui,
            accept,
            Id::new("invite-panel-accept"),
            WORDS_ACCEPT,
            theme::GOAL,
        ) {
            tools.hand.act(Act::Command(ACCEPT_COMMAND.into()));
        }
        if theme::segment_keyed(
            ui,
            decline,
            Id::new("invite-panel-decline"),
            WORDS_DECLINE,
            theme::ALARM,
        ) {
            tools.hand.act(Act::Command(DECLINE_COMMAND.into()));
        }
    }
    frame::controls(ui, panel, &spec, profile, tools);
    Some(panel)
}

#[derive(Default)]
pub struct PartyTab {
    first_row: usize,
    /// The member the words go to; None for the whole party.
    tell_to: Option<u32>,
    words: String,
}

impl PartyTab {
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        speech: &SpeechOptions,
    ) {
        let live = frame.human_control;
        let in_party = !frame.party_members.is_empty();
        if self
            .tell_to
            .is_some_and(|to| !frame.party_members.iter().any(|m| m.serial == to))
        {
            self.tell_to = None;
        }
        let mut top = body.top();
        let mut next_row = || {
            let row =
                Rect::from_min_size(Pos2::new(body.left(), top), Vec2::new(body.width(), ROW));
            top += ROW;
            row
        };
        if let Some(leader) = frame.party_invite {
            invite_band(ui, next_row(), frame, tools, leader);
        }
        if live {
            self.controls(ui, next_row(), frame, tools, in_party);
        }
        let field = Rect::from_min_max(Pos2::new(body.left(), body.bottom() - ROW), body.max);
        let list = Rect::from_min_max(
            Pos2::new(body.left(), top),
            Pos2::new(
                body.right(),
                if live && in_party {
                    field.top()
                } else {
                    body.bottom()
                },
            ),
        );
        self.list(ui, list, frame, tools);
        if live && in_party {
            self.tell_field(ui, field, frame, tools, speech);
        }
    }

    /// Loot, add a member, and leave or disband.
    fn controls(
        &mut self,
        ui: &egui::Ui,
        row: Rect,
        frame: &WatchFrame,
        tools: &Tools<'_>,
        in_party: bool,
    ) {
        let height = ROW - TAB_GAP;
        if in_party {
            let loot = Rect::from_min_size(row.min, Vec2::new(LOOT_WIDTH, height));
            let words = if frame.party_can_loot {
                WORDS_LOOT_ON
            } else {
                WORDS_LOOT_OFF
            };
            if theme::segment_keyed(ui, loot, Id::new("party-loot"), words, theme::TEXT) {
                tools.hand.act(Act::PartyLoot(!frame.party_can_loot));
            }
            let leave = Rect::from_min_size(
                Pos2::new(row.right() - LEAVE_WIDTH, row.top()),
                Vec2::new(LEAVE_WIDTH, height),
            );
            if theme::segment_keyed(
                ui,
                leave,
                Id::new("party-leave"),
                leave_words(frame),
                theme::ALARM,
            ) {
                tools.hand.act(Act::PartyLeave);
            }
        }
        if leads(frame) {
            let right = if in_party {
                row.right() - LEAVE_WIDTH - TAB_GAP
            } else {
                row.right()
            };
            let add = Rect::from_min_size(
                Pos2::new(right - ADD_WIDTH, row.top()),
                Vec2::new(ADD_WIDTH, height),
            );
            if theme::segment_keyed(ui, add, Id::new("party-add"), WORDS_ADD, theme::GOAL) {
                tools.hand.act(Act::Command(INVITE_COMMAND.into()));
            }
        }
    }

    /// The ten places, and the people near to invite.
    fn list(&mut self, ui: &egui::Ui, list: Rect, frame: &WatchFrame, tools: &mut Tools<'_>) {
        let rows = entries(frame);
        let shown = ((list.height() / ROW).floor() as usize).max(1);
        let last_first = rows.len().saturating_sub(shown);
        self.first_row = scrolled(ui, list, self.first_row.min(last_first), last_first);
        for (at, entry) in rows.iter().skip(self.first_row).take(shown).enumerate() {
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, at as f32 * ROW),
                Vec2::new(list.width(), ROW),
            );
            match entry {
                Entry::Place(place) => match frame.party_members.get(*place) {
                    Some(member) => self.member_row(ui, row, *place, member, frame, tools),
                    None => {
                        ui.painter().text(
                            row.left_center(),
                            Align2::LEFT_CENTER,
                            format!("{}.", place + 1),
                            number_font(theme::SIZE_SMALL),
                            theme::TEXT_FAINT,
                        );
                        ui.painter().text(
                            Pos2::new(row.left() + NUMBER_WIDTH, row.center().y),
                            Align2::LEFT_CENTER,
                            WORDS_EMPTY,
                            text_font(theme::SIZE_SMALL),
                            theme::TEXT_FAINT,
                        );
                    }
                },
                Entry::NearTitle => {
                    ui.painter().text(
                        row.left_bottom(),
                        Align2::LEFT_BOTTOM,
                        WORDS_NEAR,
                        text_font(theme::SIZE_SMALL),
                        theme::TEXT_DIM,
                    );
                }
                Entry::Near(serial) => near_row(ui, row, *serial, frame, tools),
            }
        }
    }

    /// One member: his place, his name over his hits, mana and stamina,
    /// Tell and Kick.
    fn member_row(
        &mut self,
        ui: &egui::Ui,
        row: Rect,
        place: usize,
        member: &WatchPartyMember,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        let live = frame.human_control;
        let kick = Rect::from_min_size(
            Pos2::new(row.right() - BUTTON_WIDTH, row.top()),
            Vec2::new(BUTTON_WIDTH, ROW - TAB_GAP),
        );
        let tell = kick.translate(Vec2::new(-(BUTTON_WIDTH + TAB_GAP), 0.0));
        let name = Rect::from_min_max(
            Pos2::new(row.left() + NUMBER_WIDTH, row.top()),
            Pos2::new(tell.left() - theme::ROW_GAP, row.bottom() - TAB_GAP / 2.0),
        );
        let response = ui.interact(name, Id::new(("party", member.serial)), Sense::click());
        ui.painter().text(
            row.left_center(),
            Align2::LEFT_CENTER,
            format!("{}.", place + 1),
            number_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let color = if self.tell_to == Some(member.serial) {
            theme::GOAL
        } else if response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        ui.painter().text(
            name.left_top(),
            Align2::LEFT_TOP,
            &member.name,
            text_font(theme::SIZE_BODY),
            color,
        );
        let track = Rect::from_min_max(
            Pos2::new(name.left(), name.bottom() - theme::PIP_HEIGHT),
            name.right_bottom(),
        );
        let pools = [
            (member.hits_percent, theme::HITS),
            (member.mana_percent, theme::MANA),
            (member.stam_percent, theme::STAM),
        ];
        let width = (track.width() - POOL_GAP * (POOLS - 1.0)) / POOLS;
        for (at, (percent, color)) in pools.into_iter().enumerate() {
            let left = track.left() + at as f32 * (width + POOL_GAP);
            let part = Rect::from_min_size(
                Pos2::new(left, track.top()),
                Vec2::new(width, track.height()),
            );
            let share = percent.map_or(0.0, |percent| f32::from(percent) / PERCENT);
            theme::bar(ui.painter(), part, share, color);
        }
        if !live {
            return;
        }
        if response.hovered() {
            super::super::tips::label(ui, &member.name, HINT_MEMBER);
        }
        if response.clicked() {
            tools.hand.act(if frame.target_cursor {
                Act::Target(member.serial)
            } else {
                Act::Look(member.serial)
            });
        }
        if member.serial != frame.serial
            && theme::segment_keyed(
                ui,
                tell,
                Id::new(("party-tell", member.serial)),
                WORDS_TELL,
                theme::TEXT,
            )
        {
            self.tell_to = if self.tell_to == Some(member.serial) {
                None
            } else {
                Some(member.serial)
            };
        }
        if leads(frame)
            && member.serial != frame.serial
            && theme::segment_keyed(
                ui,
                kick,
                Id::new(("party-kick", member.serial)),
                WORDS_KICK,
                theme::ALARM,
            )
        {
            tools.hand.act(Act::PartyKick(member.serial));
        }
    }

    /// The words to the party or to one member, and Say.
    fn tell_field(
        &mut self,
        ui: &mut egui::Ui,
        row: Rect,
        frame: &WatchFrame,
        tools: &Tools<'_>,
        speech: &SpeechOptions,
    ) {
        let field = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - SAY_WIDTH - theme::ROW_GAP, row.bottom()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let hint = match self
            .tell_to
            .and_then(|to| frame.party_members.iter().find(|m| m.serial == to))
        {
            Some(member) => format!("{WORDS_TELL} {}", member.name),
            None => HINT_TELL_PARTY.to_string(),
        };
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.words)
                .id(Id::new("party-words"))
                .frame(false)
                .margin(egui::Margin::symmetric(8, 4))
                .hint_text(hint)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        let say = Rect::from_min_max(
            Pos2::new(field.right() + theme::ROW_GAP, row.top()),
            row.max,
        );
        let pressed = theme::segment_keyed(ui, say, Id::new("party-say"), WORDS_SAY, theme::GOAL);
        let entered = typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if (pressed || entered) && !self.words.trim().is_empty() {
            let channel = match self.tell_to {
                Some(member) => Channel::PartyMember(member),
                None => Channel::Party,
            };
            tools.hand.act(Act::Speak {
                channel,
                text: self.words.trim().to_string(),
                hue: channel_hue(speech, Some(channel)),
            });
            self.words.clear();
        }
    }
}

/// One person near, with Invite.
fn near_row(ui: &egui::Ui, row: Rect, serial: u32, frame: &WatchFrame, tools: &Tools<'_>) {
    let Some(mobile) = frame.mobiles.iter().find(|mobile| mobile.serial == serial) else {
        return;
    };
    ui.painter().text(
        Pos2::new(row.left() + NUMBER_WIDTH, row.center().y),
        Align2::LEFT_CENTER,
        &mobile.name,
        text_font(theme::SIZE_BODY),
        theme::TEXT,
    );
    let invite = Rect::from_min_size(
        Pos2::new(row.right() - BUTTON_WIDTH, row.top()),
        Vec2::new(BUTTON_WIDTH, ROW - TAB_GAP),
    );
    if frame.human_control
        && theme::segment_keyed(
            ui,
            invite,
            Id::new(("party-invite", serial)),
            WORDS_INVITE,
            theme::GOAL,
        )
    {
        tools.hand.act(Act::PartyInvite(serial));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchLook, WatchMobile};

    const ME: u32 = 1;
    const BOB: u32 = 2;
    const HUMAN_BODY: u16 = 0x0190;

    #[test]
    fn the_list_holds_ten_places_then_the_people_near_for_the_leader() {
        let mut frame = WatchFrame {
            serial: ME,
            mobiles: vec![WatchMobile {
                serial: BOB,
                name: "Bob".into(),
                dist: 3,
                look: WatchLook {
                    body: HUMAN_BODY,
                    ..WatchLook::default()
                },
                ..WatchMobile::default()
            }],
            ..WatchFrame::default()
        };
        let rows = entries(&frame);
        assert_eq!(rows.len(), PARTY_PLACES + 2);
        assert_eq!(rows[PARTY_PLACES], Entry::NearTitle);
        assert_eq!(rows[PARTY_PLACES + 1], Entry::Near(BOB));
        frame.party_members = vec![
            WatchPartyMember {
                serial: BOB,
                ..WatchPartyMember::default()
            },
            WatchPartyMember {
                serial: ME,
                ..WatchPartyMember::default()
            },
        ];
        assert_eq!(entries(&frame).len(), PARTY_PLACES, "a member does not add");
    }

    #[test]
    fn tell_picks_a_member_and_the_invite_shows_in_its_own_panel() {
        use super::super::testing::{click, draw_frames};
        let member = |serial| WatchPartyMember {
            serial,
            name: format!("member {serial}"),
            hits_percent: Some(50),
            ..WatchPartyMember::default()
        };
        let frame = WatchFrame {
            serial: ME,
            human_control: true,
            party_members: vec![member(ME), member(BOB)],
            party_invite: Some(BOB),
            ..WatchFrame::default()
        };
        let body = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(400.0, 320.0));
        // The invite, the controls, then the first place and Bob's.
        let bob_row = body.top() + ROW * 3.0;
        let tell = Pos2::new(
            body.right() - BUTTON_WIDTH * 1.5 - TAB_GAP,
            bob_row + ROW / 3.0,
        );
        let mut tab = PartyTab::default();
        let mut profile = Profile::default();
        draw_frames(&mut profile, &click(tell), |ui, _, tools, profile| {
            tab.draw(ui, body, &frame, tools, &profile.speech);
        });
        assert_eq!(tab.tell_to, Some(BOB));
        let mut shown = None;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = invite_panel(ui, rect, &frame, tools, profile);
        });
        assert!(shown.is_some());
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = invite_panel(ui, rect, &WatchFrame::default(), tools, profile);
        });
        assert!(shown.is_none(), "no invite, no panel");
    }
}
