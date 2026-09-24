//! The skills gump of the classic client, in the two looks of the
//! reference client that the General option "Standard skills gump" picks between:
//!
//! - The standard look: a scroll of paper the player makes longer, with
//!   the skills in groups that fold. Each skill has its use button, its
//!   value (or its real value or its cap, by the boxes at the foot) and its
//!   lock. The player makes, names and takes away groups and drags skills
//!   between them; the profile keeps the groups. The sum of the values
//!   shows at the foot, and the gump folds to its small picture.
//! - The advanced look: a dark table of every skill with its real value,
//!   its value and its cap, sorted by a column, and the sums.
//!
//! In both, dragging a skill that can be used out of the gump makes a
//! skill button on the desktop.

use super::canvas::{ButtonArt, Canvas};
use super::message_box::{BoxLook, MessageBox, ASKING_RULES};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::skill_button::{button_place, SKILL_BUTTON};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::WatchSkill;
use crate::window::control::Act;
use crate::window::model::skills::{
    group_name, move_skill, next_lock_command, points, remove_group, short_points, shown_groups,
    sorted, total, SkillSort, NEW_GROUP, WORDS_CANNOT_DELETE,
};
use crate::window::settings::SkillGroupSet;
use eframe::egui::{Color32, Key, Rect, Vec2};
use uoterm_nav::TextAlign;

pub const SKILLS: GumpKind = GumpKind {
    id: well_known::SKILLS,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(Skills::default()),
};

/// The question before the groups go back to those of the client files.
const RESET_GROUPS_ID: &str = "reset_skill_groups";

pub const RESET_GROUPS: GumpKind = GumpKind {
    id: RESET_GROUPS_ID,
    rules: ASKING_RULES,
    open: |_| {
        Box::new(MessageBox::new(
            WORDS_RESET_QUESTION,
            BoxLook::Framed {
                size: RESET_BOX,
                cancel: true,
                background: false,
            },
            Box::new(|yes, cx, _| {
                if yes {
                    cx.profile.skill_groups.clear();
                    cx.profile_changed();
                }
            }),
        ))
    },
};

const RESET_BOX: (i32, i32) = (300, 200);
const WORDS_RESET_QUESTION: &str =
    "Skills will be placed in default groups.\nDo you want reset all groups?";
/// The size of the message the reference client shows when the first group would go.
const CANNOT_DELETE_BOX: (i32, i32) = (200, 125);

// The standard gump.
const DIFF_Y: i32 = 22;
const SCROLL: u16 = 0x1F40;
const SCROLL_HEIGHT: i32 = 200 + DIFF_Y;
const TITLE: u16 = 0x0834;
const HEADER: u16 = 0x082D;
const HEADER_AT: (i32, i32) = (160, 0);
const MINIMIZED: u16 = 0x0839;
const MINIMIZE_BOX: (i32, i32, i32, i32) = (160, 0, 23, 24);
const LINE: u16 = 0x082B;
const LINE_X: i32 = 50;
const TOP_LINE_Y: i32 = 35 + DIFF_Y;
const BOTTOM_LINE_UP: i32 = 98;
const COMMENT: u16 = 0x0836;
const COMMENT_X: i32 = 25;
const COMMENT_UP: i32 = 85;
const AREA_X: i32 = 22;
const AREA_Y: i32 = 45 + DIFF_Y;
const AREA_LINE_ROOM: i32 = 10;
const AREA_NARROWER: i32 = 14;
const AREA_SHORTER: i32 = 150 + DIFF_Y;
const SUM_GAP: i32 = 5;
const SUM_DOWN: i32 = 2;
const SUM_FONT: u8 = 3;
const SUM_HUE: u16 = 600;
const NEW_GROUP_BUTTON: ButtonArt = ButtonArt::new(0x083A, 0x083A, 0x083A);
const NEW_GROUP_X: i32 = 60;
const NEW_GROUP_UP: i32 = 52;
const CHECK: (u16, u16) = (0x0938, 0x0939);
const CHECK_GAP: i32 = 30;
const REAL_UP: i32 = 6;
const CAPS_DOWN: i32 = 7;
const CHECK_FONT: u8 = 1;
const CHECK_HUE: u16 = 0x0386;
const WORDS_SHOW_REAL: &str = " - Show Real";
const WORDS_SHOW_CAPS: &str = " - Show Caps";
const RESET_AT: (i32, i32, i32, i32) = (25, 7 + DIFF_Y, 100, 18);
const RESET_FONT: u8 = 6;
const RESET_HUE: u16 = 0xFFFF;
const WORDS_RESET: &str = "Reset groups";
// A group and its skills.
const GROUP_AT: i32 = 3;
const ROW: i32 = 17;
const GROUP_OPEN: u16 = 0x0826;
const GROUP_CLOSED: u16 = 0x0827;
const GROUP_NAME_X: i32 = 16;
const GROUP_NAME_LIFT: i32 = 3;
const GROUP_NAME_WIDTH: i32 = 200;
const GROUP_FONT: u8 = 6;
const GROUP_HUE: u16 = 0;
const GROUP_RULE: u16 = 0x0835;
const GROUP_RULE_GAP: i32 = 11 + GROUP_NAME_X;
const GROUP_RULE_END: i32 = 215;
const GROUP_RULE_DOWN: i32 = 5;
const SELECTED: Color32 = Color32::from_rgb(255, 228, 196);
const EDITING: Color32 = Color32::from_rgb(245, 245, 220);
const PRESSED: Color32 = Color32::from_rgb(245, 222, 179);
const ROW_WIDTH: i32 = 255;
const USE_BUTTON: ButtonArt = ButtonArt::new(0x0837, 0x0838, 0x0838);
const USE_X: i32 = 8;
const NAME_X: i32 = 22;
const VALUE_RIGHT: i32 = 250;
const LOCK_X: i32 = 251;
const SKILL_FONT: u8 = 9;
const SKILL_HUE: u16 = 0x0288;
/// The lock pictures of the standard gump: up, down, locked.
const LOCKS: [u16; 3] = [0x0984, 0x0986, 0x082C];

