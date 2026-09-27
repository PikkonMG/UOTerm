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
//! "Save login" keeps the form, but never the password, as a saved login:
//! the host, the port, the account, the shard, the character and the
//! encryption. A click on a saved login puts it back in the form.
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
use uoterm_runtime::{
    CharacterChoices, CharacterRequest, EncryptionMode, LoginPicker, LoginQuestion, Profile,
};

const PANEL_SIZE: Vec2 = Vec2::new(880.0, 560.0);
const LIST_WIDTH: f32 = 300.0;
const ROW: f32 = 34.0;
/// A saved login takes two lines: its name with its buttons, and the
/// account and server under them.
const SAVED_ROW: f32 = 58.0;
const SAVED_NAME_TOP: f32 = 9.0;
const SAVED_DETAIL_TOP: f32 = 35.0;
const SMALL_BUTTON_TOP: f32 = 5.0;
const SMALL_BUTTON: Vec2 = Vec2::new(54.0, 24.0);
const FIELD_ROW: f32 = 32.0;
const LABEL_WIDTH: f32 = 130.0;
const GAP: f32 = 10.0;
const TITLE_ROW: f32 = 44.0;
const FIELD_RADIUS: u8 = 6;
const REPAINT_WHILE_WAITING_MS: u64 = 100;
const SAVE_BUTTON_WIDTH: f32 = 76.0;

const WORDS_TITLE: &str = "UOTerm";
const WORDS_SAVED: &str = "Saved logins";
const WORDS_NO_SAVED: &str = "No saved logins yet. Fill in the form and press Save login.";
const WORDS_SAVE_LOGIN: &str = "Save login";
const WORDS_SAVE: &str = "Save";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_SAVE_AS: &str = "Save as";
const WORDS_PASSWORD_VARIABLE: &str = "Password variable";
const HINT_PASSWORD_VARIABLE: &str = "Optional: a variable that holds the password";
const WORDS_NOT_SAVED: &str = "The password is not saved.";
const WORDS_EDIT: &str = "Edit";
const WORDS_YES: &str = "Yes";
const WORDS_NO: &str = "No";
const WORDS_DELETE_SAVED: &str = "Delete this saved login?";
const WORDS_EDITING: &str = "Change the fields, then press Save.";
const WORDS_SAVED_AS: &str = "Saved as";
const WORDS_ENCRYPTION: &str = "Encryption";
/// The encryption choices of the form, as `play --encryption` names them.
const ENCRYPTIONS: [(EncryptionMode, &str); 2] = [
    (EncryptionMode::None, "None (most free shards)"),
    (EncryptionMode::Osi, "OSI (encrypted shards)"),
];
pub const NEEDS_ACCOUNT: &str = "Type the account.";
pub const NEEDS_HOST: &str = "Type the host.";
pub const BAD_PORT: &str = "The port must be a number from 1 to 65535.";
const NEEDS_NAME: &str = "Type a name for the saved login.";
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
const PASSWORD_ID: &str = "login-password";
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
    pub encryption: EncryptionMode,
    /// The saved login the form came from. It gives the era, the version
    /// and the environment variable of the password.
    pub profile: Option<String>,
}

