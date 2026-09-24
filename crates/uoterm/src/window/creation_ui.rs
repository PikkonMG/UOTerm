//! The making of a new character on the login panel: a plain form over the
//! model of `model::creation`, page by page: the look (name, sex, race,
//! hair and beard, the colors of the skin, the clothes, the hair and the
//! beard) with a figure of the new character, the profession, the skills
//! and stats of the Advanced choice, and the start town.

use super::model::creation::{
    facet_name, Creation, CreationFiles, Paint, Palette, Step, Style, SKILL_RANGE, STAT_RANGE,
};
use super::scene::Scene;
use super::theme;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};

const LEFT_WIDTH: f32 = 380.0;
const FIGURE_SIDE: f32 = 260.0;
/// The figure stands to the right of the controls, with a gap.
const FIGURE_GAP: f32 = 30.0;
const SWATCH_SIDE: f32 = 12.0;
const PICKED_SWATCH_SIDE: f32 = 18.0;
const PICKED_STROKE: f32 = 2.0;
const BUTTON_ROW: f32 = 34.0;
const COMBO_WIDTH: f32 = 180.0;
const PROFESSION_WIDTH: f32 = 220.0;
/// The facet the figure is drawn for.
const FIGURE_MAP: u8 = 0;
const TAG_OPEN: char = '<';
const TAG_CLOSE: char = '>';
const LINE_BREAK_TAG: &str = "br";
const SELF_CLOSING: char = '/';

const WORDS_NAME: &str = "Name";
const WORDS_MALE: &str = "Male";
const WORDS_FEMALE: &str = "Female";
const WORDS_RACE: &str = "Race";
const WORDS_HAIR: &str = "Hair";
const WORDS_BEARD: &str = "Beard";
const WORDS_LOCKED: &str = "The account cannot make this race.";
const WORDS_BACK: &str = "Back";
const WORDS_NEXT: &str = "Next";
const WORDS_FINISH: &str = "Make the character";
const WORDS_OKAY: &str = "Okay";
const WORDS_PROFESSION: &str = "Pick a profession";
const WORDS_TRADE: &str = "Skills and stats";
const WORDS_TOWN: &str = "Pick a start town";
const WORDS_NO_TOWNS: &str = "The shard lists no start town.";
const WORDS_PICK_SKILL: &str = "Pick a skill";
const STAT_WORDS: [&str; 3] = ["Strength", "Intelligence", "Dexterity"];

/// What the form asks the login screens to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Back from the first page: the list of characters again.
    Leave,
    /// The character is made: send it to the shard.
    Finish,
}

fn paint_words(paint: Paint) -> &'static str {
    match paint {
        Paint::Skin => "Skin",
        Paint::Shirt => "Shirt",
        Paint::Pants => "Pants",
        Paint::Hair => "Hair color",
        Paint::Beard => "Beard color",
    }
}

/// Words of the client with their tags taken out: a line break tag starts
/// a new line.
fn plain_words(html: &str) -> String {
    let mut words = String::with_capacity(html.len());
    let mut tag: Option<String> = None;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, TAG_OPEN) => tag = Some(String::new()),
            (Some(name), TAG_CLOSE) => {
                let name = name.trim().trim_end_matches(SELF_CLOSING);
                if name.eq_ignore_ascii_case(LINE_BREAK_TAG) {
                    words.push('\n');
                }
                tag = None;
            }
            (Some(name), c) => name.push(c),
            (None, c) => words.push(c),
        }
    }
    words.trim().to_string()
}

