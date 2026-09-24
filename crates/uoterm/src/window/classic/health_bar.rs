//! The health bars of mobiles, as the reference client draws them, in
//! both its looks: the character's own bar with mana and
//! stamina, the bars of party members with their heal and cure buttons,
//! the bars of others and of pets, whose names the player may change. The
//! General option "Use custom health bars" picks the flat look, and
//! "Opaque health bar background" its black backdrop. A bar out of range
//! turns grey, and closes when "Close health bar when" says so, unless it
//! is joined to others or shows a party member. Bars join each other, as
//! one anchor group.
//!
//! The player pulls a bar off a mobile on the map; with "Drag-select to
//! open health bars" on, a box drawn on the map opens a bar for each
//! mobile in it, from the start place the General page gives, and joined
//! together when it says so. With the new target system on, the target
//! bar follows the last target. What a bar shows and does, apart from its
//! art, is `model::health_bars`, shared with the Modern bars.

use super::canvas::{ButtonArt, Canvas};
use super::layout::bar_width;
use super::manager::GumpManager;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::WatchFrame;
use crate::window::desk::Zone;
use crate::window::look::notoriety_hue;
use crate::window::model::health_bars::{
    self, BarFacts, FirstFrame, MapAsk, MapBars, Pointer, RangeWatch, Subject, TargetBar,
    NAME_MAX_CHARS, SPELL_CURE, SPELL_GREATER_HEAL,
};
use crate::window::scene::Scene;
use crate::window::settings::Profile;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};
use uoterm_nav::TextAlign;
use uoterm_protocol::types::{NOTO_CRIMINAL, NOTO_GREY};

/// The bar of one mobile, by its serial.
pub const HEALTH_BAR: GumpKind = GumpKind {
    id: well_known::HEALTH_BAR,
    rules: BAR_RULES,
    open: |serial| Box::new(HealthBar::restored(serial.unwrap_or_default())),
};

/// The bar that follows the last target.
pub const TARGET_BAR: GumpKind = GumpKind {
    id: well_known::TARGET_BAR,
    rules: BAR_RULES,
    open: |_| Box::new(HealthBar::of(Subject::LastTarget)),
};

/// The rules of every health bar: they join as one anchor group.
pub const BAR_RULES: GumpRules = GumpRules {
    anchor: Some(well_known::HEALTH_BAR_GROUP),
    ..GumpRules::DEFAULT
};

// The classic bars.
const BACKGROUND: u16 = 0x0803;
const BACKGROUND_WAR: u16 = 0x0807;
const BACKGROUND_OTHER: u16 = 0x0804;
const LINE_RED: u16 = 0x0805;
const LINE_BLUE: u16 = 0x0806;
const LINE_POISONED: u16 = 0x0808;
const LINE_YELLOW_HITS: u16 = 0x0809;
const LINE_RED_PARTY: u16 = 0x0028;
const LINE_BLUE_PARTY: u16 = 0x0029;
const HEAL_BUTTON: (u16, u16, u16) = (0x0938, 0x093A, 0x0938);
const CURE_BUTTON: (u16, u16, u16) = (0x0939, 0x093A, 0x0939);
const HEAL_AT: (i32, i32) = (0, 20);
const CURE_AT: (i32, i32) = (0, 33);
const OWN_LINES_X: i32 = 34;
const OWN_LINES_Y: [i32; 3] = [12, 25, 38];
const OWN_LINE_WIDTH: i32 = 109;
const OTHER_LINE_Y: i32 = 38;
const PARTY_LINES_X: i32 = 18;
const PARTY_LINES_Y: [i32; 3] = [20, 33, 45];
const PARTY_LINE_WIDTH: i32 = 96;
const PARTY_NAME_AT: (i32, i32) = (0, -2);
const PARTY_OWN_NAME_WIDTH: u32 = 120;
const PARTY_NAME_WIDTH: u32 = 109;
const PARTY_NAME_FONT: u8 = 3;
/// A party member's bar takes the clicks of this box.
const PARTY_SIZE: Vec2 = Vec2::new(115.0, 55.0);
const OTHER_NAME_AT: (i32, i32) = (16, 14);
const OTHER_NAME_WIDTH: i32 = 120;
const OTHER_NAME_HEIGHT: i32 = 15;
const NAME_FONT: u8 = 1;
const PARTY_POISONED_HUE: u16 = 63;
const PARTY_YELLOW_HITS_HUE: u16 = 353;
/// The hue of words and lines of a mobile out of range, and of the words
/// of a bar with no notoriety.
const GREY_HUE: u16 = 0x0386;
/// The hue of the name of a pet, which the player may change.
const RENAMABLE_HUE: u16 = 0x000E;
/// The hue of everything of a bar out of range, of a party member's and
/// of the custom bars.
const OUT_OF_RANGE_HUE: u16 = 912;
const OWN_PARTY_NAME: &str = "[* SELF *]";

