//! The gump manager: the floating gumps of the classic look, as the
//! reference client keeps them. The last gump in the list is on top; a click on a
//! gump brings it to the top. A gump drags by its pictures and words, not
//! by its controls, and a right click closes it when its kind allows. With
//! "Alt to move gumps" on, a gump drags only while Alt is down. Ctrl+Alt
//! and a click lock a gump: it stays where it is and a right click does not
//! close it. A dropped gump keeps a quarter of itself on the screen. The
//! places, sizes, locks and open gumps are kept in the profile under the
//! place key of each gump. Health bars and other anchored kinds snap
//! together and move together.
//!
//! Every gump draws in one egui layer, in its order, so the controls of a
//! higher gump win the clicks over a lower one.

use super::anchor::Anchors;
use super::canvas::{Canvas, CanvasInput};
use super::layout::{in_screen, rest_place};
use super::registry::{
    kind, well_known, Closing, GumpBody, GumpCommand, GumpContext, GumpId, GumpRules,
};
use super::text::{TextKit, TextLook};
use crate::view::WatchFrame;
use crate::window::control::Hand;
use crate::window::desk::Desk;
use crate::window::model::journal::JournalLog;
use crate::window::model::places;
use crate::window::model::reads::Readings;
use crate::window::scene::Scene;
use crate::window::settings::{AnchorCell, GumpPlace, MacroStep, Profile};
use crate::window::tips::Tips;
use eframe::egui::{
    self, Color32, CornerRadius, Id, LayerId, Modifiers, Order, Pos2, Rect, Sense, Stroke,
    StrokeKind, Vec2,
};
use std::collections::{BTreeMap, HashMap};
use uoterm_nav::TextAlign;

const AREA_ID: &str = "classic-gumps";
/// The ids of the places that drag a gump, apart from its controls.
const BODY_KEY: &str = "body";
const TIP_LAYER: &str = "classic-tip";
const PLACE_KEY_SEPARATOR: char = ':';
const HEX_RADIX: u32 = 16;
const PERCENT_FULL: f32 = 100.0;
const MILLIS_PER_SECOND: f64 = 1000.0;
/// The candidate place of a gump that would join another.
const JOIN_PREVIEW: Color32 = Color32::from_rgba_premultiplied(96, 96, 96, 128);
const JOIN_PREVIEW_STROKE: f32 = 2.0;
/// The lock the classic client shows on a gump that stays put.
const LOCK_GUMP: u16 = 0x082C;
// The tooltip of the classic client.
const TIP_OFFSET: Vec2 = Vec2::new(0.0, 24.0);
const TIP_MAX_WIDTH: u32 = 600;
const TIP_PAD: f32 = 4.0;
const TIP_TEXT_AT: Vec2 = Vec2::new(3.0, 3.0);
const TIP_BORDER: Color32 = Color32::GRAY;

/// One open gump.
struct Shown {
    id: GumpId,
    rules: GumpRules,
    body: Box<dyn GumpBody>,
    /// Where its top left corner is, in window points.
    place: Pos2,
    /// The size the player gave it, in its own pixels.
    size: Option<Vec2>,
    locked: bool,
    /// What it drew in the last frame, in window points.
    drawn: Option<Rect>,
    body_rects: Vec<Rect>,
    hits: Vec<Rect>,
    /// The place it would take, and the gump it would join, while dragged.
    joining: Option<(GumpId, Pos2)>,
    /// It has not had its first place yet.
    fresh: bool,
    /// It opened at its first place and waits for its size, to be held
    /// wholly in the window.
    unheld: bool,
}

impl Shown {
    fn takes(&self, pointer: Pos2) -> bool {
        self.hits.iter().any(|hit| hit.contains(pointer))
    }
}

/// What the manager draws with in one frame.
pub struct ManagerInputs<'a> {
    pub frame: &'a WatchFrame,
    pub scene: &'a mut Scene,
    pub text: &'a mut TextKit,
    pub hand: &'a Hand,
    pub tips: &'a mut Tips,
    pub profile: &'a mut Profile,
    pub desk: &'a mut Desk,
    pub journal: &'a JournalLog,
    pub readings: &'a mut Readings,
    pub time: f64,
    pub sound_note: &'a str,
    /// Draw only the gumps that show in both styles.
    pub shared_only: bool,
}

/// What one frame of the manager gives the window.
#[derive(Default)]
pub struct ManagerOutcome {
    /// Where the gumps are, so the map does not take their clicks.
    pub covered: Vec<Rect>,
    /// The profile changed, so the window keeps it.
    pub profile_changed: bool,
    /// The player made the profile the start of each new character.
    pub save_as_default: bool,
    /// The macros the gumps asked to run, in order.
    pub macros: Vec<Vec<MacroStep>>,
    /// The sound effects the gumps asked to play.
    pub sounds: Vec<u16>,
}

/// What the pointer did this frame, before the gumps draw.
struct Pointer {
    at: Option<Pos2>,
    pressed_at: Option<Pos2>,
    right_click: bool,
    modifiers: Modifiers,
}