/// Draws the page of the creation that shows. Gives what the player asked.
pub fn draw(
    ui: &mut egui::Ui,
    body: Rect,
    creation: &mut Creation,
    files: &CreationFiles,
    scene: Option<&mut Scene>,
) -> Option<Asked> {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
    let ui = &mut child;
    if let Some((number, fallback)) = creation.message {
        ui.colored_label(theme::ALARM, files.words(number, fallback));
        if ui.button(WORDS_OKAY).clicked() {
            creation.message = None;
        }
        return None;
    }
    let page = Rect::from_min_max(
        body.min,
        Pos2::new(body.right(), body.bottom() - BUTTON_ROW),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(page), |ui| {
        egui::ScrollArea::vertical()
            .max_height(page.height())
            .show(ui, |ui| match creation.step.clone() {
                Step::Look => look_page(ui, body, creation, scene),
                Step::Profession(_) => profession_page(ui, creation, files),
                Step::Trade => trade_page(ui, creation, files),
                Step::Town => town_page(ui, creation, files),
            });
    });
    let buttons = Rect::from_min_max(Pos2::new(body.left(), page.bottom()), body.max);
    let mut asked = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(buttons), |ui| {
        ui.horizontal(|ui| {
            if ui.button(WORDS_BACK).clicked() && creation.back() {
                asked = Some(Asked::Leave);
            }
            match creation.step {
                Step::Look => {
                    let allowed = creation.race_allowed(creation.race);
                    if ui
                        .add_enabled(allowed, egui::Button::new(WORDS_NEXT))
                        .clicked()
                    {
                        creation.look_done();
                    }
                }
                Step::Trade => {
                    if ui.button(WORDS_NEXT).clicked() {
                        creation.trade_done();
                    }
                }
                Step::Town => {
                    let ready = !creation.towns().is_empty();
                    if ui
                        .add_enabled(ready, egui::Button::new(WORDS_FINISH))
                        .clicked()
                    {
                        asked = Some(Asked::Finish);
                    }
                }
                Step::Profession(_) => {}
            }
        });
    });
    asked
}

fn look_page(
    ui: &mut egui::Ui,
    body: Rect,
    creation: &mut Creation,
    mut scene: Option<&mut Scene>,
) {
    let figure = Rect::from_min_size(
        Pos2::new(body.left() + LEFT_WIDTH + FIGURE_GAP, body.top()),
        Vec2::splat(FIGURE_SIDE),
    );
    if let Some((texture, sprite)) = scene
        .as_deref_mut()
        .and_then(|scene| scene.doll_picture(FIGURE_MAP, &creation.look()))
    {
        let area = theme::fit_up_to(figure, sprite.width, sprite.height, 1.0);
        ui.painter().image(texture, area, sprite.uv, Color32::WHITE);
    }
    ui.set_max_width(LEFT_WIDTH);
    ui.horizontal(|ui| {
        ui.label(WORDS_NAME);
        ui.text_edit_singleline(&mut creation.name);
    });
    ui.horizontal(|ui| {
        let female = creation.female;
        if ui.radio(!female, WORDS_MALE).clicked() {
            creation.set_female(false);
        }
        if ui.radio(female, WORDS_FEMALE).clicked() {
            creation.set_female(true);
        }
    });
    ui.horizontal(|ui| {
        ui.label(WORDS_RACE);
        for race in creation.races_shown() {
            if ui.radio(creation.race == race, race.words()).clicked() {
                creation.set_race(race);
            }
        }
    });
    if !creation.race_allowed(creation.race) {
        ui.colored_label(theme::ALARM, WORDS_LOCKED);
    }
    style_combo(ui, WORDS_HAIR, creation.hair_styles(), &mut creation.hair);
    if let Some(beards) = creation.beard_styles() {
        style_combo(ui, WORDS_BEARD, beards, &mut creation.beard);
    }
    for paint in creation.paints() {
        let palette = creation.palette(paint);
        let picked = creation.color_place(paint);
        let color = |hue: u16| scene.as_deref().map(|scene| scene.words_color(hue));
        let shown = color(creation.hue(paint));
        let mut place = picked;
        ui.horizontal(|ui| {
            if let Some(shown) = shown {
                let (swatch, _) =
                    ui.allocate_exact_size(Vec2::splat(PICKED_SWATCH_SIDE), Sense::hover());
                ui.painter().rect_filled(swatch, 0.0, shown);
            }
            egui::CollapsingHeader::new(paint_words(paint))
                .id_salt(("new-character-paint", paint_words(paint)))
                .show(ui, |ui| match shown {
                    Some(_) => {
                        let colors: Vec<Color32> = palette
                            .hues
                            .iter()
                            .map(|hue| color(*hue).unwrap_or(theme::TEXT_DIM))
                            .collect();
                        if let Some(at) = palette_grid(ui, &palette, &colors, picked) {
                            place = at;
                        }
                    }
                    None => {
                        let last = palette.hues.len().saturating_sub(1);
                        ui.add(egui::Slider::new(&mut place, 0..=last));
                    }
                });
        });
        if place != picked {
            creation.set_color(paint, place);
        }
    }
}