// The custom bars.
const CUSTOM_WIDTH: i32 = 120;
const CUSTOM_HEIGHT_MULTILINE: i32 = 60;
const CUSTOM_HEIGHT_SINGLE: i32 = 36;
const CUSTOM_BAR_WIDTH: i32 = 100;
const CUSTOM_BAR_HEIGHT: i32 = 8;
const CUSTOM_BAR_LEFT: i32 = (CUSTOM_WIDTH - CUSTOM_BAR_WIDTH) / 2;
const CUSTOM_LINES_Y: [i32; 3] = [27, 36, 45];
const CUSTOM_SINGLE_LINE_Y: i32 = 21;
const CUSTOM_BORDER: i32 = 1;
const CUSTOM_OUTLINE: i32 = 1;
const CUSTOM_LINE_GAP: i32 = 2;
const CUSTOM_NAME_Y_MULTILINE: i32 = 3;
const CUSTOM_BACKGROUND_OPACITY: f32 = 0.7;
const CUSTOM_RED: Color32 = Color32::from_rgb(255, 0, 0);
const CUSTOM_BLUE: Color32 = Color32::from_rgb(30, 144, 255);
const CUSTOM_GREY: Color32 = Color32::from_rgb(128, 128, 128);
const CUSTOM_YELLOW: Color32 = Color32::from_rgb(255, 165, 0);
const CUSTOM_POISON: Color32 = Color32::from_rgb(50, 205, 50);
const CUSTOM_BLACK: Color32 = Color32::BLACK;

// Drag-select.
/// The size the reference client keeps a custom bar in when it lays out drag-selected
/// bars.
const SELECT_CUSTOM_SIZE: Vec2 = Vec2::new(CUSTOM_BAR_WIDTH as f32, CUSTOM_HEIGHT_MULTILINE as f32);
const SELECTION_FILL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 77);
const SELECTION_EDGE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 179);
const SELECTION_EDGE_WIDTH: f32 = 1.0;
const HALF: f32 = 2.0;

/// The hue of the background of a classic bar of another mobile: none for
/// a criminal, a grey one or one of no notoriety.
fn other_background_hue(profile: &Profile, facts: &BarFacts) -> u16 {
    match facts.notoriety {
        Some(noto) if facts.in_range && noto != NOTO_CRIMINAL && noto != NOTO_GREY => {
            notoriety_hue(&profile.combat, noto)
        }
        _ => 0,
    }
}

/// The hue of the name and of the backdrop of a custom bar.
fn custom_hue(profile: &Profile, facts: &BarFacts) -> u16 {
    match facts.notoriety {
        Some(noto) if facts.in_range => notoriety_hue(&profile.combat, noto),
        _ => OUT_OF_RANGE_HUE,
    }
}

/// One health bar.
pub struct HealthBar {
    subject: Subject,
    /// Opened from the places the profile kept, not by the player now.
    restored: bool,
    /// Where the bar goes in its first frame, in window points: its top
    /// left corner, or its middle for a bar pulled off a mobile.
    first: Option<FirstPlace>,
    /// The bar it joins after its first frame, as drag-selected bars do.
    join_to: Option<GumpId>,
    drawn_once: bool,
    /// The name last seen, which a bar out of range keeps.
    name: String,
    field: TextField,
    /// The player types a new name for a pet.
    editing: bool,
    /// Whether the mobile is in range, and whether its hits went to
    /// nothing.
    range: RangeWatch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FirstPlace {
    TopLeft(Pos2),
    Middle(Pos2),
}

impl HealthBar {
    fn new(subject: Subject, restored: bool, first: Option<FirstPlace>) -> Self {
        Self {
            subject,
            restored,
            first,
            join_to: None,
            drawn_once: false,
            name: String::new(),
            field: TextField::new("").with_max_chars(Some(NAME_MAX_CHARS)),
            editing: false,
            range: RangeWatch::default(),
        }
    }

