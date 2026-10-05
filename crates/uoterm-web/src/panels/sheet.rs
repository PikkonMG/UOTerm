//! The sheet of the character: what he wears and his status, his skills,
//! his spellbooks and his party, each a tab. Its clicks work only while
//! the human has control, as in the Rust window; what each row shows and
//! does is the rules of `model` and `ui::lists`.

use super::bars::{AtThing, BarLine};
use super::{Colored, DropZone, FrameSpec, Framed, TipKey, PANEL_SHEET};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_assist::spells::School;
use uoterm_view::act::{Act, Answer, Ask, Asker};
use uoterm_view::actions::windows::{CharacterView, Tab};
use uoterm_view::art::ArtRequest;
use uoterm_view::frame::{WatchFrame, WatchPackItem};
use uoterm_view::geom::Point;
use uoterm_view::input::Mods;
use uoterm_view::model::clicks::ClickDelay;
use uoterm_view::model::durability::{is_worn_layer, worn_wear};
use uoterm_view::model::key_macros;
use uoterm_view::model::party::{
    invite_words, inviter_name, leads, leave_words, member_click_act, party_say, ACCEPT_COMMAND,
    DECLINE_COMMAND, INVITE_COMMAND,
};
use uoterm_view::model::places;
use uoterm_view::model::skills::{
    group_name, move_skill, next_lock_command, points, remove_group, shown_groups, sorted, total,
    SkillSort, NEW_GROUP, WORDS_CANNOT_DELETE,
};
use uoterm_view::model::spell_data::{
    book_info, book_name, icon_hue, needs_words, power_words, reagent_lines, spell_macro, upkeep,
    BOOKS,
};
use uoterm_view::model::status::{stats, StatLocks, STAT_NAMES};
use uoterm_view::scene::doll_figure;
use uoterm_view::settings::SkillGroupSet;
use uoterm_view::ui::abilities::SHEET_PANEL_BUTTONS;
use uoterm_view::ui::deck::{layer_words, wear_choices, Slot, WearChoice, WORDS_BAR_FULL};
use uoterm_view::ui::lists::{
    next_sort, party_entries, skill_entries, spell_book_words, status_lines, wear_color,
    PartyEntry, SkillEntry, SpellTab, StatusLine,
};
use uoterm_view::ui::question::{WORDS_NO, WORDS_YES};
use uoterm_view::ui::ring::Subject as RingSubject;
use uoterm_view::ui::sheet::{
    assigned_words, assigns_spell, sheet_first_place, sheet_least, tell_hint, CHARACTER_VIEWS,
    HINT_ASSIGN, HINT_GROUP, HINT_MEMBER, HINT_SKILL, HINT_SPELL, HINT_STAT_LOCK, HINT_WEAR,
    HINT_WORN, NOTE_SECONDS, SHEET_ID, SHEET_TABS, WORDS_ACCEPT, WORDS_ADD, WORDS_ASSIGN,
    WORDS_CAST, WORDS_DECLINE, WORDS_DELETE_GROUP, WORDS_EMPTY, WORDS_EMPTY_BOOK, WORDS_GOLD,
    WORDS_GROUP_FOLDED, WORDS_GROUP_OPEN, WORDS_INVITE, WORDS_KICK, WORDS_LOOKING, WORDS_LOOT_OFF,
    WORDS_LOOT_ON, WORDS_NEAR, WORDS_NEW_GROUP, WORDS_NOTHING_WORN, WORDS_NO_BOOK,
    WORDS_PICK_SPELL, WORDS_PIN, WORDS_REAGENTS, WORDS_RESET, WORDS_RESET_ASK, WORDS_SAY,
    WORDS_SHEET, WORDS_TAKE_OFF, WORDS_TELL, WORDS_TITHING_COST, WORDS_TITHING_HAVE, WORDS_USE,
    WORDS_WEAR, WORDS_WEIGHT, WORDS_WORN,
};
use uoterm_view::ui::theme::{css_color, ALARM, WAITING};

/// What the sheet keeps between frames.
#[derive(Default)]
pub(crate) struct SheetState {
    pub open: bool,
    pub tab: Tab,
    pub view: CharacterView,
    stat_locks: StatLocks,
    /// A click on a worn item asks its name once no double click follows.
    worn_clicks: ClickDelay,
    /// The things Jev was asked about, in the order it was asked.
    wear_asked: Vec<WearChoice>,
    wear_note: Option<(String, bool, f64)>,
    sort: Option<(SkillSort, bool)>,
    asking_reset: bool,
    /// The book the spells tab shows.
    book: SpellTab,
    /// The spell the player clicked.
    spell: Option<u16>,
    /// The member the words go to; None for the whole party.
    tell_to: Option<u32>,
}

impl SheetState {
    fn sort(&self) -> (SkillSort, bool) {
        self.sort.unwrap_or((SkillSort::Name, false))
    }
}

/// A button of a row of choices, and whether it is the chosen one.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Choice {
    pub words: String,
    pub chosen: bool,
}

/// The sheet.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SheetData {
    pub tabs: Vec<Choice>,
    /// The human has control: the rows act.
    pub live: bool,
    pub character: Option<CharacterData>,
    pub skills: Option<SkillsData>,
    pub spells: Option<SpellsData>,
    pub party: Option<PartyData>,
}

/// The character tab: the worn view or the status view.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CharacterData {
    pub views: Vec<Choice>,
    /// The buttons that open and close the ability panels, right to left;
    /// an open panel is chosen.
    pub panels: Vec<Choice>,
    pub worn: Option<WornData>,
    pub status: Option<Vec<StatusRow>>,
}