/// The hues of a palette as a grid of small boxes in their colors, the
/// picked one ringed. Gives the place of a box the player clicked.
fn palette_grid(
    ui: &mut egui::Ui,
    palette: &Palette,
    colors: &[Color32],
    picked: usize,
) -> Option<usize> {
    let size = Vec2::new(
        palette.columns as f32 * SWATCH_SIDE,
        palette.rows as f32 * SWATCH_SIDE,
    );
    let (area, response) = ui.allocate_exact_size(size, Sense::click());
    for (at, color) in colors
        .iter()
        .enumerate()
        .take(palette.rows * palette.columns)
    {
        let (column, row) = ((at % palette.columns) as f32, (at / palette.columns) as f32);
        let cell = Rect::from_min_size(
            area.min + Vec2::new(column, row) * SWATCH_SIDE,
            Vec2::splat(SWATCH_SIDE),
        );
        ui.painter().rect_filled(cell, 0.0, *color);
        if at == picked {
            ui.painter().rect_stroke(
                cell,
                0.0,
                egui::Stroke::new(PICKED_STROKE, theme::TEXT),
                egui::StrokeKind::Inside,
            );
        }
    }
    let pointer = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())?;
    let cell = (pointer - area.min) / SWATCH_SIDE;
    let at = cell.y as usize * palette.columns + cell.x as usize;
    (at < colors.len()).then_some(at)
}

fn style_combo(ui: &mut egui::Ui, label: &str, styles: &[Style], picked: &mut usize) {
    let shown = styles.get(*picked).map_or("", |style| style.words);
    egui::ComboBox::from_label(label)
        .width(COMBO_WIDTH)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (at, style) in styles.iter().enumerate() {
                ui.selectable_value(picked, at, style.words);
            }
        });
}

fn profession_page(ui: &mut egui::Ui, creation: &mut Creation, files: &CreationFiles) {
    ui.label(WORDS_PROFESSION);
    let mut picked = None;
    for profession in creation.professions(files) {
        let name = files.words(profession.name_id, &profession.name);
        let about = plain_words(&files.words(profession.description_id, ""));
        let button = ui.add_sized([PROFESSION_WIDTH, 0.0], egui::Button::new(name));
        let button = if about.is_empty() {
            button
        } else {
            button.on_hover_text(about)
        };
        if button.clicked() {
            picked = Some(profession.clone());
        }
    }
    if let Some(profession) = picked {
        creation.pick_profession(&profession, files);
    }
}

