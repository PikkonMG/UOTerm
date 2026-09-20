//! The screens before the game: the login form, the list of shards and the
//! list of characters. `uoterm play` starts here. When the character is in
//! the world, the window becomes the game window.
//!
//! The password is typed in a field that hides it. It stays in memory for
//! the login and is written nowhere. A saved login can name an environment
//! variable that holds the password; then the field can stay empty.
//!
//! With a TypeSafe key, one field takes plain words, such as "my miner on
//! the test shard". Jev picks the saved login, and later the character,
//! from the lists. Jev sees the words and the names of the lists. It never
//! sees the password.

use super::link::Link;
use super::orders;
use super::theme::{self, text_font, title_font};
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use uoterm_runtime::{CharacterRequest, LoginPicker, LoginQuestion, NewCharacterWish};

const PANEL_SIZE: Vec2 = Vec2::new(760.0, 470.0);
const LIST_WIDTH: f32 = 230.0;
const ROW: f32 = 34.0;
const FIELD_ROW: f32 = 32.0;
const LABEL_WIDTH: f32 = 110.0;
const GAP: f32 = 10.0;
const TITLE_ROW: f32 = 44.0;
const FIELD_RADIUS: u8 = 6;
const LIST_ROWS: usize = 9;

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
const HINT_NEW_NAME: &str = "The name of your new character";
const WORDS_NEEDS_NAME: &str = "Give the new character a name.";
/// A new character starts with these, which every shard takes.
const NEW_STRENGTH: u8 = 45;
const NEW_DEXTERITY: u8 = 35;
const NEW_INTELLIGENCE: u8 = 10;
/// Swordsmanship and Tactics, each at 25 points, and Healing at 10.
const NEW_SKILLS: [(u8, u8); 3] = [(45, 25), (30, 25), (17, 10)];
const NEW_SKIN_HUE: u16 = 0x83EA;
const NEW_HAIR: u16 = 0x203B;
const NEW_HAIR_HUE: u16 = 0x044E;
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

enum Stage {
    Form,
    Connecting,
    /// The login waits for a pick from this list.
    Picking {
        title: &'static str,
        names: Vec<String>,
        reply: tokio::sync::oneshot::Sender<usize>,
    },
    /// The login waits for what to do with the characters of the account.
    Characters {
        names: Vec<String>,
        reply: tokio::sync::oneshot::Sender<CharacterRequest>,
    },
}

/// What Jev was asked about, so the answer goes to the right list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    SavedLogin,
    Pick,
}