/// The worn view: the figure, the stats, the weight and the gold, the
/// worn list and the field of what to wear.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WornData {
    pub doll: Option<String>,
    pub name: String,
    pub stats: Vec<StatRow>,
    pub facts: Vec<Fact>,
    pub worn_words: String,
    pub rows: Vec<WornRow>,
    pub nothing: Option<&'static str>,
    /// An item dropped on the view is put on.
    pub zone: DropZone,
    pub wear: Option<WearField>,
}

/// One stat, its value and its lock.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StatRow {
    pub stat: usize,
    pub name: &'static str,
    pub value: String,
    /// 0 up, 1 down, 2 locked.
    pub lock: u8,
    pub hover: TipKey,
}

/// A fact: its words and its value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fact {
    pub words: &'static str,
    pub value: String,
}

/// One worn item.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WornRow {
    pub serial: u32,
    pub layer: u8,
    pub picture: Option<String>,
    pub words: String,
    /// Its durability, when the Interface page asks for the bars.
    pub wear: Option<BarLine>,
    pub take_off: &'static str,
    pub hover: TipKey,
}

/// The field that takes what to wear or take off in plain words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WearField {
    pub hint: &'static str,
    pub button: &'static str,
    /// Jev may be asked: there is something to wear or take off.
    pub on: bool,
    pub note: Option<Colored>,
}

/// One line of the status view.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatusRow {
    Title { words: &'static str },
    Stat(StatRow),
    Fact(Fact),
}

/// The skills tab.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SkillsData {
    pub sums: String,
    /// The player's groups, or one table that sorts by a column.
    pub grouped: bool,
    pub columns: Vec<SkillColumn>,
    pub new_group: Option<&'static str>,
    pub reset: Option<&'static str>,
    pub reset_ask: Option<ResetAsk>,
    pub rows: Vec<SkillRow>,
}

/// The head of a column.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SkillColumn {
    pub words: &'static str,
    pub chosen: bool,
    pub descending: bool,
}

/// The question before the groups go back to those of the client files.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResetAsk {
    pub words: &'static str,
    pub yes: &'static str,
    pub no: &'static str,
}

/// One row of the skills: a group, or a skill.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillRow {
    Group {
        at: usize,
        name: String,
        words: String,
        fold: &'static str,
        delete: Option<&'static str>,
        hover: TipKey,
    },
    Skill {
        id: u16,
        name: String,
        values: Vec<String>,
        lock: u8,
        /// Use and Pin, for a skill the player starts.
        buttons: Option<[&'static str; 2]>,
        hover: TipKey,
    },
}

/// The spells tab.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpellsData {
    pub books: Vec<Choice>,
    /// With no book: the words, and the books the shard may open.
    pub no_book: Option<OpenBooks>,
    pub empty: Option<&'static str>,
    pub list: Vec<SpellRow>,
    /// The mark of a spell Ctrl+Alt and a click makes a macro of, while
    /// "Fast spell assign" is on.
    pub assign: Option<&'static str>,
    pub detail: Option<SpellDetail>,
    pub pick: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OpenBooks {
    pub words: &'static str,
    /// Each book by its name for the open command, and its title.
    pub books: Vec<[&'static str; 2]>,
}

/// One spell of the book.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpellRow {
    pub id: u16,
    pub name: String,
    pub icon: Option<String>,
    pub chosen: bool,
    pub hover: TipKey,
}

/// What the chosen spell is.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpellDetail {
    pub id: u16,
    pub icon: Option<String>,
    pub name: String,
    pub group: Option<&'static str>,
    /// The words of power first, then the reagents and what it asks for.
    pub power: Option<String>,
    pub lines: Vec<DetailLine>,
    /// Cast and Pin, while the human has control.
    pub buttons: Option<[&'static str; 2]>,
}

/// A line of the detail of a spell: a dim label, or plain words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DetailLine {
    pub words: String,
    pub dim: bool,
}

/// The party tab.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PartyData {
    pub invite: Option<Invite>,
    pub loot: Option<String>,
    pub leave: Option<&'static str>,
    pub add: Option<&'static str>,
    pub rows: Vec<PartyRow>,
    pub tell: Option<TellField>,
}

/// An invite of the shard: its words, and Accept and Decline while the
/// human has control.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Invite {
    pub words: String,
    pub buttons: Option<[&'static str; 2]>,
}

/// One row of the party list.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PartyRow {
    Place {
        number: String,
        member: Option<Member>,
        empty: &'static str,
    },
    NearTitle {
        words: &'static str,
    },
    Near {
        serial: u32,
        name: String,
        invite: Option<&'static str>,
    },
}

/// A member of the party.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Member {
    pub serial: u32,
    pub name: String,
    /// Hits, mana and stamina, each a share.
    pub pools: [f32; 3],
    /// The words go to this member.
    pub chosen: bool,
    pub tell: Option<&'static str>,
    pub kick: Option<&'static str>,
    pub hover: TipKey,
}

/// The field of words to the party or to one member.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TellField {
    pub hint: String,
    pub say: &'static str,
}

/// A group of skills and its new name.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct GroupName {
    pub at: usize,
    pub name: String,
}

/// A click on a spell, with the keys held: `{id, ctrl, alt}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct SpellClick {
    pub id: u16,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
}

/// A skill and the group it goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct SkillTo {
    pub skill: u16,
    pub to: usize,
}

