//! The health bars of the Modern style. The near list shows every mobile in
//! view as a row with its hits; a row, like a bar, targets its mobile with
//! a click while the shard waits for a target, attacks it with a double
//! click in war and uses it in peace, opens its menu with a right click,
//! and takes an item dropped on it. A row pulled out of the list, or a
//! mobile pulled off the map, opens a bar of its own that the player moves
//! and locks; a box drawn on the map opens a bar for each mobile in it,
//! as the drag-select options of the General page say. The bar of a party
//! member has Heal and Cure, and a pet's bar lets the player rename it.
//! With the new target system on, the target bar follows the last target.
//! A bar closes out of range or dead as "Close health bar when" says, and
//! the bars come back with the next game when the General page saves them.
//! What each bar shows and does is `model::health_bars`', as the classic
//! bars have it.

use super::super::boxes_ui::Tools;
use super::super::control::Act;
use super::super::desk::Zone;
use super::super::model::health_bars::{
    self, BarFacts, FirstFrame, MapAsk, MapBars, Pointer, RangeWatch, Subject, TargetBar,
    NAME_MAX_CHARS, SPELL_CURE, SPELL_GREATER_HEAL,
};
use super::super::model::places;
use super::super::ring_ui::Subject as RingSubject;
use super::super::scene::PickKind;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::{WatchFrame, WatchMobile, MOBILE_LINES};
use eframe::egui::{
    self, text::LayoutJob, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextFormat, Vec2,
};

pub const NEAR_ID: &str = "modern:near";
const BAR_ID_PREFIX: &str = "modern:bar:";
/// The target bar keeps its place under its own id; it opens again with
/// the next target, not with the next game.
const TARGET_BAR_ID: &str = "modern:bar:target";
pub(super) const NEAR_WIDTH: f32 = 300.0;
pub(super) const NEAR_LEAST: Vec2 = Vec2::new(220.0, 110.0);
const ROW: f32 = 20.0;
const DOT_RADIUS: f32 = 4.0;
const GAP: f32 = 6.0;
const PIP_WIDTH: f32 = 40.0;
const DIST_WIDTH: f32 = 30.0;
const BAR_WIDTH: f32 = 210.0;
const LINE_GAP: f32 = 4.0;
const BUTTON_ROW: f32 = 24.0;
const NAME_ROOM: f32 = 60.0;
const SELECTION_FILL_ALPHA: f32 = 0.12;
const SELECTION_EDGE: f32 = 1.0;
/// Each notch of the wheel moves the near list this many rows.
const ROWS_PER_NOTCH: usize = 1;
const WORDS_NEAR: &str = "Near";
const WORDS_NOBODY: &str = "Nobody is near.";
const WORDS_HEAL: &str = "Heal";
const WORDS_CURE: &str = "Cure";
const WORDS_RENAME: &str = "Rename";
const WORDS_CLOSE: &str = "Close bar";
const WORDS_TARGET: &str = "Target";
const HINT_ROW: &str = "Double-click: attack in war, use in peace. Drag out: a bar of its own.";
const HINT_TARGETING: &str = "Click: target it.";

/// The id a bar of a mobile keeps its place and its being open under.
fn bar_id(serial: u32) -> String {
    format!("{BAR_ID_PREFIX}{serial:08X}")
}

/// The mobile of a kept bar id. None for other ids and the target bar.
fn bar_serial(id: &str) -> Option<u32> {
    u32::from_str_radix(id.strip_prefix(BAR_ID_PREFIX)?, 16).ok()
}

/// How many lines of hits, mana and stamina a bar shows.
fn line_count(facts: &BarFacts) -> usize {
    [facts.hits, facts.mana, facts.stam]
        .iter()
        .filter(|value| value.is_some())
        .count()
        .max(1)
}

/// The size of the near list with room for `rows` rows.
pub(super) fn near_size(rows: usize) -> Vec2 {
    Vec2::new(
        NEAR_WIDTH,
        frame::TITLE_ROW + ROW * rows as f32 + theme::PANEL_PAD * 2.0,
    )
}

/// The size of a bar for its facts.
fn bar_size(facts: &BarFacts) -> Vec2 {
    let lines = line_count(facts) as f32;
    let buttons = if party_buttons(facts) {
        BUTTON_ROW + LINE_GAP
    } else {
        0.0
    };
    Vec2::new(
        BAR_WIDTH,
        frame::TITLE_ROW
            + lines * (theme::BAR_HEIGHT + LINE_GAP)
            + buttons
            + theme::PANEL_PAD * 2.0,
    )
}

