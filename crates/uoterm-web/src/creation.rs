//! The making of a new character in the browser: the model of
//! `uoterm_view::model::creation`, as the creation screen of the window
//! drives it, and what the page draws of each page as one plain value
//! ([`CreationScreen`]). The rules and the words are the view's; this only
//! puts them together for the page.
//!
//! The view also asks for what the page fetches, as the play view does:
//! the creation files, the color of each hue, and the pictures of the
//! figure and of the professions. The art feed of the page fetches them.

use crate::to_js;
use crate::web_art::{text_rgb_of, text_rgb_path, Wanted};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use uoterm_nav::Profession;
use uoterm_protocol::ClientVersion;
use uoterm_view::art::ArtRequest;
use uoterm_view::geom::Vector;
use uoterm_view::map_lay::{near_map_path, SPAN};
use uoterm_view::model::creation::{
    card_short, creation_files_path, facet_name, name_rules, paint_words, profession_about,
    profession_name, race_words, skill_rule, stat_rule, total_words, Creation, CreationFiles,
    Paint, Progress, Race, SkillChoice, Stage, Step, SummaryRow, SKILL_RANGE, STAT_RANGE,
    STAT_WORDS, TOWN_MAP_TILES, WORDS_PICK_SKILL,
};
use uoterm_view::model::creation::{preview_scale, CREATION_WORDS};
use uoterm_world::login::{CharacterChoices, NewCharacterWish};
use wasm_bindgen::prelude::*;

/// The profession pictures show in their own colors.
const NO_HUE: u16 = 0;

/// What the page has of a thing it fetches: asked for, here, or not on
/// the server.
enum Fetched<T> {
    Asked,
    Here(T),
    Missing,
}

/// The size of a picture that came, and the point of it that goes on the
/// ground.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShownPicture {
    /// The key the page keeps the pixels under.
    pub key: String,
    pub width: usize,
    pub height: usize,
    pub anchor_x: f32,
    pub anchor_y: f32,
}

/// One part of the creation in the row of steps.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StagePill {
    pub words: &'static str,
    pub progress: Progress,
}

/// The figure of the new character and the words under it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Preview {
    /// None while the picture is on its way, or with no client files.
    pub figure: Option<ShownPicture>,
    pub title: String,
    pub about: String,
}

/// The bottom row: why Next waits or the keys, and the main button.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Footer {
    pub words: String,
    pub blocked: bool,
    pub next: &'static str,
}

/// One choice of a row: its words and whether it is picked.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Chip {
    pub words: String,
    pub picked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RaceChoice {
    pub race: Race,
    pub words: String,
    pub picked: bool,
    /// The account may not make it: it shows faint.
    pub locked: bool,
}

/// The hues of one color of the look, in rows and columns, each as its
/// color once the page has it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ColorGrid {
    pub paint: Paint,
    pub words: &'static str,
    pub rows: usize,
    pub columns: usize,
    pub cells: Vec<Option<[u8; 3]>>,
    pub picked: usize,
}

/// One profession as a card.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Card {
    pub true_name: String,
    pub name: String,
    /// All its words, shown on hover.
    pub about: String,
    pub short: String,
    pub locked: bool,
    pub picked: bool,
    pub picture: Option<ShownPicture>,
}

/// A group of values that add up to a total: the stats or the skills.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PointsGroup {
    pub total: String,
    /// The values add up to their total.
    pub whole: bool,
    pub rule: String,
    pub limit: Option<String>,
    pub least: i32,
    pub most: i32,
    pub rows: Vec<PointsRow>,
}

/// One value of a group: what it is, and its points.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PointsRow {
    pub words: String,
    /// False for a skill row with no skill picked.
    pub picked: bool,
    pub value: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TownRow {
    pub name: String,
    pub building: String,
    pub facet: Option<&'static str>,
    pub picked: bool,
}

/// The land round the start town picked: a picture of [`SPAN`] tiles a
/// side with the town in the middle, of which the page shows `tiles`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TownMap {
    pub path: String,
    pub span: usize,
    pub tiles: f32,
}

/// A box of one color of the look in the summary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Swatch {
    pub words: &'static str,
    pub color: Option<[u8; 3]>,
}