#[derive(Default)]
pub struct GumpManager {
    shown: Vec<Shown>,
    anchors: Anchors,
    /// The places of gumps the profile does not keep, until the window
    /// closes, as the classic client remembers the gumps of the shard.
    session_places: HashMap<String, Pos2>,
    answers: HashMap<String, u16>,
    /// The words of the tooltip under the pointer, and since when.
    tip: Option<(String, f64)>,
    /// A gump opened, moved or closed since the last frame changed the
    /// profile, so the window keeps it.
    dirty: bool,
    /// A gump just opened that the mouse drags once it was drawn.
    pulled: Option<GumpId>,
}

/// The gump id a place key names, when its kind is known.
fn id_of_place(key: &str) -> Option<GumpId> {
    match key.split_once(PLACE_KEY_SEPARATOR) {
        Some((name, serial)) => Some(GumpId::of(
            kind(name)?.id,
            u32::from_str_radix(serial, HEX_RADIX).ok()?,
        )),
        None => Some(GumpId::one(kind(key)?.id)),
    }
}

impl GumpManager {
    /// A gump waits for a key press.
    pub fn wants_keys(&self) -> bool {
        self.shown.iter().any(|gump| gump.body.wants_keys())
    }

    pub fn is_open(&self, id: &GumpId) -> bool {
        self.shown.iter().any(|gump| gump.id == *id)
    }

    /// Opens the gumps the profile says were open, or `first` when the
    /// profile knows no classic gump yet.
    pub fn open_kept(&mut self, profile: &mut Profile, first: &[GumpId]) {
        let open: Vec<GumpId> = profile
            .interface
            .open_panels
            .iter()
            .filter_map(|key| id_of_place(key))
            .collect();
        let known = profile.gumps.keys().any(|key| id_of_place(key).is_some());
        let ids = if open.is_empty() && !known {
            first.to_vec()
        } else {
            open
        };
        for id in ids {
            self.open(id, profile);
        }
    }

    /// Closes every gump the profile keeps, before another profile comes.
    pub fn close_kept(&mut self) {
        let (kept, rest): (Vec<Shown>, Vec<Shown>) =
            self.shown.drain(..).partition(|gump| gump.rules.kept);
        self.shown = rest;
        for gump in kept {
            self.anchors.leave(&gump.id);
        }
    }

    /// Opens a gump of a registered kind, or brings it to the top when it
    /// is open. False when no kind has its id.
    pub fn open(&mut self, id: GumpId, profile: &mut Profile) -> bool {
        let Some(kind) = kind(id.kind) else {
            return false;
        };
        self.open_body(id, (kind.open)(id.serial), profile);
        true
    }

    /// Opens a gump with a body the caller made, or brings it to the top
    /// when it is open. Nothing opens for a kind no one registered.
    pub fn open_body(&mut self, id: GumpId, body: Box<dyn GumpBody>, profile: &mut Profile) {
        if let Some(at) = self.shown.iter().position(|gump| gump.id == id) {
            self.raise(at);
            return;
        }
        let Some(kind) = kind(id.kind) else {
            return;
        };
        let kept = profile.gumps.get(&id.place_key()).copied();
        if let Some(cell) = profile.anchored.get(&id.place_key()) {
            self.anchors.restore(id, *cell);
        }
        let remembered = self.remembered_place(&id, profile);
        self.shown.push(Shown {
            id,
            rules: kind.rules,
            body,
            place: remembered.unwrap_or(kind.rules.first_place),
            size: kept
                .and_then(|place| place.size)
                .map(|(w, h)| Vec2::new(w, h)),
            locked: kept.is_some_and(|place| place.locked),
            drawn: None,
            body_rects: Vec::new(),
            hits: Vec::new(),
            joining: None,
            fresh: remembered.is_none(),
            unheld: remembered.is_none(),
        });
        let last = self.shown.len() - 1;
        self.keep(last, profile);
    }

    /// Opens a gump with its top left corner at a window place, or moves
    /// it there when it is open.
    pub fn open_at(&mut self, id: GumpId, place: Pos2, profile: &mut Profile) {
        self.open(id, profile);
        if let Some(at) = self.shown.iter().position(|gump| gump.id == id) {
            self.shown[at].place = place;
            self.shown[at].fresh = false;
            self.shown[at].unheld = false;
            self.keep(at, profile);
        }
    }

    /// Where an open gump is, in window points.
    pub fn place_of(&self, id: &GumpId) -> Option<Pos2> {
        self.shown
            .iter()
            .find(|gump| gump.id == *id)
            .map(|gump| gump.place)
    }

    /// Where a gump that is not open would open by the places kept for
    /// it, in the profile or since the window opened.
    pub fn remembered_place(&self, id: &GumpId, profile: &Profile) -> Option<Pos2> {
        let key = id.place_key();
        profile
            .gumps
            .get(&key)
            .map(|place| Pos2::new(place.x, place.y))
            .or_else(|| self.session_places.get(&key).copied())
    }

    /// Closes a gump, if it is open.
    pub fn close(&mut self, id: &GumpId, profile: &mut Profile) {
        if let Some(at) = self.shown.iter().position(|gump| gump.id == *id) {
            self.remove(at, profile);
        }
    }