/// The party buttons show on the bar of another member in range.
fn party_buttons(facts: &BarFacts) -> bool {
    facts.party && !facts.own && facts.in_range
}

/// The color of the hits of a bar.
fn hits_color(facts: &BarFacts) -> Color32 {
    if facts.poisoned {
        theme::HITS_POISONED
    } else if facts.yellow_hits {
        theme::STAM
    } else {
        theme::HITS
    }
}

/// Sends an act of the character, when the human has control.
fn act(frame: &WatchFrame, tools: &Tools<'_>, act: Act) {
    if frame.human_control {
        tools.hand.act(act);
    }
}

/// One bar of its own.
struct OneBar {
    subject: Subject,
    /// Opened from the bars the profile kept, not by the player now.
    restored: bool,
    drawn_once: bool,
    range: RangeWatch,
    /// The name last seen, which a bar out of range keeps.
    name: String,
}

impl OneBar {
    fn new(subject: Subject, restored: bool) -> Self {
        Self {
            subject,
            restored,
            drawn_once: false,
            range: RangeWatch::default(),
            name: String::new(),
        }
    }

    fn id(&self) -> String {
        match self.subject {
            Subject::Mobile(serial) => bar_id(serial),
            _ => TARGET_BAR_ID.to_string(),
        }
    }
}

/// What became of a bar in one frame.
enum Drawn {
    Shown(Rect),
    /// The target bar waits for someone else to be the target.
    Waiting,
    Closed,
}

/// What a click on a row or a bar asks.
#[derive(Default)]
struct Clicks {
    pulled: Option<u32>,
}

#[derive(Default)]
pub struct BarsUi {
    map: MapBars,
    bars: Vec<OneBar>,
    target: Option<OneBar>,
    /// The bars the profile kept are open.
    started: bool,
    /// A bar pulled off a mobile follows the pointer while the button is
    /// held.
    pulling: Option<u32>,
    /// The first row the near list shows.
    first_row: usize,
    /// The name the player types for a pet.
    renaming: String,
}

impl BarsUi {
    /// True when a mobile has a bar of its own.
    fn has_bar(&self, serial: u32) -> bool {
        self.bars
            .iter()
            .any(|bar| bar.subject == Subject::Mobile(serial))
    }

    /// Opens the bar of a mobile, when it has none.
    fn open(&mut self, serial: u32, profile: &mut Profile) {
        places::set_open(profile, &bar_id(serial), true);
        if !self.has_bar(serial) {
            self.bars.push(OneBar::new(Subject::Mobile(serial), false));
        }
    }

    /// Closes every bar of its own, or only those whose mobile is out of
    /// view.
    pub fn close_bars(&mut self, frame: &WatchFrame, inactive_only: bool, profile: &mut Profile) {
        let in_view = |serial: u32| frame.mobiles.iter().any(|mobile| mobile.serial == serial);
        let closing: Vec<String> = self
            .bars
            .iter()
            .filter(|bar| match bar.subject {
                Subject::Mobile(serial) => !inactive_only || !in_view(serial),
                _ => false,
            })
            .map(OneBar::id)
            .collect();
        for id in &closing {
            places::set_open(profile, id, false);
        }
        self.bars.retain(|bar| !closing.contains(&bar.id()));
        if !inactive_only {
            self.target = None;
        }
    }

