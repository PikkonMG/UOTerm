//! The skills tab of the sheet, in the two ways the General option
//! "Standard skills gump" picks between, as the classic skills gumps are:
//! the skills in the player's groups, which he makes, names, takes away,
//! folds and drags skills between, or one table of every skill that sorts
//! by a column. Each skill shows its real value, its value with the items
//! and the spells on, and its cap, with its lock and, for a skill the
//! player starts, its Use and Pin buttons; the sums show at the top. A
//! skill the player starts drags onto the hotbar too.

use super::super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::deck_ui::{
    lock_mark, row_buttons, Offer, RowPress, Slot, LOCK_SIDE, PIN_WIDTH, ROW, TAB_GAP, USE_WIDTH,
    WORDS_USE,
};
use super::super::model::skills::{
    group_name, move_skill, next_lock_command, points, remove_group, shown_groups, sorted, total,
    SkillSort, NEW_GROUP, WORDS_CANNOT_DELETE,
};
use super::super::settings::{Profile, SkillGroupSet};
use super::super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchFrame, WatchSkill};
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};

/// The width of each value column.
const VALUE_WIDTH: f32 = 48.0;
/// The room of the Use and Pin buttons at the right of a row.
const BUTTONS_WIDTH: f32 = PIN_WIDTH + USE_WIDTH + TAB_GAP * 2.0;
const BAR_BUTTON_WIDTH: f32 = 92.0;
/// The lock arrows of the sort mark: up for the least first.
const SORT_UP: u8 = 0;
const SORT_DOWN: u8 = 1;
const WORDS_NEW_GROUP: &str = "New group";
const WORDS_RESET: &str = "Reset groups";
const WORDS_RESET_ASK: &str = "Back to the first groups?";
const WORDS_YES: &str = "Yes";
const WORDS_NO: &str = "No";
const WORDS_DELETE: &str = "x";
const WORDS_OPEN: &str = "-";
const WORDS_FOLDED: &str = "+";
const HINT_GROUP: &str = "Click: select, again: rename.  Delete: take it away.";
const HINT_SKILL: &str = "Drag: to another group, or a skill to use onto the hotbar.";

/// One row of the grouped list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Group(usize),
    Skill(u16),
}

/// The rows of the grouped list: each group, and the skills of an open one.
fn entries(groups: &[SkillGroupSet], skills: &[WatchSkill]) -> Vec<Entry> {
    let mut rows = Vec::new();
    for (at, group) in groups.iter().enumerate() {
        rows.push(Entry::Group(at));
        if group.open {
            rows.extend(
                group
                    .skills
                    .iter()
                    .filter(|id| skills.iter().any(|skill| skill.id == **id))
                    .map(|id| Entry::Skill(*id)),
            );
        }
    }
    rows
}

/// The right edge of a value column of a row, from the first value column.
fn value_right(row: Rect, column: usize) -> f32 {
    let columns = SkillSort::COLUMNS.len() - 1;
    row.right() - BUTTONS_WIDTH - VALUE_WIDTH * (columns - 1 - column) as f32
}

/// The delete mark at the right end of a group row.
fn delete_area(row: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(row.right() - USE_WIDTH, row.top()),
        Vec2::new(USE_WIDTH, row.height() - TAB_GAP / 2.0),
    )
}

/// The sort after a click on a column: the same column turns round, a new
/// one sorts from the least.
fn next_sort(now: (SkillSort, bool), clicked: SkillSort) -> (SkillSort, bool) {
    if now.0 == clicked {
        (clicked, !now.1)
    } else {
        (clicked, false)
    }
}

pub struct SkillsTab {
    first_row: usize,
    sort: SkillSort,
    descending: bool,
    /// The group whose name the player clicked once.
    selected: Option<usize>,
    /// The group whose name the player types, the words, and whether the
    /// field took the keys yet.
    renaming: Option<(usize, String, bool)>,
    /// The skill the player drags to another group.
    dragging: Option<u16>,
    asking_reset: bool,
}

impl Default for SkillsTab {
    fn default() -> Self {
        Self {
            first_row: 0,
            sort: SkillSort::Name,
            descending: false,
            selected: None,
            renaming: None,
            dragging: None,
            asking_reset: false,
        }
    }
}