    pub fn toggle(&mut self, id: GumpId, profile: &mut Profile) {
        if self.is_open(&id) {
            self.close(&id, profile);
        } else {
            self.open(id, profile);
        }
    }

    /// Closes every gump a right click would close, except the gumps of
    /// the shard, which wait for an answer.
    pub fn close_all(&mut self, profile: &mut Profile) {
        let closing: Vec<GumpId> = self
            .shown
            .iter()
            .filter(|gump| gump.rules.right_click_closes && gump.id.kind != well_known::SHARD)
            .map(|gump| gump.id)
            .collect();
        for id in closing {
            self.close(&id, profile);
        }
    }

    /// Where the open gumps of one kind were drawn, in window points, for
    /// the tests of the gumps.
    #[cfg(test)]
    pub fn drawn_of(&self, kind: &str) -> Vec<(GumpId, Rect)> {
        self.drawn_where(|gump| gump.id.kind == kind)
    }

    /// Where the open gumps of one anchor group were drawn, in window
    /// points.
    pub fn drawn_in_group(&self, group: &str) -> Vec<(GumpId, Rect)> {
        self.drawn_where(|gump| gump.rules.anchor == Some(group))
    }

    fn drawn_where(&self, pick: impl Fn(&Shown) -> bool) -> Vec<(GumpId, Rect)> {
        self.shown
            .iter()
            .filter(|gump| pick(gump))
            .filter_map(|gump| gump.drawn.map(|drawn| (gump.id, drawn)))
            .collect()
    }

    /// The mouse, whose left button is down, drags this gump once it was
    /// drawn, as a health bar pulled off a mobile follows it.
    pub fn drag_with_pointer(&mut self, id: GumpId) {
        self.pulled = Some(id);
    }

    /// Hands the drag of the mouse to the pulled gump once it has pictures
    /// to drag by. A button that came up ends the pull.
    fn start_pull(&mut self, ctx: &egui::Context, primary_down: bool) {
        let Some(id) = self.pulled else {
            return;
        };
        match self.shown.iter().find(|gump| gump.id == id) {
            Some(gump) if primary_down && !gump.body_rects.is_empty() => {
                ctx.set_dragged_id(Id::new((AREA_ID, id)).with((BODY_KEY, 0)));
                self.pulled = None;
            }
            Some(_) if primary_down => {}
            _ => self.pulled = None,
        }
    }

    /// Joins an open gump to another of its anchor group, at the side it
    /// stands, as a drop on it does.
    fn join(&mut self, id: GumpId, host: GumpId, profile: &mut Profile) {
        let rect_of = |gump: &Shown| {
            gump.drawn
                .map(|drawn| Rect::from_min_size(gump.place, drawn.size()))
        };
        let at = self.shown.iter().position(|gump| gump.id == id);
        let host_at = self.shown.iter().position(|gump| gump.id == host);
        let (Some(at), Some(host_at)) = (at, host_at) else {
            return;
        };
        let group = self.shown[at].rules.anchor;
        if group.is_none() || group != self.shown[host_at].rules.anchor {
            return;
        }
        let (Some(rect), Some(host_rect)) =
            (rect_of(&self.shown[at]), rect_of(&self.shown[host_at]))
        else {
            return;
        };
        if let Some(place) = self.anchors.join((&id, rect), (&host, host_rect)) {
            self.shown[at].place = place;
            self.keep(at, profile);
        }
    }

    /// Closes the gumps of one anchor group that `pick` names.
    pub fn close_group(
        &mut self,
        group: &str,
        profile: &mut Profile,
        pick: impl Fn(&GumpId) -> bool,
    ) {
        let closing: Vec<GumpId> = self
            .shown
            .iter()
            .filter(|gump| gump.rules.anchor == Some(group) && pick(&gump.id))
            .map(|gump| gump.id)
            .collect();
        for id in closing {
            self.close(&id, profile);
        }
    }

    fn remove(&mut self, at: usize, profile: &mut Profile) -> bool {
        let gump = self.shown.remove(at);
        self.anchors.leave(&gump.id);
        let key = gump.id.place_key();
        let was_open = places::is_open(profile, &key);
        let changed = gump.rules.kept && was_open;
        if changed {
            places::set_open(profile, &key, false);
            self.dirty = true;
        }
        changed
    }

    /// Moves a gump and the gumps joined to it to the top.
    fn raise(&mut self, at: usize) {
        let id = self.shown[at].id;
        let mut group = self.anchors.partners(&id);
        group.push(id);
        let (mut rest, mut top): (Vec<Shown>, Vec<Shown>) = self
            .shown
            .drain(..)
            .partition(|gump| !group.contains(&gump.id));
        rest.append(&mut top);
        self.shown = rest;
    }

