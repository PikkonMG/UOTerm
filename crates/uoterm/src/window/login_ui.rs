//! The screens before the game: the login form, the list of shards, the
//! list of characters and the making of a new character. `uoterm play`
//! starts here. When the character is in the world, the window becomes the
//! game window.
//!
//! The screens are the same plain panel in each UI style. The login itself
//! is one flow ([`LoginFlow`]), apart from its drawing. A new character is
//! made in full on the model of `model::creation`, on a screen of its own
//! that fills the window (see `creation_ui`).
//!
//! The password is typed in a field that hides it. It stays in memory for
//! the login and is written nowhere. A saved login can name an environment
//! variable that holds the password; then the field can stay empty.
//!
//! With a TypeSafe key, one field takes plain words,
//! such as "my miner on the test shard". Jev picks the saved login, and
//! later the character, from the lists. Jev sees the words and the names of
//! the lists. It never sees the password.

use super::creation_ui::{self, Art, Asked as CreationAsked};
use super::link::Link;
use super::map_view::MapPictures;
use super::model::creation::{can_make, Creation, CreationFiles};
use super::orders;
use super::scene::Scene;
use super::theme::{self, text_font, title_font};
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use uoterm_protocol::ClientVersion;
use uoterm_runtime::{CharacterChoices, CharacterRequest, LoginPicker, LoginQuestion};

const PANEL_SIZE: Vec2 = Vec2::new(760.0, 470.0);
const LIST_WIDTH: f32 = 230.0;
const ROW: f32 = 34.0;
const FIELD_ROW: f32 = 32.0;
const LABEL_WIDTH: f32 = 110.0;
const GAP: f32 = 10.0;
const TITLE_ROW: f32 = 44.0;
const FIELD_RADIUS: u8 = 6;
const LIST_ROWS: usize = 9;
const REPAINT_WHILE_WAITING_MS: u64 = 100;

const WORDS_TITLE: &str = "UOTerm";
const WORDS_SAVED: &str = "Saved logins";
const WORDS_NO_SAVED: &str = "No saved logins. Put one in the profiles folder.";
const WORDS_CONNECT: &str = "Connect";
const WORDS_CONNECTING: &str = "Connecting...";
const WORDS_PICK_SHARD: &str = "Pick a shard";
const WORDS_PICK_CHARACTER: &str = "Pick a character";
const WORDS_MAKE: &str = "New character";
const WORDS_DELETE: &str = "Delete";
const WORDS_DELETE_SURE: &str = "Delete?";
const WORDS_EMPTY_SLOT: &str = "(empty)";
const WORDS_NO_ROOM: &str = "The account has no room for another character.";
const CHARACTER_SLOTS: usize = 7;
const WORDS_FIND: &str = "Find";
const WORDS_ASKING: &str = "Jev looks at the list...";
const WORDS_NOT_SURE: &str = "Jev is not sure which one you mean. Click one.";
const HINT_WISH: &str = "Say who you want to play, for example: my miner on the test shard";
const HINT_WISH_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
const HINT_PASSWORD_ENV: &str = "From the environment when empty";
const LABELS: [&str; 6] = ["Host", "Port", "Account", "Password", "Shard", "Character"];
const PASSWORD_FIELD: usize = 3;
const FIND_WIDTH: f32 = 70.0;
const DELETE_WIDTH: f32 = 70.0;

/// What the human typed in the login form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoginForm {
    pub host: String,
    pub port: String,
    pub account: String,
    pub password: String,
    pub shard: String,
    pub character: String,
    /// The saved login the form came from. It gives the era, the version
    /// and the environment variable of the password.
    pub profile: Option<String>,
}

/// One saved login: its name and what it puts in the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedLogin {
    pub name: String,
    pub account: String,
    pub character: String,
    pub shard: String,
    /// The client version the login speaks, which a new character follows.
    pub version: ClientVersion,
}

