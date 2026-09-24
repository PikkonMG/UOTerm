//! The durability of what the character wears, from the property lines of
//! each item, for the durability window and the bars of the paperdoll.

use super::properties;
use super::reads::Readings;
use crate::view::{WatchEquip, WatchFrame};
use crate::window::deck_ui::is_worn_layer;

const PERCENT: f32 = 100.0;

/// One worn item that has a durability.
#[derive(Clone, Debug, PartialEq)]
pub struct Wear {
    pub serial: u32,
    pub layer: u8,
    pub graphic: u16,
    pub hue: u16,
    /// The first property line: the name of the item.
    pub name: String,
    pub now: u16,
    pub max: u16,
}

impl Wear {
    /// The share of durability left, from 0 to 1.
    pub fn share(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            (f32::from(self.now) / f32::from(self.max)).clamp(0.0, 1.0)
        }
    }

    /// True when the durability left is under the warning percent.
    pub fn warns(&self, warning_percent: u8) -> bool {
        self.share() * PERCENT < f32::from(warning_percent)
    }
}

/// Each worn item whose lines name a durability, the most worn first.
/// `lines_of` gives the property lines of an item.
pub fn worn_durability(
    worn: &[&WatchEquip],
    mut lines_of: impl FnMut(u32) -> Vec<String>,
) -> Vec<Wear> {
    let mut wears: Vec<Wear> = worn
        .iter()
        .filter_map(|item| {
            let lines = lines_of(item.serial);
            let (now, max) = properties::durability(&lines)?;
            Some(Wear {
                serial: item.serial,
                layer: item.layer,
                graphic: item.graphic,
                hue: item.hue,
                name: lines.first().cloned().unwrap_or_default(),
                now,
                max,
            })
        })
        .collect();
    wears.sort_by(|a, b| a.share().total_cmp(&b.share()));
    wears
}

/// The items the character wears that have a durability, from their
/// property lines, which the session is asked for.
pub fn worn_wear(frame: &WatchFrame, readings: &mut Readings) -> Vec<Wear> {
    let worn: Vec<&WatchEquip> = frame
        .look
        .equipment
        .iter()
        .filter(|item| is_worn_layer(item.layer))
        .collect();
    worn_durability(&worn, |serial| properties::lines_of(readings, serial))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equip(serial: u32, layer: u8) -> WatchEquip {
        WatchEquip {
            serial,
            layer,
            ..WatchEquip::default()
        }
    }

    #[test]
    fn worn_items_with_a_durability_come_most_worn_first() {
        let (sword, shirt, helm) = (equip(1, 1), equip(2, 5), equip(3, 6));
        let worn = vec![&sword, &shirt, &helm];
        let wears = worn_durability(&worn, |serial| match serial {
            1 => vec!["a katana".into(), "Durability 40 / 50".into()],
            3 => vec!["a helm".into(), "Durability 5 / 50".into()],
            _ => vec!["a shirt".into()],
        });
        let names: Vec<&str> = wears.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, vec!["a helm", "a katana"]);
        assert!(wears[0].warns(20) && !wears[1].warns(20));
        assert!((wears[1].share() - 0.8).abs() < f32::EPSILON);
    }
}
