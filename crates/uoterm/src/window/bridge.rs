//! The edge between egui and the shared rules: each egui type the window
//! holds becomes the plain type of `uoterm-view` here, and back.

// The window moves its rules to `uoterm-view` task by task, and each move
// starts to call these. Until all of them are called, some stay unused
// outside the tests, which call every one. The expectation fails once
// every function is used: then remove it.
#![cfg_attr(not(test), expect(dead_code))]

use eframe::egui;
use uoterm_view::geom::{Area, Point, Rgba, Vector};
use uoterm_view::input::{KeyName, Mods};

pub fn point(position: egui::Pos2) -> Point {
    Point::new(position.x, position.y)
}

pub fn pos2(point: Point) -> egui::Pos2 {
    egui::pos2(point.x, point.y)
}

pub fn vector(vec: egui::Vec2) -> Vector {
    Vector::new(vec.x, vec.y)
}

pub fn vec2(vector: Vector) -> egui::Vec2 {
    egui::vec2(vector.x, vector.y)
}

pub fn area(rect: egui::Rect) -> Area {
    Area {
        min: point(rect.min),
        max: point(rect.max),
    }
}

pub fn rect(area: Area) -> egui::Rect {
    egui::Rect::from_min_max(pos2(area.min), pos2(area.max))
}

pub fn rgba(color: egui::Color32) -> Rgba {
    let [r, g, b, a] = color.to_array();
    Rgba::from_rgba_premultiplied(r, g, b, a)
}

pub fn color(rgba: Rgba) -> egui::Color32 {
    let [r, g, b, a] = rgba.to_array();
    egui::Color32::from_rgba_premultiplied(r, g, b, a)
}

pub fn mods(modifiers: egui::Modifiers) -> Mods {
    Mods {
        ctrl: modifiers.ctrl,
        alt: modifiers.alt,
        shift: modifiers.shift,
        command: modifiers.command,
    }
}

/// The egui modifiers of plain ones. The Command key of a Mac is
/// `command`, which egui matches on every system.
pub fn modifiers(mods: Mods) -> egui::Modifiers {
    egui::Modifiers {
        alt: mods.alt,
        ctrl: mods.ctrl,
        shift: mods.shift,
        mac_cmd: false,
        command: mods.command,
    }
}

pub fn key_name(key: egui::Key) -> KeyName {
    KeyName(key.name().to_owned())
}

pub fn egui_key(name: &KeyName) -> Option<egui::Key> {
    egui::Key::from_name(&name.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    #[test]
    fn a_rect_survives_the_round_trip() {
        let original = egui::Rect::from_min_max(egui::pos2(1.0, 2.0), egui::pos2(30.0, 40.0));
        assert_eq!(rect(area(original)), original);
    }

    #[test]
    fn a_key_keeps_its_egui_name() {
        assert_eq!(key_name(egui::Key::F1).0, "F1");
        assert_eq!(
            egui_key(&KeyName("ArrowUp".into())),
            Some(egui::Key::ArrowUp)
        );
    }

    #[test]
    fn modifiers_survive_the_round_trip() {
        let original = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
        assert_eq!(modifiers(mods(original)), original);
    }

    #[test]
    fn a_color_survives_the_round_trip() {
        let original = egui::Color32::from_rgba_premultiplied(10, 20, 30, 40);
        assert_eq!(color(rgba(original)), original);
    }
}