impl SavedLogin {
    /// The words Jev reads to tell one saved login from another.
    fn words(&self) -> String {
        format!(
            "{}: character {}, shard {}",
            self.name, self.character, self.shard
        )
    }
}

/// Logs in with the form. It blocks until the character is in the world, so
/// the screen calls it on a thread of its own. It gives the link to the new
/// session, or words for the human.
pub type Connect = Arc<dyn Fn(LoginForm, LoginPicker) -> Result<Link, String> + Send + Sync>;

/// Which list the login waits for a pick from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickOf {
    Shard,
    Character,
}

enum Stage {
    Form,
    Connecting,
    /// The login waits for a pick from this list.
    Picking {
        of: PickOf,
        names: Vec<String>,
        reply: tokio::sync::oneshot::Sender<usize>,
    },
    /// The login waits for what to do with the characters of the account.
    Characters {
        names: Vec<String>,
        choices: CharacterChoices,
        reply: tokio::sync::oneshot::Sender<CharacterRequest>,
    },
}

/// What a stage shows, apart from the answer it waits to send.
enum Shown {
    Form,
    Connecting,
    Picking(PickOf, Vec<String>),
    Characters(Vec<String>),
}

impl Stage {
    fn shown(&self) -> Shown {
        match self {
            Stage::Form => Shown::Form,
            Stage::Connecting => Shown::Connecting,
            Stage::Picking { of, names, .. } => Shown::Picking(*of, names.clone()),
            Stage::Characters { names, .. } => Shown::Characters(names.clone()),
        }
    }
}

/// What Jev was asked about, so the answer goes to the right list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    SavedLogin,
    Pick,
}

/// The login as the screens drive it, with no drawing: the form, the
/// questions of the shard, and the answers the screens give.
struct LoginFlow {
    form: LoginForm,
    saved: Vec<SavedLogin>,
    connect: Connect,
    stage: Stage,
    questions: Option<tokio::sync::mpsc::UnboundedReceiver<LoginQuestion>>,
    done: Option<Receiver<Result<Link, String>>>,
    wish: String,
    jev_key: Option<String>,
    jev_asks: Sender<(Asked, Result<Option<usize>, String>)>,
    jev_answers: Receiver<(Asked, Result<Option<usize>, String>)>,
    /// Words for the human: a fault of the login, or what Jev does.
    note: Option<(String, bool)>,
    /// The first frame starts the login. A fault brings the form back.
    connect_at_once: bool,
    /// The slot waiting to be deleted once the human says yes.
    delete_asked: Option<usize>,
    /// The character the human played, whose name answers the question of
    /// the login which character to play.
    played: Option<String>,
    /// The new character the human makes, while he makes one.
    creating: Option<Creation>,
    files: CreationFiles,
    /// The version of a login with no saved login.
    version: ClientVersion,
}

/// The form with a saved login put in. The host, the port and the password
/// stay as they are.
fn with_saved(form: &LoginForm, saved: &SavedLogin) -> LoginForm {
    LoginForm {
        account: saved.account.clone(),
        character: saved.character.clone(),
        shard: saved.shard.clone(),
        profile: Some(saved.name.clone()),
        ..form.clone()
    }
}

impl LoginFlow {
    fn new(start: LoginStart) -> Self {
        let (jev_asks, jev_answers) = mpsc::channel();
        Self {
            form: start.form,
            saved: start.saved,
            connect: start.connect,
            stage: Stage::Form,
            questions: None,
            done: None,
            wish: String::new(),
            jev_key: orders::api_key(),
            jev_asks,
            jev_answers,
            note: None,
            connect_at_once: start.connect_at_once,
            delete_asked: None,
            played: None,
            creating: None,
            files: CreationFiles::read(start.uopath),
            version: start.version,
        }
    }