impl LoginForm {
    /// The account, trimmed, or words for the human when there is none.
    pub fn account_name(&self) -> Result<&str, &'static str> {
        Some(self.account.trim())
            .filter(|account| !account.is_empty())
            .ok_or(NEEDS_ACCOUNT)
    }

    pub fn host_name(&self) -> Result<&str, &'static str> {
        Some(self.host.trim())
            .filter(|host| !host.is_empty())
            .ok_or(NEEDS_HOST)
    }

    pub fn port_number(&self) -> Result<u16, &'static str> {
        self.port
            .trim()
            .parse()
            .ok()
            .filter(|port| *port != 0)
            .ok_or(BAD_PORT)
    }

    /// The form as a saved login. The password is never in it; a variable
    /// that holds the password is, when `password_env` names one.
    fn as_profile(&self, password_env: &str) -> Result<Profile, &'static str> {
        let named = |words: &str| Some(words.trim().to_string()).filter(|w| !w.is_empty());
        Ok(Profile {
            account: self.account_name()?.to_string(),
            host: Some(self.host_name()?.to_string()),
            port: Some(self.port_number()?),
            encryption: Some(self.encryption),
            password_env: named(password_env),
            character: self.character.trim().to_string(),
            shard: named(&self.shard),
            version: None,
            era: None,
        })
    }

    /// The form with a saved login put in. The password is typed again:
    /// it belongs to the account.
    pub fn filled_from(&self, saved: &SavedLogin) -> LoginForm {
        LoginForm {
            host: saved.host.clone(),
            port: saved.port.to_string(),
            account: saved.account.clone(),
            password: String::new(),
            shard: saved.shard.clone(),
            character: saved.character.clone(),
            encryption: saved.encryption,
            profile: Some(saved.name.clone()),
        }
    }
}

/// One saved login: its name and what it puts in the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedLogin {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub encryption: EncryptionMode,
    pub account: String,
    pub character: String,
    pub shard: String,
    /// The variable that holds the password, when the saved login names
    /// one.
    pub password_env: Option<String>,
    /// The client version the login speaks, which a new character follows.
    pub version: ClientVersion,
    /// The screen wrote it, so the screen may delete it. An older one of
    /// the profiles folder is only read.
    pub deletable: bool,
}

impl SavedLogin {
    /// The words Jev reads to tell one saved login from another.
    fn words(&self) -> String {
        let mut words = self.name.clone();
        for (what, value) in [("character", &self.character), ("shard", &self.shard)] {
            if !value.is_empty() {
                words.push_str(&format!(", {what} {value}"));
            }
        }
        words.push_str(&format!(", server {}", self.server()));
        words
    }

    fn server(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// The line under the name in the list.
    fn detail(&self) -> String {
        format!("{} @ {}", self.account, self.server())
    }
}

/// What the login screen asks to change in the saved logins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeepLogin {
    /// Save the login under this name, over one of the same name.
    Save {
        name: String,
        profile: Profile,
    },
    Delete(String),
}

/// Changes the saved logins. Gives the list after, or words for the human.
pub type KeepLogins = Arc<dyn Fn(KeepLogin) -> Result<Vec<SavedLogin>, String> + Send + Sync>;

/// The question of "Save login": the name, and a variable that holds the
/// password, which may stay empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SaveAsk {
    name: String,
    password_env: String,
}

/// What a click in the list of saved logins asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedAct {
    Pick(usize),
    Edit(usize),
    Delete(usize),
    /// The answer to "Delete this saved login?".
    Sure(bool),
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
    keep: KeepLogins,
    /// The saved login the human edits: a save goes over it.
    editing: Option<String>,
    /// The question of "Save login", while it is asked.
    saving: Option<SaveAsk>,
    /// The saved login waiting to be deleted once the human says yes.
    forget_asked: Option<String>,
    /// The next frame puts the cursor in the password field.
    focus_password: bool,
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