// The advanced gump.
const WIDTH: i32 = 500;
const HEIGHT: i32 = 360;
const BACK_ALPHA: f32 = 0.95;
const BORDER: Color32 = Color32::GRAY;
const RULE: Color32 = Color32::WHITE;
const ONE: i32 = 1;
const TABLE: (i32, i32, i32, i32) = (20, 60, WIDTH - 40, 250);
const RULE_WIDTH: i32 = 435;
const LOWER_RULE_Y: i32 = 310;
const HEADERS_Y: i32 = 25;
const HEADER_HEIGHT: i32 = 25;
/// The place and the width of each column button, in the order of
/// `SkillSort::COLUMNS`.
const COLUMN_PLACES: [(i32, i32); 4] = [(40, 180), (220, 80), (300, 80), (380, 80)];
const SORT_UP: u16 = 0x0985;
const SORT_DOWN: u16 = 0x0983;
const SORT_MARK_IN: i32 = 15;
const SORT_MARK_DOWN: i32 = 5;
const ENTRY_HEIGHT: i32 = 20;
const ENTRY_USE_Y: i32 = 4;
const ENTRY_NAME_X: i32 = 20;
const ENTRY_REAL_X: i32 = 200;
const ENTRY_VALUE_X: i32 = 280;
const ENTRY_CAP_X: i32 = 360;
const ENTRY_LOCK: (i32, i32) = (425, 4);
/// The lock pictures of the advanced gump: up, down, locked.
const ENTRY_LOCKS: [u16; 3] = [0x0983, 0x0985, 0x082C];
const ENTRY_FONT: u8 = 3;
const TABLE_HUE: u16 = 1153;
const HEADER_FONT: u8 = 1;
const HEADER_HUE: u16 = 0xFFFF;
const TOTALS_Y: i32 = 320;
const TOTAL_WORDS_X: i32 = 40;
const TOTAL_REAL_X: i32 = 220;
const TOTAL_VALUE_X: i32 = 300;
const WORDS_TOTAL: &str = "Total: ";
const TOTAL_FONT: u8 = 1;
const HALF: i32 = 2;

/// The lock picture of a skill, from a list of the three.
fn lock_picture(locks: &[u16; 3], lock: u8) -> u16 {
    locks[usize::from(lock).min(locks.len() - 1)]
}

/// The value a row of the standard gump shows: the real value or the cap
/// when a box at the foot asks, and else the value.
fn shown_value(skill: &WatchSkill, show_real: bool, show_caps: bool) -> String {
    points(if show_real {
        skill.base
    } else if show_caps {
        skill.cap
    } else {
        skill.value
    })
}

/// The skills gump: the look the General page picks.
#[derive(Default)]
pub struct Skills {
    standard: StandardSkills,
    advanced: AdvancedSkills,
}

