//! The improved buff bar: the icon of each buff from the gump art of the
//! client, with the time it has left, the one that ends first on the left.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::buffs;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};

pub const BUFFS_ID: &str = "modern:buffs";
const ICON: f32 = 34.0;
const ICON_GAP: f32 = 4.0;
const TIME_ROW: f32 = 14.0;
/// The words of a buff with no picture show this many letters.
const SHORT_NAME_CHARS: usize = 4;
const WORDS_TITLE: &str = "Buffs";

/// Draws the bar while a buff is on. Gives its place.
pub fn draw(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> Option<Rect> {
    let ordered = buffs::in_order(&frame.buff_icons);
    if ordered.is_empty() {
        return None;
    }
    let show_time = profile.combat.buff_duration;
    let row = ICON + if show_time { TIME_ROW } else { 0.0 };
    let width = ordered.len() as f32 * (ICON + ICON_GAP) - ICON_GAP;
    let size = frame::with_title_room(
        Vec2::new(width, row + frame::TITLE_ROW) + Vec2::splat(theme::PANEL_PAD * 2.0),
    );
    let spec = PanelSpec {
        id: BUFFS_ID,
        title: WORDS_TITLE,
        default: layout::first_place(rect, Spot::MiddleTop(0), size),
        min_size: None,
        closable: false,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
    for (at, buff) in ordered.iter().enumerate() {
        let icon = Rect::from_min_size(
            body.left_top() + Vec2::new(at as f32 * (ICON + ICON_GAP), 0.0),
            Vec2::splat(ICON),
        );
        let response = ui.interact(icon, Id::new(("buff", buff.icon)), Sense::hover());
        let picture = buffs::gump_of(buff.icon).and_then(|gump| tools.scene.gump_picture(gump, 0));
        let painter = ui.painter();
        painter.rect_filled(icon, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        match picture {
            Some((texture, sprite)) => {
                painter.image(
                    texture,
                    theme::fit(icon, sprite.width, sprite.height),
                    sprite.uv,
                    Color32::WHITE,
                );
            }
            None => {
                let short: String = buff.title.chars().take(SHORT_NAME_CHARS).collect();
                painter.text(
                    icon.center(),
                    Align2::CENTER_CENTER,
                    short,
                    text_font(theme::SIZE_SMALL),
                    theme::TEXT,
                );
            }
        }
        if show_time {
            let color = if buffs::is_ending(buff) {
                theme::ALARM
            } else {
                theme::TEXT_DIM
            };
            painter.text(
                Pos2::new(icon.center().x, icon.bottom()),
                Align2::CENTER_TOP,
                buffs::time_words(buff),
                number_font(theme::SIZE_SMALL),
                color,
            );
        }
        if response.hovered() {
            super::super::tips::label(ui, &buff.title, &buff.text);
        }
    }
    frame::controls(ui, panel, &spec, profile, tools);
    Some(panel)
}