/// What the player does on the sheet.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SheetAction {
    Tab(usize),
    View(usize),
    StatLock(usize),
    WornClick(u32),
    WornDouble(u32),
    TakeOff(u8),
    WornDrag(u32),
    WornMenu(AtThing),
    Wear(String),
    Sort(usize),
    SkillLock(u16),
    UseSkill(u16),
    PinSkill(u16),
    DragSkill(u16),
    FoldGroup(usize),
    DeleteGroup(usize),
    NewGroup(bool),
    ResetGroups(bool),
    ResetAnswer(bool),
    RenameGroup(GroupName),
    MoveSkill(SkillTo),
    Book(usize),
    Spell(SpellClick),
    Cast(u16),
    PinSpell(u16),
    DragSpell(u16),
    OpenBook(String),
    Loot(bool),
    Leave(bool),
    Add(bool),
    Member(u32),
    TellTo(u32),
    Kick(u32),
    Invite(u32),
    Accept(bool),
    Decline(bool),
    Say(String),
    /// A button of the character tab that opens or closes an ability
    /// panel, by its place.
    Panel(usize),
}

const POOL_PERCENT: f32 = 100.0;

fn choices<T: PartialEq + Copy>(all: &[(T, &'static str)], chosen: T) -> Vec<Choice> {
    all.iter()
        .map(|(which, words)| Choice {
            words: (*words).to_string(),
            chosen: *which == chosen,
        })
        .collect()
}

impl WebView {
    pub(super) fn sheet_spec(&self, frame: &WatchFrame) -> FrameSpec {
        let title = if frame.name.is_empty() {
            WORDS_SHEET
        } else {
            &frame.name
        };
        let default = sheet_first_place(self.panel_room());
        FrameSpec::fixed(SHEET_ID, title, default)
            .sized(sheet_least())
            .closable()
    }

    /// The sheet in one frame: the name a click asked for once no double
    /// click came, what Jev picked to wear, and the book of a school asked
    /// for before the shard told of it.
    pub(crate) fn follow_sheet(&mut self, frame: &WatchFrame, time: f64) {
        let state = &mut self.panels.sheet;
        if let Some(act) = state.worn_clicks.due_look(time) {
            self.hand.act(act);
        }
        for answer in self.hand.new_answers(Asker::Deck) {
            let state = &mut self.panels.sheet;
            match answer {
                Answer::Picked(Ok(place)) => {
                    let act = match state.wear_asked.get(place) {
                        Some(WearChoice::Wear(serial)) => Act::Wear(*serial),
                        Some(WearChoice::TakeOff(layer)) => Act::TakeOff(*layer),
                        None => continue,
                    };
                    state.wear_note = None;
                    self.hand.act(act);
                }
                Answer::Picked(Err(words)) => state.wear_note = Some((words, true, time)),
                _ => {}
            }
        }
        let state = &mut self.panels.sheet;
        if state
            .wear_note
            .as_ref()
            .is_some_and(|(_, _, since)| time - since > NOTE_SECONDS)
        {
            state.wear_note = None;
        }
        state.book.follow(frame);
        if state
            .tell_to
            .is_some_and(|to| !frame.party_members.iter().any(|member| member.serial == to))
        {
            state.tell_to = None;
        }
    }

    pub(super) fn sheet_data(&mut self, frame: &WatchFrame) -> Option<Framed<SheetData>> {
        if !self.panels.sheet.open {
            return None;
        }
        let tab = self.panels.sheet.tab;
        let body = SheetData {
            tabs: choices(&SHEET_TABS, tab),
            live: frame.human_control,
            character: (tab == Tab::Character).then(|| self.character_data(frame)),
            skills: (tab == Tab::Skills).then(|| self.skills_data(frame)),
            spells: (tab == Tab::Spells).then(|| self.spells_data(frame)),
            party: (tab == Tab::Party).then(|| self.party_data(frame)),
        };
        Some(self.framed(PANEL_SHEET, &self.sheet_spec(frame), body))
    }

    fn stat_row(&mut self, frame: &WatchFrame, stat: usize) -> StatRow {
        StatRow {
            stat,
            name: STAT_NAMES[stat],
            value: stats(frame)[stat].to_string(),
            lock: self.panels.sheet.stat_locks.shown(frame, stat),
            hover: TipKey::label(STAT_NAMES[stat], HINT_STAT_LOCK),
        }
    }

    fn character_data(&mut self, frame: &WatchFrame) -> CharacterData {
        let view = self.panels.sheet.view;
        CharacterData {
            views: choices(&CHARACTER_VIEWS, view),
            panels: SHEET_PANEL_BUTTONS
                .iter()
                .map(|(id, words)| Choice {
                    words: (*words).to_string(),
                    chosen: places::is_open(&self.profile, id),
                })
                .collect(),
            worn: (view == CharacterView::Worn).then(|| self.worn_data(frame)),
            status: (view == CharacterView::Status).then(|| {
                status_lines(frame)
                    .into_iter()
                    .map(|line| match line {
                        StatusLine::Title(words) => StatusRow::Title { words },
                        StatusLine::Stat(stat) => StatusRow::Stat(self.stat_row(frame, stat)),
                        StatusLine::Fact(words, value) => StatusRow::Fact(Fact { words, value }),
                    })
                    .collect()
            }),
        }
    }

    fn worn_data(&mut self, frame: &WatchFrame) -> WornData {
        let interface = &self.profile.interface;
        let wears = if interface.durability_bars {
            worn_wear(frame, self.hand.reads())
        } else {
            Vec::new()
        };
        let warning = self.profile.interface.durability_warning;
        let footer = if frame.human_control { HINT_WORN } else { "" };
        let worn: Vec<_> = frame
            .look
            .equipment
            .iter()
            .filter(|item| is_worn_layer(item.layer))
            .cloned()
            .collect();
        let rows = worn
            .iter()
            .map(|item| {
                let request = self.item_picture_request(item.graphic, item.hue);
                WornRow {
                    serial: item.serial,
                    layer: item.layer,
                    picture: Some(self.picture_key(&request)),
                    words: layer_words(item.layer),
                    wear: wears
                        .iter()
                        .find(|wear| wear.serial == item.serial)
                        .map(|wear| BarLine {
                            share: wear.share(),
                            color: css_color(wear_color(wear, warning)),
                        }),
                    take_off: WORDS_TAKE_OFF,
                    hover: TipKey::thing(item.serial, "", footer),
                }
            })
            .collect();
        let doll = self.picture_key(&doll_figure(&frame.look));
        let state = &self.panels.sheet;
        let (choices, _) = wear_choices(frame);
        let wear = frame.human_control.then(|| WearField {
            hint: HINT_WEAR,
            button: WORDS_WEAR,
            on: !choices.is_empty(),
            note: state.wear_note.as_ref().map(|(words, failed, _)| Colored {
                words: words.clone(),
                color: css_color(if *failed { ALARM } else { WAITING }),
            }),
        });
        WornData {
            doll: Some(doll),
            name: frame.name.clone(),
            stats: (0..STAT_NAMES.len())
                .map(|stat| self.stat_row(frame, stat))
                .collect(),
            facts: vec![
                Fact {
                    words: WORDS_WEIGHT,
                    value: frame.carried(),
                },
                Fact {
                    words: WORDS_GOLD,
                    value: frame.gold.to_string(),
                },
            ],
            worn_words: format!("{WORDS_WORN} ({})", worn.len()),
            nothing: worn.is_empty().then_some(WORDS_NOTHING_WORN),
            rows,
            zone: DropZone::Wear,
            wear,
        }
    }

    fn skills_data(&self, frame: &WatchFrame) -> SkillsData {
        let grouped = self.profile.general.standard_skills_gump;
        let state = &self.panels.sheet;
        let (sort, descending) = state.sort();
        let live = frame.human_control;
        let columns = SkillSort::COLUMNS
            .into_iter()
            .map(|column| SkillColumn {
                words: column.label(),
                chosen: !grouped && column == sort,
                descending,
            })
            .collect();
        let skill_row = |skill: &uoterm_view::frame::WatchSkill| SkillRow::Skill {
            id: skill.id,
            name: skill.name.clone(),
            values: SkillSort::COLUMNS
                .into_iter()
                .skip(1)
                .map(|column| column.tenths(skill).map(points).unwrap_or_default())
                .collect(),
            lock: skill.lock,
            buttons: (skill.usable && live).then_some([WORDS_USE, WORDS_PIN]),
            hover: TipKey::label(&skill.name, HINT_SKILL),
        };
        let rows = if grouped {
            let groups = shown_groups(&self.profile.skill_groups, &frame.skills);
            skill_entries(&groups, &frame.skills)
                .into_iter()
                .filter_map(|entry| match entry {
                    SkillEntry::Group(at) => {
                        let group = &groups[at];
                        Some(SkillRow::Group {
                            at,
                            name: group.name.clone(),
                            words: format!("{}  ({})", group.name, group.skills.len()),
                            fold: if group.open {
                                WORDS_GROUP_OPEN
                            } else {
                                WORDS_GROUP_FOLDED
                            },
                            delete: (live && at > 0).then_some(WORDS_DELETE_GROUP),
                            hover: TipKey::label(&group.name, HINT_GROUP),
                        })
                    }
                    SkillEntry::Skill(id) => frame
                        .skills
                        .iter()
                        .find(|skill| skill.id == id)
                        .map(skill_row),
                })
                .collect()
        } else {
            sorted(&frame.skills, sort, descending)
                .into_iter()
                .map(skill_row)
                .collect()
        };
        SkillsData {
            sums: format!(
                "{} {}   {} {}",
                SkillSort::Real.label(),
                total(&frame.skills, true),
                SkillSort::Value.label(),
                total(&frame.skills, false)
            ),
            grouped,
            columns,
            new_group: (grouped && !state.asking_reset).then_some(WORDS_NEW_GROUP),
            reset: (grouped && !state.asking_reset).then_some(WORDS_RESET),
            reset_ask: (grouped && state.asking_reset).then_some(ResetAsk {
                words: WORDS_RESET_ASK,
                yes: WORDS_YES,
                no: WORDS_NO,
            }),
            rows,
        }
    }

    fn spells_data(&mut self, frame: &WatchFrame) -> SpellsData {
        let live = frame.human_control;
        let Some(contents) = self.panels.sheet.book.shown(frame).cloned() else {
            return SpellsData {
                books: Vec::new(),
                no_book: Some(OpenBooks {
                    words: WORDS_NO_BOOK,
                    books: if live {
                        BOOKS
                            .iter()
                            .filter(|book| book.school != School::Mastery)
                            .map(|book| [book.name, book.title])
                            .collect()
                    } else {
                        Vec::new()
                    },
                }),
                empty: None,
                list: Vec::new(),
                assign: None,
                detail: None,
                pick: None,
            };
        };
        let books: Vec<_> = frame.spellbooks.iter().collect();
        let book_choices = (0..books.len())
            .map(|at| Choice {
                words: spell_book_words(&books, at),
                chosen: books[at].serial == contents.serial,
            })
            .collect();
        let book = book_info(&contents.school, contents.graphic);
        let held = book.held_places(Some(&contents));
        let chosen = self.panels.sheet.spell;
        let footer = if self.profile.combat.fast_spell_assign {
            HINT_ASSIGN
        } else {
            HINT_SPELL
        };
        let list = held
            .iter()
            .map(|place| {
                let spell = &book.spells[*place];
                let name = book_name(spell.id);
                let icon = ArtRequest::Gump {
                    gump: spell.small_icon,
                    hue: icon_hue(frame, spell.id),
                    partial: false,
                };
                SpellRow {
                    id: spell.id,
                    icon: Some(self.picture_key(&icon)),
                    chosen: chosen == Some(spell.id),
                    hover: TipKey::label(&name, footer),
                    name,
                }
            })
            .collect();
        let place = chosen.and_then(|id| held.iter().copied().find(|at| book.spells[*at].id == id));
        let detail = place.map(|place| {
            let spell = &book.spells[place];
            let mut lines = Vec::new();
            let reagents = reagent_lines(spell);
            if !reagents.is_empty() {
                lines.push(DetailLine {
                    words: WORDS_REAGENTS.to_string(),
                    dim: true,
                });
                lines.extend(reagents.lines().map(|line| DetailLine {
                    words: line.to_string(),
                    dim: false,
                }));
            }
            if book.school != School::Magery {
                let needs = needs_words(spell.mana, spell.skill, upkeep(book, spell));
                lines.extend(needs.lines().map(|line| DetailLine {
                    words: line.to_string(),
                    dim: false,
                }));
            }
            if book.school == School::Chivalry {
                lines.push(DetailLine {
                    words: format!("{WORDS_TITHING_COST}: {}", spell.tithing),
                    dim: false,
                });
                lines.push(DetailLine {
                    words: format!("{WORDS_TITHING_HAVE}: {}", frame.status.tithing),
                    dim: true,
                });
            }
            let power = power_words(spell.id);
            let icon = ArtRequest::Gump {
                gump: book.icon(place),
                hue: icon_hue(frame, spell.id),
                partial: false,
            };
            SpellDetail {
                id: spell.id,
                icon: Some(self.picture_key(&icon)),
                name: book_name(spell.id),
                group: book.spell_group(place),
                power: (!power.is_empty()).then_some(power),
                lines,
                buttons: live.then_some([WORDS_CAST, WORDS_PIN]),
            }
        });
        SpellsData {
            books: book_choices,
            no_book: None,
            empty: held.is_empty().then_some(WORDS_EMPTY_BOOK),
            list,
            assign: (live && self.profile.combat.fast_spell_assign).then_some(WORDS_ASSIGN),
            pick: (detail.is_none() && !held.is_empty()).then_some(WORDS_PICK_SPELL),
            detail,
        }
    }

    fn party_data(&self, frame: &WatchFrame) -> PartyData {
        let live = frame.human_control;
        let in_party = !frame.party_members.is_empty();
        let tell_to = self.panels.sheet.tell_to;
        let rows = party_entries(frame)
            .into_iter()
            .map(|entry| match entry {
                PartyEntry::Place(place) => PartyRow::Place {
                    number: format!("{}.", place + 1),
                    member: frame.party_members.get(place).map(|member| {
                        let share = |percent: Option<u8>| {
                            percent.map_or(0.0, |percent| f32::from(percent) / POOL_PERCENT)
                        };
                        let other = member.serial != frame.serial;
                        Member {
                            serial: member.serial,
                            name: member.name.clone(),
                            pools: [
                                share(member.hits_percent),
                                share(member.mana_percent),
                                share(member.stam_percent),
                            ],
                            chosen: tell_to == Some(member.serial),
                            tell: (live && other).then_some(WORDS_TELL),
                            kick: (live && other && leads(frame)).then_some(WORDS_KICK),
                            hover: TipKey::label(&member.name, HINT_MEMBER),
                        }
                    }),
                    empty: WORDS_EMPTY,
                },
                PartyEntry::NearTitle => PartyRow::NearTitle { words: WORDS_NEAR },
                PartyEntry::Near(serial) => PartyRow::Near {
                    serial,
                    name: frame
                        .mobiles
                        .iter()
                        .find(|mobile| mobile.serial == serial)
                        .map(|mobile| mobile.name.clone())
                        .unwrap_or_default(),
                    invite: live.then_some(WORDS_INVITE),
                },
            })
            .collect();
        let told = tell_to.and_then(|to| {
            frame
                .party_members
                .iter()
                .find(|member| member.serial == to)
                .map(|member| member.name.as_str())
        });
        PartyData {
            invite: frame.party_invite.map(|leader| Invite {
                words: invite_words(&inviter_name(frame, leader)),
                buttons: live.then_some([WORDS_ACCEPT, WORDS_DECLINE]),
            }),
            loot: (live && in_party).then(|| {
                if frame.party_can_loot {
                    WORDS_LOOT_ON
                } else {
                    WORDS_LOOT_OFF
                }
                .to_string()
            }),
            leave: (live && in_party).then(|| leave_words(frame)),
            add: (live && leads(frame)).then_some(WORDS_ADD),
            rows,
            tell: (live && in_party).then(|| TellField {
                hint: tell_hint(told),
                say: WORDS_SAY,
            }),
        }
    }

    pub(super) fn sheet_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<SheetAction>(action) else {
            return;
        };
        let Some(frame) = self.frame.clone() else {
            return;
        };
        match action {
            SheetAction::Tab(at) => {
                if let Some((tab, _)) = SHEET_TABS.get(at) {
                    self.panels.sheet.tab = *tab;
                }
            }
            SheetAction::View(at) => {
                if let Some((view, _)) = CHARACTER_VIEWS.get(at) {
                    self.panels.sheet.view = *view;
                }
            }
            SheetAction::Panel(at) => {
                if let Some((id, _)) = SHEET_PANEL_BUTTONS.get(at) {
                    let open = places::is_open(&self.profile, id);
                    places::set_open(&mut self.profile, id, !open);
                    self.keep_profile();
                }
            }
            SheetAction::Sort(column) => {
                if let Some(clicked) = SkillSort::COLUMNS.get(column) {
                    let state = &mut self.panels.sheet;
                    state.sort = Some(next_sort(state.sort(), *clicked));
                }
            }
            SheetAction::FoldGroup(at) => {
                let mut groups = shown_groups(&self.profile.skill_groups, &frame.skills);
                if let Some(group) = groups.get_mut(at) {
                    group.open = !group.open;
                    self.keep_groups(groups);
                }
            }
            SheetAction::Book(at) => {
                if let Some(book) = frame.spellbooks.get(at) {
                    self.panels.sheet.book.book = Some(book.serial);
                }
            }
            SheetAction::Spell(click) => {
                let mods = Mods {
                    ctrl: click.ctrl,
                    alt: click.alt,
                    ..Mods::default()
                };
                if frame.human_control && assigns_spell(self.profile.combat.fast_spell_assign, mods)
                {
                    self.assign_spell(click.id);
                } else {
                    self.panels.sheet.spell = Some(click.id);
                }
            }
            other if frame.human_control => self.sheet_act(&frame, other),
            _ => {}
        }
    }

    /// The actions of the sheet that act, while the human has control.
    fn sheet_act(&mut self, frame: &WatchFrame, action: SheetAction) {
        let time = self.hand.time();
        match action {
            SheetAction::StatLock(stat) if stat < STAT_NAMES.len() => {
                let line = self.panels.sheet.stat_locks.turn(frame, stat);
                self.hand.act(Act::Command(line));
            }
            SheetAction::WornClick(serial) => {
                if let Some(act) = self
                    .panels
                    .sheet
                    .worn_clicks
                    .single_click(frame, serial, time)
                {
                    self.hand.act(act);
                }
            }
            SheetAction::WornDouble(serial) => {
                self.panels.sheet.worn_clicks.double_clicked();
                self.hand.act(Act::Use(serial));
            }
            SheetAction::TakeOff(layer) => self.hand.act(Act::TakeOff(layer)),
            SheetAction::WornDrag(serial) => {
                if let Some(item) = frame
                    .look
                    .equipment
                    .iter()
                    .find(|item| item.serial == serial)
                {
                    self.pick_up(&WatchPackItem {
                        serial: item.serial,
                        graphic: item.graphic,
                        hue: item.hue,
                        amount: 1,
                        ..WatchPackItem::default()
                    });
                }
            }
            SheetAction::WornMenu(at) => {
                self.open_ring(Point::new(at.x, at.y), at.serial, "", RingSubject::Packed);
            }
            SheetAction::Wear(wish) => {
                let (choices, words) = wear_choices(frame);
                let wish = wish.trim().to_string();
                if choices.is_empty() || wish.is_empty() {
                    return;
                }
                self.hand.ask(
                    Asker::Deck,
                    Ask::WearItem {
                        wish,
                        options: words,
                    },
                );
                self.panels.sheet.wear_asked = choices;
                self.panels.sheet.wear_note = Some((WORDS_LOOKING.to_string(), false, time));
            }
            SheetAction::SkillLock(id) => {
                if let Some(skill) = frame.skills.iter().find(|skill| skill.id == id) {
                    self.hand.act(Act::Command(next_lock_command(skill)));
                }
            }
            SheetAction::UseSkill(id) => self.hand.act(Act::UseSkill(id)),
            SheetAction::PinSkill(id) => {
                if let Some(slot) = skill_slot(frame, id) {
                    self.pin_slot(frame, slot);
                }
            }
            SheetAction::DragSkill(id) => {
                if let Some(slot) = skill_slot(frame, id) {
                    self.drag_slot(slot);
                }
            }
            SheetAction::DeleteGroup(at) => {
                let mut groups = shown_groups(&self.profile.skill_groups, &frame.skills);
                if remove_group(&mut groups, at) {
                    self.keep_groups(groups);
                } else {
                    self.hand.report(WORDS_CANNOT_DELETE);
                }
            }
            SheetAction::NewGroup(_) => {
                let mut groups = shown_groups(&self.profile.skill_groups, &frame.skills);
                groups.push(SkillGroupSet {
                    name: NEW_GROUP.to_string(),
                    skills: Vec::new(),
                    open: true,
                });
                self.keep_groups(groups);
            }
            SheetAction::ResetGroups(_) => self.panels.sheet.asking_reset = true,
            SheetAction::ResetAnswer(yes) => {
                self.panels.sheet.asking_reset = false;
                if yes {
                    self.keep_groups(Vec::new());
                }
            }
            SheetAction::RenameGroup(GroupName { at, name }) => {
                let mut groups = shown_groups(&self.profile.skill_groups, &frame.skills);
                if let Some(group) = groups.get_mut(at) {
                    group.name = group_name(&name);
                    self.keep_groups(groups);
                }
            }
            SheetAction::MoveSkill(SkillTo { skill, to }) => {
                let mut groups = shown_groups(&self.profile.skill_groups, &frame.skills);
                let home = groups
                    .iter()
                    .position(|group| group.skills.contains(&skill));
                if home != Some(to) && to < groups.len() {
                    move_skill(&mut groups, skill, to);
                    self.keep_groups(groups);
                }
            }
            SheetAction::Cast(id) => {
                if let Some(book) = self.panels.sheet.book.shown(frame) {
                    self.hand.act(Act::CastFrom {
                        spell: id,
                        book: book.serial,
                    });
                }
            }
            SheetAction::PinSpell(id) => self.pin_slot(frame, spell_slot(id)),
            SheetAction::DragSpell(id) => self.drag_slot(spell_slot(id)),
            SheetAction::OpenBook(name) => {
                if let Some(book) = BOOKS.iter().find(|book| book.name == name) {
                    self.hand.act(Act::OpenSpellbook(book.name));
                }
            }
            SheetAction::Loot(_) => self.hand.act(Act::PartyLoot(!frame.party_can_loot)),
            SheetAction::Leave(_) => self.hand.act(Act::PartyLeave),
            SheetAction::Add(_) if leads(frame) => {
                self.hand.act(Act::Command(INVITE_COMMAND.into()));
            }
            SheetAction::Member(serial) => self.hand.act(member_click_act(frame, serial)),
            SheetAction::TellTo(serial) => {
                let state = &mut self.panels.sheet;
                state.tell_to = (state.tell_to != Some(serial)).then_some(serial);
            }
            SheetAction::Kick(serial) if leads(frame) => self.hand.act(Act::PartyKick(serial)),
            SheetAction::Invite(serial) => self.hand.act(Act::PartyInvite(serial)),
            SheetAction::Accept(_) => self.hand.act(Act::Command(ACCEPT_COMMAND.into())),
            SheetAction::Decline(_) => self.hand.act(Act::Command(DECLINE_COMMAND.into())),
            SheetAction::Say(words) if !frame.party_members.is_empty() => {
                let tell_to = self.panels.sheet.tell_to;
                if let Some(said) = party_say(&words, tell_to, &self.profile.speech) {
                    self.hand.act(said);
                }
            }
            _ => {}
        }
    }

    /// Makes a macro of a spell, as "Fast spell assign" does.
    fn assign_spell(&mut self, id: u16) {
        let (name, steps) = spell_macro(id);
        if key_macros::ensure(&mut self.profile.macros.key_bindings, &name, steps) {
            self.keep_profile();
        }
        self.hand.report(&assigned_words(&name));
    }

    /// Puts a slot of the sheet on the first free slot of the hotbar.
    pub(super) fn pin_slot(&mut self, frame: &WatchFrame, slot: Slot) {
        if !self.pin(&frame.name, slot) {
            self.hand.report(WORDS_BAR_FULL);
        }
    }

    /// Keeps the groups of the skills the player changed.
    fn keep_groups(&mut self, groups: Vec<SkillGroupSet>) {
        self.profile.skill_groups = groups;
        self.keep_profile();
    }

    /// Turns the spells tab to a book of this school, now or when the
    /// shard tells of one. False when the character has none yet.
    pub(crate) fn choose_school(&mut self, frame: &WatchFrame, school: School) -> bool {
        self.panels.sheet.book.choose_school(frame, school)
    }

    /// The school of the book the spells tab shows.
    pub(crate) fn shown_school(&self, frame: &WatchFrame) -> Option<School> {
        self.panels.sheet.book.school(frame)
    }
}

/// The slot of a skill the player starts.
fn skill_slot(frame: &WatchFrame, id: u16) -> Option<Slot> {
    frame
        .skills
        .iter()
        .find(|skill| skill.id == id && skill.usable)
        .map(|skill| Slot::Skill {
            id,
            name: skill.name.clone(),
        })
}

/// The slot of a spell.
fn spell_slot(id: u16) -> Slot {
    Slot::Spell {
        id,
        name: book_name(id),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, saved_profiles};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, MARA};
    use serde_json::json;

    const ME: u32 = 1;
    const BOB: u32 = 2;
    const SWORD: u32 = 0x4000_0050;
    const HIDING: u16 = 21;

    /// A view of Mara with the sheet open, a sword worn, a skill, a book of
    /// chivalry and Bob in her party.
    fn view_with_sheet() -> WebView {
        let mut view = WebView::new("{}");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["self_state"]["equipment"] = json!([
            { "serial": SWORD, "graphic": 0x0F5E, "layer": 1, "hue": 0 }
        ]);
        watch["skills"] = json!([
            { "id": HIDING, "name": "Hiding", "usable": true, "value": 500, "base": 500, "cap": 1000, "lock": 0 }
        ]);
        watch["spellbooks"] = json!([{
            "book": 7, "school": "chivalry", "graphic": 0x2252,
            "spells": [{ "number": 201, "name": "" }, { "number": 203, "name": "" }]
        }]);
        watch["party_members"] = json!([
            { "serial": ME, "name": MARA },
            { "serial": BOB, "name": "Bob", "hits": 8, "hits_max": 10 }
        ]);
        view.frame(&watch.to_string(), 0.0);
        view.panels.sheet.open = true;
        view
    }

    #[test]
    fn a_worn_item_is_taken_off_and_a_double_click_uses_it() {
        let mut view = view_with_sheet();
        let sheet = view.panel_data(0.0).sheet.unwrap().body;
        let worn = sheet.character.unwrap().worn.unwrap();
        let sword: Vec<_> = worn.rows.iter().filter(|row| row.serial == SWORD).collect();
        assert_eq!(sword.len(), 1, "{:?}", worn.rows);
        let out = press(&mut view, PANEL_SHEET, json!({ "take_off": 1 }));
        assert_eq!(out_acts(&out), vec![Act::TakeOff(1).for_page()]);
        let out = press(&mut view, PANEL_SHEET, json!({ "worn_double": SWORD }));
        assert_eq!(out_acts(&out), vec![Act::Use(SWORD).for_page()]);
    }

    #[test]
    fn a_click_on_a_worn_item_asks_its_name_after_the_double_click_time() {
        let mut view = view_with_sheet();
        press(&mut view, PANEL_SHEET, json!({ "worn_click": SWORD }));
        let frame = view.frame_ref().unwrap().clone();
        view.follow_sheet(&frame, 1.0);
        assert_eq!(
            out_acts(&view.take_out_native()),
            vec![Act::Look(SWORD).for_page()]
        );
    }

    #[test]
    fn a_stat_lock_turns_as_in_the_window() {
        let mut view = view_with_sheet();
        let frame = view.frame_ref().unwrap().clone();
        let expected = StatLocks::default().turn(&frame, 0);
        let out = press(&mut view, PANEL_SHEET, json!({ "stat_lock": 0 }));
        assert_eq!(out_acts(&out), vec![Act::Command(expected).for_page()]);
    }

    #[test]
    fn a_skill_is_used_pinned_and_sorted() {
        let mut view = view_with_sheet();
        press(&mut view, PANEL_SHEET, json!({ "tab": 1 }));
        let skills = view.panel_data(0.0).sheet.unwrap().body.skills.unwrap();
        assert!(
            matches!(skills.rows.as_slice(), [SkillRow::Group { at: 0, .. }]),
            "folded"
        );
        press(&mut view, PANEL_SHEET, json!({ "fold_group": 0 }));
        let skills = view.panel_data(0.0).sheet.unwrap().body.skills.unwrap();
        assert!(skills
            .rows
            .iter()
            .any(|row| matches!(row, SkillRow::Skill { id, .. } if *id == HIDING)));
        let out = press(&mut view, PANEL_SHEET, json!({ "use_skill": HIDING }));
        assert_eq!(out_acts(&out), vec![Act::UseSkill(HIDING).for_page()]);
        press(&mut view, PANEL_SHEET, json!({ "pin_skill": HIDING }));
        assert!(view.hotbars.slot(MARA, 0).is_some());
        let out = press(&mut view, PANEL_SHEET, json!({ "new_group": true }));
        let groups = &saved_profiles(&out)[0].skill_groups;
        assert_eq!(groups.last().unwrap().name, NEW_GROUP);
    }

    #[test]
    fn a_spell_of_the_book_is_read_and_cast_from_its_book() {
        let mut view = view_with_sheet();
        press(&mut view, PANEL_SHEET, json!({ "tab": 2 }));
        let spells = view.panel_data(0.0).sheet.unwrap().body.spells.unwrap();
        assert_eq!(spells.list.len(), 2);
        press(
            &mut view,
            PANEL_SHEET,
            json!({ "spell": { "id": 201, "ctrl": true, "alt": true } }),
        );
        let detail = view
            .panel_data(0.0)
            .sheet
            .unwrap()
            .body
            .spells
            .unwrap()
            .detail
            .unwrap();
        assert_eq!(detail.id, 201);
        let out = press(&mut view, PANEL_SHEET, json!({ "cast": 201 }));
        assert_eq!(
            out_acts(&out),
            vec![Act::CastFrom {
                spell: 201,
                book: 7
            }
            .for_page()]
        );
    }

    #[test]
    fn the_party_tab_tells_one_member_and_the_leader_kicks() {
        let mut view = view_with_sheet();
        press(&mut view, PANEL_SHEET, json!({ "tab": 3 }));
        press(&mut view, PANEL_SHEET, json!({ "tell_to": BOB }));
        let party = view.panel_data(0.0).sheet.unwrap().body.party.unwrap();
        assert_eq!(party.tell.unwrap().hint, "Tell Bob");
        let out = press(&mut view, PANEL_SHEET, json!({ "say": "heal me" }));
        let [act] = out_acts(&out).try_into().unwrap();
        assert_eq!(act.calls.len(), 1);
        let out = press(&mut view, PANEL_SHEET, json!({ "kick": BOB }));
        assert_eq!(out_acts(&out), vec![Act::PartyKick(BOB).for_page()]);
    }

    #[test]
    fn nothing_on_the_sheet_acts_without_control() {
        let mut view = view_with_sheet();
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["human_control"] = json!(false);
        view.frame(&watch.to_string(), 0.1);
        let acts = [
            json!({ "take_off": 1 }),
            json!({ "worn_double": SWORD }),
            json!({ "stat_lock": 0 }),
            json!({ "use_skill": HIDING }),
            json!({ "cast": 201 }),
            json!({ "member": BOB }),
            json!({ "kick": BOB }),
            json!({ "say": "hail" }),
            json!({ "leave": true }),
        ];
        for act in acts {
            assert!(
                press(&mut view, PANEL_SHEET, act.clone()).is_empty(),
                "{act}"
            );
        }
    }

    #[test]
    fn a_spell_click_with_ctrl_and_alt_makes_its_macro_when_the_option_is_on() {
        let mut view = view_with_sheet();
        view.profile.combat.fast_spell_assign = true;
        let before = view.profile.macros.key_bindings.len();
        press(
            &mut view,
            PANEL_SHEET,
            json!({ "spell": { "id": 201, "ctrl": true, "alt": true } }),
        );
        assert_eq!(view.profile.macros.key_bindings.len(), before + 1);
        assert_eq!(
            view.panels.sheet.spell, None,
            "the click made a macro, it read nothing"
        );
        press(&mut view, PANEL_SHEET, json!({ "spell": { "id": 201 } }));
        assert_eq!(view.panels.sheet.spell, Some(201));
    }

    #[test]
    fn a_member_click_and_words_to_the_party_are_the_windows() {
        let mut view = view_with_sheet();
        let frame = view.frame_ref().unwrap().clone();
        let out = press(&mut view, PANEL_SHEET, json!({ "member": BOB }));
        assert_eq!(
            out_acts(&out),
            vec![member_click_act(&frame, BOB).for_page()]
        );
        let out = press(&mut view, PANEL_SHEET, json!({ "say": " heal me " }));
        let said = party_say("heal me", None, &view.profile.speech).unwrap();
        assert_eq!(out_acts(&out), vec![said.for_page()]);
    }
}