impl GumpBody for Skills {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if cx.profile.general.standard_skills_gump {
            self.standard.draw(g, cx);
        } else {
            self.advanced.draw(g, cx);
        }
    }

    /// A picked group takes Delete.
    fn wants_keys(&self) -> bool {
        self.standard.selected.is_some()
    }
}

/// How the player has a group of the standard gump.
#[derive(Default)]
struct StandardSkills {
    minimized: bool,
    show_real: bool,
    show_caps: bool,
    /// The group whose name the player clicked once: Delete takes it away.
    selected: Option<usize>,
    /// The group whose name the player types.
    editing: Option<(usize, TextField)>,
    /// The skill the player drags, from the group it lies in.
    dragging: Option<u16>,
    /// Where each group was in the last frame, in window points.
    group_areas: Vec<Rect>,
}

impl StandardSkills {
    /// Keeps the groups in the profile after the player changed them.
    fn keep(cx: &mut GumpContext<'_>, groups: Vec<SkillGroupSet>) {
        cx.profile.skill_groups = groups;
        cx.profile_changed();
    }

    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if self.minimized {
            g.pic(0, 0, MINIMIZED, 0);
            if g.body_double_click() {
                self.minimized = false;
            }
            return;
        }
        g.pic(HEADER_AT.0, HEADER_AT.1, HEADER, 0);
        let scroll = g.expandable_scroll(0, DIFF_Y, SCROLL, SCROLL_HEIGHT);
        let top = g.gump_size(SCROLL).unwrap_or(Vec2::ZERO);
        let title = g.gump_size(TITLE).unwrap_or(Vec2::ZERO);
        g.pic(
            ((top.x - title.x) / HALF as f32) as i32,
            DIFF_Y + ((top.y - title.y) / HALF as f32) as i32,
            TITLE,
            0,
        );
        let height = DIFF_Y + scroll.y as i32;
        let line = g.pic(LINE_X, TOP_LINE_Y, LINE, 0);
        g.pic(LINE_X, height - BOTTOM_LINE_UP, LINE, 0);
        let comment = g.pic(COMMENT_X, height - COMMENT_UP, COMMENT, 0);
        let mut groups = shown_groups(&cx.profile.skill_groups, &cx.frame.skills);
        let area_y = AREA_Y + line.y as i32 - AREA_LINE_ROOM;
        let mut changed = false;
        g.scroll_area(
            "skills",
            AREA_X,
            area_y,
            scroll.x as i32 - AREA_NARROWER,
            height - AREA_SHORTER,
            |g| self.groups(g, cx, &mut groups, &mut changed),
        );
        if changed {
            Self::keep(cx, groups.clone());
        }
        self.follow_drag(g, cx, &mut groups);
        g.label(
            COMMENT_X + comment.x as i32 + SUM_GAP,
            height - COMMENT_UP + SUM_DOWN,
            &total(&cx.frame.skills, self.show_real),
            &TextLook::ascii(SUM_FONT, SUM_HUE),
        );
        let new_group_y = height - NEW_GROUP_UP;
        if g.button("new_group", NEW_GROUP_X, new_group_y, NEW_GROUP_BUTTON) {
            groups.push(SkillGroupSet {
                name: NEW_GROUP.to_string(),
                skills: Vec::new(),
                open: true,
            });
            Self::keep(cx, groups);
        }
        let button_width = g
            .gump_size(NEW_GROUP_BUTTON.normal)
            .map_or(0, |s| s.x as i32);
        let check_x = NEW_GROUP_X + button_width + CHECK_GAP;
        let check_look = TextLook::ascii(CHECK_FONT, CHECK_HUE);
        if g.checkbox(
            "show_real",
            check_x,
            new_group_y - REAL_UP,
            CHECK,
            &mut self.show_real,
            Some((WORDS_SHOW_REAL, &check_look)),
        ) && self.show_real
        {
            self.show_caps = false;
        }
        if g.checkbox(
            "show_caps",
            check_x,
            new_group_y + CAPS_DOWN,
            CHECK,
            &mut self.show_caps,
            Some((WORDS_SHOW_CAPS, &check_look)),
        ) && self.show_caps
        {
            self.show_real = false;
        }
        let (x, y, w, h) = RESET_AT;
        let reset_look = TextLook::ascii(RESET_FONT, RESET_HUE).aligned(TextAlign::Center);
        if g.nice_button("reset", x, y, w, h, WORDS_RESET, &reset_look, false) {
            cx.open(GumpId::one(RESET_GROUPS.id));
        }
        let (x, y, w, h) = MINIMIZE_BOX;
        if g.hit_box("minimize", x, y, w, h).clicked() {
            self.minimized = true;
        }
    }

    /// The groups in the scroll area. Gives the height they take.
    fn groups(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        groups: &mut Vec<SkillGroupSet>,
        changed: &mut bool,
    ) -> i32 {
        let mut y = GROUP_AT;
        let mut areas = Vec::with_capacity(groups.len());
        let delete = g.ui().input(|i| i.key_pressed(Key::Delete));
        for at in 0..groups.len() {
            let rows = if groups[at].open {
                groups[at].skills.len() as i32
            } else {
                0
            };
            let group_height = ROW + rows * ROW;
            areas.push(g.area(
                GROUP_AT,
                y,
                Vec2::new(ROW_WIDTH as f32, group_height as f32),
            ));
            *changed |= self.group_header(g, groups, at, y);
            if groups[at].open {
                let skills = groups[at].skills.clone();
                for (row, id) in skills.iter().enumerate() {
                    if let Some(skill) = cx.frame.skills.iter().find(|s| s.id == *id) {
                        self.skill_row(g, cx, skill, y + ROW + row as i32 * ROW);
                    }
                }
            }
            y += group_height;
        }
        if delete && self.editing.is_none() {
            if let Some(at) = self.selected.take() {
                if remove_group(groups, at) {
                    *changed = true;
                } else {
                    cx.open_with(
                        GumpId::one(well_known::QUESTION),
                        Box::new(MessageBox::told(WORDS_CANNOT_DELETE, CANNOT_DELETE_BOX)),
                    );
                }
            }
        }
        self.group_areas = areas;
        y + GROUP_AT
    }

    /// The fold button, the name and the rule of a group. True when the
    /// player changed the groups.
    fn group_header(
        &mut self,
        g: &mut Canvas<'_>,
        groups: &mut [SkillGroupSet],
        at: usize,
        y: i32,
    ) -> bool {
        let mut changed = false;
        let x = GROUP_AT;
        if !groups[at].skills.is_empty() {
            let picture = if groups[at].open {
                GROUP_OPEN
            } else {
                GROUP_CLOSED
            };
            if g.button(
                ("fold", at),
                x,
                y,
                ButtonArt::new(picture, picture, picture),
            ) {
                groups[at].open = !groups[at].open;
                changed = true;
            }
        }
        let look = TextLook::ascii(GROUP_FONT, GROUP_HUE);
        let name_x = x + GROUP_NAME_X;
        let name_y = y - GROUP_NAME_LIFT;
        match self.editing.as_mut().filter(|(editing, _)| *editing == at) {
            Some((_, field)) => {
                g.fill(x, y, GROUP_NAME_WIDTH, ROW, EDITING);
                let typed = g.text_box(
                    ("group_name", at),
                    name_x,
                    name_y,
                    GROUP_NAME_WIDTH,
                    ROW,
                    field,
                    &look,
                );
                let clicked_away = g.ui().input(|i| i.pointer.primary_clicked())
                    && !g.hovered(name_x, name_y, GROUP_NAME_WIDTH, ROW);
                if typed.submitted {
                    groups[at].name = group_name(field.text());
                    self.editing = None;
                    changed = true;
                } else if clicked_away {
                    self.editing = None;
                }
            }
            None => {
                if self.selected == Some(at) {
                    g.fill(name_x, y, GROUP_NAME_WIDTH, ROW, SELECTED);
                }
                let size = g.label(name_x, name_y, &groups[at].name, &look);
                let rule_x = x + size.x as i32 + GROUP_RULE_GAP;
                g.pic_width(
                    rule_x,
                    y + GROUP_RULE_DOWN,
                    GROUP_RULE,
                    0,
                    GROUP_RULE_END - rule_x + x,
                );
                let name = g.click_area(("group", at), name_x, y, GROUP_NAME_WIDTH, ROW);
                if name.clicked_elsewhere() && self.selected == Some(at) {
                    self.selected = None;
                }
                if name.clicked() {
                    if self.selected == Some(at) {
                        self.selected = None;
                        self.editing = Some((at, TextField::new(&groups[at].name)));
                        g.focus(("group_name", at));
                    } else {
                        self.selected = Some(at);
                    }
                }
            }
        }
        changed
    }

    fn skill_row(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        skill: &WatchSkill,
        y: i32,
    ) {
        let x = GROUP_AT;
        if self.dragging == Some(skill.id) {
            g.fill(x, y, ROW_WIDTH, ROW, PRESSED);
        }
        let row = g.drag_area(("row", skill.id), x, y, ROW_WIDTH, ROW);
        if row.drag_started() {
            self.dragging = Some(skill.id);
        }
        let look = TextLook::ascii(SKILL_FONT, SKILL_HUE);
        g.label(x + NAME_X, y, &skill.name, &look);
        let value = shown_value(skill, self.show_real, self.show_caps);
        let width = g.measure(&value, &look).x as i32;
        g.label(x + VALUE_RIGHT - width, y, &value, &look);
        if skill.usable && g.button(("use", skill.id), x + USE_X, y, USE_BUTTON) {
            cx.act(Act::UseSkill(skill.id));
        }
        let lock = lock_picture(&LOCKS, skill.lock);
        if g.button(
            ("lock", skill.id),
            x + LOCK_X,
            y,
            ButtonArt::new(lock, lock, lock),
        ) {
            cx.act(Act::Command(next_lock_command(skill)));
        }
    }

    /// While a skill is dragged: over another group it moves there; let go
    /// out of the gump, a skill that can be used becomes a skill button.
    fn follow_drag(
        &mut self,
        g: &Canvas<'_>,
        cx: &mut GumpContext<'_>,
        groups: &mut [SkillGroupSet],
    ) {
        let Some(skill) = self.dragging else {
            return;
        };
        let (mouse, down) = g
            .ui()
            .input(|i| (i.pointer.interact_pos(), i.pointer.primary_down()));
        let Some(mouse) = mouse else {
            return;
        };
        let over = self
            .group_areas
            .iter()
            .position(|area| area.contains(mouse));
        if down {
            let home = groups
                .iter()
                .position(|group| group.skills.contains(&skill));
            if let Some(to) = over.filter(|to| Some(*to) != home) {
                move_skill(groups, skill, to);
                Self::keep(cx, groups.to_vec());
            }
            return;
        }
        self.dragging = None;
        let usable = cx.frame.skills.iter().any(|s| s.id == skill && s.usable);
        if usable && !g.pointer_on_gump() {
            cx.open_at(
                GumpId::of(SKILL_BUTTON.id, u32::from(skill)),
                button_place(mouse),
            );
        }
    }
}