impl SkillsTab {
    /// Draws the tab in `body`. Gives what the player pinned or dragged
    /// toward the hotbar.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Offer> {
        let grouped = profile.general.standard_skills_gump;
        let bar = Rect::from_min_size(body.min, Vec2::new(body.width(), ROW));
        self.bar(ui, bar, frame, tools, profile, grouped);
        let header = bar.translate(Vec2::new(0.0, ROW));
        self.header(ui, header, grouped);
        let list = Rect::from_min_max(Pos2::new(body.left(), header.bottom()), body.max);
        if grouped {
            self.groups(ui, list, frame, tools, profile)
        } else {
            self.table(ui, list, frame, tools)
        }
    }

    /// The sums, and the buttons of the groups.
    fn bar(
        &mut self,
        ui: &egui::Ui,
        bar: Rect,
        frame: &WatchFrame,
        tools: &Tools<'_>,
        profile: &mut Profile,
        grouped: bool,
    ) {
        if grouped && self.asking_reset {
            self.reset_ask(ui, bar, tools, profile);
            return;
        }
        let sums = format!(
            "{} {}   {} {}",
            SkillSort::Real.label(),
            total(&frame.skills, true),
            SkillSort::Value.label(),
            total(&frame.skills, false)
        );
        ui.painter().text(
            bar.left_center(),
            Align2::LEFT_CENTER,
            sums,
            number_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        if !grouped {
            return;
        }
        let button = |from_right: f32| {
            Rect::from_min_size(
                Pos2::new(bar.right() - from_right, bar.top()),
                Vec2::new(BAR_BUTTON_WIDTH, ROW - TAB_GAP),
            )
        };
        let second = BAR_BUTTON_WIDTH * 2.0 + TAB_GAP;
        if theme::segment_keyed(
            ui,
            button(second),
            Id::new("skills-new-group"),
            WORDS_NEW_GROUP,
            theme::GOAL,
        ) {
            let mut groups = shown_groups(&profile.skill_groups, &frame.skills);
            groups.push(SkillGroupSet {
                name: NEW_GROUP.to_string(),
                skills: Vec::new(),
                open: true,
            });
            profile.skill_groups = groups;
            tools.keep_profile(profile);
        }
        if theme::segment_keyed(
            ui,
            button(BAR_BUTTON_WIDTH),
            Id::new("skills-reset"),
            WORDS_RESET,
            theme::TEXT,
        ) {
            self.asking_reset = true;
        }
    }

    /// The question before the groups go back to those of the client
    /// files.
    fn reset_ask(&mut self, ui: &egui::Ui, bar: Rect, tools: &Tools<'_>, profile: &mut Profile) {
        ui.painter().text(
            bar.left_center(),
            Align2::LEFT_CENTER,
            WORDS_RESET_ASK,
            text_font(theme::SIZE_SMALL),
            theme::WAITING,
        );
        let yes = Rect::from_min_size(
            Pos2::new(bar.right() - BAR_BUTTON_WIDTH * 2.0 - TAB_GAP, bar.top()),
            Vec2::new(BAR_BUTTON_WIDTH, ROW - TAB_GAP),
        );
        let no = yes.translate(Vec2::new(BAR_BUTTON_WIDTH + TAB_GAP, 0.0));
        if theme::segment_keyed(
            ui,
            yes,
            Id::new("skills-reset-yes"),
            WORDS_YES,
            theme::ALARM,
        ) {
            profile.skill_groups.clear();
            tools.keep_profile(profile);
            self.asking_reset = false;
        } else if theme::segment_keyed(ui, no, Id::new("skills-reset-no"), WORDS_NO, theme::TEXT) {
            self.asking_reset = false;
        }
    }

    /// The heads of the columns. In the table, a click sorts by one.
    fn header(&mut self, ui: &egui::Ui, header: Rect, grouped: bool) {
        let sorts = !grouped;
        for (column, sort) in SkillSort::COLUMNS.into_iter().enumerate() {
            let (area, align) = match column {
                0 => (
                    Rect::from_min_max(
                        Pos2::new(header.left() + LOCK_SIDE + theme::ROW_GAP, header.top()),
                        Pos2::new(value_right(header, 0) - VALUE_WIDTH, header.bottom()),
                    ),
                    Align2::LEFT_CENTER,
                ),
                _ => (
                    Rect::from_min_max(
                        Pos2::new(value_right(header, column - 1) - VALUE_WIDTH, header.top()),
                        Pos2::new(value_right(header, column - 1), header.bottom()),
                    ),
                    Align2::RIGHT_CENTER,
                ),
            };
            let response = ui.interact(area, Id::new(("skills-sort", column)), Sense::click());
            let chosen = sorts && self.sort == sort;
            let color = if chosen || (sorts && response.hovered()) {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            let at = if align == Align2::LEFT_CENTER {
                area.left_center()
            } else {
                area.right_center()
            };
            let words = ui.painter().text(
                at,
                align,
                sort.label(),
                title_font(theme::SIZE_SMALL),
                color,
            );
            if chosen {
                let mark = Rect::from_center_size(
                    Pos2::new(
                        if align == Align2::LEFT_CENTER {
                            words.right() + LOCK_SIDE / 2.0
                        } else {
                            words.left() - LOCK_SIDE / 2.0
                        },
                        area.center().y,
                    ),
                    Vec2::splat(LOCK_SIDE),
                );
                let way = if self.descending { SORT_DOWN } else { SORT_UP };
                lock_mark(ui.painter(), mark, way, color);
            }
            if sorts && response.clicked() {
                (self.sort, self.descending) = next_sort((self.sort, self.descending), sort);
            }
        }
    }

    /// Every skill, sorted by the chosen column.
    fn table(
        &mut self,
        ui: &egui::Ui,
        list: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Option<Offer> {
        let skills = sorted(&frame.skills, self.sort, self.descending);
        let rows = ((list.height() / ROW).floor() as usize).max(1);
        let last_first = skills.len().saturating_sub(rows);
        self.first_row = scrolled(ui, list, self.first_row.min(last_first), last_first);
        let mut offer = None;
        for (at, skill) in skills
            .into_iter()
            .skip(self.first_row)
            .take(rows)
            .enumerate()
        {
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, at as f32 * ROW),
                Vec2::new(list.width(), ROW),
            );
            offer = offer.or(self.skill_row(ui, row, skill, frame, tools));
        }
        offer
    }

    /// The groups, and the skills of each open one.
    fn groups(
        &mut self,
        ui: &mut egui::Ui,
        list: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Offer> {
        let mut groups = shown_groups(&profile.skill_groups, &frame.skills);
        let rows = entries(&groups, &frame.skills);
        let shown_rows = ((list.height() / ROW).floor() as usize).max(1);
        let last_first = rows.len().saturating_sub(shown_rows);
        self.first_row = scrolled(ui, list, self.first_row.min(last_first), last_first);
        let mut changed = false;
        let mut offer = None;
        let mut deleted = None;
        let mut areas: Vec<(usize, Rect)> = Vec::new();
        let mut group_at = 0;
        for entry in rows.iter().take(self.first_row) {
            if let Entry::Group(at) = entry {
                group_at = *at;
            }
        }
        let shown = rows.iter().skip(self.first_row).take(shown_rows);
        for (place, entry) in shown.enumerate() {
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, place as f32 * ROW),
                Vec2::new(list.width(), ROW),
            );
            match entry {
                Entry::Group(at) => {
                    group_at = *at;
                    changed |= self.group_row(ui, row, &mut groups, *at, frame);
                    if self.delete_pressed(ui, row, *at, frame) {
                        deleted = Some(*at);
                    }
                }
                Entry::Skill(id) => {
                    if let Some(skill) = frame.skills.iter().find(|skill| skill.id == *id) {
                        offer = offer.or(self.skill_row(ui, row, skill, frame, tools));
                    }
                }
            }
            match areas.last_mut() {
                Some((at, area)) if *at == group_at => *area = area.union(row),
                _ => areas.push((group_at, row)),
            }
        }
        if let Some(at) = deleted.or_else(|| self.delete_key(ui)) {
            self.selected = None;
            if remove_group(&mut groups, at) {
                changed = true;
            } else {
                tools.hand.report(WORDS_CANNOT_DELETE);
            }
        }
        changed |= self.follow_drag(ui, &mut groups, &areas);
        if changed {
            profile.skill_groups = groups;
            tools.keep_profile(profile);
        }
        offer
    }

    /// The delete mark of a group but the first. True when it was pressed.
    fn delete_pressed(&self, ui: &egui::Ui, row: Rect, at: usize, frame: &WatchFrame) -> bool {
        frame.human_control
            && at > 0
            && self.renaming.is_none()
            && theme::segment_keyed(
                ui,
                delete_area(row),
                Id::new(("skill-group-delete", at)),
                WORDS_DELETE,
                theme::ALARM,
            )
    }

    /// The fold mark and the name of a group. True when the player changed
    /// the groups.
    fn group_row(
        &mut self,
        ui: &mut egui::Ui,
        row: Rect,
        groups: &mut [SkillGroupSet],
        at: usize,
        frame: &WatchFrame,
    ) -> bool {
        let live = frame.human_control;
        let mut changed = false;
        let fold = Rect::from_min_size(row.min, Vec2::new(LOCK_SIDE, row.height()));
        let fold_words = if groups[at].open {
            WORDS_OPEN
        } else {
            WORDS_FOLDED
        };
        if theme::segment_keyed(
            ui,
            fold,
            Id::new(("skill-fold", at)),
            fold_words,
            theme::TEXT,
        ) {
            groups[at].open = !groups[at].open;
            changed = true;
        }
        let name = Rect::from_min_max(
            Pos2::new(fold.right() + theme::ROW_GAP, row.top()),
            Pos2::new(delete_area(row).left() - theme::ROW_GAP, row.bottom()),
        );
        if let Some((_, typed, focused)) = self
            .renaming
            .as_mut()
            .filter(|(editing, ..)| *editing == at)
        {
            ui.painter()
                .rect_filled(name, CornerRadius::same(CELL_RADIUS), theme::TRACK);
            let field = ui.put(
                name,
                egui::TextEdit::singleline(typed)
                    .id(Id::new(("skill-group-name", at)))
                    .frame(false)
                    .font(title_font(theme::SIZE_SMALL))
                    .text_color(theme::TEXT),
            );
            if !*focused {
                field.request_focus();
                *focused = true;
            } else if field.lost_focus() {
                if ui.input(|i| i.key_pressed(Key::Enter)) {
                    groups[at].name = group_name(typed);
                    changed = true;
                }
                self.renaming = None;
            }
            return changed;
        }
        let response = ui.interact(name, Id::new(("skill-group", at)), Sense::click());
        if self.selected == Some(at) {
            ui.painter()
                .rect_filled(name, CornerRadius::same(CELL_RADIUS), theme::BUTTON);
        }
        ui.painter().text(
            name.left_center(),
            Align2::LEFT_CENTER,
            format!("{}  ({})", groups[at].name, groups[at].skills.len()),
            title_font(theme::SIZE_SMALL),
            theme::GOAL,
        );
        if response.hovered() && live {
            super::super::tips::label(ui, &groups[at].name, HINT_GROUP);
        }
        if live && (response.double_clicked() || (response.clicked() && self.selected == Some(at)))
        {
            self.selected = None;
            self.renaming = Some((at, groups[at].name.clone(), false));
        } else if live && response.clicked() {
            self.selected = Some(at);
        } else if response.clicked_elsewhere() && self.selected == Some(at) {
            self.selected = None;
        }
        changed
    }

    /// The selected group, when Delete was pressed, as in the classic gump.
    fn delete_key(&self, ui: &egui::Ui) -> Option<usize> {
        let pressed = self.renaming.is_none()
            && !ui.ctx().wants_keyboard_input()
            && ui.input(|i| i.key_pressed(Key::Delete));
        self.selected.filter(|_| pressed)
    }

    /// While a skill is dragged, the group under the mouse lights; let go
    /// over it, the skill moves there.
    fn follow_drag(
        &mut self,
        ui: &egui::Ui,
        groups: &mut [SkillGroupSet],
        areas: &[(usize, Rect)],
    ) -> bool {
        let Some(skill) = self.dragging else {
            return false;
        };
        let (mouse, down) = ui.input(|i| (i.pointer.hover_pos(), i.pointer.primary_down()));
        let over = mouse.and_then(|mouse| areas.iter().find(|(_, area)| area.contains(mouse)));
        if down {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if let Some((_, area)) = over {
                ui.painter().rect_stroke(
                    *area,
                    CornerRadius::same(CELL_RADIUS),
                    egui::Stroke::new(1.0, theme::GOAL),
                    egui::StrokeKind::Inside,
                );
            }
            return false;
        }
        self.dragging = None;
        let home = groups
            .iter()
            .position(|group| group.skills.contains(&skill));
        match over {
            Some((to, _)) if Some(*to) != home => {
                move_skill(groups, skill, *to);
                true
            }
            _ => false,
        }
    }

    /// One skill: its lock, its name, its three values and, for a skill
    /// the player starts, Use and Pin.
    fn skill_row(
        &mut self,
        ui: &egui::Ui,
        row: Rect,
        skill: &WatchSkill,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Option<Offer> {
        let live = frame.human_control;
        let slot = || Slot::Skill {
            id: skill.id,
            name: skill.name.clone(),
        };
        let mut offer = None;
        let dragged = ui.interact(row, Id::new(("skill-row", skill.id)), Sense::drag());
        if live && dragged.drag_started() {
            self.dragging = Some(skill.id);
            if skill.usable {
                offer = Some(Offer::Drag(slot()));
            }
        }
        if dragged.hovered() && live && !dragged.dragged() {
            super::super::tips::label(ui, &skill.name, HINT_SKILL);
        }
        let lock = Rect::from_center_size(
            Pos2::new(row.left() + LOCK_SIDE / 2.0, row.center().y),
            Vec2::splat(LOCK_SIDE),
        );
        let lock_response = ui.interact(lock, Id::new(("skill-lock", skill.id)), Sense::click());
        let lock_color = if live && lock_response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        lock_mark(ui.painter(), lock, skill.lock, lock_color);
        if live && lock_response.clicked() {
            tools.hand.act(Act::Command(next_lock_command(skill)));
        }
        ui.painter().text(
            Pos2::new(lock.right() + theme::ROW_GAP, row.center().y),
            Align2::LEFT_CENTER,
            &skill.name,
            text_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        for (column, sort) in SkillSort::COLUMNS.into_iter().skip(1).enumerate() {
            if let Some(tenths) = sort.tenths(skill) {
                ui.painter().text(
                    Pos2::new(value_right(row, column), row.center().y),
                    Align2::RIGHT_CENTER,
                    points(tenths),
                    number_font(theme::SIZE_SMALL),
                    theme::TEXT_DIM,
                );
            }
        }
        if skill.usable && live {
            match row_buttons(ui, row, ("skill", skill.id), WORDS_USE) {
                Some(RowPress::Pin) => offer = Some(Offer::Pin(slot())),
                Some(RowPress::Go) => tools.hand.act(Act::UseSkill(skill.id)),
                None => {}
            }
        }
        offer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: u16) -> WatchSkill {
        WatchSkill {
            id,
            ..WatchSkill::default()
        }
    }

    #[test]
    fn an_open_group_lists_its_known_skills_and_a_folded_one_none() {
        let groups = vec![
            SkillGroupSet {
                name: "Open".into(),
                skills: vec![1, 2, 99],
                open: true,
            },
            SkillGroupSet {
                name: "Folded".into(),
                skills: vec![3],
                open: false,
            },
        ];
        let skills = [skill(1), skill(2), skill(3)];
        assert_eq!(
            entries(&groups, &skills),
            vec![
                Entry::Group(0),
                Entry::Skill(1),
                Entry::Skill(2),
                Entry::Group(1)
            ]
        );
    }

    #[test]
    fn a_column_sorts_from_the_least_and_turns_round_on_a_second_click() {
        assert_eq!(
            next_sort((SkillSort::Name, false), SkillSort::Cap),
            (SkillSort::Cap, false)
        );
        assert_eq!(
            next_sort((SkillSort::Cap, false), SkillSort::Cap),
            (SkillSort::Cap, true)
        );
        let row = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, ROW));
        assert_eq!(value_right(row, 2), 400.0 - BUTTONS_WIDTH);
        assert_eq!(
            value_right(row, 0),
            400.0 - BUTTONS_WIDTH - VALUE_WIDTH * 2.0
        );
    }

    #[test]
    fn a_click_on_a_column_of_the_table_sorts_by_it_and_the_groups_draw() {
        use super::super::testing::{click, draw_frames};
        let frame = WatchFrame {
            human_control: true,
            skills: vec![skill(1), skill(2)],
            ..WatchFrame::default()
        };
        let body = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(400.0, 300.0));
        let header =
            Rect::from_min_size(body.min + Vec2::new(0.0, ROW), Vec2::new(body.width(), ROW));
        let cap = Pos2::new(
            value_right(header, 2) - VALUE_WIDTH / 2.0,
            header.center().y,
        );
        let mut tab = SkillsTab::default();
        let mut profile = Profile::default();
        profile.general.standard_skills_gump = false;
        draw_frames(&mut profile, &click(cap), |ui, _, tools, profile| {
            tab.draw(ui, body, &frame, tools, profile);
        });
        assert_eq!((tab.sort, tab.descending), (SkillSort::Cap, false));
        profile.general.standard_skills_gump = true;
        draw_frames(&mut profile, &[Vec::new()], |ui, _, tools, profile| {
            tab.draw(ui, body, &frame, tools, profile);
        });
        assert!(profile.skill_groups.is_empty(), "drawing changes no group");
    }
}