/// The page of the creation that shows, with all it draws.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "step")]
pub enum Page {
    Look {
        female: bool,
        races: Vec<RaceChoice>,
        hair: Vec<Chip>,
        /// None for a woman, or a race with no beard.
        beards: Option<Vec<Chip>>,
        colors: Vec<ColorGrid>,
    },
    Profession {
        cards: Vec<Card>,
        /// What the profession picked gives; empty with none.
        details: Vec<String>,
    },
    Trade {
        stats: PointsGroup,
        skills: PointsGroup,
    },
    Town {
        towns: Vec<TownRow>,
        map: Option<TownMap>,
        words: String,
    },
    Name {
        name: String,
        verdict: String,
        good: bool,
        rules: Vec<String>,
        summary: Vec<SummaryRow>,
        swatches: Vec<Swatch>,
    },
}

/// What the page draws of the creation now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CreationScreen {
    /// The creation files came, or the server has none.
    pub ready: bool,
    pub stages: Vec<StagePill>,
    pub preview: Preview,
    pub footer: Footer,
    pub page: Page,
}

/// The new character, the files it reads, and what the page fetches for
/// it.
#[wasm_bindgen]
pub struct CreationView {
    creation: Creation,
    files_path: String,
    files: Fetched<CreationFiles>,
    pictures: HashMap<u64, Fetched<ShownPicture>>,
    colors: HashMap<u16, Fetched<[u8; 3]>>,
    wanted: Vec<Wanted>,
    data: Vec<String>,
}

impl CreationView {
    /// The creation of a login that speaks `version`, for an account with
    /// `choices`. It asks for the creation files at once.
    pub fn new(version: ClientVersion, choices: CharacterChoices) -> Self {
        let files_path = creation_files_path(&choices.towns);
        Self {
            creation: Creation::new(version, choices),
            data: vec![files_path.clone()],
            files_path,
            files: Fetched::Asked,
            pictures: HashMap::new(),
            colors: HashMap::new(),
            wanted: Vec::new(),
        }
    }

    pub fn creation_mut(&mut self) -> &mut Creation {
        &mut self.creation
    }

    /// The creation files, or none while they are on their way or when
    /// the server has none.
    fn files(&self) -> Option<&CreationFiles> {
        match &self.files {
            Fetched::Here(files) => Some(files),
            Fetched::Asked | Fetched::Missing => None,
        }
    }

    /// Picks the profession of this true name among those the page shows.
    pub fn pick_profession(&mut self, true_name: &str) {
        let Fetched::Here(files) = &self.files else {
            return;
        };
        let picked = self
            .creation
            .professions(files)
            .into_iter()
            .find(|profession| profession.true_name == true_name)
            .cloned();
        if let Some(profession) = picked {
            self.creation.pick_profession(&profession, files);
        }
    }

    /// The skills a row picks from, whose names hold `search`.
    pub fn skill_choices(&self, row: usize, search: &str) -> Vec<SkillChoice> {
        let empty = CreationFiles::default();
        let menu = self.creation.skill_menu(self.files().unwrap_or(&empty));
        self.creation.skill_choices(&menu, row, search)
    }

    /// The new character as the login sends it, in the first free slot of
    /// `names`.
    pub fn wish(&self, names: &[String]) -> NewCharacterWish {
        self.creation.wish(names)
    }

    /// The pictures to post to the art route, each one time.
    pub fn take_wanted(&mut self) -> Vec<Wanted> {
        std::mem::take(&mut self.wanted)
    }

    /// The paths to get, each one time.
    pub fn take_data(&mut self) -> Vec<String> {
        std::mem::take(&mut self.data)
    }

    pub fn picture_arrived(&mut self, key: &str, width: usize, height: usize, anchor: Vector) {
        if let Ok(number) = key.parse() {
            let shown = ShownPicture {
                key: key.to_string(),
                width,
                height,
                anchor_x: anchor.x,
                anchor_y: anchor.y,
            };
            self.pictures.insert(number, Fetched::Here(shown));
        }
    }

    pub fn picture_missing(&mut self, key: &str) {
        if let Ok(number) = key.parse() {
            self.pictures.insert(number, Fetched::Missing);
        }
    }