    /// The client version of the login: the one of its saved login.
    fn version(&self) -> ClientVersion {
        self.form
            .profile
            .as_deref()
            .and_then(|name| self.saved.iter().find(|saved| saved.name == name))
            .map_or(self.version, |saved| saved.version)
    }

    /// Starts the login with the form.
    fn start(&mut self, ctx: &egui::Context) {
        let (ask, questions) = tokio::sync::mpsc::unbounded_channel();
        let (tell, done) = mpsc::channel();
        let (connect, form, ctx) = (Arc::clone(&self.connect), self.form.clone(), ctx.clone());
        thread::spawn(move || {
            let _ = tell.send(connect(form, LoginPicker(ask)));
            ctx.request_repaint();
        });
        self.questions = Some(questions);
        self.done = Some(done);
        self.stage = Stage::Connecting;
        self.note = None;
        self.played = None;
    }

    /// Asks Jev which of `options` the wish names. The answer comes back
    /// through the channel, so the screen never waits.
    fn ask_jev(
        &mut self,
        asked: Asked,
        ask: &'static str,
        options: Vec<String>,
        ctx: &egui::Context,
    ) {
        let Some(key) = self.jev_key.clone() else {
            return;
        };
        let (wish, tell, ctx) = (
            self.wish.trim().to_string(),
            self.jev_asks.clone(),
            ctx.clone(),
        );
        self.note = Some((WORDS_ASKING.into(), false));
        thread::spawn(move || {
            let answer = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
                .and_then(|rt| {
                    let options: Vec<&str> = options.iter().map(String::as_str).collect();
                    rt.block_on(orders::pick(&key, ask, &wish, &options))
                });
            let _ = tell.send((asked, answer));
            ctx.request_repaint();
        });
    }

    fn take_jev_answers(&mut self) {
        for (asked, answer) in self.jev_answers.try_iter().collect::<Vec<_>>() {
            match (asked, answer) {
                (_, Err(words)) => self.note = Some((words, true)),
                (_, Ok(None)) => self.note = Some((WORDS_NOT_SURE.into(), true)),
                (Asked::SavedLogin, Ok(Some(place))) => {
                    if let Some(saved) = self.saved.get(place) {
                        self.form = with_saved(&self.form, saved);
                        self.note = None;
                    }
                }
                (Asked::Pick, Ok(Some(place))) => {
                    self.note = None;
                    self.answer_pick(place);
                }
            }
        }
    }

    /// Sends what the human wants done with the characters, and goes back
    /// to waiting for the shard.
    fn answer_request(&mut self, request: CharacterRequest) {
        if let Stage::Characters { reply, .. } =
            std::mem::replace(&mut self.stage, Stage::Connecting)
        {
            self.creating = None;
            self.delete_asked = None;
            let _ = reply.send(request);
        }
    }

    /// Plays the character in a slot of the list.
    fn play(&mut self, slot: usize) {
        let Stage::Characters { names, .. } = &self.stage else {
            return;
        };
        if let Some(name) = names.get(slot).filter(|name| !name.is_empty()) {
            self.played = Some(name.clone());
            self.answer_request(CharacterRequest::Play(slot));
        }
    }

    /// Opens the making of a new character, when the account has room.
    fn begin_creation(&mut self) {
        let Stage::Characters { names, choices, .. } = &self.stage else {
            return;
        };
        if can_make(names, choices.list_flags) {
            self.creating = Some(Creation::new(self.version(), choices.clone()));
        } else {
            self.note = Some((WORDS_NO_ROOM.into(), true));
        }
    }

    /// Sends the new character to the shard.
    fn finish_creation(&mut self) {
        let Stage::Characters { names, .. } = &self.stage else {
            return;
        };
        if let Some(creation) = self.creating.as_ref() {
            let wish = creation.wish(names);
            self.answer_request(CharacterRequest::Make(Box::new(wish)));
        }
    }

