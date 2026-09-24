//! The color picker of the classic client:
//! a grid of hues with a slider that shifts it, a dye tub that shows the
//! picked hue, an eyedropper that takes the hue of a thing the player
//! clicks in the world or in a gump, and an Okay button. Okay hands the hue
//! to the gump that asked for it under its key, or answers the dye tub the
//! shard asked for a colour with (packet 0x95). The dye picker does not
//! close by a right click, and shows in both styles.

use super::canvas::{ButtonArt, Canvas, SliderStyle};
use super::manager::GumpManager;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use crate::view::WatchFrame;
use crate::window::actions::LocalAim;
use crate::window::control::Act;
use crate::window::model::hue_grid::{
    grid_hue, hue_of, HuePick, BAD_HUE_CLILOC, BAD_HUE_WORDS, GRADUATION_MAX, GRADUATION_MIN,
    GRID_COLUMNS, GRID_ROWS,
};
use crate::window::settings::Profile;
use eframe::egui::Color32;

pub const HUE_PICKER: GumpKind = GumpKind {
    id: well_known::HUE_PICKER,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(HuePicker::new(String::new(), 0)),
};

/// The id of the picker a dye tub opens, by the serial of the tub.
const DYE_PICKER_ID: &str = "dye_picker";

pub const DYE_PICKER: GumpKind = GumpKind {
    id: DYE_PICKER_ID,
    rules: GumpRules {
        right_click_closes: false,
        both_styles: true,
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(HuePicker::for_dye(serial.unwrap_or_default())),
};

/// Opens the picker of the dye tub that asks for a colour.
pub fn sync_dye(manager: &mut GumpManager, frame: &WatchFrame, profile: &mut Profile) {
    if let Some(dye) = &frame.dye {
        let id = GumpId::of(DYE_PICKER.id, dye.serial);
        if !manager.is_open(&id) {
            manager.open(id, profile);
        }
    }
}

/// Who the picked hue goes to.
enum Asker {
    /// A gump, under this key.
    Key(String),
    /// The dye tub of this serial.
    Dye(u32),
}

const BACKGROUND: u16 = 0x0906;
const OKAY: ButtonArt = ButtonArt::new(0x0907, 0x0908, 0x0909);
const OKAY_AT: (i32, i32) = (208, 138);
const SLIDER_AT: (i32, i32) = (39, 142);
const SLIDER_WIDTH: i32 = 145;
const GRID_AT: (i32, i32) = (34, 34);
const CELL: i32 = 8;
const DYE_TUB: u16 = 0x0FAB;
const DYE_TUB_AT: (i32, i32) = (200, 78);
const EYEDROPPER: ButtonArt = ButtonArt::new(0x5669, 0x566B, 0x566A);
const EYEDROPPER_AT: (i32, i32) = (212, 33);
const MARK_SIDE: i32 = 2;
const HALF: i32 = 2;

pub struct HuePicker {
    asker: Asker,
    pick: HuePick,
    /// The eyedropper waits for a click on a thing.
    picking: bool,
}

impl HuePicker {
    /// A picker that starts at `hue` and answers under `key`.
    pub fn new(key: String, hue: u16) -> Self {
        Self::asked_by(Asker::Key(key), hue)
    }

    /// The picker of the dye tub of `tub`.
    pub fn for_dye(tub: u32) -> Self {
        Self::asked_by(Asker::Dye(tub), 0)
    }

    fn asked_by(asker: Asker, hue: u16) -> Self {
        Self {
            asker,
            pick: HuePick::of(hue),
            picking: false,
        }
    }

    /// Takes the hue of the thing the eyedropper clicked, when the grid
    /// shows it.
    fn take_picked(&mut self, g: &Canvas<'_>, cx: &GumpContext<'_>) {
        if !self.picking {
            return;
        }
        if cx.hand.aiming() != Some(LocalAim::PickThing) {
            self.picking = false;
        }
        let Some(serial) = cx.hand.take_picked(LocalAim::PickThing) else {
            return;
        };
        self.picking = false;
        if !self.pick.take(hue_of(cx.frame, serial)) {
            cx.hand.report(&g.words(BAD_HUE_CLILOC, BAD_HUE_WORDS));
        }
    }

    fn picked(&self) -> u16 {
        self.pick.hue()
    }
}

impl GumpBody for HuePicker {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        g.pic(0, 0, BACKGROUND, 0);
        let (left, top) = GRID_AT;
        for row in 0..GRID_ROWS {
            for column in 0..GRID_COLUMNS {
                let index = row * GRID_COLUMNS + column;
                let (x, y) = (left + column as i32 * CELL, top + row as i32 * CELL);
                g.hue_box(x, y, CELL, CELL, grid_hue(self.pick.graduation, index));
                if g.click_area(("cell", index), x, y, CELL, CELL).clicked() {
                    self.pick.index = index;
                }
            }
        }
        let (row, column) = (
            self.pick.index / GRID_COLUMNS,
            self.pick.index % GRID_COLUMNS,
        );
        let mark = (
            left + column as i32 * CELL + CELL / HALF - MARK_SIDE / HALF,
            top + row as i32 * CELL + CELL / HALF - MARK_SIDE / HALF,
        );
        g.fill(mark.0, mark.1, MARK_SIDE, MARK_SIDE, Color32::WHITE);
        g.slider(
            "graduation",
            SLIDER_AT.0,
            SLIDER_AT.1,
            SLIDER_WIDTH,
            (GRADUATION_MIN, GRADUATION_MAX),
            &mut self.pick.graduation,
            SliderStyle::BlueKnob,
        );
        g.item(DYE_TUB_AT.0, DYE_TUB_AT.1, DYE_TUB, self.picked());
        if g.button("eyedropper", EYEDROPPER_AT.0, EYEDROPPER_AT.1, EYEDROPPER) {
            if cx.frame.target_cursor {
                cx.act(Act::CancelTarget);
            }
            cx.hand.aim(LocalAim::PickThing);
            self.picking = true;
        }
        self.take_picked(g, cx);
        if g.button("okay", OKAY_AT.0, OKAY_AT.1, OKAY) {
            match &self.asker {
                Asker::Key(key) => cx.answer(key, self.picked()),
                Asker::Dye(_) => cx.act(Act::Dye(self.picked())),
            }
            cx.close(cx.me);
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        match self.asker {
            Asker::Key(_) => true,
            Asker::Dye(tub) => frame.dye.as_ref().is_some_and(|dye| dye.serial == tub),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picker_starts_at_its_hue() {
        assert_eq!(HuePicker::new(String::new(), 1001).picked(), 1001);
    }

    #[test]
    fn the_dye_picker_lives_with_its_tub() {
        use crate::view::{WatchContainer, WatchDye, WatchItem, WatchPackItem};
        const TUB: u32 = 0x4000_0100;
        const SHIRT: u32 = 0x4000_0200;
        let frame = WatchFrame {
            items: vec![WatchItem {
                serial: TUB,
                hue: 13,
                ..WatchItem::default()
            }],
            containers: vec![WatchContainer {
                items: vec![WatchPackItem {
                    serial: SHIRT,
                    hue: 23,
                    ..WatchPackItem::default()
                }],
                ..WatchContainer::default()
            }],
            dye: Some(WatchDye {
                serial: TUB,
                graphic: 0x0FAB,
            }),
            ..WatchFrame::default()
        };
        assert!(HuePicker::for_dye(TUB).alive(&frame));
        assert!(!HuePicker::for_dye(SHIRT).alive(&frame));
        assert!(HuePicker::new(String::new(), 0).alive(&WatchFrame::default()));
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        sync_dye(&mut manager, &frame, &mut profile);
        assert!(manager.is_open(&GumpId::of(DYE_PICKER.id, TUB)));
    }
}