fn trade_page(ui: &mut egui::Ui, creation: &mut Creation, files: &CreationFiles) {
    ui.label(WORDS_TRADE);
    for (at, words) in STAT_WORDS.iter().enumerate() {
        let mut value = creation.stats[at];
        if ui
            .add(egui::Slider::new(&mut value, STAT_RANGE.0..=STAT_RANGE.1).text(*words))
            .changed()
        {
            creation.set_stat(at, value);
        }
    }
    let menu = creation.skill_menu(files);
    for at in 0..creation.skills.len() {
        let pick = creation.skills[at];
        ui.horizontal(|ui| {
            let shown = pick
                .skill
                .and_then(|skill| menu.iter().find(|(known, _)| *known == skill))
                .map_or(WORDS_PICK_SKILL, |(_, name)| name.as_str());
            egui::ComboBox::from_id_salt(("new-character-skill", at))
                .width(COMBO_WIDTH)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (skill, name) in &menu {
                        if ui
                            .selectable_label(pick.skill == Some(*skill), name)
                            .clicked()
                        {
                            creation.set_skill(at, *skill);
                        }
                    }
                });
            let mut value = pick.value;
            if ui
                .add(egui::Slider::new(&mut value, SKILL_RANGE.0..=SKILL_RANGE.1))
                .changed()
            {
                creation.set_skill_value(at, value);
            }
        });
    }
}

fn town_page(ui: &mut egui::Ui, creation: &mut Creation, files: &CreationFiles) {
    ui.label(WORDS_TOWN);
    if creation.towns().is_empty() {
        ui.colored_label(theme::ALARM, WORDS_NO_TOWNS);
        return;
    }
    let mut picked = None;
    for (at, town) in creation.towns().iter().enumerate() {
        let facet = town
            .place
            .map(|place| format!(" ({})", facet_name(place.map)))
            .unwrap_or_default();
        let words = format!("{}: {}{facet}", town.name, town.building);
        if ui.radio(creation.town == at, words).clicked() {
            picked = Some(at);
        }
    }
    if let Some(at) = picked {
        creation.set_town(at);
    }
    if let Some(town) = creation.towns().get(creation.town) {
        ui.separator();
        ui.label(plain_words(&files.town_words(town, creation.town)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::model::creation::Race;
    use uoterm_protocol::ClientVersion;
    use uoterm_runtime::CharacterChoices;

    /// The side of the texture the art of the client goes into.
    const ART_TEXTURE_SIDE: usize = 4096;

    #[test]
    fn the_words_of_the_client_lose_their_tags() {
        assert_eq!(
            plain_words("<b>Yew</b> is a town.<BR>In the woods."),
            "Yew is a town.\nIn the woods."
        );
        assert_eq!(plain_words("<br/>plain"), "plain");
    }

    /// Draws each page with no screen and no client files.
    #[test]
    fn every_page_draws_and_back_from_the_first_leaves() {
        let ctx = egui::Context::default();
        let mut creation = Creation::new(ClientVersion::MODERN, CharacterChoices::default());
        let files = CreationFiles::default();
        for step in [Step::Look, Step::Profession(None), Step::Trade, Step::Town] {
            creation.step = step;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let body = ui.max_rect();
                    assert_eq!(draw(ui, body, &mut creation, &files, None), None);
                });
            });
        }
        creation.step = Step::Look;
        assert!(creation.back());
        assert_eq!(Race::Human.words(), "Human");
    }

    /// Draws the look with the real client files: the figure and the
    /// colors come from them.
    #[test]
    fn the_look_draws_its_figure_on_the_client_files() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        let ctx = egui::Context::default();
        let mut scene = Scene::new(Some(&dir));
        let files = CreationFiles::read(Some(&dir));
        let mut creation = Creation::new(ClientVersion::MODERN, CharacterChoices::default());
        let input = egui::RawInput {
            max_texture_side: Some(ART_TEXTURE_SIDE),
            ..egui::RawInput::default()
        };
        let _ = ctx.run(input, |ctx| {
            scene.make_atlas(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                let body = ui.max_rect();
                assert_eq!(
                    draw(ui, body, &mut creation, &files, Some(&mut scene)),
                    None
                );
            });
        });
        assert!(scene.doll_picture(FIGURE_MAP, &creation.look()).is_some());
        assert!(!files.professions.top().is_empty());
    }
}