    /// Draws the near list, the bars and the box of a drag-select. Gives
    /// the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<Rect> {
        if !self.started {
            self.started = true;
            let kept: Vec<u32> = profile
                .interface
                .open_panels
                .iter()
                .filter_map(|id| bar_serial(id))
                .collect();
            self.bars = kept
                .into_iter()
                .map(|serial| OneBar::new(Subject::Mobile(serial), true))
                .collect();
        }
        self.follow_target(frame, profile);
        self.follow_map(ui, rect, frame, tools, profile);
        let mut covered = Vec::new();
        let near = self.near(ui, rect, frame, tools, profile);
        covered.push(near);
        let mut closing = Vec::new();
        let mut bars = std::mem::take(&mut self.bars);
        if let Some(target) = self.target.take() {
            bars.push(target);
        }
        for bar in &mut bars {
            match self.bar(ui, rect, bar, frame, tools, profile) {
                Drawn::Shown(panel) => covered.push(panel),
                Drawn::Waiting => {}
                Drawn::Closed => closing.push(bar.id()),
            }
        }
        bars.retain(|bar| !closing.contains(&bar.id()));
        for bar in bars {
            if bar.subject == Subject::LastTarget {
                self.target = Some(bar);
            } else {
                self.bars.push(bar);
            }
        }
        if !closing.is_empty() {
            for id in &closing {
                places::set_open(profile, id, false);
            }
            tools.keep_profile(profile);
        }
        covered
    }

    /// Opens the target bar for each new last target while the new target
    /// system is on, and closes it when the option is off.
    fn follow_target(&mut self, frame: &WatchFrame, profile: &Profile) {
        match self
            .map
            .follow_target(frame, profile.combat.new_target_system)
        {
            Some(TargetBar::Open) => self.target = Some(OneBar::new(Subject::LastTarget, false)),
            Some(TargetBar::Close) => self.target = None,
            None => {}
        }
    }

    /// Takes the drags of the map: a bar pulled off a mobile, and the box
    /// of a drag-select. A pulled bar follows the pointer until the
    /// button comes up.
    fn follow_map(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let pointer = ui.input(|i| Pointer {
            at: i.pointer.hover_pos(),
            down: i.pointer.primary_down(),
            modifiers: i.modifiers,
        });
        let drag = tools.scene.take_map_drag();
        match self.map.follow(drag, pointer, &profile.general) {
            Some(MapAsk::Pull { serial, .. }) => self.pull(serial, profile),
            Some(MapAsk::Selecting(area)) => {
                let painter = ui.painter();
                painter.rect_filled(
                    area,
                    CornerRadius::ZERO,
                    theme::with_alpha(theme::GOAL, SELECTION_FILL_ALPHA),
                );
                painter.rect_stroke(
                    area,
                    CornerRadius::ZERO,
                    Stroke::new(SELECTION_EDGE, theme::GOAL),
                    StrokeKind::Inside,
                );
                ui.ctx().request_repaint();
            }
            Some(MapAsk::Selected(area)) => self.select(area, rect, frame, tools, profile),
            None => {}
        }
        let Some(serial) = self.pulling else {
            return;
        };
        let id = bar_id(serial);
        match pointer.at.filter(|_| pointer.down) {
            Some(at) => {
                let size = bar_size(&health_bars::facts(frame, serial));
                places::remember(profile, &id, Rect::from_center_size(at, size), false);
                ui.ctx().request_repaint();
            }
            None => {
                self.pulling = None;
                tools.keep_profile(profile);
            }
        }
    }

    /// Opens the bar of a mobile at the pointer, to follow it while the
    /// button is held.
    fn pull(&mut self, serial: u32, profile: &mut Profile) {
        self.open(serial, profile);
        self.pulling = Some(serial);
    }

    /// Opens a bar for each mobile in the box that has none, by the
    /// options, laid out from the start place of the General page.
    fn select(
        &mut self,
        area: Rect,
        screen: Rect,
        frame: &WatchFrame,
        tools: &Tools<'_>,
        profile: &mut Profile,
    ) {
        let general = &profile.general;
        let chosen =
            health_bars::selected_mobiles(tools.scene.mobiles_in(area), frame, general, |serial| {
                self.has_bar(serial)
            });
        let size = bar_size(&BarFacts::default());
        let open: Vec<(u32, Rect)> = self
            .bars
            .iter()
            .filter_map(|bar| match bar.subject {
                Subject::Mobile(serial) => places::kept(profile, &bar_id(serial)).map(|place| {
                    (
                        serial,
                        Rect::from_min_size(Pos2::new(place.x, place.y), size),
                    )
                }),
                _ => None,
            })
            .collect();
        let start = health_bars::select_start(screen, general);
        let joined = general.drag_select_anchored;
        let placed =
            health_bars::select_layout(chosen.len(), start, size, screen, joined, open, |at| {
                chosen[at]
            });
        for (serial, (place, _)) in chosen.iter().zip(placed) {
            self.open(*serial, profile);
            places::remember(
                profile,
                &bar_id(*serial),
                Rect::from_min_size(place, size),
                false,
            );
        }
        if !chosen.is_empty() {
            tools.keep_profile(profile);
        }
    }

    /// The near list: every mobile in view, each row with its hits.
    fn near(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        // Until the player sizes it, the list grows with the mobiles in
        // view, up to its most rows, as far as its room lets it.
        let size = near_size(frame.mobiles.len().clamp(1, MOBILE_LINES));
        let spec = PanelSpec {
            id: NEAR_ID,
            title: WORDS_NEAR,
            default: layout::first_place(rect, Spot::Near, size),
            min_size: Some(NEAR_LEAST),
            closable: false,
        };
        let whole = frame::place(rect, &spec, profile);
        let folded = places::is_folded(profile, NEAR_ID);
        let panel = frame::shown_rect(whole, folded);
        let body = frame::draw(ui.painter(), panel, WORDS_NEAR);
        ui.painter().text(
            Pos2::new(
                frame::mark_area(panel, frame::marks(&spec)).left() - GAP,
                panel.top() + theme::PANEL_PAD,
            ),
            Align2::RIGHT_TOP,
            frame.mobiles.len().to_string(),
            number_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        if !folded {
            self.rows(ui, body, frame, tools, profile);
        }
        frame::foldable_controls(ui, whole, &spec, profile, tools);
        panel
    }

    fn rows(
        &mut self,
        ui: &egui::Ui,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        if frame.mobiles.is_empty() {
            ui.painter().text(
                body.left_top(),
                Align2::LEFT_TOP,
                WORDS_NOBODY,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
            return;
        }
        let fit = ((body.height() / ROW).floor() as usize).max(1);
        let last_first = frame.mobiles.len().saturating_sub(fit);
        let turned = ui.input(|i| {
            let over = i.pointer.hover_pos().is_some_and(|at| body.contains(at));
            if over {
                i.raw_scroll_delta.y
            } else {
                0.0
            }
        });
        self.first_row = if turned < 0.0 {
            self.first_row + ROWS_PER_NOTCH
        } else if turned > 0.0 {
            self.first_row.saturating_sub(ROWS_PER_NOTCH)
        } else {
            self.first_row
        }
        .min(last_first);
        let mut clicks = Clicks::default();
        for (at, mobile) in frame
            .mobiles
            .iter()
            .skip(self.first_row)
            .take(fit)
            .enumerate()
        {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, at as f32 * ROW),
                Vec2::new(body.width(), ROW),
            );
            self.row(ui, row, mobile, frame, tools, &mut clicks);
        }
        if let Some(serial) = clicks.pulled {
            self.pull(serial, profile);
        }
    }

    /// One row of the near list.
    fn row(
        &self,
        ui: &egui::Ui,
        row: Rect,
        mobile: &WatchMobile,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        clicks: &mut Clicks,
    ) {
        let painter = ui.painter();
        let middle = row.center().y;
        let color = theme::notoriety_color(mobile.notoriety);
        painter.circle_filled(
            Pos2::new(row.left() + DOT_RADIUS, middle),
            DOT_RADIUS,
            color,
        );
        let name_left = row.left() + DOT_RADIUS * 2.0 + GAP;
        let pip_left = row.right() - DIST_WIDTH - PIP_WIDTH;
        let marked = mobile.name == frame.combatant || frame.last_target == Some(mobile.serial);
        let name_color = if marked { theme::ALARM } else { theme::TEXT };
        let mut name = LayoutJob::default();
        name.wrap.max_width = pip_left - GAP - name_left;
        name.wrap.max_rows = 1;
        name.wrap.break_anywhere = true;
        let format = |color: Color32| TextFormat::simple(text_font(theme::SIZE_BODY), color);
        name.append(&mobile.name, 0.0, format(name_color));
        if !mobile.title.is_empty() {
            name.append(&mobile.title, GAP, format(theme::TEXT_FAINT));
        }
        let galley = painter.layout_job(name);
        painter.galley(
            Pos2::new(name_left, middle - galley.size().y / 2.0),
            galley,
            name_color,
        );
        if let Some(percent) = mobile.hits_percent {
            let track = Rect::from_min_size(
                Pos2::new(pip_left, middle - theme::PIP_HEIGHT / 2.0),
                Vec2::new(PIP_WIDTH - GAP, theme::PIP_HEIGHT),
            );
            let share = f32::from(percent) / health_bars::PERCENT_FULL as f32;
            theme::bar(painter, track, share, color);
        }
        painter.text(
            Pos2::new(row.right(), middle),
            Align2::RIGHT_CENTER,
            mobile.dist.to_string(),
            number_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let response = ui.interact(
            row,
            Id::new(("near-row", mobile.serial)),
            Sense::click_and_drag(),
        );
        tools.desk.zone(row, Zone::Into(mobile.serial));
        if response.hovered() {
            let words = if frame.target_cursor {
                HINT_TARGETING
            } else {
                HINT_ROW
            };
            super::super::tips::label(ui, &mobile.name, words);
        }
        self.clicked(ui, &response, mobile.serial, &mobile.name, frame, tools);
        if response.drag_started_by(egui::PointerButton::Primary) {
            clicks.pulled = Some(mobile.serial);
        }
    }

    /// The clicks on a row or a bar of a mobile.
    fn clicked(
        &self,
        ui: &egui::Ui,
        response: &egui::Response,
        serial: u32,
        name: &str,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        if response.clicked() {
            if let Some(targeted) = health_bars::click_act(frame, serial) {
                act(frame, tools, targeted);
            }
        }
        if response.double_clicked() {
            if let Some(used) = health_bars::double_click_act(frame, serial) {
                act(frame, tools, used);
            }
        }
        if response.secondary_clicked() && frame.human_control {
            let at = ui
                .input(|i| i.pointer.interact_pos())
                .unwrap_or(response.rect.center());
            tools.ring.open_at(
                at,
                serial,
                name,
                RingSubject::OnMap(PickKind::Mobile),
                tools.hand,
            );
        }
    }

    /// Draws one bar of its own.
    fn bar(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        bar: &mut OneBar,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Drawn {
        let Some(serial) = bar.subject.serial(frame) else {
            return Drawn::Waiting;
        };
        if !bar.drawn_once {
            bar.drawn_once = true;
            match health_bars::first_frame(frame, serial, bar.restored, &profile.general) {
                FirstFrame::Close => return Drawn::Closed,
                FirstFrame::Ask(ask) => act(frame, tools, ask),
                FirstFrame::Stay => {}
            }
        }
        let facts = health_bars::facts(frame, serial);
        if !facts.name.is_empty() {
            bar.name.clone_from(&facts.name);
        }
        if let Some(ask) = bar.range.follow(frame, serial, &facts) {
            act(frame, tools, ask);
        }
        let rule = profile.general.close_health_bar;
        if health_bars::closes_by_rule(rule, &facts, bar.range.hits_gone(), false) {
            return Drawn::Closed;
        }
        let id = bar.id();
        let size = bar_size(&facts);
        let spec = PanelSpec {
            id: &id,
            title: &bar.name,
            default: layout::first_place(rect, Spot::MiddleTop(0), size),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let edge = if facts.marked && facts.in_range {
            theme::ALARM
        } else {
            theme::GLASS_EDGE
        };
        theme::panel_with(ui.painter(), panel, theme::GLASS, edge);
        let inner = panel.shrink(theme::PANEL_PAD);
        let name_color = match facts.notoriety {
            Some(notoriety) if facts.in_range && !facts.dead => theme::notoriety_color(notoriety),
            _ => theme::TEXT_FAINT,
        };
        let mut title = LayoutJob::simple(
            bar.name.clone(),
            text_font(theme::SIZE_BODY),
            name_color,
            (inner.width() - NAME_ROOM).max(0.0),
        );
        title.wrap.max_rows = 1;
        title.wrap.break_anywhere = true;
        ui.painter()
            .galley(inner.left_top(), ui.painter().layout_job(title), name_color);
        let mut y = inner.top() + frame::TITLE_ROW;
        let lines = [
            (facts.hits, hits_color(&facts)),
            (facts.mana, theme::MANA),
            (facts.stam, theme::STAM),
        ];
        for (value, color) in lines.into_iter().take(line_count(&facts)) {
            let track = Rect::from_min_size(
                Pos2::new(inner.left(), y),
                Vec2::new(inner.width(), theme::BAR_HEIGHT),
            );
            let share = health_bars::share(value.filter(|_| facts.in_range)).unwrap_or_default();
            theme::bar(ui.painter(), track, share, color);
            y += theme::BAR_HEIGHT + LINE_GAP;
        }
        if party_buttons(&facts) {
            let width = (inner.width() - GAP) / 2.0;
            for (at, (words, spell)) in [(WORDS_HEAL, SPELL_GREATER_HEAL), (WORDS_CURE, SPELL_CURE)]
                .into_iter()
                .enumerate()
            {
                let button = Rect::from_min_size(
                    Pos2::new(inner.left() + at as f32 * (width + GAP), y),
                    Vec2::new(width, BUTTON_ROW),
                );
                if theme::segment_keyed(
                    ui,
                    button,
                    Id::new(("bar-cast", &id, at)),
                    words,
                    theme::GOAL,
                ) {
                    act(frame, tools, health_bars::cast_on(spell, serial));
                }
            }
        }
        let body = Rect::from_min_max(
            Pos2::new(panel.left(), inner.top() + frame::TITLE_ROW),
            Pos2::new(panel.right(), y),
        );
        let response = ui.interact(body, Id::new(("bar-body", &id)), Sense::click());
        tools.desk.zone(panel, Zone::Into(serial));
        self.clicked(ui, &response, serial, &bar.name, frame, tools);
        let mut closed = false;
        response.context_menu(|ui| {
            closed = self.bar_menu(ui, serial, &facts, frame, tools);
        });
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            closed = true;
        }
        if closed {
            Drawn::Closed
        } else {
            Drawn::Shown(panel)
        }
    }

    /// The menu of a bar: target it, rename a pet, close the bar. True
    /// when the bar is to close.
    fn bar_menu(
        &mut self,
        ui: &mut egui::Ui,
        serial: u32,
        facts: &BarFacts,
        frame: &WatchFrame,
        tools: &Tools<'_>,
    ) -> bool {
        if frame.target_cursor && ui.button(WORDS_TARGET).clicked() {
            act(frame, tools, Act::Target(serial));
            ui.close_menu();
        }
        if health_bars::name_editable(frame, facts) {
            ui.horizontal(|ui| {
                let typed = ui.add(
                    egui::TextEdit::singleline(&mut self.renaming)
                        .char_limit(NAME_MAX_CHARS)
                        .hint_text(facts.name.as_str()),
                );
                let entered = typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(WORDS_RENAME).clicked() || entered {
                    if let Some(renamed) = health_bars::rename(serial, &self.renaming) {
                        act(frame, tools, renamed);
                    }
                    self.renaming.clear();
                    ui.close_menu();
                }
            });
        }
        let closed = ui.button(WORDS_CLOSE).clicked();
        if closed {
            ui.close_menu();
        }
        closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPartyMember;

    const ORC: u32 = 0x0000_0B01;
    const FRIEND: u32 = 0x0000_0B02;

    #[test]
    fn a_bar_id_names_its_mobile_and_nothing_else_does() {
        assert_eq!(bar_serial(&bar_id(ORC)), Some(ORC));
        assert_eq!(bar_serial(TARGET_BAR_ID), None);
        assert_eq!(bar_serial(NEAR_ID), None);
    }

    #[test]
    fn a_party_bar_is_taller_than_the_bar_of_another() {
        let frame = WatchFrame {
            party_members: vec![WatchPartyMember {
                serial: FRIEND,
                name: "Bob".into(),
                hits_percent: Some(80),
                ..WatchPartyMember::default()
            }],
            ..WatchFrame::default()
        };
        let other = health_bars::facts(&frame, ORC);
        let mut member = health_bars::facts(&frame, FRIEND);
        member.in_range = true;
        member.mana = Some((1, 2));
        assert!(!party_buttons(&other) && party_buttons(&member));
        assert!(bar_size(&member).y > bar_size(&other).y);
        assert_eq!(
            line_count(&other),
            1,
            "an unknown mobile still has its line"
        );
        member.poisoned = true;
        assert_eq!(hits_color(&member), theme::HITS_POISONED);
    }

    #[test]
    fn closing_bars_keeps_those_in_view_when_asked() {
        let frame = WatchFrame {
            mobiles: vec![WatchMobile {
                serial: ORC,
                ..WatchMobile::default()
            }],
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        let mut bars = BarsUi::default();
        bars.open(ORC, &mut profile);
        bars.open(FRIEND, &mut profile);
        bars.open(FRIEND, &mut profile);
        assert_eq!(bars.bars.len(), 2, "a mobile has one bar");
        bars.close_bars(&frame, true, &mut profile);
        assert!(bars.has_bar(ORC) && !bars.has_bar(FRIEND));
        assert!(places::is_open(&profile, &bar_id(ORC)));
        assert!(!places::is_open(&profile, &bar_id(FRIEND)));
        bars.close_bars(&frame, false, &mut profile);
        assert!(bars.bars.is_empty());
        assert!(profile.interface.open_panels.is_empty());
    }
}