    pub fn of(subject: Subject) -> Self {
        Self::new(subject, false, None)
    }

    /// The character's own bar.
    pub fn own() -> Self {
        Self::of(Subject::Own)
    }

    /// A bar the profile kept open, opened again at the start.
    pub fn restored(serial: u32) -> Self {
        Self::new(Subject::Mobile(serial), true, None)
    }

    /// A bar the player pulls off a mobile, with its middle at the mouse.
    pub fn pulled(serial: u32, mouse: Pos2) -> Self {
        Self::new(
            Subject::Mobile(serial),
            false,
            Some(FirstPlace::Middle(mouse)),
        )
    }

    /// A bar opened at a place, joined to `join_to` when it is given.
    pub fn placed(serial: u32, place: Pos2, join_to: Option<GumpId>) -> Self {
        Self {
            join_to,
            ..Self::new(
                Subject::Mobile(serial),
                false,
                Some(FirstPlace::TopLeft(place)),
            )
        }
    }

    /// The first frame: a restored bar of another mobile stays only when
    /// the profile saves health bars; a new bar asks the shard for the
    /// status of its mobile.
    fn first_frame(&mut self, cx: &mut GumpContext<'_>, serial: u32) -> bool {
        match health_bars::first_frame(cx.frame, serial, self.restored, &cx.profile.general) {
            FirstFrame::Stay => true,
            FirstFrame::Ask(act) => {
                cx.act(act);
                true
            }
            FirstFrame::Close => {
                cx.close(cx.me);
                false
            }
        }
    }

    /// The name: a pet's may be written over when it is `editable`.
    fn name(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &GumpContext<'_>,
        serial: u32,
        spot: (i32, i32, i32, i32),
        look: &TextLook,
        editable: bool,
    ) {
        let (x, y, w, h) = spot;
        if editable {
            if !self.editing && self.field.text() != self.name {
                self.field.set_text(&self.name);
            }
            let done = g.text_box(("name", serial), x, y, w, h, &mut self.field, look);
            self.editing |= done.changed;
            if done.submitted {
                self.editing = false;
                if let Some(act) = health_bars::rename(serial, self.field.text()) {
                    cx.act(act);
                }
            }
        } else {
            let look = look.cropped(w.max(0) as u32);
            g.label(x, y, &self.name, &look);
        }
    }

    /// Draws a classic bar. Gives its size.
    fn draw_classic(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        serial: u32,
        facts: &BarFacts,
    ) -> Vec2 {
        if facts.party {
            self.draw_classic_party(g, cx, serial, facts);
            return PARTY_SIZE;
        }
        let blue = if facts.poisoned {
            LINE_POISONED
        } else if facts.yellow_hits {
            LINE_YELLOW_HITS
        } else {
            LINE_BLUE
        };
        if facts.own {
            let background = if facts.war {
                BACKGROUND_WAR
            } else {
                BACKGROUND
            };
            let size = g.pic(0, 0, background, 0);
            let values = [facts.hits, facts.mana, facts.stam];
            for (line, (value, top)) in values.into_iter().zip(OWN_LINES_Y).enumerate() {
                g.pic(OWN_LINES_X, top, LINE_RED, 0);
                let art = if line == 0 { blue } else { LINE_BLUE };
                bar_line(g, OWN_LINES_X, top, art, 0, value, OWN_LINE_WIDTH);
            }
            return size;
        }
        let size = g.pic(
            0,
            0,
            BACKGROUND_OTHER,
            other_background_hue(cx.profile, facts),
        );
        let red_hue = if facts.in_range { 0 } else { GREY_HUE };
        g.pic(OWN_LINES_X, OTHER_LINE_Y, LINE_RED, red_hue);
        if facts.in_range {
            bar_line(
                g,
                OWN_LINES_X,
                OTHER_LINE_Y,
                blue,
                0,
                facts.hits,
                OWN_LINE_WIDTH,
            );
        }
        let hue = if facts.renamable && facts.in_range {
            RENAMABLE_HUE
        } else {
            GREY_HUE
        };
        let look = TextLook::ascii(NAME_FONT, hue);
        let (x, y) = OTHER_NAME_AT;
        self.name(
            g,
            cx,
            serial,
            (x, y, OTHER_NAME_WIDTH, OTHER_NAME_HEIGHT),
            &look,
            health_bars::name_editable(cx.frame, facts),
        );
        size
    }