impl LoginFlow {
    fn new(start: LoginStart) -> Self {
        let (jev_asks, jev_answers) = mpsc::channel();
        Self {
            form: start.form,
            saved: start.saved,
            connect: start.connect,
            keep: start.keep,
            editing: None,
            saving: None,
            forget_asked: None,
            focus_password: false,
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

    /// Puts a saved login in the form, and the cursor in the password
    /// field.
    fn pick_saved(&mut self, place: usize) {
        if let Some(saved) = self.saved.get(place) {
            self.form = self.form.filled_from(saved);
            self.focus_password = true;
            self.editing = None;
            self.saving = None;
            self.note = None;
        }
    }

    /// Puts a saved login in the form, to change it and save it over.
    fn edit_saved(&mut self, place: usize) {
        self.pick_saved(place);
        self.focus_password = false;
        self.editing = self.saved.get(place).map(|saved| saved.name.clone());
        self.begin_save();
        self.note = Some((WORDS_EDITING.into(), false));
    }

    /// Asks the name to save the form under: the one edited, else
    /// account@host.
    fn begin_save(&mut self) {
        let edited = self
            .editing
            .as_deref()
            .and_then(|name| self.saved.iter().find(|saved| saved.name == name));
        self.saving = Some(SaveAsk {
            name: edited.map_or_else(
                || format!("{}@{}", self.form.account.trim(), self.form.host.trim()),
                |saved| saved.name.clone(),
            ),
            password_env: edited
                .and_then(|saved| saved.password_env.clone())
                .unwrap_or_default(),
        });
    }

    /// Saves the form under the name asked. The password is not saved.
    fn finish_save(&mut self) {
        let Some(ask) = self.saving.as_ref() else {
            return;
        };
        let name = ask.name.trim().to_string();
        if name.is_empty() {
            self.note = Some((NEEDS_NAME.into(), true));
            return;
        }
        let profile = match self.form.as_profile(&ask.password_env) {
            Ok(profile) => profile,
            Err(words) => {
                self.note = Some((words.into(), true));
                return;
            }
        };
        match (self.keep)(KeepLogin::Save {
            name: name.clone(),
            profile,
        }) {
            Ok(saved) => {
                self.saved = saved;
                self.saving = None;
                self.editing = None;
                self.note = Some((format!("{WORDS_SAVED_AS} {name}."), false));
                self.form.profile = Some(name);
            }
            Err(words) => self.note = Some((words, true)),
        }
    }

    /// Deletes the saved login asked about, when the answer is yes.
    fn answer_forget(&mut self, sure: bool) {
        let Some(name) = self.forget_asked.take().filter(|_| sure) else {
            return;
        };
        match (self.keep)(KeepLogin::Delete(name.clone())) {
            Ok(saved) => {
                self.saved = saved;
                let still_saved = self.saved.iter().any(|saved| saved.name == name);
                if !still_saved && self.form.profile.as_deref() == Some(name.as_str()) {
                    self.form.profile = None;
                }
                if self.editing.as_deref() == Some(name.as_str()) {
                    self.editing = None;
                    self.saving = None;
                }
                self.note = None;
            }
            Err(words) => self.note = Some((words, true)),
        }
    }

    fn act_on_saved(&mut self, act: SavedAct) {
        match act {
            SavedAct::Pick(place) => self.pick_saved(place),
            SavedAct::Edit(place) => self.edit_saved(place),
            SavedAct::Delete(place) => {
                self.forget_asked = self.saved.get(place).map(|saved| saved.name.clone());
            }
            SavedAct::Sure(sure) => self.answer_forget(sure),
        }
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
                (Asked::SavedLogin, Ok(Some(place))) => self.pick_saved(place),
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
    pub keep: KeepLogins,
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
    if let Some(act) = saved_list(flow, ui, list) {
        flow.act_on_saved(act);
    }
    let right = Rect::from_min_max(Pos2::new(list.right() + GAP * 2.0, body.top()), body.max);
    let margin = egui::Margin::symmetric(8, 6);
    let row_at = |place: usize| {
        Rect::from_min_size(
            right.left_top() + Vec2::new(0.0, place as f32 * (FIELD_ROW + GAP)),
            Vec2::new(right.width(), FIELD_ROW),
        )
    };
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
        let field = labeled_field(ui, row_at(i), label);
        let secret = i == PASSWORD_FIELD;
        let mut edit = plain_edit(value, margin)
            .password(secret)
            .hint_text(if secret { HINT_PASSWORD_ENV } else { "" });
        if secret {
            edit = edit.id(Id::new(PASSWORD_ID));
        }
        let response = ui.put(field, edit);
        if secret && std::mem::take(&mut flow.focus_password) {
            response.request_focus();
        }
        enter |= response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    }
    let encryption_row = row_at(LABELS.len());
    let choices = labeled_field(ui, encryption_row, WORDS_ENCRYPTION);
    let choice_width = (choices.width() - GAP) / ENCRYPTIONS.len() as f32;
    for (i, (mode, words)) in ENCRYPTIONS.into_iter().enumerate() {
        let choice = Rect::from_min_size(
            choices.left_top() + Vec2::new(i as f32 * (choice_width + GAP), 0.0),
            Vec2::new(choice_width, choices.height()),
        );
        if list_row(
            ui,
            choice,
            ("login-encryption", i),
            words,
            flow.form.encryption == mode,
        ) {
            flow.form.encryption = mode;
        }
    }
    let wish_row = Rect::from_min_size(
        encryption_row.left_bottom() + Vec2::new(0.0, GAP * 2.0),
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
        plain_edit(&mut flow.wish, margin).hint_text(if jev_on {
            HINT_WISH
        } else {
            HINT_WISH_OFF
        }),
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
    let save_top = wish_row.bottom() + GAP * 2.0;
    save_question(flow, ui, right, save_top, margin);
    let bottom = Pos2::new(right.left() + LABEL_WIDTH, right.bottom() - FIELD_ROW);
    let (connect_rect, connect) = theme::button(ui, bottom, WORDS_CONNECT, theme::GOAL);
    let (save_rect, save) = theme::button(
        ui,
        Pos2::new(connect_rect.right() + GAP, bottom.y),
        WORDS_SAVE_LOGIN,
        theme::TEXT,
    );
    ui.painter().text(
        Pos2::new(save_rect.right() + GAP, save_rect.center().y),
        Align2::LEFT_CENTER,
        WORDS_NOT_SAVED,
        text_font(theme::SIZE_SMALL),
        theme::TEXT_FAINT,
    );
    if save {
        flow.begin_save();
    }
    if connect || enter {
        flow.start(ctx);
    }
}

/// A label on the left of a row, and the field of the row after it.
/// Gives the field.
fn labeled_field(ui: &egui::Ui, row: Rect, label: &str) -> Rect {
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
    field
}

/// The question of "Save login", while it is asked: the name and the
/// variable that holds the password, with Save and Cancel.
fn save_question(
    flow: &mut LoginFlow,
    ui: &mut egui::Ui,
    right: Rect,
    top: f32,
    margin: egui::Margin,
) {
    let Some(ask) = flow.saving.as_mut() else {
        return;
    };
    let name_row = Rect::from_min_size(
        Pos2::new(right.left(), top),
        Vec2::new(right.width() - (SAVE_BUTTON_WIDTH + GAP) * 2.0, FIELD_ROW),
    );
    let variable_row = name_row.translate(Vec2::new(0.0, FIELD_ROW + GAP));
    let variable_row = variable_row.with_max_x(right.right());
    let name_field = labeled_field(ui, name_row, WORDS_SAVE_AS);
    let name = ui.put(name_field, plain_edit(&mut ask.name, margin));
    let variable_field = labeled_field(ui, variable_row, WORDS_PASSWORD_VARIABLE);
    ui.put(
        variable_field,
        plain_edit(&mut ask.password_env, margin).hint_text(HINT_PASSWORD_VARIABLE),
    );
    let button_at = |place: f32| {
        Rect::from_min_size(
            Pos2::new(
                name_row.right() + GAP + place * (SAVE_BUTTON_WIDTH + GAP),
                top,
            ),
            Vec2::new(SAVE_BUTTON_WIDTH, FIELD_ROW),
        )
    };
    let save = theme::segment(ui, button_at(0.0), WORDS_SAVE, theme::GOAL);
    let cancel = theme::segment(ui, button_at(1.0), WORDS_CANCEL, theme::TEXT);
    if save || (name.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
        flow.finish_save();
    } else if cancel {
        flow.saving = None;
        flow.editing = None;
        flow.note = None;
    }
}

/// A one-line field with no frame, in the words of the form.
fn plain_edit(value: &mut String, margin: egui::Margin) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(value)
        .frame(false)
        .margin(margin)
        .font(text_font(theme::SIZE_BODY))
        .text_color(theme::TEXT)
}

/// The saved logins, each with its name, "account @ host:port", and Edit
/// and Delete. Gives what a click asked.
fn saved_list(flow: &LoginFlow, ui: &mut egui::Ui, list: Rect) -> Option<SavedAct> {
    ui.painter().text(
        list.left_top(),
        Align2::LEFT_TOP,
        WORDS_SAVED,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
    let rows = Rect::from_min_max(list.left_top() + Vec2::new(0.0, ROW), list.max);
    if flow.saved.is_empty() {
        let mut job = egui::text::LayoutJob::single_section(
            WORDS_NO_SAVED.into(),
            egui::TextFormat::simple(text_font(theme::SIZE_SMALL), theme::TEXT_FAINT),
        );
        job.wrap.max_width = LIST_WIDTH;
        let galley = ui.painter().layout_job(job);
        ui.painter()
            .galley(rows.left_top(), galley, theme::TEXT_FAINT);
        return None;
    }
    let mut act = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(rows), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("saved-logins")
            .max_height(rows.height())
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for (i, saved) in flow.saved.iter().enumerate() {
                    let (spot, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), SAVED_ROW),
                        Sense::hover(),
                    );
                    let row = spot.with_max_y(spot.bottom() - theme::ROW_GAP / 2.0);
                    let chosen = flow.form.profile.as_deref() == Some(saved.name.as_str());
                    let asked = flow.forget_asked.as_deref() == Some(saved.name.as_str());
                    act = saved_row(ui, row, i, saved, chosen, asked).or(act);
                }
            });
    });
    act
}