    fn answer_pick(&mut self, place: usize) {
        if let Stage::Picking { reply, .. } = std::mem::replace(&mut self.stage, Stage::Connecting)
        {
            let _ = reply.send(place);
        }
    }

    fn take_questions(&mut self, ctx: &egui::Context) {
        let question = self.questions.as_mut().and_then(|q| q.try_recv().ok());
        let (of, ask, names, reply) = match question {
            Some(LoginQuestion::Shard { names, reply }) => {
                (PickOf::Shard, orders::ASK_SHARD, names, reply)
            }
            Some(LoginQuestion::Character { names, reply }) => {
                (PickOf::Character, orders::ASK_CHARACTER, names, reply)
            }
            Some(LoginQuestion::Characters {
                names,
                refused,
                choices,
                reply,
            }) => {
                if let Some(words) = refused {
                    self.note = Some((words, true));
                }
                self.delete_asked = None;
                self.creating = None;
                self.stage = Stage::Characters {
                    names,
                    choices,
                    reply,
                };
                return;
            }
            None => return,
        };
        // The character the human played in the list answers at once.
        let played = self
            .played
            .as_ref()
            .filter(|_| of == PickOf::Character)
            .and_then(|name| names.iter().position(|known| known == name));
        self.stage = Stage::Picking { of, names, reply };
        if let Some(place) = played {
            self.answer_pick(place);
            return;
        }
        // The wish may name the pick already. Jev answers, or the human clicks.
        if !self.wish.trim().is_empty() {
            if let Stage::Picking { names, .. } = &self.stage {
                let names = names.clone();
                self.ask_jev(Asked::Pick, ask, names, ctx);
            }
        }
    }

    /// Follows the login for one frame. Gives the link when the character
    /// is in the world.
    fn follow(&mut self, ctx: &egui::Context) -> Option<Link> {
        if std::mem::take(&mut self.connect_at_once) {
            self.start(ctx);
        }
        self.take_jev_answers();
        self.take_questions(ctx);
        match self.done.as_ref().and_then(|done| done.try_recv().ok()) {
            Some(Ok(link)) => return Some(link),
            Some(Err(words)) => {
                self.stage = Stage::Form;
                self.done = None;
                self.questions = None;
                self.creating = None;
                self.note = Some((words, true));
            }
            None => {}
        }
        if matches!(self.stage, Stage::Connecting) {
            ctx.request_repaint_after(std::time::Duration::from_millis(REPAINT_WHILE_WAITING_MS));
        }
        None
    }

    /// The login server of the form, as the profiles name a shard.
    fn shard_address(&self) -> String {
        super::settings::shard_address(&self.form.host, self.form.port.trim())
    }
}

/// What the login screens start with.
pub struct LoginStart<'a> {
    pub form: LoginForm,
    pub saved: Vec<SavedLogin>,
    pub connect: Connect,
    /// Log in with the form as it is, with no click on Connect.
    pub connect_at_once: bool,
    /// The client version of a login with no saved login.
    pub version: ClientVersion,
    pub uopath: Option<&'a Path>,
}

pub struct LoginUi {
    flow: LoginFlow,
    /// The client art the figure of a new character is drawn with. None
    /// with no client files.
    scene: Option<Scene>,
    /// The land round the start town a new character picks.
    town_map: MapPictures,
}

impl LoginUi {
    pub fn new(start: LoginStart<'_>) -> Self {
        Self {
            scene: start.uopath.map(|dir| Scene::new(Some(dir))),
            town_map: MapPictures::default(),
            flow: LoginFlow::new(start),
        }
    }

    /// The login server of the form, as the profiles name a shard.
    pub fn shard_address(&self) -> String {
        self.flow.shard_address()
    }