    /// The answer of a path came. Creation files that do not read count as
    /// none.
    pub fn data_arrived(&mut self, path: &str, answer: &Value) {
        if path == self.files_path {
            self.files =
                serde_json::from_value(answer.clone()).map_or(Fetched::Missing, Fetched::Here);
        } else if let Some(hue) = text_rgb_of(path) {
            let color = serde_json::from_value(answer.clone());
            self.colors
                .insert(hue, color.map_or(Fetched::Missing, Fetched::Here));
        }
    }

    pub fn data_missing(&mut self, path: &str) {
        if path == self.files_path {
            self.files = Fetched::Missing;
        } else if let Some(hue) = text_rgb_of(path) {
            self.colors.insert(hue, Fetched::Missing);
        }
    }

    /// A picture, once it came; asked for the first time.
    fn picture(&mut self, request: &ArtRequest) -> Option<ShownPicture> {
        let key = request.key();
        match self.pictures.get(&key) {
            Some(Fetched::Here(shown)) => Some(shown.clone()),
            Some(Fetched::Asked | Fetched::Missing) => None,
            None => {
                self.pictures.insert(key, Fetched::Asked);
                self.wanted.push(Wanted {
                    key: key.to_string(),
                    request: request.clone(),
                });
                None
            }
        }
    }

    /// The color of words in a hue, once it came; asked for the first
    /// time.
    fn color(&mut self, hue: u16) -> Option<[u8; 3]> {
        match self.colors.get(&hue) {
            Some(Fetched::Here(rgb)) => Some(*rgb),
            Some(Fetched::Asked | Fetched::Missing) => None,
            None => {
                self.colors.insert(hue, Fetched::Asked);
                self.data.push(text_rgb_path(hue));
                None
            }
        }
    }

    /// What the page draws now.
    pub fn screen(&mut self) -> CreationScreen {
        // The files are held apart while the pages ask for their art.
        let fetched = std::mem::replace(&mut self.files, Fetched::Asked);
        let ready = !matches!(fetched, Fetched::Asked);
        let empty = CreationFiles::default();
        let files = match &fetched {
            Fetched::Here(files) => files,
            Fetched::Asked | Fetched::Missing => &empty,
        };
        let now = self.creation.step.stage();
        let (title, about) = self.creation.caption(files);
        let figure_request = self.creation.figure_request();
        let preview = Preview {
            figure: self.picture(&figure_request),
            title,
            about,
        };
        let footer = Footer {
            words: self.creation.footer_words(),
            blocked: self.creation.blocker().is_some(),
            next: self.creation.next_words(),
        };
        let page = match self.creation.step {
            Step::Look => self.look_page(),
            Step::Profession(_) => self.profession_page(files),
            Step::Trade => self.trade_page(files),
            Step::Town => self.town_page(files),
            Step::Name => self.name_page(files),
        };
        self.files = fetched;
        CreationScreen {
            ready,
            stages: Stage::ALL
                .iter()
                .map(|stage| StagePill {
                    words: stage.words(),
                    progress: stage.progress(now),
                })
                .collect(),
            preview,
            footer,
            page,
        }
    }

    fn look_page(&mut self) -> Page {
        let creation = &self.creation;
        let races = creation
            .races_shown()
            .into_iter()
            .map(|race| {
                let allowed = creation.race_allowed(race);
                RaceChoice {
                    race,
                    words: race_words(race, allowed),
                    picked: creation.race == race,
                    locked: !allowed,
                }
            })
            .collect();
        let chips = |styles: &[uoterm_view::model::creation::Style], picked: usize| {
            styles
                .iter()
                .enumerate()
                .map(|(at, style)| Chip {
                    words: style.words.to_string(),
                    picked: at == picked,
                })
                .collect::<Vec<_>>()
        };
        let hair = chips(creation.hair_styles(), creation.hair);
        let beards = creation
            .beard_styles()
            .map(|styles| chips(styles, creation.beard));
        let female = creation.female;
        let race = creation.race;
        let grids: Vec<(Paint, _, usize)> = creation
            .shown_paints()
            .into_iter()
            .map(|paint| (paint, creation.palette(paint), creation.color_place(paint)))
            .collect();
        let colors = grids
            .into_iter()
            .map(|(paint, palette, picked)| ColorGrid {
                paint,
                words: paint_words(paint, race),
                rows: palette.rows,
                columns: palette.columns,
                cells: palette.hues.iter().map(|hue| self.color(*hue)).collect(),
                picked,
            })
            .collect();
        Page::Look {
            female,
            races,
            hair,
            beards,
            colors,
        }
    }