    /// Keeps the place of one open gump. True when the profile changed. The
    /// window leaves the places out of the file when the Interface page
    /// does not keep them.
    fn keep(&mut self, at: usize, profile: &mut Profile) -> bool {
        let gump = &self.shown[at];
        let key = gump.id.place_key();
        if !gump.rules.kept {
            self.session_places.insert(key, gump.place);
            return false;
        }
        let place = GumpPlace {
            x: gump.place.x,
            y: gump.place.y,
            size: gump.size.map(|size| (size.x, size.y)),
            locked: gump.locked,
        };
        let opened = !places::is_open(profile, &key);
        if opened {
            places::set_open(profile, &key, true);
        }
        let moved = profile.gumps.insert(key, place) != Some(place);
        self.dirty |= opened || moved;
        opened || moved
    }

    /// Keeps the anchor cells of the open kept gumps. True when the profile
    /// changed.
    fn keep_anchors(&self, profile: &mut Profile) -> bool {
        let anchored: BTreeMap<String, AnchorCell> = self
            .shown
            .iter()
            .filter(|gump| gump.rules.kept)
            .filter_map(|gump| Some((gump.id.place_key(), self.anchors.cell(&gump.id)?)))
            .collect();
        let changed = profile.anchored != anchored;
        profile.anchored = anchored;
        changed
    }

    /// The top gump that takes the pointer at a place.
    fn top_at(&self, pointer: Pos2, shared_only: bool) -> Option<usize> {
        self.shown
            .iter()
            .rposition(|gump| (!shared_only || gump.rules.both_styles) && gump.takes(pointer))
    }