    /// Draws the screen for this frame. Gives the link when the character
    /// is in the world.
    pub fn draw(&mut self, ui: &mut egui::Ui, rect: Rect) -> Option<Link> {
        let ctx = ui.ctx().clone();
        if let Some(link) = self.flow.follow(&ctx) {
            // The game window loads its own art.
            self.scene = None;
            return Some(link);
        }
        if let Some(scene) = self.scene.as_mut() {
            scene.make_atlas(&ctx);
        }
        self.draw_panel(ui, rect, &ctx);
        None
    }

    fn draw_panel(&mut self, ui: &mut egui::Ui, rect: Rect, ctx: &egui::Context) {
        let flow = &mut self.flow;
        if let (Stage::Characters { .. }, Some(creation)) = (&flow.stage, flow.creating.as_mut()) {
            let art = Art {
                scene: self.scene.as_mut(),
                town_map: &mut self.town_map,
            };
            match creation_ui::draw(ui, rect, creation, &flow.files, art) {
                Some(CreationAsked::Leave) => flow.creating = None,
                Some(CreationAsked::Finish) => flow.finish_creation(),
                None => {}
            }
            return;
        }
        ui.painter().rect_filled(rect, 0.0, theme::VOID);
        let panel = Rect::from_center_size(rect.center(), PANEL_SIZE);
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD * 1.5);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_TITLE,
            title_font(theme::SIZE_TITLE * 1.4),
            theme::TEXT,
        );
        let body = Rect::from_min_max(inner.left_top() + Vec2::new(0.0, TITLE_ROW), inner.max);
        match flow.stage.shown() {
            Shown::Form => form_stage(flow, ui, body, ctx),
            Shown::Connecting => {
                ui.painter().text(
                    body.center(),
                    Align2::CENTER_CENTER,
                    WORDS_CONNECTING,
                    text_font(theme::SIZE_TITLE),
                    theme::WAITING,
                );
            }
            Shown::Picking(of, names) => {
                let title = match of {
                    PickOf::Shard => WORDS_PICK_SHARD,
                    PickOf::Character => WORDS_PICK_CHARACTER,
                };
                if let Some(place) = pick_list(ui, body, title, &names) {
                    flow.answer_pick(place);
                }
            }
            Shown::Characters(names) => character_stage(flow, ui, body, &names),
        }
        if let Some((words, failed)) = &flow.note {
            let color = if *failed {
                theme::ALARM
            } else {
                theme::WAITING
            };
            ui.painter().text(
                Pos2::new(panel.center().x, panel.bottom() + GAP),
                Align2::CENTER_TOP,
                words,
                text_font(theme::SIZE_BODY),
                color,
            );
        }
    }
}

/// The characters of the account: play one, delete one, or make one.
fn character_stage(flow: &mut LoginFlow, ui: &mut egui::Ui, body: Rect, names: &[String]) {
    ui.painter().text(
        body.center_top(),
        Align2::CENTER_TOP,
        WORDS_PICK_CHARACTER,
        text_font(theme::SIZE_TITLE),
        theme::TEXT,
    );
    let width = LIST_WIDTH * 1.5;
    let left = body.center().x - width / 2.0;
    for slot in 0..CHARACTER_SLOTS {
        let name = names.get(slot).filter(|name| !name.is_empty());
        let row = Rect::from_min_size(
            Pos2::new(left, body.top() + TITLE_ROW + slot as f32 * ROW),
            Vec2::new(width, ROW - theme::ROW_GAP / 2.0),
        );
        let words = name.map_or(WORDS_EMPTY_SLOT, |name| name.as_str());
        if list_row(ui, row, ("character-slot", slot), words, false) && name.is_some() {
            flow.play(slot);
            return;
        }
        if name.is_none() {
            continue;
        }
        let asked = flow.delete_asked == Some(slot);
        let cross = Rect::from_min_size(
            Pos2::new(row.right() + GAP, row.top()),
            Vec2::new(DELETE_WIDTH, row.height()),
        );
        let words = if asked {
            WORDS_DELETE_SURE
        } else {
            WORDS_DELETE
        };
        if theme::segment(ui, cross, words, theme::ALARM) {
            if asked {
                flow.answer_request(CharacterRequest::Delete(slot));
                return;
            }
            flow.delete_asked = Some(slot);
        }
    }
    let (_, make) = theme::button(
        ui,
        Pos2::new(
            left,
            body.top() + TITLE_ROW + CHARACTER_SLOTS as f32 * ROW + GAP,
        ),
        WORDS_MAKE,
        theme::GOAL,
    );
    if make {
        flow.begin_creation();
    }
}