    fn profession_page(&mut self, files: &CreationFiles) -> Page {
        let professions: Vec<Profession> = self
            .creation
            .professions(files)
            .into_iter()
            .cloned()
            .collect();
        let cards = professions
            .iter()
            .map(|profession| {
                let locked = self.creation.profession_locked(profession, files);
                let about = profession_about(profession, files);
                let picked = self
                    .creation
                    .profession
                    .as_ref()
                    .is_some_and(|known| known.true_name == profession.true_name);
                let request = ArtRequest::Gump {
                    gump: profession.gump,
                    hue: NO_HUE,
                    partial: false,
                };
                Card {
                    true_name: profession.true_name.clone(),
                    name: profession_name(profession, files),
                    short: card_short(&about, locked).to_string(),
                    about,
                    locked,
                    picked,
                    picture: self.picture(&request),
                }
            })
            .collect();
        Page::Profession {
            cards,
            details: self
                .creation
                .profession_details(files)
                .map(Vec::from)
                .unwrap_or_default(),
        }
    }

    fn trade_page(&self, files: &CreationFiles) -> Page {
        let creation = &self.creation;
        let stat_sum: i32 = creation.stats.iter().sum();
        let stats = PointsGroup {
            total: total_words(stat_sum, creation.stat_total()),
            whole: stat_sum == creation.stat_total(),
            rule: stat_rule(),
            limit: creation.stat_limit_note(),
            least: STAT_RANGE.0,
            most: STAT_RANGE.1,
            rows: STAT_WORDS
                .iter()
                .zip(creation.stats)
                .map(|(words, value)| PointsRow {
                    words: words.to_string(),
                    picked: true,
                    value,
                })
                .collect(),
        };
        let menu = creation.skill_menu(files);
        let skill_sum: i32 = creation.skills.iter().map(|pick| pick.value).sum();
        let skills = PointsGroup {
            total: total_words(skill_sum, creation.skill_total()),
            whole: skill_sum == creation.skill_total(),
            rule: skill_rule(),
            limit: creation.skill_limit_note(files),
            least: SKILL_RANGE.0,
            most: SKILL_RANGE.1,
            rows: creation
                .skills
                .iter()
                .enumerate()
                .map(|(row, pick)| {
                    let name = creation.skill_row_name(&menu, row);
                    PointsRow {
                        words: name.unwrap_or(WORDS_PICK_SKILL).to_string(),
                        picked: name.is_some(),
                        value: pick.value,
                    }
                })
                .collect(),
        };
        Page::Trade { stats, skills }
    }

    fn town_page(&self, files: &CreationFiles) -> Page {
        let creation = &self.creation;
        let towns = creation
            .towns()
            .iter()
            .enumerate()
            .map(|(at, town)| TownRow {
                name: town.name.clone(),
                building: town.building.clone(),
                facet: town.place.map(|place| facet_name(place.map)),
                picked: creation.town == at,
            })
            .collect();
        let map = creation
            .towns()
            .get(creation.town)
            .and_then(|town| town.place)
            .and_then(|place| {
                Some(TownMap {
                    path: near_map_path(u8::try_from(place.map).ok()?, place.x, place.y),
                    span: SPAN,
                    tiles: TOWN_MAP_TILES,
                })
            });
        Page::Town {
            towns,
            map,
            words: creation.town_about(files),
        }
    }

    fn name_page(&mut self, files: &CreationFiles) -> Page {
        let paints = self.creation.shown_paints();
        let race = self.creation.race;
        let hues: Vec<(Paint, u16)> = paints
            .into_iter()
            .map(|paint| (paint, self.creation.hue(paint)))
            .collect();
        let swatches = hues
            .into_iter()
            .map(|(paint, hue)| Swatch {
                words: paint_words(paint, race),
                color: self.color(hue),
            })
            .collect();
        let creation = &self.creation;
        Page::Name {
            name: creation.name.clone(),
            verdict: creation.name_verdict(),
            good: creation.blocker().is_none(),
            rules: name_rules().into(),
            summary: creation.summary_rows(files).into(),
            swatches,
        }
    }
}