    fn draw_classic_party(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        serial: u32,
        facts: &BarFacts,
    ) {
        // The background is there to be clicked and dragged, not seen.
        g.faded(0.0, |g| g.pic(0, 0, BACKGROUND, 0));
        let hue = match facts.notoriety {
            Some(noto) if facts.in_range => notoriety_hue(&cx.profile.combat, noto),
            _ => OUT_OF_RANGE_HUE,
        };
        let (words, width) = if facts.own {
            (OWN_PARTY_NAME, PARTY_OWN_NAME_WIDTH)
        } else {
            (self.name.as_str(), PARTY_NAME_WIDTH)
        };
        let look = TextLook::ascii(PARTY_NAME_FONT, hue).cropped(width);
        g.label(PARTY_NAME_AT.0, PARTY_NAME_AT.1, words, &look);
        let line_hue = if facts.in_range { 0 } else { OUT_OF_RANGE_HUE };
        for top in PARTY_LINES_Y {
            g.pic(PARTY_LINES_X, top, LINE_RED_PARTY, line_hue);
        }
        if !facts.in_range {
            return;
        }
        let hits_hue = if facts.poisoned {
            PARTY_POISONED_HUE
        } else if facts.yellow_hits {
            PARTY_YELLOW_HITS_HUE
        } else {
            0
        };
        let values = [facts.hits, facts.mana, facts.stam];
        for (line, (value, top)) in values.into_iter().zip(PARTY_LINES_Y).enumerate() {
            let hue = if line == 0 { hits_hue } else { 0 };
            bar_line(
                g,
                PARTY_LINES_X,
                top,
                LINE_BLUE_PARTY,
                hue,
                value,
                PARTY_LINE_WIDTH,
            );
        }
        let buttons = [
            (HEAL_BUTTON, HEAL_AT, SPELL_GREATER_HEAL),
            (CURE_BUTTON, CURE_AT, SPELL_CURE),
        ];
        for ((normal, pressed, over), (x, y), spell) in buttons {
            let art = ButtonArt::new(normal, pressed, over);
            if g.button(("cast", spell), x, y, art) {
                cx.act(health_bars::cast_on(spell, serial));
            }
        }
    }

    /// Draws a custom bar. Gives its size.
    fn draw_custom(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        serial: u32,
        facts: &BarFacts,
    ) -> Vec2 {
        let multiline = facts.own || facts.party;
        let height = if multiline {
            CUSTOM_HEIGHT_MULTILINE
        } else {
            CUSTOM_HEIGHT_SINGLE
        };
        let hue = custom_hue(cx.profile, facts);
        let backdrop = if facts.dead || cx.profile.general.opaque_health_bars {
            OUT_OF_RANGE_HUE
        } else {
            hue
        };
        g.shade(
            0,
            0,
            CUSTOM_WIDTH,
            height,
            backdrop,
            CUSTOM_BACKGROUND_OPACITY,
        );
        let tops: &[i32] = if multiline {
            &CUSTOM_LINES_Y
        } else {
            &[CUSTOM_SINGLE_LINE_Y]
        };
        let outline_height =
            CUSTOM_BAR_HEIGHT * tops.len() as i32 + CUSTOM_LINE_GAP * (tops.len() as i32 - 1);
        g.fill(
            CUSTOM_BAR_LEFT - CUSTOM_OUTLINE,
            tops[0] - CUSTOM_OUTLINE,
            CUSTOM_BAR_WIDTH + CUSTOM_OUTLINE * 2,
            outline_height + CUSTOM_OUTLINE * 2,
            CUSTOM_BLACK,
        );
        let red = if facts.in_range {
            CUSTOM_RED
        } else {
            CUSTOM_GREY
        };
        let blue_hits = if facts.poisoned {
            CUSTOM_POISON
        } else if facts.yellow_hits {
            CUSTOM_YELLOW
        } else {
            CUSTOM_BLUE
        };
        let values = [facts.hits, facts.mana, facts.stam];
        for (line, (top, value)) in tops.iter().zip(values).enumerate() {
            g.fill(
                CUSTOM_BAR_LEFT,
                *top,
                CUSTOM_BAR_WIDTH,
                CUSTOM_BAR_HEIGHT,
                red,
            );
            let Some((current, most)) = value.filter(|_| facts.in_range) else {
                continue;
            };
            let width = bar_width(most, current, CUSTOM_BAR_WIDTH);
            let color = if line == 0 { blue_hits } else { CUSTOM_BLUE };
            if width > 0 {
                g.fill(CUSTOM_BAR_LEFT, *top, width, CUSTOM_BAR_HEIGHT, color);
            }
        }
        let alarm = if facts.own {
            facts.war
        } else {
            facts.marked && facts.in_range
        };
        let border = if alarm { CUSTOM_RED } else { CUSTOM_BLACK };
        for (x, y, w, h) in [
            (0, 0, CUSTOM_WIDTH, CUSTOM_BORDER),
            (0, height - CUSTOM_BORDER, CUSTOM_WIDTH, CUSTOM_BORDER),
            (0, 0, CUSTOM_BORDER, height),
            (CUSTOM_WIDTH - CUSTOM_BORDER, 0, CUSTOM_BORDER, height),
        ] {
            g.fill(x, y, w, h, border);
        }
        let name_y = if multiline {
            CUSTOM_NAME_Y_MULTILINE
        } else {
            0
        };
        let look = TextLook::unicode(NAME_FONT, hue)
            .bordered()
            .aligned(TextAlign::Center)
            .wrap(CUSTOM_WIDTH as u32);
        self.name(
            g,
            cx,
            serial,
            (0, name_y, CUSTOM_WIDTH, OTHER_NAME_HEIGHT),
            &look,
            health_bars::name_editable(cx.frame, facts),
        );
        Vec2::new(CUSTOM_WIDTH as f32, height as f32)
    }

