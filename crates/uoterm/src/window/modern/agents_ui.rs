//! The agent windows: one for each agent the Agents page offers, and one
//! that lists them. Each window reads the agent's settings from the session
//! and changes them with the agent tools; the session keeps them. The
//! ignore list is the profile's own, for speech the journal hides.

use super::super::boxes_ui::Tools;
use super::super::control::Act;
use super::super::deck_ui::{is_worn_layer, layer_words};
use super::super::model::agents::{
    set_field, AgentPanel, AgentsView, Lists, RuleRow, AUTOLOOT, BANDAGE, BANDAGE_WHOM,
    BONE_CUTTER, CARVER, DRESS, FRIENDS, HEAL_SPELLS, KEY_ACTIVE, KEY_BLADE, KEY_DESTINATION,
    KEY_HEAL_SPELL, KEY_ITEMS, KEY_LAYER, KEY_MOUNT, KEY_SERIAL, KEY_SOURCE, KEY_WHOM, ORGANIZER,
    REMOUNT, SELF_HEAL, UNDRESS,
};
use super::super::model::{counters, places};
use super::super::settings::Profile;
use super::super::theme;
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use super::rows::{Picked, Rows};
use crate::view::{WatchFrame, WatchPackItem};
use eframe::egui::{self, Color32, Rect, Vec2};
use serde_json::{json, Value};
use std::collections::HashMap;
use uoterm_runtime::tools::TOOL_AGENTS;

pub const AGENTS_ID: &str = "modern:agents";
pub const IGNORE_ID: &str = "modern:ignore";
/// The agents are read this often while a window shows, in seconds.
const AGENTS_MAX_AGE: f64 = 2.0;
const WINDOW_SIZE: Vec2 = Vec2::new(320.0, 420.0);
const MIN_SIZE: Vec2 = Vec2::new(260.0, 160.0);
/// The list opens at the top of the right column and the ignore list a
/// step from it; each agent window opens in the middle, a step from the
/// one before.
const AGENTS_SPOT: Spot = Spot::RightColumn(0);
const IGNORE_SPOT: Spot = Spot::RightColumn(1);
const BUTTON_WIDTH: f32 = 64.0;
const SMALL_BUTTON_WIDTH: f32 = 30.0;
/// A mount is picked from the mobiles this near, a friend from farther.
const MOUNT_TILES: u16 = 3;
const FRIEND_TILES: u16 = 12;

const WORDS_AGENTS: &str = "Agents";
const WORDS_IGNORE: &str = "Ignore list";
const WORDS_OPEN: &str = "Open";
const WORDS_CLOSE: &str = "Close";
const WORDS_ON: &str = "on";
const WORDS_OFF: &str = "off";
const WORDS_TURN_ON: &str = "Turn on";
const WORDS_TURN_OFF: &str = "Turn off";
const WORDS_LESS: &str = "-";
const WORDS_MORE: &str = "+";
const WORDS_REMOVE: &str = "x";
const WORDS_ADD: &str = "Add";
const WORDS_PICK: &str = "Use";
const WORDS_JOB: &str = "Job running:";
const WORDS_NO_JOB: &str = "No job runs.";
const WORDS_STOP: &str = "Stop";
const WORDS_RUN: &str = "Run";
const WORDS_DRESS: &str = "Dress";
const WORDS_UNDRESS: &str = "Undress";
const WORDS_LOOT_NOW: &str = "Loot corpses in reach";
const WORDS_LISTS: &str = "Lists (the one in use is bright):";
const WORDS_ADD_FROM_BAG: &str = "Add from your bag:";
const WORDS_NEW_LIST: &str = "Add list";
const WORDS_SAVE_WORN: &str = "Save worn";
const HINT_LIST_NAME: &str = "new list name";
const HINT_IGNORE_NAME: &str = "a name to ignore";
const WORDS_HEAL_WHOM: &str = "Heal:";
const WORDS_SPELL: &str = "Spell:";
const WORDS_MOUNT: &str = "Mount:";
const WORDS_BLADE: &str = "Blade:";
const WORDS_NONE_SET: &str = "none";
const WORDS_TO: &str = "Into:";
const WORDS_FROM: &str = "From:";
const WORDS_THE_PACK: &str = "the pack";
const WORDS_NEAR: &str = "Near you:";
const WORDS_NOT_READ: &str = "The session has not answered yet.";