/// The fixed words of the creation screen: `CreationWords`.
#[wasm_bindgen(js_name = creationWords)]
pub fn creation_words() -> JsValue {
    to_js(&CREATION_WORDS)
}

/// A value of the page as a value of the model: a race or a paint by its
/// name, as `screen()` names them.
fn named<T: serde::de::DeserializeOwned>(name: &str) -> Option<T> {
    serde_json::from_value(Value::String(name.to_string())).ok()
}

#[wasm_bindgen]
impl CreationView {
    /// The creation of a login that speaks `version` (`7.0.102.3`), for
    /// the `CharacterChoices` JSON of the character list. A version that
    /// does not read is the newest; choices that do not read are none.
    #[wasm_bindgen(constructor)]
    pub fn create(version: &str, choices_json: &str) -> CreationView {
        CreationView::new(
            version.parse().unwrap_or_default(),
            serde_json::from_str(choices_json).unwrap_or_default(),
        )
    }

    /// What the page draws now: `CreationScreen`. It asks for the art it
    /// names that the page has not fetched yet.
    #[wasm_bindgen(js_name = screen)]
    pub fn screen_js(&mut self) -> JsValue {
        to_js(&self.screen())
    }

    #[wasm_bindgen(js_name = setFemale)]
    pub fn set_female(&mut self, female: bool) {
        self.creation.set_female(female);
    }

    /// `race` is a race of the screen: `Human`, `Elf` or `Gargoyle`.
    #[wasm_bindgen(js_name = setRace)]
    pub fn set_race(&mut self, race: &str) {
        if let Some(race) = named(race) {
            self.creation.set_race(race);
        }
    }

    /// The hair style at `at` of the list of the look page.
    #[wasm_bindgen(js_name = setHair)]
    pub fn set_hair(&mut self, at: usize) {
        if at < self.creation.hair_styles().len() {
            self.creation.hair = at;
        }
    }

    /// The beard style at `at` of the list of the look page.
    #[wasm_bindgen(js_name = setBeard)]
    pub fn set_beard(&mut self, at: usize) {
        if self
            .creation
            .beard_styles()
            .is_some_and(|styles| at < styles.len())
        {
            self.creation.beard = at;
        }
    }

    /// The hue at `at` of the palette of `paint` (`Skin`, `Shirt`, ...).
    #[wasm_bindgen(js_name = setColor)]
    pub fn set_color(&mut self, paint: &str, at: usize) {
        if let Some(paint) = named(paint) {
            self.creation.set_color(paint, at);
        }
    }

    /// Picks the card of this true name.
    #[wasm_bindgen(js_name = pickProfession)]
    pub fn pick_profession_js(&mut self, true_name: &str) {
        self.pick_profession(true_name);
    }

    /// Moves the stat at `at` to `value`; the others make up the change.
    #[wasm_bindgen(js_name = setStat)]
    pub fn set_stat(&mut self, at: usize, value: i32) {
        self.creation.set_stat(at, value);
    }

    #[wasm_bindgen(js_name = setSkill)]
    pub fn set_skill(&mut self, row: usize, skill: u8) {
        self.creation.set_skill(row, skill);
    }

    /// Moves the points of the skill row `row` to `value`; the others make
    /// up the change.
    #[wasm_bindgen(js_name = setSkillValue)]
    pub fn set_skill_value(&mut self, row: usize, value: i32) {
        self.creation.set_skill_value(row, value);
    }

    /// The skills row `row` picks from whose names hold `search`:
    /// `SkillChoice[]`.
    #[wasm_bindgen(js_name = skillChoices)]
    pub fn skill_choices_js(&self, row: usize, search: &str) -> JsValue {
        to_js(&self.skill_choices(row, search))
    }

    #[wasm_bindgen(js_name = setTown)]
    pub fn set_town(&mut self, at: usize) {
        self.creation.set_town(at);
    }

    #[wasm_bindgen(js_name = setName)]
    pub fn set_name(&mut self, name: &str) {
        self.creation.name = name.to_string();
    }

    /// Turns the figure one eighth to the right, or to the left.
    pub fn turn(&mut self, right: bool) {
        self.creation.turn(right);
    }