/// How the advanced gump sorts.
struct AdvancedSkills {
    sort: SkillSort,
    descending: bool,
    /// The skill the player drags out.
    dragging: Option<u16>,
}

impl Default for AdvancedSkills {
    fn default() -> Self {
        Self {
            sort: SkillSort::Name,
            descending: false,
            dragging: None,
        }
    }
}

impl AdvancedSkills {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        g.shade(
            ONE,
            ONE,
            WIDTH - ONE * HALF,
            HEIGHT - ONE * HALF,
            0,
            BACK_ALPHA,
        );
        g.outline(0, 0, WIDTH, HEIGHT, BORDER);
        let header = TextLook::unicode(HEADER_FONT, HEADER_HUE).aligned(TextAlign::Center);
        for (sort, (x, w)) in SkillSort::COLUMNS.into_iter().zip(COLUMN_PLACES) {
            let words = sort.label();
            let selected = self.sort == sort;
            if g.nice_button(
                ("sort", words),
                x,
                HEADERS_Y,
                w,
                HEADER_HEIGHT,
                words,
                &header,
                selected,
            ) {
                if selected {
                    self.descending = !self.descending;
                }
                self.sort = sort;
            }
            if selected {
                let mark = if self.descending { SORT_UP } else { SORT_DOWN };
                g.pic(x + w - SORT_MARK_IN, HEADERS_Y + SORT_MARK_DOWN, mark, 0);
            }
        }
        let (x, y, w, h) = TABLE;
        g.fill(x, y, RULE_WIDTH, ONE, RULE);
        g.fill(x, LOWER_RULE_Y, RULE_WIDTH, ONE, RULE);
        let skills: Vec<WatchSkill> = sorted(&cx.frame.skills, self.sort, self.descending)
            .into_iter()
            .cloned()
            .collect();
        g.scroll_area("table", x, y, w, h, |g| {
            for (row, skill) in skills.iter().enumerate() {
                self.entry(g, cx, skill, row as i32 * ENTRY_HEIGHT);
            }
            skills.len() as i32 * ENTRY_HEIGHT
        });
        let look = TextLook::unicode(TOTAL_FONT, TABLE_HUE);
        g.label(TOTAL_WORDS_X, TOTALS_Y, WORDS_TOTAL, &look);
        g.label(
            TOTAL_REAL_X,
            TOTALS_Y,
            &total(&cx.frame.skills, true),
            &look,
        );
        g.label(
            TOTAL_VALUE_X,
            TOTALS_Y,
            &total(&cx.frame.skills, false),
            &look,
        );
        self.follow_drag(g, cx);
    }

    fn entry(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, skill: &WatchSkill, y: i32) {
        let row = g.drag_area(("entry", skill.id), 0, y, RULE_WIDTH, ENTRY_HEIGHT);
        if row.drag_started() && skill.usable {
            self.dragging = Some(skill.id);
        }
        let look = TextLook::unicode(ENTRY_FONT, TABLE_HUE);
        if skill.usable && g.button(("use", skill.id), 0, y + ENTRY_USE_Y, USE_BUTTON) {
            cx.act(Act::UseSkill(skill.id));
        }
        g.label(ENTRY_NAME_X, y, &skill.name, &look);
        g.label(ENTRY_REAL_X, y, &short_points(skill.base), &look);
        g.label(ENTRY_VALUE_X, y, &short_points(skill.value), &look);
        g.label(ENTRY_CAP_X, y, &short_points(skill.cap), &look);
        let lock = lock_picture(&ENTRY_LOCKS, skill.lock);
        if g.pic_button(("lock", skill.id), ENTRY_LOCK.0, y + ENTRY_LOCK.1, lock, 0)
            .clicked()
        {
            cx.act(Act::Command(next_lock_command(skill)));
        }
    }

    /// A skill dragged out of the table becomes a skill button that follows
    /// the mouse until the button comes up.
    fn follow_drag(&mut self, g: &Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(skill) = self.dragging else {
            return;
        };
        let (mouse, down) = g
            .ui()
            .input(|i| (i.pointer.interact_pos(), i.pointer.primary_down()));
        if let Some(mouse) = mouse.filter(|_| !g.pointer_on_gump()) {
            cx.open_at(
                GumpId::of(SKILL_BUTTON.id, u32::from(skill)),
                button_place(mouse),
            );
        }
        if !down {
            self.dragging = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_and_locks_show_as_the_boxes_ask() {
        let skill = WatchSkill {
            value: 505,
            base: 500,
            cap: 1000,
            lock: 2,
            ..WatchSkill::default()
        };
        assert_eq!(shown_value(&skill, false, false), "50.5");
        assert_eq!(shown_value(&skill, true, false), "50.0");
        assert_eq!(shown_value(&skill, false, true), "100.0");
        assert_eq!(lock_picture(&LOCKS, skill.lock), 0x082C);
        assert_eq!(lock_picture(&LOCKS, 9), 0x082C);
    }

    #[test]
    fn both_looks_draw_the_skills() {
        use crate::view::WatchFrame;
        use crate::window::classic::manager::GumpManager;
        use crate::window::classic::testing::draw_frames;
        use crate::window::settings::Profile;
        let skill = |id: u16, name: &str, usable| WatchSkill {
            id,
            name: name.into(),
            usable,
            value: 505,
            base: 500,
            cap: 1000,
            group: "Combat".into(),
            group_index: 1,
            ..WatchSkill::default()
        };
        let frame = WatchFrame {
            skills: vec![skill(1, "Anatomy", true), skill(5, "Parrying", false)],
            ..WatchFrame::default()
        };
        let id = GumpId::one(SKILLS.id);
        let mut manager = GumpManager::default();
        let mut profile = Profile {
            skill_groups: vec![SkillGroupSet {
                name: "Combat".into(),
                skills: vec![1, 5],
                open: true,
            }],
            ..Profile::default()
        };
        manager.open(id, &mut profile);
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        let standard = manager.drawn_of(SKILLS.id)[0].1;
        profile.general.standard_skills_gump = false;
        draw_frames(&mut manager, &mut profile, &frame);
        let advanced = manager.drawn_of(SKILLS.id)[0].1;
        assert!(manager.is_open(&id));
        assert!(advanced.width() > standard.width());
    }
}