#[derive(Default)]
pub struct AgentsUi {
    scroll: HashMap<String, f32>,
    new_list: HashMap<AgentPanel, String>,
    /// The job list each window shows.
    chosen_list: HashMap<AgentPanel, String>,
    ignore_name: String,
}

/// The items of the backpack and its open bags, one of each graphic and
/// hue, with their pictures.
fn bag_items(frame: &WatchFrame, tools: &mut Tools<'_>) -> (Vec<Picked>, Vec<WatchPackItem>) {
    let mut seen: Vec<WatchPackItem> = Vec::new();
    for item in counters::backpack_containers(frame)
        .into_iter()
        .flat_map(|bag| bag.items.iter())
    {
        if !seen
            .iter()
            .any(|kept| kept.graphic == item.graphic && kept.hue == item.hue)
        {
            seen.push(item.clone());
        }
    }
    let pictures = seen
        .iter()
        .map(|item| {
            (
                tools.scene.item_picture(frame.map, item.graphic, item.hue),
                item.name.clone(),
            )
        })
        .collect();
    (pictures, seen)
}

/// The name of a thing by its serial: a mobile or an item the frame knows,
/// else its number.
fn name_of(frame: &WatchFrame, serial: u32) -> String {
    frame
        .mobiles
        .iter()
        .find(|m| m.serial == serial)
        .map(|m| m.name.clone())
        .or_else(|| {
            frame
                .containers
                .iter()
                .flat_map(|c| c.items.iter())
                .find(|i| i.serial == serial)
                .map(|i| i.name.clone())
        })
        .or_else(|| {
            frame
                .containers
                .iter()
                .find(|c| c.serial == serial)
                .map(|c| c.name.clone())
        })
        .unwrap_or_else(|| format!("0x{serial:08X}"))
}

fn on_words(on: bool) -> &'static str {
    if on {
        WORDS_ON
    } else {
        WORDS_OFF
    }
}

fn chosen_color(chosen: bool) -> Color32 {
    if chosen {
        theme::GOAL
    } else {
        theme::TEXT_DIM
    }
}