    /// Draws every open gump and follows what the player did to them.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        screen: Rect,
        inputs: ManagerInputs<'_>,
    ) -> ManagerOutcome {
        let ManagerInputs {
            frame,
            scene,
            text,
            hand,
            tips,
            profile,
            desk,
            journal,
            readings,
            time,
            sound_note,
            shared_only,
        } = inputs;
        let mut macros = Vec::new();
        let mut sounds = Vec::new();
        let mut changed = false;
        let mut save_default = false;
        let mut at = 0;
        while at < self.shown.len() {
            if self.shown[at].body.alive(frame) {
                at += 1;
            } else {
                changed |= self.remove(at, profile);
            }
        }
        let pointer = ui.input(|i| Pointer {
            at: i.pointer.hover_pos(),
            pressed_at: i
                .pointer
                .primary_pressed()
                .then(|| i.pointer.press_origin())
                .flatten(),
            right_click: i.pointer.secondary_clicked(),
            modifiers: i.modifiers,
        });
        let primary_down = ui.input(|i| i.pointer.primary_down());
        self.start_pull(ui.ctx(), primary_down);
        if let Some(pressed) = pointer.pressed_at {
            if let Some(at) = self.top_at(pressed, shared_only) {
                if pointer.modifiers.ctrl && pointer.modifiers.alt {
                    self.shown[at].locked = !self.shown[at].locked;
                    changed |= self.keep(at, profile);
                }
                self.raise(at);
            }
        }
        // The gumps that stay on top draw last, over the one raised.
        let (mut below, mut over): (Vec<Shown>, Vec<Shown>) = self
            .shown
            .drain(..)
            .partition(|gump| !gump.body.on_top(profile));
        below.append(&mut over);
        self.shown = below;
        let right_clicked = pointer
            .at
            .filter(|_| pointer.right_click)
            .and_then(|at| self.top_at(at, shared_only))
            .map(|at| self.shown[at].id);
        let scale = profile.video.ui_scale;
        let alpha = f32::from(profile.interface.gump_opacity) / PERCENT_FULL;
        let top_under_pointer = pointer
            .at
            .and_then(|at| self.top_at(at, shared_only))
            .map(|at| self.shown[at].id);
        let mut commands = Vec::new();
        let mut closing = Vec::new();
        let mut tip = None;
        let ctx = ui.ctx().clone();
        egui::Area::new(Id::new(AREA_ID))
            .order(Order::Middle)
            .fixed_pos(screen.min)
            .interactable(false)
            .constrain(false)
            .show(&ctx, |ui| {
                for at in 0..self.shown.len() {
                    if shared_only && !self.shown[at].rules.both_styles {
                        continue;
                    }
                    let gump_id = self.shown[at].id;
                    let egui_id = Id::new((AREA_ID, gump_id));
                    if self.shown[at].rules.modal {
                        ui.interact(screen, egui_id.with("modal"), Sense::click_and_drag());
                    }
                    let locks = self.shown[at].body.locks(frame);
                    let mut answers = std::mem::take(&mut self.answers);
                    let mut cx = GumpContext {
                        frame,
                        hand,
                        tips: &mut *tips,
                        profile: &mut *profile,
                        sound_note,
                        me: gump_id,
                        anchored: self.anchors.is_joined(&gump_id),
                        desk: &mut *desk,
                        journal,
                        readings: &mut *readings,
                        commands: &mut commands,
                        macros: &mut macros,
                        sounds: &mut sounds,
                        answers: &mut answers,
                        profile_changed: &mut changed,
                        save_default: &mut save_default,
                    };
                    let rules = self.shown[at].rules;
                    let mut right_click = false;
                    if right_clicked == Some(gump_id) {
                        let may_close = rules.right_click_closes && !locks.no_close;
                        if may_close && !self.shown[at].locked {
                            let joined = self.anchors.is_joined(&gump_id);
                            let general = &cx.profile.general;
                            let held_back = joined
                                && general.alt_right_click_closes_anchored
                                && !pointer.modifiers.alt;
                            let whole_group = general.right_click_closes_anchored_group;
                            if !held_back && self.shown[at].body.close(&mut cx) == Closing::Now {
                                closing.push(gump_id);
                                if whole_group {
                                    closing.extend(self.anchors.partners(&gump_id));
                                }
                            }
                        } else if !rules.right_click_closes {
                            right_click = true;
                        }
                    }
                    if closing.contains(&gump_id) {
                        self.answers = answers;
                        continue;
                    }
                    if self.shown[at].fresh {
                        if let Some(first) = self.shown[at].body.first_place(frame) {
                            self.shown[at].place = first;
                        }
                        self.shown[at].fresh = false;
                    }
                    let mut drag = Vec2::ZERO;
                    let mut drag_stopped = false;
                    let mut body_click = None;
                    let mut body_double_click = false;
                    for (n, rect) in self.shown[at].body_rects.iter().enumerate() {
                        let response = ui.interact(
                            *rect,
                            egui_id.with((BODY_KEY, n)),
                            Sense::click_and_drag(),
                        );
                        drag += response.drag_delta();
                        drag_stopped |= response.drag_stopped();
                        if response.clicked() {
                            body_click = response.interact_pointer_pos();
                        }
                        body_double_click |= response.double_clicked();
                    }
                    let input = CanvasInput {
                        id: egui_id,
                        origin: self.shown[at].place,
                        scale,
                        alpha,
                        pointer: pointer.at.filter(|_| top_under_pointer == Some(gump_id)),
                        body_click,
                        body_double_click,
                        right_click,
                        size: match rules.resizable {
                            Some((first, least)) => {
                                Some(self.shown[at].size.unwrap_or(first).max(least))
                            }
                            None => self.shown[at].size,
                        },
                        map: frame.map,
                    };
                    let mut canvas = Canvas::new(ui, &mut *scene, &mut *text, input);
                    self.shown[at].body.draw(&mut canvas, &mut cx);
                    if let Some((_, least)) = rules.resizable {
                        canvas.resize_grip(least);
                    }
                    let out = canvas.finish();
                    let whole = out.whole;
                    self.answers = answers;
                    if out.tip.is_some() {
                        tip = out.tip;
                    }
                    let gump = &mut self.shown[at];
                    gump.drawn = out.drawn;
                    gump.body_rects = out.body;
                    gump.hits = out.hits;
                    if let Some(size) = out.size_kept {
                        gump.size = Some(size);
                        changed |= self.keep(at, profile);
                    }
                    if let Some(place) = out.move_to {
                        self.shown[at].place = place;
                        changed |= self.keep(at, profile);
                    }
                    // A gump that opens at its first place opens wholly in
                    // the window, as far as it fits, once its size is known.
                    let gump = &mut self.shown[at];
                    if let Some(whole) = whole.filter(|_| gump.unheld) {
                        gump.place += places::held_inside(whole, screen).min - whole.min;
                        gump.unheld = false;
                        changed |= self.keep(at, profile);
                    }
                    if self.shown[at].locked && pointer.modifiers.alt {
                        self.draw_lock(ui, at, &mut *scene, scale);
                    }
                    let may_move = rules.movable
                        && !self.shown[at].locked
                        && !locks.no_move
                        && (!profile.general.alt_moves_gumps || pointer.modifiers.alt);
                    if may_move && drag != Vec2::ZERO {
                        self.drag(at, drag, &pointer.modifiers, profile);
                        if let Some((_, place)) = self.shown[at].joining {
                            let size = self.shown[at].drawn.map_or(Vec2::ZERO, |r| r.size());
                            preview_join(ui, Rect::from_min_size(place, size));
                        }
                    }
                    if drag_stopped {
                        changed |= self.drop(at, screen, profile);
                    }
                }
            });
        for id in closing {
            if let Some(at) = self.shown.iter().position(|gump| gump.id == id) {
                changed |= self.remove(at, profile);
            }
        }
        // Opening, moving and closing a gump may change its kept place.
        changed |= !commands.is_empty();
        for command in commands {
            match command {
                GumpCommand::Open(id) => {
                    self.open(id, profile);
                }
                GumpCommand::OpenAt(id, place) => self.open_at(id, place, profile),
                GumpCommand::OpenWith(id, body) => self.open_body(id, body, profile),
                GumpCommand::Close(id) => self.close(&id, profile),
                GumpCommand::Toggle(id) => self.toggle(id, profile),
                GumpCommand::Join(id, host) => self.join(id, host, profile),
                GumpCommand::DragWithPointer(id) => self.drag_with_pointer(id),
            }
        }
        for gump in &mut self.shown {
            let size = gump.drawn.map_or(Vec2::ZERO, |rect| rect.size());
            gump.place = in_screen(gump.place, size, screen);
        }
        changed |= self.keep_anchors(profile);
        self.show_tip(&ctx, text, profile, tip, pointer.at, time, screen);
        ManagerOutcome {
            covered: self
                .shown
                .iter()
                .filter(|gump| !shared_only || gump.rules.both_styles)
                .filter_map(|gump| gump.drawn)
                .collect(),
            profile_changed: changed || std::mem::take(&mut self.dirty),
            save_as_default: save_default,
            macros,
            sounds,
        }
    }

    /// Moves a dragged gump and the gumps joined to it. With Alt down and
    /// "Alt to move gumps" off, the gump leaves its group instead.
    fn drag(&mut self, at: usize, delta: Vec2, modifiers: &Modifiers, profile: &Profile) {
        let id = self.shown[at].id;
        if modifiers.alt && !profile.general.alt_moves_gumps {
            self.anchors.leave(&id);
        }
        self.shown[at].place += delta;
        for partner in self.anchors.partners(&id) {
            if let Some(gump) = self.shown.iter_mut().find(|gump| gump.id == partner) {
                gump.place += delta;
            }
        }
        self.shown[at].joining = self.join_candidate(at);
    }

    /// The gump a dragged gump would join if dropped now, and where it
    /// would go: the nearest one of its anchor group that it overlaps.
    fn join_candidate(&self, at: usize) -> Option<(GumpId, Pos2)> {
        let dragged = &self.shown[at];
        let group = dragged.rules.anchor?;
        let rect = Rect::from_min_size(dragged.place, dragged.drawn?.size());
        self.shown
            .iter()
            .filter(|host| host.id != dragged.id && host.rules.anchor == Some(group))
            .filter_map(|host| {
                let host_rect = Rect::from_min_size(host.place, host.drawn?.size());
                let distance = (host.place - dragged.place).abs();
                rect.intersects(host_rect).then(|| {
                    let place = self
                        .anchors
                        .drop_place((&dragged.id, rect), (&host.id, host_rect));
                    (distance.x + distance.y, host.id, place)
                })
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .and_then(|(_, host, place)| place.map(|place| (host, place)))
    }

    /// A dragged gump is let go: it joins the gump it was dropped on, or
    /// rests where a quarter of it stays on the screen.
    fn drop(&mut self, at: usize, screen: Rect, profile: &mut Profile) -> bool {
        let size = self.shown[at].drawn.map_or(Vec2::ZERO, |rect| rect.size());
        let id = self.shown[at].id;
        let joined = self.shown[at].joining.take().and_then(|(host, _)| {
            let host_gump = self.shown.iter().find(|gump| gump.id == host)?;
            let host_rect = Rect::from_min_size(host_gump.place, host_gump.drawn?.size());
            let rect = Rect::from_min_size(self.shown[at].place, size);
            self.anchors.join((&id, rect), (&host, host_rect))
        });
        self.shown[at].place =
            joined.unwrap_or_else(|| rest_place(self.shown[at].place, size, screen));
        let mut changed = self.keep(at, profile);
        for partner in self.anchors.partners(&id) {
            if let Some(other) = self.shown.iter().position(|gump| gump.id == partner) {
                changed |= self.keep(other, profile);
            }
        }
        changed
    }

    /// The lock at the top right corner of a locked gump, while Alt is down.
    fn draw_lock(&self, ui: &egui::Ui, at: usize, scene: &mut Scene, scale: f32) {
        let Some(drawn) = self.shown[at].drawn else {
            return;
        };
        if let Some((texture, sprite)) = scene.gump_picture(LOCK_GUMP, 0) {
            let size = Vec2::new(sprite.width, sprite.height) * scale;
            let rect = Rect::from_min_size(Pos2::new(drawn.right() - size.x, drawn.top()), size);
            ui.painter().image(texture, rect, sprite.uv, Color32::WHITE);
        }
    }

    /// The classic tooltip: words in a black box by the pointer, after the
    /// wait the profile sets.
    #[allow(clippy::too_many_arguments)]
    fn show_tip(
        &mut self,
        ctx: &egui::Context,
        text: &mut TextKit,
        profile: &Profile,
        words: Option<String>,
        pointer: Option<Pos2>,
        time: f64,
        screen: Rect,
    ) {
        let options = &profile.tooltip;
        let (Some(words), Some(pointer), true) = (words, pointer, options.enabled) else {
            self.tip = None;
            return;
        };
        let since = match &self.tip {
            Some((known, since)) if *known == words => *since,
            _ => time,
        };
        self.tip = Some((words.clone(), since));
        if (time - since) * MILLIS_PER_SECOND < f64::from(options.delay_ms) {
            ctx.request_repaint();
            return;
        }
        // The hue that means "the font's own color" is white for a Unicode
        // font, as the classic tooltip draws.
        let hue = options.text_hue;
        let width = text
            .fonts
            .width(super::text::UoFont::Unicode(options.font), &words);
        let look = TextLook::unicode(options.font, hue)
            .bordered()
            .aligned(TextAlign::Center)
            .wrap(width.clamp(1, TIP_MAX_WIDTH));
        let Some(texture) = text.label(ctx, &words, &look) else {
            return;
        };
        let zoom = f32::from(options.zoom) / PERCENT_FULL;
        let size = (texture.size + Vec2::splat(TIP_PAD * 2.0)) * zoom;
        let mut corner = pointer + TIP_OFFSET;
        corner.x = corner
            .x
            .clamp(screen.left(), (screen.right() - size.x).max(screen.left()));
        corner.y = corner
            .y
            .clamp(screen.top(), (screen.bottom() - size.y).max(screen.top()));
        let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new(TIP_LAYER)));
        let back = Rect::from_min_size(corner - Vec2::new(TIP_PAD, TIP_PAD / 2.0), size);
        let opacity = f32::from(options.background_opacity) / PERCENT_FULL;
        painter.rect_filled(
            back,
            CornerRadius::ZERO,
            Color32::BLACK.gamma_multiply(opacity),
        );
        painter.rect_stroke(
            back,
            CornerRadius::ZERO,
            Stroke::new(1.0, TIP_BORDER.gamma_multiply(opacity)),
            StrokeKind::Inside,
        );
        let words_rect = Rect::from_min_size(corner + TIP_TEXT_AT, texture.size * zoom);
        painter.image(
            texture.id(),
            words_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// The silver box that shows where a dragged gump would join another.
fn preview_join(ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, JOIN_PREVIEW);
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(JOIN_PREVIEW_STROKE, Color32::from_gray(192)),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::registry::GumpBody;

    struct Plain;

    impl GumpBody for Plain {
        fn draw(&mut self, _: &mut Canvas<'_>, _: &mut GumpContext<'_>) {}
    }

    fn shown(id: GumpId, x: f32) -> Shown {
        Shown {
            id,
            rules: GumpRules::DEFAULT,
            body: Box::new(Plain),
            place: Pos2::new(x, 0.0),
            size: None,
            locked: false,
            drawn: None,
            body_rects: Vec::new(),
            hits: vec![Rect::from_min_size(Pos2::new(x, 0.0), Vec2::splat(50.0))],
            joining: None,
            fresh: false,
            unheld: false,
        }
    }

    const FIRST: GumpId = GumpId::of("test", 1);
    const SECOND: GumpId = GumpId::of("test", 2);
    const THIRD: GumpId = GumpId::of("test", 3);

    fn order(manager: &GumpManager) -> Vec<GumpId> {
        manager.shown.iter().map(|gump| gump.id).collect()
    }

    #[test]
    fn the_top_gump_takes_the_pointer_and_a_raise_keeps_the_rest_in_order() {
        let mut manager = GumpManager {
            shown: vec![shown(FIRST, 0.0), shown(SECOND, 20.0), shown(THIRD, 200.0)],
            ..GumpManager::default()
        };
        assert_eq!(manager.top_at(Pos2::new(30.0, 10.0), false), Some(1));
        assert_eq!(manager.top_at(Pos2::new(10.0, 10.0), false), Some(0));
        assert_eq!(manager.top_at(Pos2::new(500.0, 10.0), false), None);
        manager.raise(0);
        assert_eq!(order(&manager), vec![SECOND, THIRD, FIRST]);
        assert_eq!(manager.top_at(Pos2::new(30.0, 10.0), false), Some(2));
    }

    #[test]
    fn a_raise_takes_the_joined_gumps_along() {
        let mut manager = GumpManager {
            shown: vec![shown(FIRST, 0.0), shown(SECOND, 60.0), shown(THIRD, 200.0)],
            ..GumpManager::default()
        };
        let rect = |x: f32| Rect::from_min_size(Pos2::new(x, 0.0), Vec2::splat(50.0));
        manager
            .anchors
            .join((&SECOND, rect(40.0)), (&FIRST, rect(0.0)));
        manager.raise(0);
        assert_eq!(order(&manager), vec![THIRD, FIRST, SECOND]);
    }

    #[test]
    fn places_are_kept_under_their_keys_and_open_again() {
        let mut profile = Profile::default();
        let mut manager = GumpManager {
            shown: vec![shown(FIRST, 30.0)],
            ..GumpManager::default()
        };
        manager.shown[0].locked = true;
        assert!(manager.keep(0, &mut profile));
        assert!(
            !manager.keep(0, &mut profile),
            "an unchanged place changes nothing"
        );
        let kept = profile.gumps["test:1"];
        assert_eq!((kept.x, kept.y, kept.locked), (30.0, 0.0, true));
        assert!(places::is_open(&profile, "test:1"));
        assert!(manager.remove(0, &mut profile));
        assert!(!places::is_open(&profile, "test:1"));
        assert!(profile.gumps.contains_key("test:1"), "the place stays");
        manager.shown = vec![shown(SECOND, 70.0)];
        manager.shown[0].rules.kept = false;
        assert!(!manager.keep(0, &mut profile));
        assert_eq!(manager.session_places["test:2"], Pos2::new(70.0, 0.0));
    }

    #[test]
    fn joined_gumps_keep_their_group_and_join_again_when_they_open() {
        let status = GumpId::one(well_known::STATUS);
        let bar = GumpId::one(well_known::SELF_BAR);
        let mut profile = Profile::default();
        let mut manager = GumpManager {
            shown: vec![shown(status, 0.0), shown(bar, 60.0)],
            ..GumpManager::default()
        };
        let rect = |x: f32| Rect::from_min_size(Pos2::new(x, 0.0), Vec2::splat(50.0));
        manager
            .anchors
            .join((&bar, rect(40.0)), (&status, rect(0.0)));
        assert!(manager.keep_anchors(&mut profile));
        assert!(!manager.keep_anchors(&mut profile), "nothing new to keep");
        assert_eq!(profile.anchored.len(), 2);
        manager.close_kept();
        assert!(!manager.anchors.is_joined(&status));
        let mut next_start = GumpManager::default();
        next_start.open_body(status, Box::new(Plain), &mut profile);
        next_start.open_body(bar, Box::new(Plain), &mut profile);
        assert_eq!(next_start.anchors.partners(&status), vec![bar]);
        assert!(!next_start.keep_anchors(&mut profile));
    }

    /// What the demo gump saw in its last frame.
    #[derive(Default)]
    struct Seen {
        size: Option<Vec2>,
        scroll: Vec2,
    }

    /// A gump that draws with the controls of the paperdoll, the journal
    /// and the skills gump.
    struct Demo(std::rc::Rc<std::cell::RefCell<Seen>>);

    /// The scroll of paper the journal is drawn on.
    const JOURNAL_SCROLL: u16 = 0x1F40;
    const JOURNAL_HEIGHT: i32 = 300;
    /// The least height of the classic expandable scroll.
    const SCROLL_LEAST: f32 = 274.0;

    impl GumpBody for Demo {
        fn draw(&mut self, g: &mut Canvas<'_>, _: &mut GumpContext<'_>) {
            let mut seen = self.0.borrow_mut();
            seen.size = g.size();
            let sprite = crate::window::atlas::Sprite {
                uv: Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                width: 10.0,
                height: 10.0,
                anchor: Vec2::ZERO,
            };
            g.sprite(0, 0, egui::TextureId::default(), sprite);
            seen.scroll = g.expandable_scroll(0, 0, JOURNAL_SCROLL, JOURNAL_HEIGHT);
        }
    }

    #[test]
    fn a_resizable_gump_draws_at_its_first_size_and_scrolls_stay_in_range() {
        use crate::window::classic::testing::draw_frames;
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Seen::default()));
        let mut gump = shown(FIRST, 0.0);
        let (first, least) = (Vec2::new(300.0, 200.0), Vec2::new(100.0, 100.0));
        gump.rules.resizable = Some((first, least));
        gump.body = Box::new(Demo(seen.clone()));
        let mut manager = GumpManager {
            shown: vec![gump],
            ..GumpManager::default()
        };
        let mut profile = Profile::default();
        if !draw_frames(&mut manager, &mut profile, &WatchFrame::default()) {
            return;
        }
        assert_eq!(seen.borrow().size, Some(first));
        // The scroll takes the kept height of the gump, held at its least.
        assert_eq!(seen.borrow().scroll.y, SCROLL_LEAST);
        assert!(manager.shown[0].drawn.is_some());
    }

    /// A gump of a fixed size, filled.
    struct Block;

    const BLOCK: (i32, i32) = (200, 100);
    /// How far a gump first stands past the right edge of the screen.
    const PAST_EDGE: f32 = 50.0;

    impl GumpBody for Block {
        fn draw(&mut self, g: &mut Canvas<'_>, _: &mut GumpContext<'_>) {
            g.fill(0, 0, BLOCK.0, BLOCK.1, Color32::WHITE);
        }
    }

    #[test]
    fn a_gump_at_its_first_place_opens_wholly_in_the_window() {
        use crate::window::classic::testing::{draw_frames, SCREEN};
        let off_edge = SCREEN.x - PAST_EDGE;
        let mut first = shown(FIRST, off_edge);
        first.body = Box::new(Block);
        first.unheld = true;
        let mut moved = shown(SECOND, off_edge);
        moved.body = Box::new(Block);
        let mut manager = GumpManager {
            shown: vec![first, moved],
            ..GumpManager::default()
        };
        let mut profile = Profile::default();
        if !draw_frames(&mut manager, &mut profile, &WatchFrame::default()) {
            return;
        }
        let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);
        let drawn = manager.shown[0].drawn.expect("the gump drew");
        assert!(screen.contains_rect(drawn), "{drawn:?}");
        assert_eq!(
            manager.shown[1].place.x, off_edge,
            "a place the player gave stays"
        );
    }

    #[test]
    fn a_place_key_names_a_known_kind_and_its_serial() {
        let status = super::super::status::STATUS.id;
        assert_eq!(id_of_place(status), Some(GumpId::one(status)));
        assert_eq!(
            id_of_place(&format!("{status}:1F")),
            Some(GumpId::of(status, 0x1F))
        );
        assert_eq!(id_of_place("unknown"), None);
        assert_eq!(id_of_place(&format!("{status}:zz")), None);
    }
}