    /// A click while the shard waits for a target targets the mobile. A
    /// double click opens the status of the character, attacks another in
    /// war and uses him in peace.
    fn clicks(&self, g: &Canvas<'_>, cx: &mut GumpContext<'_>, serial: u32) {
        if let Some(act) = g
            .body_click()
            .and_then(|_| health_bars::click_act(cx.frame, serial))
        {
            cx.act(act);
            return;
        }
        if !g.body_double_click() {
            return;
        }
        if let Some(act) = health_bars::double_click_act(cx.frame, serial) {
            cx.act(act);
            return;
        }
        cx.open_at(GumpId::one(well_known::STATUS), g.at(0, 0));
        if cx.profile.general.status_and_bar_exclusive {
            cx.close(cx.me);
        }
    }
}

/// One line of a classic bar, as wide as `value` of the whole width.
fn bar_line(
    g: &mut Canvas<'_>,
    x: i32,
    y: i32,
    art: u16,
    hue: u16,
    value: Option<(i32, i32)>,
    width: i32,
) {
    if let Some((current, most)) = value {
        let shown = bar_width(most, current, width);
        if shown > 0 {
            g.pic_width(x, y, art, hue, shown);
        }
    }
}

impl GumpBody for HealthBar {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(serial) = self.subject.serial(cx.frame) else {
            return;
        };
        if !self.drawn_once && !self.first_frame(cx, serial) {
            return;
        }
        let facts = health_bars::facts(cx.frame, serial);
        if !facts.name.is_empty() {
            self.name.clone_from(&facts.name);
        }
        if let Some(act) = self.range.follow(cx.frame, serial, &facts) {
            cx.act(act);
        }
        let rule = cx.profile.general.close_health_bar;
        if health_bars::closes_by_rule(rule, &facts, self.range.hits_gone(), cx.anchored) {
            cx.close(cx.me);
            return;
        }
        let size = if cx.profile.general.custom_health_bars {
            self.draw_custom(g, cx, serial, &facts)
        } else {
            self.draw_classic(g, cx, serial, &facts)
        };
        self.clicks(g, cx, serial);
        // An item dropped on the bar goes to the mobile, as on the mobile.
        cx.desk.zone(g.area(0, 0, size), Zone::Into(serial));
        if !self.drawn_once {
            self.drawn_once = true;
            match self.first.take() {
                Some(FirstPlace::TopLeft(place)) => g.move_to(place),
                Some(FirstPlace::Middle(middle)) => {
                    let shown = g.area(0, 0, size).size();
                    g.move_to(middle - shown / HALF);
                }
                None => {}
            }
        } else if let Some(host) = self.join_to.take() {
            cx.join(host);
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        self.subject.serial(frame).is_some()
    }
}

/// The size of a new bar in window points, for the layout of drag-selected
/// bars.
fn new_bar_size(scene: &mut Scene, profile: &Profile) -> Vec2 {
    let size = if profile.general.custom_health_bars {
        SELECT_CUSTOM_SIZE
    } else {
        scene
            .gump_picture(BACKGROUND_OTHER, 0)
            .map_or(Vec2::ZERO, |(_, sprite)| {
                Vec2::new(sprite.width, sprite.height)
            })
    };
    size * profile.video.ui_scale
}

/// What the map hands the classic health bars: a bar pulled off a mobile,
/// the box of a drag-select, and the last target the target bar follows.
#[derive(Default)]
pub struct WorldBars {
    map: MapBars,
}

impl WorldBars {
    /// Follows the map for one frame in the Classic style. The Modern
    /// style leaves the drags of the map to its own bars.
    #[allow(clippy::too_many_arguments)]
    pub fn follow(
        &mut self,
        ui: &egui::Ui,
        screen: Rect,
        manager: &mut GumpManager,
        scene: &mut Scene,
        frame: &WatchFrame,
        profile: &mut Profile,
        classic: bool,
    ) {
        if !classic {
            self.map.stop_selecting();
            return;
        }
        self.follow_target(manager, frame, profile);
        let pointer = ui.input(|i| Pointer {
            at: i.pointer.hover_pos(),
            down: i.pointer.primary_down(),
            modifiers: i.modifiers,
        });
        let drag = scene.take_map_drag();
        match self.map.follow(drag, pointer, &profile.general) {
            Some(MapAsk::Pull { serial, mouse }) => {
                let id = GumpId::of(well_known::HEALTH_BAR, serial);
                manager.close(&id, profile);
                manager.open_body(id, Box::new(HealthBar::pulled(serial, mouse)), profile);
                manager.drag_with_pointer(id);
            }
            Some(MapAsk::Selecting(area)) => {
                let painter = ui.painter();
                painter.rect_filled(area, CornerRadius::ZERO, SELECTION_FILL);
                painter.rect_stroke(
                    area,
                    CornerRadius::ZERO,
                    Stroke::new(SELECTION_EDGE_WIDTH, SELECTION_EDGE),
                    StrokeKind::Inside,
                );
                ui.ctx().request_repaint();
            }
            Some(MapAsk::Selected(area)) => select(area, screen, manager, scene, frame, profile),
            None => {}
        }
    }