impl AgentsUi {
    /// Draws the agent windows that are open. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<Rect> {
        let chooser = places::is_open(profile, AGENTS_ID);
        let windows: Vec<AgentPanel> = AgentPanel::ALL
            .into_iter()
            .filter(|panel| {
                panel.offered(&profile.agents) && places::is_open(profile, &panel.place_id())
            })
            .collect();
        let ignore = places::is_open(profile, IGNORE_ID);
        let mut covered = Vec::new();
        if ignore {
            covered.push(self.ignore_window(ui, rect, frame, tools, profile));
        }
        if !chooser && windows.is_empty() {
            return covered;
        }
        let view = tools
            .readings
            .want(TOOL_AGENTS, json!({}), AGENTS_MAX_AGE)
            .map(AgentsView::new);
        if chooser {
            covered.push(self.chooser(ui, rect, frame, tools, profile, view.as_ref()));
        }
        for (at, panel) in windows.into_iter().enumerate() {
            covered.push(self.window(ui, rect, frame, tools, profile, panel, at, view.as_ref()));
        }
        covered
    }

    fn scroll_of(&self, id: &str) -> f32 {
        self.scroll.get(id).copied().unwrap_or_default()
    }

    /// Acts on the character's side and reads the agents again soon.
    fn send(acts: Vec<Act>, tools: &mut Tools<'_>) {
        if acts.is_empty() {
            return;
        }
        for act in acts {
            tools.hand.act(act);
        }
        tools.readings.refresh(TOOL_AGENTS);
    }

    fn close_on(event: Option<FrameEvent>, id: &str, tools: &mut Tools<'_>, profile: &mut Profile) {
        if event == Some(FrameEvent::Closed) {
            places::set_open(profile, id, false);
            tools.keep_profile(profile);
        }
    }

    /// The list of the agent windows, with the job that runs.
    fn chooser(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        view: Option<&AgentsView>,
    ) -> Rect {
        let spec = PanelSpec {
            id: AGENTS_ID,
            title: WORDS_AGENTS,
            default: layout::first_place(rect, AGENTS_SPOT, WINDOW_SIZE),
            min_size: Some(MIN_SIZE),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_AGENTS);
        let live = frame.human_control;
        let mut toggles = Vec::new();
        let mut acts = Vec::new();
        let scroll = {
            let mut rows = Rows::new(ui, body, AGENTS_ID, self.scroll_of(AGENTS_ID));
            match view.and_then(AgentsView::job) {
                Some(job) => {
                    let stop = [(WORDS_STOP, theme::ALARM)];
                    if rows
                        .labeled("job", 0, &format!("{WORDS_JOB} {job}"), &stop, BUTTON_WIDTH)
                        .is_some()
                        && live
                    {
                        acts.push(Act::AgentStop);
                    }
                }
                None => rows.words(WORDS_NO_JOB, theme::TEXT_FAINT),
            }
            let offered = AgentPanel::ALL
                .into_iter()
                .filter(|panel| panel.offered(&profile.agents));
            for (at, agent) in offered.enumerate() {
                let open = places::is_open(profile, &agent.place_id());
                let on =
                    view.is_some_and(|view| agent.switches().iter().any(|name| view.is_on(name)));
                let words = if agent.switches().is_empty() {
                    agent.title().to_string()
                } else {
                    format!("{} ({})", agent.title(), on_words(on))
                };
                let button = [(if open { WORDS_CLOSE } else { WORDS_OPEN }, theme::TEXT)];
                if rows
                    .labeled("agent", at, &words, &button, BUTTON_WIDTH)
                    .is_some()
                {
                    toggles.push((agent.place_id(), !open));
                }
            }
            let open = places::is_open(profile, IGNORE_ID);
            let button = [(if open { WORDS_CLOSE } else { WORDS_OPEN }, theme::TEXT)];
            if rows
                .labeled("ignore", 0, WORDS_IGNORE, &button, BUTTON_WIDTH)
                .is_some()
            {
                toggles.push((IGNORE_ID.to_string(), !open));
            }
            rows.finish()
        };
        self.scroll.insert(AGENTS_ID.to_string(), scroll);
        for (id, open) in &toggles {
            places::set_open(profile, id, *open);
        }
        if !toggles.is_empty() {
            tools.keep_profile(profile);
        }
        Self::send(acts, tools);
        let event = frame::controls(ui, panel, &spec, profile, tools);
        Self::close_on(event, AGENTS_ID, tools, profile);
        panel
    }

    /// One agent's window.
    #[allow(clippy::too_many_arguments)]
    fn window(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        agent: AgentPanel,
        at: usize,
        view: Option<&AgentsView>,
    ) -> Rect {
        let id = agent.place_id();
        let spec = PanelSpec {
            id: &id,
            title: agent.title(),
            default: layout::first_place(rect, Spot::Middle(at), WINDOW_SIZE),
            min_size: Some(MIN_SIZE),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, agent.title());
        let (pictures, bag) = bag_items(frame, tools);
        let mut acts = Vec::new();
        let scroll = {
            let mut rows = Rows::new(ui, body, &id, self.scroll_of(&id));
            match view {
                None => {
                    let failure = tools.readings.failure(TOOL_AGENTS, &json!({}));
                    rows.words(failure.unwrap_or(WORDS_NOT_READ), theme::TEXT_FAINT);
                }
                Some(view) => {
                    self.settings_rows(&mut rows, agent, view, frame, &pictures, &bag, &mut acts);
                    self.list_rows(&mut rows, agent, view, frame, &pictures, &bag, &mut acts);
                }
            }
            rows.finish()
        };
        self.scroll.insert(id.clone(), scroll);
        if frame.human_control {
            Self::send(acts, tools);
        }
        let event = frame::controls(ui, panel, &spec, profile, tools);
        Self::close_on(event, &id, tools, profile);
        panel
    }

    /// The switches, numbers, flags and the rows of each agent's own.
    #[allow(clippy::too_many_arguments)]
    fn settings_rows(
        &mut self,
        rows: &mut Rows<'_>,
        agent: AgentPanel,
        view: &AgentsView,
        frame: &WatchFrame,
        pictures: &[Picked],
        bag: &[WatchPackItem],
        acts: &mut Vec<Act>,
    ) {
        for (at, name) in agent.switches().iter().enumerate() {
            let on = view.is_on(name);
            let button = [(
                if on { WORDS_TURN_OFF } else { WORDS_TURN_ON },
                chosen_color(!on),
            )];
            if rows
                .labeled(
                    "switch",
                    at,
                    &format!("{name}: {}", on_words(on)),
                    &button,
                    BUTTON_WIDTH,
                )
                .is_some()
            {
                acts.push(Act::AgentOn {
                    agent: (*name).to_string(),
                    on: !on,
                });
            }
        }
        if let Some(name) = agent.settings_agent() {
            for (at, (key, words, step)) in agent.numbers().iter().enumerate() {
                let value = view.number(name, key).unwrap_or_default();
                let buttons = [(WORDS_LESS, theme::TEXT), (WORDS_MORE, theme::TEXT)];
                match rows.labeled(
                    "number",
                    at,
                    &format!("{words}: {value}"),
                    &buttons,
                    SMALL_BUTTON_WIDTH,
                ) {
                    Some(0) => acts.push(set_field(
                        view,
                        name,
                        key,
                        json!(value.saturating_sub(*step)),
                    )),
                    Some(_) => acts.push(set_field(view, name, key, json!(value + step))),
                    None => {}
                }
            }
            for (at, (key, words)) in agent.flags().iter().enumerate() {
                let on = view.flag(name, key);
                let button = [(on_words(on), chosen_color(on))];
                if rows
                    .labeled("flag", at, words, &button, BUTTON_WIDTH)
                    .is_some()
                {
                    acts.push(set_field(view, name, key, json!(!on)));
                }
            }
        }
        match agent {
            AgentPanel::Bandage => {
                rows.words(WORDS_HEAL_WHOM, theme::TEXT_DIM);
                let whom = view
                    .settings(BANDAGE)
                    .and_then(|s| s.get(KEY_WHOM))
                    .and_then(Value::as_str);
                let labels: Vec<(&str, Color32)> = BANDAGE_WHOM
                    .iter()
                    .map(|(word, words)| (*words, chosen_color(whom == Some(*word))))
                    .collect();
                if let Some(at) = rows.buttons("whom", &labels) {
                    acts.push(set_field(
                        view,
                        BANDAGE,
                        KEY_WHOM,
                        json!(BANDAGE_WHOM[at].0),
                    ));
                }
            }
            AgentPanel::SelfHeal => {
                rows.words(WORDS_SPELL, theme::TEXT_DIM);
                let spell = view.number(SELF_HEAL, KEY_HEAL_SPELL);
                let labels: Vec<(&str, Color32)> = HEAL_SPELLS
                    .iter()
                    .map(|(id, words)| (*words, chosen_color(spell == Some(u64::from(*id)))))
                    .collect();
                if let Some(at) = rows.buttons("spell", &labels) {
                    acts.push(set_field(
                        view,
                        SELF_HEAL,
                        KEY_HEAL_SPELL,
                        json!(HEAL_SPELLS[at].0),
                    ));
                }
            }
            AgentPanel::Loot => {
                let pressed = rows.buttons("loot-now", &[(WORDS_LOOT_NOW, theme::GOAL)]);
                if pressed.is_some() {
                    acts.push(Act::AgentRun {
                        agent: AUTOLOOT.to_string(),
                        list: None,
                    });
                }
            }
            AgentPanel::Remount => {
                let mount = view
                    .number(REMOUNT, KEY_MOUNT)
                    .map(|serial| name_of(frame, serial as u32));
                rows.words(
                    &format!(
                        "{WORDS_MOUNT} {}",
                        mount.as_deref().unwrap_or(WORDS_NONE_SET)
                    ),
                    theme::TEXT,
                );
                rows.words(WORDS_NEAR, theme::TEXT_DIM);
                let near = frame.mobiles.iter().filter(|m| m.dist <= MOUNT_TILES);
                for (at, mobile) in near.enumerate() {
                    if rows
                        .labeled(
                            "mount",
                            at,
                            &mobile.name,
                            &[(WORDS_PICK, theme::TEXT)],
                            BUTTON_WIDTH,
                        )
                        .is_some()
                    {
                        acts.push(set_field(view, REMOUNT, KEY_MOUNT, json!(mobile.serial)));
                    }
                }
                rows.words(WORDS_ADD_FROM_BAG, theme::TEXT_DIM);
                if let Some(at) = rows.pictures("mount-item", pictures) {
                    acts.push(set_field(view, REMOUNT, KEY_MOUNT, json!(bag[at].serial)));
                }
            }
            AgentPanel::Skinning => {
                let blade = view
                    .number(CARVER, KEY_BLADE)
                    .map(|serial| name_of(frame, serial as u32));
                rows.words(
                    &format!(
                        "{WORDS_BLADE} {}",
                        blade.as_deref().unwrap_or(WORDS_NONE_SET)
                    ),
                    theme::TEXT,
                );
                rows.words(WORDS_ADD_FROM_BAG, theme::TEXT_DIM);
                if let Some(at) = rows.pictures("blade", pictures) {
                    for name in [CARVER, BONE_CUTTER] {
                        acts.push(set_field(view, name, KEY_BLADE, json!(bag[at].serial)));
                    }
                }
            }
            AgentPanel::Friends => {
                for (at, friend) in view.friends().into_iter().enumerate() {
                    let words = name_of(frame, friend);
                    if rows
                        .labeled(
                            "friend",
                            at,
                            &words,
                            &[(WORDS_REMOVE, theme::ALARM)],
                            SMALL_BUTTON_WIDTH,
                        )
                        .is_some()
                    {
                        acts.push(Act::AgentSet {
                            agent: FRIENDS.to_string(),
                            list: None,
                            settings: view.friends_toggled(friend),
                        });
                    }
                }
                rows.words(WORDS_NEAR, theme::TEXT_DIM);
                let friends = view.friends();
                let near = frame
                    .mobiles
                    .iter()
                    .filter(|m| m.dist <= FRIEND_TILES && !friends.contains(&m.serial));
                for (at, mobile) in near.enumerate() {
                    if rows
                        .labeled(
                            "near",
                            at,
                            &mobile.name,
                            &[(WORDS_ADD, theme::TEXT)],
                            BUTTON_WIDTH,
                        )
                        .is_some()
                    {
                        acts.push(Act::AgentSet {
                            agent: FRIENDS.to_string(),
                            list: None,
                            settings: view.friends_toggled(mobile.serial),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// The lists of an agent that keeps them.
    #[allow(clippy::too_many_arguments)]
    fn list_rows(
        &mut self,
        rows: &mut Rows<'_>,
        agent: AgentPanel,
        view: &AgentsView,
        frame: &WatchFrame,
        pictures: &[Picked],
        bag: &[WatchPackItem],
        acts: &mut Vec<Act>,
    ) {
        let lists = agent.lists();
        let (name, active) = match lists {
            Lists::None => return,
            Lists::Items(name) => (name, view.active_list(name)),
            Lists::Jobs(name) => {
                let names = view.list_names(lists);
                let chosen = self
                    .chosen_list
                    .get(&agent)
                    .filter(|chosen| names.contains(chosen))
                    .cloned()
                    .or_else(|| names.first().cloned())
                    .unwrap_or_default();
                (name, chosen)
            }
        };
        let names = view.list_names(lists);
        rows.words(WORDS_LISTS, theme::TEXT_DIM);
        let labels: Vec<(&str, Color32)> = names
            .iter()
            .map(|list| (list.as_str(), chosen_color(*list == active)))
            .collect();
        if let Some(at) = rows.buttons("lists", &labels) {
            match lists {
                Lists::Items(_) => acts.push(set_field(view, name, KEY_ACTIVE, json!(names[at]))),
                _ => {
                    self.chosen_list.insert(agent, names[at].clone());
                }
            }
        }
        if !active.is_empty() {
            self.chosen_rows(rows, agent, view, frame, &active, pictures, bag, acts);
        }
        let typed = self.new_list.entry(agent).or_default();
        let (button, fresh) = match agent {
            AgentPanel::Dress => (WORDS_SAVE_WORN, dress_of_worn(frame)),
            AgentPanel::Organizer => (WORDS_NEW_LIST, json!({ KEY_ITEMS: [] })),
            _ => (WORDS_NEW_LIST, json!([])),
        };
        if rows.field(
            "new-list",
            typed,
            HINT_LIST_NAME,
            button,
            BUTTON_WIDTH * 1.5,
        ) && !typed.trim().is_empty()
        {
            acts.push(Act::AgentSet {
                agent: name.to_string(),
                list: Some(typed.trim().to_string()),
                settings: fresh,
            });
            self.chosen_list.insert(agent, typed.trim().to_string());
            typed.clear();
        }
    }

    /// The items of the list in use or chosen, and its buttons.
    #[allow(clippy::too_many_arguments)]
    fn chosen_rows(
        &mut self,
        rows: &mut Rows<'_>,
        agent: AgentPanel,
        view: &AgentsView,
        frame: &WatchFrame,
        list: &str,
        pictures: &[Picked],
        bag: &[WatchPackItem],
        acts: &mut Vec<Act>,
    ) {
        let lists = agent.lists();
        let (Lists::Items(name) | Lists::Jobs(name)) = lists else {
            return;
        };
        let set_list = |settings: Value| Act::AgentSet {
            agent: name.to_string(),
            list: Some(list.to_string()),
            settings,
        };
        if agent == AgentPanel::Dress {
            let items = view
                .settings(DRESS)
                .and_then(|s| s.get(list))
                .and_then(|l| l.get(KEY_ITEMS))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for (at, item) in items.iter().enumerate() {
                let layer = item
                    .get(KEY_LAYER)
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u8;
                let serial = item
                    .get(KEY_SERIAL)
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u32;
                let words = format!("{}: {}", layer_words(layer), name_of(frame, serial));
                if rows
                    .labeled(
                        "dress-item",
                        at,
                        &words,
                        &[(WORDS_REMOVE, theme::ALARM)],
                        SMALL_BUTTON_WIDTH,
                    )
                    .is_some()
                {
                    acts.push(set_list(view.list_without(lists, list, at)));
                }
            }
            match rows.buttons(
                "dress-run",
                &[
                    (WORDS_DRESS, theme::GOAL),
                    (WORDS_UNDRESS, theme::TEXT),
                    (WORDS_STOP, theme::ALARM),
                ],
            ) {
                Some(0) => acts.push(Act::AgentRun {
                    agent: DRESS.to_string(),
                    list: Some(list.to_string()),
                }),
                Some(1) => acts.push(Act::AgentRun {
                    agent: UNDRESS.to_string(),
                    list: Some(list.to_string()),
                }),
                Some(_) => acts.push(Act::AgentStop),
                None => {}
            }
            return;
        }
        for (at, rule) in view.rules(lists, list).iter().enumerate() {
            let words = match (rule.name.is_empty(), rule.graphic, rule.amount) {
                (false, _, Some(amount)) => format!("{} x{amount}", rule.name),
                (false, _, None) => rule.name.clone(),
                (true, Some(graphic), _) => format!("0x{graphic:04X}"),
                (true, None, _) => WORDS_NONE_SET.to_string(),
            };
            if rows
                .labeled(
                    "rule",
                    at,
                    &words,
                    &[(WORDS_REMOVE, theme::ALARM)],
                    SMALL_BUTTON_WIDTH,
                )
                .is_some()
            {
                acts.push(set_list(view.list_without(lists, list, at)));
            }
        }
        rows.words(WORDS_ADD_FROM_BAG, theme::TEXT_DIM);
        if let Some(at) = rows.pictures("rule-add", pictures) {
            let item = &bag[at];
            let rule = RuleRow {
                name: item.name.clone(),
                graphic: Some(item.graphic),
                color: (item.hue != 0).then_some(item.hue),
                amount: None,
            };
            acts.push(set_list(view.list_with(lists, list, rule.to_value())));
        }
        if agent == AgentPanel::Organizer {
            for (part, key, words) in [
                ("from", KEY_SOURCE, WORDS_FROM),
                ("to", KEY_DESTINATION, WORDS_TO),
            ] {
                let bag_now = view.job_list_number(ORGANIZER, list, key).map_or_else(
                    || WORDS_THE_PACK.to_string(),
                    |serial| name_of(frame, serial as u32),
                );
                rows.words(&format!("{words} {bag_now}"), theme::TEXT);
                let open: Vec<(u32, String)> = frame
                    .containers
                    .iter()
                    .map(|c| (c.serial, c.name.clone()))
                    .collect();
                let labels: Vec<(&str, Color32)> = open
                    .iter()
                    .map(|(_, name)| (name.as_str(), theme::TEXT_DIM))
                    .collect();
                if let Some(at) = rows.buttons(part, &labels) {
                    acts.push(set_list(view.job_list_with(
                        ORGANIZER,
                        list,
                        key,
                        json!(open[at].0),
                    )));
                }
            }
            match rows.buttons(
                "organize",
                &[(WORDS_RUN, theme::GOAL), (WORDS_STOP, theme::ALARM)],
            ) {
                Some(0) => acts.push(Act::AgentRun {
                    agent: ORGANIZER.to_string(),
                    list: Some(list.to_string()),
                }),
                Some(_) => acts.push(Act::AgentStop),
                None => {}
            }
        }
    }

    /// The ignore list of the profile: names whose speech the journal hides.
    fn ignore_window(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let spec = PanelSpec {
            id: IGNORE_ID,
            title: WORDS_IGNORE,
            default: layout::first_place(rect, IGNORE_SPOT, WINDOW_SIZE),
            min_size: Some(MIN_SIZE),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_IGNORE);
        let names = profile.ignore.names.clone();
        let mut remove = None;
        let mut add = None;
        let scroll = {
            let mut rows = Rows::new(ui, body, IGNORE_ID, self.scroll_of(IGNORE_ID));
            for (at, name) in names.iter().enumerate() {
                if rows
                    .labeled(
                        "name",
                        at,
                        name,
                        &[(WORDS_REMOVE, theme::ALARM)],
                        SMALL_BUTTON_WIDTH,
                    )
                    .is_some()
                {
                    remove = Some(at);
                }
            }
            if rows.field(
                "typed",
                &mut self.ignore_name,
                HINT_IGNORE_NAME,
                WORDS_ADD,
                BUTTON_WIDTH,
            ) && !self.ignore_name.trim().is_empty()
            {
                add = Some(self.ignore_name.trim().to_string());
                self.ignore_name.clear();
            }
            rows.words(WORDS_NEAR, theme::TEXT_DIM);
            let near = frame
                .mobiles
                .iter()
                .filter(|m| m.dist <= FRIEND_TILES && !names.contains(&m.name));
            for (at, mobile) in near.enumerate() {
                if rows
                    .labeled(
                        "near",
                        at,
                        &mobile.name,
                        &[(WORDS_ADD, theme::TEXT)],
                        BUTTON_WIDTH,
                    )
                    .is_some()
                {
                    add = Some(mobile.name.clone());
                }
            }
            rows.finish()
        };
        self.scroll.insert(IGNORE_ID.to_string(), scroll);
        let changed = remove.is_some() || add.is_some();
        if let Some(at) = remove {
            profile.ignore.names.remove(at);
        }
        if let Some(name) = add {
            profile.ignore.add(&name);
        }
        if changed {
            tools.keep_profile(profile);
        }
        let event = frame::controls(ui, panel, &spec, profile, tools);
        Self::close_on(event, IGNORE_ID, tools, profile);
        panel
    }
}

/// A dress list of what the character wears now.
fn dress_of_worn(frame: &WatchFrame) -> Value {
    let items: Vec<Value> = frame
        .look
        .equipment
        .iter()
        .filter(|item| is_worn_layer(item.layer))
        .map(|item| json!({ KEY_LAYER: item.layer, KEY_SERIAL: item.serial }))
        .collect();
    json!({ KEY_ITEMS: items })
}
