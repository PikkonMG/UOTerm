//! The improved buff bar: the icon of each buff from the gump art of the
//! client, with the time it has left, the one that ends first on the left.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::buffs;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use uoterm_view::ui::buff_bar::{
    buffs_first_place, short_name, time_color, BUFFS_ID, BUFF_GAP as ICON_GAP, BUFF_ICON as ICON,
    WORDS_BUFFS as WORDS_TITLE,
};

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
    let spec = PanelSpec {
        id: BUFFS_ID,
        title: WORDS_TITLE,
        default: bridge::rect(buffs_first_place(
            bridge::area(rect),
            ordered.len(),
            profile.combat.buff_duration,
        )),
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
                    bridge::rect(sprite.uv),
                    Color32::WHITE,
                );
            }
            None => {
                painter.text(
                    icon.center(),
                    Align2::CENTER_CENTER,
                    short_name(buff),
                    text_font(theme::SIZE_SMALL),
                    theme::TEXT,
                );
            }
        }
        if profile.combat.buff_duration {
            let color = bridge::color(time_color(buff));
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