    /// Opens the target bar for each new last target while the new target
    /// system is on, and closes it when the option is off.
    fn follow_target(
        &mut self,
        manager: &mut GumpManager,
        frame: &WatchFrame,
        profile: &mut Profile,
    ) {
        let id = GumpId::one(well_known::TARGET_BAR);
        match self
            .map
            .follow_target(frame, profile.combat.new_target_system)
        {
            Some(TargetBar::Open) => {
                manager.open(id, profile);
            }
            Some(TargetBar::Close) if manager.is_open(&id) => manager.close(&id, profile),
            _ => {}
        }
    }
}

/// Opens a bar for each mobile in the box that has none, by the options.
fn select(
    area: Rect,
    screen: Rect,
    manager: &mut GumpManager,
    scene: &mut Scene,
    frame: &WatchFrame,
    profile: &mut Profile,
) {
    let general = &profile.general;
    let chosen = health_bars::selected_mobiles(scene.mobiles_in(area), frame, general, |serial| {
        manager.is_open(&GumpId::of(well_known::HEALTH_BAR, serial))
    });
    let start = health_bars::select_start(screen, general);
    let joined = general.drag_select_anchored;
    let size = new_bar_size(scene, profile);
    let bars = manager.drawn_in_group(well_known::HEALTH_BAR_GROUP);
    let places =
        health_bars::select_layout(chosen.len(), start, size, screen, joined, bars, |at| {
            GumpId::of(well_known::HEALTH_BAR, chosen[at])
        });
    for (serial, (place, join)) in chosen.iter().zip(places) {
        let id = GumpId::of(well_known::HEALTH_BAR, *serial);
        manager.open_body(
            id,
            Box::new(HealthBar::placed(*serial, place, join)),
            profile,
        );
    }
}

/// Opens the bar of a mobile with its middle at a window place, as the
/// Status button of another's paperdoll does. A bar that is open stays.
pub fn open_bar_at(cx: &mut GumpContext<'_>, serial: u32, middle: Pos2) {
    let id = GumpId::of(well_known::HEALTH_BAR, serial);
    cx.open_with(id, Box::new(HealthBar::pulled(serial, middle)));
}