pub struct LoginUi {
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
    /// The name typed for a new character, and the slot waiting to be
    /// deleted once the human presses Delete a second time.
    new_name: String,
    delete_asked: Option<usize>,
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

impl LoginUi {
    pub fn new(
        form: LoginForm,
        saved: Vec<SavedLogin>,
        connect: Connect,
        connect_at_once: bool,
    ) -> Self {
        let (jev_asks, jev_answers) = mpsc::channel();
        Self {
            form,
            saved,
            connect,
            stage: Stage::Form,
            questions: None,
            done: None,
            wish: String::new(),
            jev_key: orders::api_key(),
            jev_asks,
            jev_answers,
            note: None,
            connect_at_once,
            new_name: String::new(),
            delete_asked: None,
        }
    }

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
            let _ = reply.send(request);
        }
    }

    /// The characters of the account: play one, delete one, or make one.
    fn character_stage(
        &mut self,
        ui: &mut egui::Ui,
        body: Rect,
        names: &[String],
    ) -> Option<CharacterRequest> {
        ui.painter().text(
            body.center_top(),
            Align2::CENTER_TOP,
            WORDS_PICK_CHARACTER,
            text_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let width = LIST_WIDTH * 1.5;
        let left = body.center().x - width / 2.0;
        let mut request = None;
        for slot in 0..CHARACTER_SLOTS {
            let name = names.get(slot).filter(|name| !name.is_empty());
            let row = Rect::from_min_size(
                Pos2::new(left, body.top() + TITLE_ROW + slot as f32 * ROW),
                Vec2::new(width, ROW - theme::ROW_GAP / 2.0),
            );
            let words = name.map_or(WORDS_EMPTY_SLOT, |name| name.as_str());
            if list_row(ui, row, ("character-slot", slot), words, false) && name.is_some() {
                request = Some(CharacterRequest::Play(slot));
            }
            let Some(_) = name else {
                continue;
            };
            let asked = self.delete_asked == Some(slot);
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
                    request = Some(CharacterRequest::Delete(slot));
                } else {
                    self.delete_asked = Some(slot);
                }
            }
        }
        let make_row = Rect::from_min_size(
            Pos2::new(
                left,
                body.top() + TITLE_ROW + CHARACTER_SLOTS as f32 * ROW + GAP,
            ),
            Vec2::new(width, FIELD_ROW),
        );
        ui.painter()
            .rect_filled(make_row, CornerRadius::same(FIELD_RADIUS), theme::TRACK);
        ui.put(
            make_row,
            egui::TextEdit::singleline(&mut self.new_name)
                .frame(false)
                .margin(egui::Margin::symmetric(8, 6))
                .hint_text(HINT_NEW_NAME)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let (_, make) = theme::button(
            ui,
            Pos2::new(make_row.right() + GAP, make_row.top()),
            WORDS_MAKE,
            theme::GOAL,
        );
        if make {
            let name = self.new_name.trim();
            if name.is_empty() {
                self.note = Some((WORDS_NEEDS_NAME.into(), true));
            } else {
                let slot = names
                    .iter()
                    .position(|name| name.is_empty())
                    .unwrap_or(names.len());
                request = Some(CharacterRequest::Make(Box::new(NewCharacterWish {
                    name: name.to_string(),
                    female: false,
                    race: 0,
                    strength: NEW_STRENGTH,
                    dexterity: NEW_DEXTERITY,
                    intelligence: NEW_INTELLIGENCE,
                    skills: NEW_SKILLS.to_vec(),
                    skin_hue: NEW_SKIN_HUE,
                    hair: NEW_HAIR,
                    hair_hue: NEW_HAIR_HUE,
                    start_city: 0,
                    slot: slot as u16,
                })));
                self.new_name.clear();
            }
        }
        request
    }

    fn answer_pick(&mut self, place: usize) {
        if let Stage::Picking { reply, .. } = std::mem::replace(&mut self.stage, Stage::Connecting)
        {
            let _ = reply.send(place);
        }
    }

    fn take_questions(&mut self, ctx: &egui::Context) {
        let question = self.questions.as_mut().and_then(|q| q.try_recv().ok());
        let (title, ask, names, reply) = match question {
            Some(LoginQuestion::Shard { names, reply }) => {
                (WORDS_PICK_SHARD, orders::ASK_SHARD, names, reply)
            }
            Some(LoginQuestion::Character { names, reply }) => {
                (WORDS_PICK_CHARACTER, orders::ASK_CHARACTER, names, reply)
            }
            Some(LoginQuestion::Characters {
                names,
                refused,
                reply,
            }) => {
                if let Some(words) = refused {
                    self.note = Some((words, true));
                }
                self.delete_asked = None;
                self.stage = Stage::Characters { names, reply };
                return;
            }
            None => return,
        };
        // The wish may name the pick already. Jev answers, or the human clicks.
        if !self.wish.trim().is_empty() {
            self.ask_jev(Asked::Pick, ask, names.clone(), ctx);
        }
        self.stage = Stage::Picking {
            title,
            names,
            reply,
        };
    }

    /// Draws the screen for this frame. Gives the link when the character
    /// is in the world.
    pub fn draw(&mut self, ui: &mut egui::Ui, rect: Rect) -> Option<Link> {
        let ctx = ui.ctx().clone();
        if std::mem::take(&mut self.connect_at_once) {
            self.start(&ctx);
        }
        self.take_jev_answers();
        self.take_questions(&ctx);
        match self.done.as_ref().and_then(|done| done.try_recv().ok()) {
            Some(Ok(link)) => return Some(link),
            Some(Err(words)) => {
                self.stage = Stage::Form;
                self.done = None;
                self.questions = None;
                self.note = Some((words, true));
            }
            None => {}
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
        match &self.stage {
            Stage::Form => self.form_stage(ui, body, &ctx),
            Stage::Connecting => {
                ui.painter().text(
                    body.center(),
                    Align2::CENTER_CENTER,
                    WORDS_CONNECTING,
                    text_font(theme::SIZE_TITLE),
                    theme::WAITING,
                );
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Stage::Picking { title, names, .. } => {
                let (title, names) = (*title, names.clone());
                if let Some(place) = pick_list(ui, body, title, &names) {
                    self.answer_pick(place);
                }
            }
            Stage::Characters { names, .. } => {
                let names = names.clone();
                if let Some(request) = self.character_stage(ui, body, &names) {
                    self.answer_request(request);
                }
            }
        }
        if let Some((words, failed)) = &self.note {
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
        None
    }

    fn form_stage(&mut self, ui: &mut egui::Ui, body: Rect, ctx: &egui::Context) {
        let list = Rect::from_min_size(body.min, Vec2::new(LIST_WIDTH, body.height()));
        ui.painter().text(
            list.left_top(),
            Align2::LEFT_TOP,
            WORDS_SAVED,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        if self.saved.is_empty() {
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
        for (i, saved) in self.saved.iter().take(LIST_ROWS).enumerate() {
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, ROW * (i + 1) as f32),
                Vec2::new(LIST_WIDTH, ROW - theme::ROW_GAP / 2.0),
            );
            let chosen = self.form.profile.as_deref() == Some(saved.name.as_str());
            if list_row(ui, row, ("saved-login", i), &saved.name, chosen) {
                picked = Some(i);
            }
        }
        if let Some(saved) = picked.and_then(|i| self.saved.get(i)) {
            self.form = with_saved(&self.form, saved);
        }
        let right = Rect::from_min_max(Pos2::new(list.right() + GAP * 2.0, body.top()), body.max);
        let margin = egui::Margin::symmetric(8, 6);
        let fields: [&mut String; 6] = [
            &mut self.form.host,
            &mut self.form.port,
            &mut self.form.account,
            &mut self.form.password,
            &mut self.form.shard,
            &mut self.form.character,
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
        let jev_on = self.jev_key.is_some();
        let wish_field = Rect::from_min_max(
            wish_row.min,
            Pos2::new(wish_row.right() - FIND_WIDTH - GAP, wish_row.bottom()),
        );
        ui.painter()
            .rect_filled(wish_field, CornerRadius::same(FIELD_RADIUS), theme::TRACK);
        let wish = ui.put(
            wish_field,
            egui::TextEdit::singleline(&mut self.wish)
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
        if wished && jev_on && !self.wish.trim().is_empty() && !self.saved.is_empty() {
            let options = self.saved.iter().map(SavedLogin::words).collect();
            self.ask_jev(Asked::SavedLogin, orders::ASK_PROFILE, options, ctx);
        }
        let (_, connect) = theme::button(
            ui,
            Pos2::new(right.left() + LABEL_WIDTH, right.bottom() - FIELD_ROW),
            WORDS_CONNECT,
            theme::GOAL,
        );
        if connect || enter {
            self.start(ctx);
        }
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

    fn cedric() -> SavedLogin {
        SavedLogin {
            name: "cedric".into(),
            account: "acct2".into(),
            character: "Cedric".into(),
            shard: "Britannia".into(),
        }
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
        let connect: Connect = Arc::new(|_, _| Err("not used".into()));
        let mut screen = LoginUi::new(LoginForm::default(), vec![cedric()], connect, false);
        let (reply, mut answer) = tokio::sync::oneshot::channel();
        screen.stage = Stage::Picking {
            title: WORDS_PICK_CHARACTER,
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
}