    /// One page on, when the page allows it. True when the character is
    /// made and goes to the shard.
    #[wasm_bindgen(js_name = next)]
    pub fn next_page(&mut self) -> bool {
        self.creation.next_page()
    }

    /// One page back. True when the player left the creation.
    #[wasm_bindgen(js_name = back)]
    pub fn back_page(&mut self) -> bool {
        self.creation.back()
    }

    /// The `NewCharacterWish` of the character, in the first free slot of
    /// the names of the character list (JSON).
    #[wasm_bindgen(js_name = wish)]
    pub fn wish_js(&self, names_json: &str) -> JsValue {
        let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
        to_js(&self.wish(&names))
    }

    /// The scale the figure of `art_width` by `art_height` pixels takes in
    /// a box of `room_width` by `room_height`: whole steps, so the pixels
    /// stay sharp.
    #[wasm_bindgen(js_name = previewScale)]
    pub fn preview_scale_js(
        &self,
        room_width: f32,
        room_height: f32,
        art_width: f32,
        art_height: f32,
    ) -> f32 {
        preview_scale(
            Vector::new(room_width, room_height),
            Vector::new(art_width, art_height),
        )
    }

    /// The pictures to post to `/v1/art`, each once: `{key, request}[]`.
    #[wasm_bindgen(js_name = artWanted)]
    pub fn art_wanted(&mut self) -> JsValue {
        to_js(&self.take_wanted())
    }

    #[wasm_bindgen(js_name = artArrived)]
    pub fn art_arrived(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        anchor_x: f32,
        anchor_y: f32,
    ) {
        self.picture_arrived(
            key,
            width as usize,
            height as usize,
            Vector::new(anchor_x, anchor_y),
        );
    }

    #[wasm_bindgen(js_name = artMissing)]
    pub fn art_missing(&mut self, key: &str) {
        self.picture_missing(key);
    }

    /// The creation forgets no picture: `[]`.
    #[wasm_bindgen(js_name = artForgotten)]
    pub fn art_forgotten(&mut self) -> JsValue {
        to_js(&Vec::<String>::new())
    }

    /// The paths to get, each once: the creation files and the hue colors.
    #[wasm_bindgen(js_name = dataWanted)]
    pub fn data_wanted(&mut self) -> JsValue {
        to_js(&self.take_data())
    }

    #[wasm_bindgen(js_name = dataArrived)]
    pub fn data_arrived_js(&mut self, path: &str, json: &str) {
        match serde_json::from_str::<Value>(json) {
            Ok(answer) => self.data_arrived(path, &answer),
            Err(_) => self.data_missing(path),
        }
    }

    #[wasm_bindgen(js_name = dataMissing)]
    pub fn data_missing_js(&mut self, path: &str) {
        self.data_missing(path);
    }

    /// The creation posts nothing: `[]`.
    #[wasm_bindgen(js_name = postsWanted)]
    pub fn posts_wanted(&mut self) -> JsValue {
        to_js(&Vec::<String>::new())
    }

    /// The creation posts nothing, so no answer comes.
    #[wasm_bindgen(js_name = postArrived)]
    pub fn post_arrived(&mut self, _key: &str, _json: &str) {}