fn form_stage(flow: &mut LoginFlow, ui: &mut egui::Ui, body: Rect, ctx: &egui::Context) {
    let list = Rect::from_min_size(body.min, Vec2::new(LIST_WIDTH, body.height()));
    ui.painter().text(
        list.left_top(),
        Align2::LEFT_TOP,
        WORDS_SAVED,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
    if flow.saved.is_empty() {
        let mut job = egui::text::LayoutJob::single_section(
            WORDS_NO_SAVED.into(),
            egui::TextFormat::simple(text_font(theme::SIZE_SMALL), theme::TEXT_FAINT),
        );
        job.wrap.max_width = LIST_WIDTH;
        let galley = ui.painter().layout_job(job);
        ui.painter().galley(
            list.left_top() + Vec2::new(0.0, ROW),
            galley,
            theme::TEXT_FAINT,
        );
    }
    let mut picked = None;
    for (i, saved) in flow.saved.iter().take(LIST_ROWS).enumerate() {
        let row = Rect::from_min_size(
            list.left_top() + Vec2::new(0.0, ROW * (i + 1) as f32),
            Vec2::new(LIST_WIDTH, ROW - theme::ROW_GAP / 2.0),
        );
        let chosen = flow.form.profile.as_deref() == Some(saved.name.as_str());
        if list_row(ui, row, ("saved-login", i), &saved.name, chosen) {
            picked = Some(i);
        }
    }
    if let Some(saved) = picked.and_then(|i| flow.saved.get(i)) {
        flow.form = with_saved(&flow.form, saved);
    }
    let right = Rect::from_min_max(Pos2::new(list.right() + GAP * 2.0, body.top()), body.max);
    let margin = egui::Margin::symmetric(8, 6);
    let fields: [&mut String; 6] = [
        &mut flow.form.host,
        &mut flow.form.port,
        &mut flow.form.account,
        &mut flow.form.password,
        &mut flow.form.shard,
        &mut flow.form.character,
    ];
    let mut enter = false;
    for (i, (label, value)) in LABELS.into_iter().zip(fields).enumerate() {
        let row = Rect::from_min_size(
            right.left_top() + Vec2::new(0.0, i as f32 * (FIELD_ROW + GAP)),
            Vec2::new(right.width(), FIELD_ROW),
        );
        ui.painter().text(
            row.left_center(),
            Align2::LEFT_CENTER,
            label,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let field = Rect::from_min_max(
            Pos2::new(row.left() + LABEL_WIDTH, row.top()),
            row.right_bottom(),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(FIELD_RADIUS), theme::TRACK);
        let secret = i == PASSWORD_FIELD;
        let edit = egui::TextEdit::singleline(value)
            .frame(false)
            .margin(margin)
            .password(secret)
            .hint_text(if secret { HINT_PASSWORD_ENV } else { "" })
            .font(text_font(theme::SIZE_BODY))
            .text_color(theme::TEXT);
        let response = ui.put(field, edit);
        enter |= response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    }
    let wish_row = Rect::from_min_size(
        right.left_top() + Vec2::new(0.0, LABELS.len() as f32 * (FIELD_ROW + GAP) + GAP),
        Vec2::new(right.width(), FIELD_ROW),
    );
    let jev_on = flow.jev_key.is_some();
    let wish_field = Rect::from_min_max(
        wish_row.min,
        Pos2::new(wish_row.right() - FIND_WIDTH - GAP, wish_row.bottom()),
    );
    ui.painter()
        .rect_filled(wish_field, CornerRadius::same(FIELD_RADIUS), theme::TRACK);
    let wish = ui.put(
        wish_field,
        egui::TextEdit::singleline(&mut flow.wish)
            .frame(false)
            .margin(margin)
            .hint_text(if jev_on { HINT_WISH } else { HINT_WISH_OFF })
            .font(text_font(theme::SIZE_BODY))
            .text_color(theme::TEXT),
    );
    let (_, find) = theme::button(
        ui,
        Pos2::new(wish_field.right() + GAP, wish_row.top()),
        WORDS_FIND,
        if jev_on {
            theme::GOAL
        } else {
            theme::TEXT_FAINT
        },
    );
    let wished = find || (wish.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
    if wished && jev_on && !flow.wish.trim().is_empty() && !flow.saved.is_empty() {
        let options = flow.saved.iter().map(SavedLogin::words).collect();
        flow.ask_jev(Asked::SavedLogin, orders::ASK_PROFILE, options, ctx);
    }
    let (_, connect) = theme::button(
        ui,
        Pos2::new(right.left() + LABEL_WIDTH, right.bottom() - FIELD_ROW),
        WORDS_CONNECT,
        theme::GOAL,
    );
    if connect || enter {
        flow.start(ctx);
    }
}

/// One row of a list. True when it was clicked.
fn list_row(
    ui: &egui::Ui,
    row: Rect,
    key: (&'static str, usize),
    words: &str,
    chosen: bool,
) -> bool {
    let response = ui.interact(row, Id::new(key), Sense::click());
    let fill = if chosen || response.hovered() {
        theme::BUTTON_HOVER
    } else {
        theme::BUTTON
    };
    ui.painter()
        .rect_filled(row, CornerRadius::same(FIELD_RADIUS), fill);
    ui.painter().text(
        row.left_center() + Vec2::new(GAP, 0.0),
        Align2::LEFT_CENTER,
        words,
        text_font(theme::SIZE_BODY),
        theme::TEXT,
    );
    response.clicked()
}

/// A list of shards or of characters. Gives the place that was clicked.
fn pick_list(ui: &egui::Ui, body: Rect, title: &str, names: &[String]) -> Option<usize> {
    ui.painter().text(
        body.center_top(),
        Align2::CENTER_TOP,
        title,
        text_font(theme::SIZE_TITLE),
        theme::TEXT,
    );
    let width = LIST_WIDTH * 1.5;
    names.iter().enumerate().find_map(|(i, name)| {
        let row = Rect::from_min_size(
            Pos2::new(
                body.center().x - width / 2.0,
                body.top() + TITLE_ROW + i as f32 * ROW,
            ),
            Vec2::new(width, ROW - theme::ROW_GAP / 2.0),
        );
        list_row(ui, row, ("login-pick", i), name, false).then_some(i)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD_VERSION: ClientVersion = ClientVersion::new(5, 0, 9, 1);

    fn cedric() -> SavedLogin {
        SavedLogin {
            name: "cedric".into(),
            account: "acct2".into(),
            character: "Cedric".into(),
            shard: "Britannia".into(),
            version: OLD_VERSION,
        }
    }

    fn flow() -> LoginFlow {
        let connect: Connect = Arc::new(|_, _| Err("not used".into()));
        LoginFlow::new(LoginStart {
            form: LoginForm::default(),
            saved: vec![cedric()],
            connect,
            connect_at_once: false,
            version: ClientVersion::MODERN,
            uopath: None,
        })
    }

    #[test]
    fn a_saved_login_fills_the_form_and_keeps_the_host_and_the_password() {
        let typed = LoginForm {
            host: "play.example".into(),
            port: "2593".into(),
            password: "typed".into(),
            ..LoginForm::default()
        };
        let form = with_saved(&typed, &cedric());
        assert_eq!(
            (form.account.as_str(), form.character.as_str()),
            ("acct2", "Cedric")
        );
        assert_eq!(form.profile.as_deref(), Some("cedric"));
        assert_eq!(
            (form.host.as_str(), form.password.as_str()),
            ("play.example", "typed")
        );
    }

    #[test]
    fn jev_reads_the_name_the_character_and_the_shard_and_never_the_account() {
        let words = cedric().words();
        assert_eq!(words, "cedric: character Cedric, shard Britannia");
        assert!(!words.contains("acct2"));
    }

    #[test]
    fn a_pick_of_jev_answers_the_login_and_an_unsure_jev_leaves_it_to_the_human() {
        let mut screen = flow();
        let (reply, mut answer) = tokio::sync::oneshot::channel();
        screen.stage = Stage::Picking {
            of: PickOf::Character,
            names: vec!["Mara".into(), "Cedric".into()],
            reply,
        };
        screen.jev_asks.send((Asked::Pick, Ok(None))).unwrap();
        screen.take_jev_answers();
        assert!(matches!(screen.stage, Stage::Picking { .. }));
        assert!(answer.try_recv().is_err());
        screen.jev_asks.send((Asked::Pick, Ok(Some(1)))).unwrap();
        screen.take_jev_answers();
        assert_eq!(answer.try_recv().ok(), Some(1));
        screen
            .jev_asks
            .send((Asked::SavedLogin, Ok(Some(0))))
            .unwrap();
        screen.take_jev_answers();
        assert_eq!(screen.form.character, "Cedric");
    }

    #[test]
    fn the_played_character_answers_the_question_which_one_to_play() {
        let mut screen = flow();
        let (reply, mut request) = tokio::sync::oneshot::channel();
        screen.stage = Stage::Characters {
            names: vec![String::new(), "Mara".into()],
            choices: CharacterChoices::default(),
            reply,
        };
        screen.play(0);
        assert!(matches!(screen.stage, Stage::Characters { .. }), "empty");
        screen.play(1);
        assert_eq!(request.try_recv().ok(), Some(CharacterRequest::Play(1)));
        let (ask, questions) = tokio::sync::mpsc::unbounded_channel();
        screen.questions = Some(questions);
        let (reply, mut pick) = tokio::sync::oneshot::channel();
        ask.send(LoginQuestion::Character {
            names: vec!["Cedric".into(), "Mara".into()],
            reply,
        })
        .unwrap();
        screen.take_questions(&egui::Context::default());
        assert_eq!(pick.try_recv().ok(), Some(1));
    }

    #[test]
    fn a_saved_login_gives_the_version_a_new_character_follows() {
        let mut screen = flow();
        assert_eq!(screen.version(), ClientVersion::MODERN);
        screen.form = with_saved(&screen.form, &cedric());
        assert_eq!(screen.version(), OLD_VERSION);
    }

    #[test]
    fn a_new_character_needs_room_and_goes_as_a_make_request() {
        let mut screen = flow();
        let (reply, mut request) = tokio::sync::oneshot::channel();
        screen.stage = Stage::Characters {
            names: vec!["A".into(); 5],
            choices: CharacterChoices::default(),
            reply,
        };
        screen.begin_creation();
        assert!(screen.creating.is_none());
        assert_eq!(screen.note, Some((WORDS_NO_ROOM.to_string(), true)));
        if let Stage::Characters { names, .. } = &mut screen.stage {
            names[4] = String::new();
        }
        screen.begin_creation();
        screen.creating.as_mut().unwrap().name = "Mara".into();
        screen.finish_creation();
        let Ok(CharacterRequest::Make(wish)) = request.try_recv() else {
            panic!("no make request");
        };
        assert_eq!((wish.name.as_str(), wish.slot), ("Mara", 4));
        assert!(screen.creating.is_none());
    }
}