/// One saved login of the list. Gives what a click on it asked.
fn saved_row(
    ui: &egui::Ui,
    row: Rect,
    place: usize,
    saved: &SavedLogin,
    chosen: bool,
    asked: bool,
) -> Option<SavedAct> {
    let response = ui.interact(row, Id::new(("saved-login", place)), Sense::click());
    let fill = if chosen || response.hovered() {
        theme::BUTTON_HOVER
    } else {
        theme::BUTTON
    };
    ui.painter()
        .rect_filled(row, CornerRadius::same(FIELD_RADIUS), fill);
    let buttons: &[(&str, SavedAct, egui::Color32)] = match (asked, saved.deletable) {
        (true, _) => &[
            (WORDS_YES, SavedAct::Sure(true), theme::ALARM),
            (WORDS_NO, SavedAct::Sure(false), theme::TEXT),
        ],
        (false, true) => &[
            (WORDS_EDIT, SavedAct::Edit(place), theme::TEXT),
            (WORDS_DELETE, SavedAct::Delete(place), theme::ALARM),
        ],
        (false, false) => &[(WORDS_EDIT, SavedAct::Edit(place), theme::TEXT)],
    };
    let buttons_left = row.right() - (SMALL_BUTTON.x + GAP) * buttons.len() as f32;
    let name_room = Rect::from_min_max(row.min, Pos2::new(buttons_left, row.bottom()));
    let painter = ui.painter().with_clip_rect(row.intersect(ui.clip_rect()));
    painter
        .with_clip_rect(name_room.intersect(ui.clip_rect()))
        .text(
            row.left_top() + Vec2::new(GAP, SAVED_NAME_TOP),
            Align2::LEFT_TOP,
            &saved.name,
            text_font(theme::SIZE_BODY),
            theme::TEXT,
        );
    let (detail, detail_color) = if asked {
        (WORDS_DELETE_SAVED.to_string(), theme::ALARM)
    } else {
        (saved.detail(), theme::TEXT_DIM)
    };
    painter.text(
        row.left_top() + Vec2::new(GAP, SAVED_DETAIL_TOP),
        Align2::LEFT_TOP,
        detail,
        text_font(theme::SIZE_SMALL),
        detail_color,
    );
    let mut act = response.clicked().then_some(SavedAct::Pick(place));
    for (i, (words, asks, color)) in buttons.iter().enumerate() {
        let button = Rect::from_min_size(
            Pos2::new(
                buttons_left + i as f32 * (SMALL_BUTTON.x + GAP),
                row.top() + SMALL_BUTTON_TOP,
            ),
            SMALL_BUTTON,
        );
        let key = Id::new(("saved-login-button", place, *words));
        if theme::segment_keyed(ui, button, key, words, *color) {
            act = Some(*asks);
        }
    }
    act
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
    use super::super::modern::testing::{click, Canvas, ENV_PICTURES, SCREEN};
    use super::*;
    use std::sync::Mutex;

    const OLD_VERSION: ClientVersion = ClientVersion::new(5, 0, 9, 1);
    const TYPED_PASSWORD: &str = "hunter2-typed";

    fn cedric() -> SavedLogin {
        SavedLogin {
            name: "cedric".into(),
            host: "play.example.com".into(),
            port: 2594,
            encryption: EncryptionMode::Osi,
            account: "acct2".into(),
            character: "Cedric".into(),
            shard: "Britannia".into(),
            password_env: Some("CEDRIC_PASS".into()),
            version: OLD_VERSION,
            deletable: true,
        }
    }

    /// A keeper of saved logins in memory, which also tells what it was
    /// asked.
    fn keeper(asked: Arc<Mutex<Vec<KeepLogin>>>) -> KeepLogins {
        let saved = Mutex::new(vec![cedric()]);
        Arc::new(move |ask: KeepLogin| {
            let mut saved = saved.lock().unwrap();
            match &ask {
                KeepLogin::Save { name, profile } => {
                    saved.retain(|known| &known.name != name);
                    saved.push(SavedLogin {
                        name: name.clone(),
                        account: profile.account.clone(),
                        ..cedric()
                    });
                }
                KeepLogin::Delete(name) => saved.retain(|known| &known.name != name),
            }
            asked.lock().unwrap().push(ask);
            Ok(saved.clone())
        })
    }

    fn flow_asked() -> (LoginFlow, Arc<Mutex<Vec<KeepLogin>>>) {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let connect: Connect = Arc::new(|_, _| Err("not used".into()));
        let flow = LoginFlow::new(LoginStart {
            form: LoginForm::default(),
            saved: vec![cedric()],
            connect,
            keep: keeper(Arc::clone(&asked)),
            connect_at_once: false,
            version: ClientVersion::MODERN,
            uopath: None,
        });
        (flow, asked)
    }

    fn flow() -> LoginFlow {
        flow_asked().0
    }

    #[test]
    fn a_saved_login_fills_the_form_and_the_password_is_typed_again() {
        let mut screen = flow();
        screen.form = LoginForm {
            host: "other.example".into(),
            port: "2593".into(),
            password: TYPED_PASSWORD.into(),
            ..LoginForm::default()
        };
        screen.pick_saved(0);
        let form = &screen.form;
        assert_eq!(
            (
                form.host.as_str(),
                form.port.as_str(),
                form.account.as_str()
            ),
            ("play.example.com", "2594", "acct2")
        );
        assert_eq!(
            (form.shard.as_str(), form.character.as_str()),
            ("Britannia", "Cedric")
        );
        assert_eq!(form.encryption, EncryptionMode::Osi);
        assert_eq!(form.profile.as_deref(), Some("cedric"));
        assert!(form.password.is_empty(), "the password is typed again");
        assert!(screen.focus_password, "the cursor goes to the password");
    }

    #[test]
    fn jev_reads_the_name_the_character_the_shard_and_the_server_and_never_the_account() {
        let words = cedric().words();
        assert_eq!(
            words,
            "cedric, character Cedric, shard Britannia, server play.example.com:2594"
        );
        assert!(!words.contains("acct2"));
        let bare = SavedLogin {
            character: String::new(),
            shard: String::new(),
            ..cedric()
        };
        assert_eq!(bare.words(), "cedric, server play.example.com:2594");
        assert_eq!(cedric().detail(), "acct2 @ play.example.com:2594");
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
        assert_eq!(screen.form.host, "play.example.com");
    }

    #[test]
    fn saving_asks_a_name_and_never_saves_the_password() {
        let (mut screen, asked) = flow_asked();
        screen.form = LoginForm {
            host: " 10.0.0.7 ".into(),
            port: "2593".into(),
            account: "mara".into(),
            password: TYPED_PASSWORD.into(),
            encryption: EncryptionMode::Osi,
            ..LoginForm::default()
        };
        screen.begin_save();
        let ask = screen.saving.clone().unwrap();
        assert_eq!(ask.name, "mara@10.0.0.7");
        assert!(ask.password_env.is_empty(), "no variable unless typed");
        screen.finish_save();
        let asked = asked.lock().unwrap();
        let [KeepLogin::Save { name, profile }] = asked.as_slice() else {
            panic!("one save: {asked:?}");
        };
        assert_eq!(name, "mara@10.0.0.7");
        assert_eq!(profile.host.as_deref(), Some("10.0.0.7"));
        assert_eq!(profile.port, Some(2593));
        assert_eq!(profile.encryption, Some(EncryptionMode::Osi));
        assert_eq!(profile.password_env, None);
        let text = toml::to_string(profile).unwrap();
        assert!(!text.contains(TYPED_PASSWORD), "{text}");
        assert!(!text.contains("password"), "{text}");
        assert!(screen.saving.is_none());
        assert_eq!(screen.form.profile.as_deref(), Some("mara@10.0.0.7"));
        assert!(screen
            .saved
            .iter()
            .any(|saved| saved.name == "mara@10.0.0.7"));
        assert_eq!(screen.form.password, TYPED_PASSWORD, "the form keeps it");
    }

    #[test]
    fn a_login_with_no_account_or_a_bad_port_is_not_saved() {
        let (mut screen, asked) = flow_asked();
        screen.form.host = "10.0.0.7".into();
        screen.form.port = "2593".into();
        screen.begin_save();
        screen.finish_save();
        assert_eq!(screen.note, Some((NEEDS_ACCOUNT.to_string(), true)));
        screen.form.account = "mara".into();
        screen.form.port = "none".into();
        screen.finish_save();
        assert_eq!(screen.note, Some((BAD_PORT.to_string(), true)));
        screen.form.port = "2593".into();
        screen.saving.as_mut().unwrap().name = "  ".into();
        screen.finish_save();
        assert_eq!(screen.note, Some((NEEDS_NAME.to_string(), true)));
        assert!(asked.lock().unwrap().is_empty());
        assert!(screen.saving.is_some(), "the question stays");
    }

    #[test]
    fn edit_saves_over_the_saved_login_with_its_password_variable() {
        let (mut screen, asked) = flow_asked();
        screen.edit_saved(0);
        assert_eq!(screen.editing.as_deref(), Some("cedric"));
        assert!(!screen.focus_password);
        let ask = screen.saving.clone().unwrap();
        assert_eq!(
            (ask.name.as_str(), ask.password_env.as_str()),
            ("cedric", "CEDRIC_PASS")
        );
        screen.form.character = "Mara".into();
        screen.finish_save();
        let asked = asked.lock().unwrap();
        let [KeepLogin::Save { name, profile }] = asked.as_slice() else {
            panic!("one save: {asked:?}");
        };
        assert_eq!(name, "cedric");
        assert_eq!(profile.character, "Mara");
        assert_eq!(profile.password_env.as_deref(), Some("CEDRIC_PASS"));
        assert!(screen.editing.is_none());
    }

    #[test]
    fn delete_asks_first() {
        let (mut screen, asked) = flow_asked();
        screen.pick_saved(0);
        screen.act_on_saved(SavedAct::Delete(0));
        assert_eq!(screen.forget_asked.as_deref(), Some("cedric"));
        assert!(asked.lock().unwrap().is_empty(), "nothing yet");
        screen.act_on_saved(SavedAct::Sure(false));
        assert!(screen.forget_asked.is_none());
        assert!(asked.lock().unwrap().is_empty(), "no is no");
        screen.act_on_saved(SavedAct::Delete(0));
        screen.act_on_saved(SavedAct::Sure(true));
        assert_eq!(
            asked.lock().unwrap().as_slice(),
            [KeepLogin::Delete("cedric".into())]
        );
        assert!(screen.saved.is_empty());
        assert_eq!(screen.form.profile, None);
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
        screen.pick_saved(0);
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

    /// Draws the login screen once for each list of events. Gives the
    /// output of the last frame.
    fn draw_frames(
        login: &mut LoginUi,
        ctx: &egui::Context,
        canvas: &mut Canvas,
        frames: &[Vec<egui::Event>],
    ) -> egui::FullOutput {
        let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);
        let mut last = None;
        for events in frames {
            let input = egui::RawInput {
                events: events.clone(),
                screen_rect: Some(screen),
                ..egui::RawInput::default()
            };
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(login.draw(ui, screen).is_none());
                });
            });
            canvas.take(&output.textures_delta);
            last = Some(output);
        }
        last.expect("at least one frame")
    }

    /// The first saved login of the list on the screen, as the form stage
    /// lays it out.
    fn first_saved_row() -> Rect {
        let panel =
            Rect::from_center_size(Rect::from_min_size(Pos2::ZERO, SCREEN).center(), PANEL_SIZE);
        let inner = panel.shrink(theme::PANEL_PAD * 1.5);
        Rect::from_min_size(
            inner.left_top() + Vec2::new(0.0, TITLE_ROW + ROW),
            Vec2::new(LIST_WIDTH, SAVED_ROW - theme::ROW_GAP / 2.0),
        )
    }

    #[test]
    fn a_click_on_a_saved_login_fills_the_form_and_delete_asks_yes_or_no() {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let connect: Connect = Arc::new(|_, _| Err("not used".into()));
        let mut login = LoginUi::new(LoginStart {
            form: LoginForm::default(),
            saved: vec![cedric()],
            connect,
            keep: keeper(Arc::clone(&asked)),
            connect_at_once: false,
            version: ClientVersion::MODERN,
            uopath: None,
        });
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut canvas = Canvas::default();
        let row = first_saved_row();
        let first_button = Pos2::new(
            row.right() - (SMALL_BUTTON.x + GAP) * 2.0 + SMALL_BUTTON.x / 2.0,
            row.top() + SMALL_BUTTON_TOP + SMALL_BUTTON.y / 2.0,
        );
        let second_button = first_button + Vec2::new(SMALL_BUTTON.x + GAP, 0.0);
        let mut frames = vec![Vec::new()];
        frames.extend(click(row.left_center() + Vec2::new(GAP * 2.0, 0.0)));
        draw_frames(&mut login, &ctx, &mut canvas, &frames);
        assert_eq!(login.flow.form.account, "acct2");
        assert_eq!(login.flow.form.encryption, EncryptionMode::Osi);
        draw_frames(&mut login, &ctx, &mut canvas, &click(second_button));
        assert_eq!(login.flow.forget_asked.as_deref(), Some("cedric"));
        let mut frames = click(first_button);
        frames.push(Vec::new());
        let asking = draw_frames(&mut login, &ctx, &mut canvas, &[Vec::new()]);
        save_picture(&ctx, &canvas, asking, "login-delete.png");
        draw_frames(&mut login, &ctx, &mut canvas, &frames);
        assert_eq!(
            asked.lock().unwrap().as_slice(),
            [KeepLogin::Delete("cedric".into())]
        );
        assert!(login.flow.saved.is_empty());
        // The question of Save, as Edit opens it.
        login.flow.saved = vec![cedric()];
        login.flow.edit_saved(0);
        let saving = draw_frames(&mut login, &ctx, &mut canvas, &[Vec::new()]);
        save_picture(&ctx, &canvas, saving, "login-save.png");
    }

    /// Saves the picture of a frame when the test is asked for pictures.
    fn save_picture(ctx: &egui::Context, canvas: &Canvas, output: egui::FullOutput, name: &str) {
        if let Some(out) = std::env::var_os(ENV_PICTURES) {
            let picture = canvas.paint(ctx, output, SCREEN);
            let path = std::path::PathBuf::from(out).join(name);
            super::super::save_png(&path, &picture).expect("the picture is saved");
        }
    }
}