    /// The creation posts nothing, so no answer fails.
    #[wasm_bindgen(js_name = postMissing)]
    pub fn post_missing(&mut self, _key: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uoterm_view::model::creation::{sample_choices, NameFault};

    fn sample_view() -> CreationView {
        let mut view = CreationView::new(ClientVersion::MODERN, sample_choices());
        let files = serde_json::to_value(CreationFiles::sample()).unwrap();
        let path = view.take_data().remove(0);
        view.data_arrived(&path, &files);
        view
    }

    #[test]
    fn it_asks_for_the_creation_files_of_its_towns_first_and_waits_for_them() {
        let mut view = CreationView::new(ClientVersion::MODERN, sample_choices());
        assert_eq!(
            view.take_data(),
            ["/v1/data/creation?towns=0,0,1075074".to_string()]
        );
        assert!(!view.screen().ready);
        view.data_missing("/v1/data/creation?towns=0,0,1075074");
        assert!(view.screen().ready, "no client files: no professions");
    }

    #[test]
    fn the_look_page_asks_for_the_figure_and_each_hue_once() {
        let mut view = sample_view();
        let screen = view.screen();
        assert!(screen.ready);
        assert_eq!(screen.stages[0].progress, Progress::Current);
        assert_eq!(screen.preview.figure, None);
        let wanted = view.take_wanted();
        assert_eq!(wanted.len(), 1, "the figure");
        let hues = view.take_data();
        assert!(hues
            .iter()
            .all(|path| path.starts_with("/v1/data/hues-text/")));
        assert!(!hues.is_empty());
        view.screen();
        assert!(view.take_wanted().is_empty() && view.take_data().is_empty());
        view.picture_arrived(&wanted[0].key, 40, 70, Vector::new(20.0, 66.0));
        view.data_arrived(&hues[0], &json!([10, 20, 30]));
        let screen = view.screen();
        assert_eq!(screen.preview.figure.unwrap().width, 40);
        let Page::Look { colors, races, .. } = screen.page else {
            panic!("the look page");
        };
        assert_eq!(colors[0].paint, Paint::Skin);
        assert_eq!(colors[0].cells[0], Some([10, 20, 30]));
        assert!(races
            .iter()
            .any(|race| race.race == Race::Elf && !race.locked));
    }

    #[test]
    fn a_card_picks_its_profession_by_its_true_name() {
        let mut view = sample_view();
        view.creation_mut().next_page();
        view.pick_profession("warrior");
        let Page::Profession { cards, details } = view.screen().page else {
            panic!("the profession page");
        };
        assert!(cards[0].picked && cards[0].name == "Warrior");
        assert!(!cards[1].locked, "the account has the Samurai Empire flag");
        assert_eq!(details.len(), 2);
        let gumps = view
            .take_wanted()
            .into_iter()
            .filter(|want| matches!(want.request, ArtRequest::Gump { .. }))
            .count();
        assert!(gumps > 0, "the cards ask for their pictures");
    }

    #[test]
    fn the_custom_page_tells_its_totals_and_names_its_rows() {
        let mut view = sample_view();
        view.creation_mut().next_page();
        view.pick_profession("advanced");
        view.creation_mut().next_page();
        let Page::Trade { stats, skills } = view.screen().page else {
            panic!("the custom page");
        };
        assert_eq!(stats.total, "Total 90 of 90");
        assert!(stats.whole);
        assert_eq!(stats.rows[0].words, "Strength");
        assert_eq!(skills.rows[0].words, WORDS_PICK_SKILL);
        assert!(!skills.rows[0].picked);
        assert_eq!((skills.least, skills.most), SKILL_RANGE);
        assert_eq!(view.skill_choices(0, "tact")[0].words, "Tactics");
    }

    #[test]
    fn the_town_page_shows_the_land_round_a_placed_town() {
        let mut view = sample_view();
        view.creation_mut().step = Step::Town;
        view.creation_mut().set_town(2);
        let Page::Town { towns, map, .. } = view.screen().page else {
            panic!("the town page");
        };
        assert!(towns[2].picked);
        assert_eq!(towns[2].facet, Some("Trammel"));
        assert_eq!(map.unwrap().path, "/v1/map/near/1/1496/1628");
    }

    #[test]
    fn the_name_page_tells_why_a_name_waits_and_create_sends_the_wish() {
        let mut view = sample_view();
        view.creation_mut().step = Step::Name;
        view.creation_mut().name = "M".into();
        let screen = view.screen();
        assert!(screen.footer.blocked);
        assert_eq!(screen.footer.words, NameFault::Short.words());
        let Page::Name { verdict, good, .. } = screen.page else {
            panic!("the name page");
        };
        assert_eq!((verdict, good), (NameFault::Short.words(), false));
        view.creation_mut().name = "Mara".into();
        assert!(view.creation_mut().next_page());
        let wish = view.wish(&["Old".into()]);
        assert_eq!((wish.name.as_str(), wish.slot), ("Mara", 1));
    }

    #[test]
    fn the_screen_goes_to_the_page_as_tagged_json() {
        let mut view = sample_view();
        let value = serde_json::to_value(view.screen()).unwrap();
        assert_eq!(value["page"]["step"], "Look");
        assert_eq!(value["stages"][0]["progress"], "Current");
        assert_eq!(value["page"]["races"][0]["race"], "Human");
    }
}
