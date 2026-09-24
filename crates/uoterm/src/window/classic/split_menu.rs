//! The split menu of the classic client: a
//! pile the player drags asks how much of it to take, with a slider and a
//! box of digits. Okay, or Enter, puts that much of the pile on the mouse.

use super::canvas::{ButtonArt, Canvas, SliderStyle};
use super::registry::{GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::WatchFrame;
use crate::view::WatchPackItem;
use eframe::egui::{Pos2, Vec2};

/// The id of the split menu of one pile, by the serial of the pile.
const SPLIT_MENU_ID: &str = "split_menu";

pub const SPLIT_MENU: GumpKind = GumpKind {
    id: SPLIT_MENU_ID,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| {
        Box::new(SplitMenu::new(
            WatchPackItem {
                serial: serial.unwrap_or_default(),
                ..WatchPackItem::default()
            },
            Vec2::ZERO,
            Pos2::ZERO,
        ))
    },
};

const BACKGROUND: u16 = 0x085C;
const OKAY: ButtonArt = ButtonArt::new(0x085D, 0x085E, 0x085F);
const OKAY_AT: (i32, i32) = (102, 37);
const SLIDER_AT: (i32, i32) = (29, 16);
const SLIDER_WIDTH: i32 = 105;
const FIELD_AT: (i32, i32) = (29, 42);
const FIELD_SIZE: (i32, i32) = (60, 20);
const FIELD_FONT: u8 = 1;
const FIELD_HUE: u16 = 0x0386;
/// The menu opens with the mouse this far into it.
const MOUSE_IN_MENU: Vec2 = Vec2::new(80.0, 40.0);
const LEAST: i32 = 1;

/// The amount the digits of the box ask for, held to the pile.
fn typed_amount(words: &str, most: i32) -> Option<i32> {
    words
        .parse::<i32>()
        .ok()
        .map(|amount| amount.clamp(LEAST, most))
}

pub struct SplitMenu {
    item: WatchPackItem,
    /// Where the item is held against the mouse once it is taken.
    grab: Vec2,
    /// Where the menu opens, in window points.
    place: Option<Pos2>,
    amount: i32,
    field: TextField,
    focused: bool,
}

impl SplitMenu {
    /// The menu for a pile the player grabbed with the mouse at `mouse`.
    pub fn new(item: WatchPackItem, grab: Vec2, mouse: Pos2) -> Self {
        let amount = i32::from(item.amount.max(1));
        let mut field = TextField::new(&amount.to_string());
        field.numeric = true;
        Self {
            item,
            grab,
            place: Some(mouse - MOUSE_IN_MENU),
            amount,
            field,
            focused: false,
        }
    }

    fn most(&self) -> i32 {
        i32::from(self.item.amount.max(1))
    }

    /// Puts the amount on the mouse.
    fn take(&self, cx: &mut GumpContext<'_>) {
        let amount = u16::try_from(self.amount.clamp(LEAST, self.most())).unwrap_or(1);
        let taken = WatchPackItem {
            amount,
            ..self.item.clone()
        };
        cx.desk.pick_up_at(&taken, self.grab);
        cx.close(cx.me);
    }
}

impl GumpBody for SplitMenu {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if let Some(place) = self.place.take() {
            g.move_to(place);
        }
        g.pic(0, 0, BACKGROUND, 0);
        let most = self.most();
        if g.slider(
            "amount",
            SLIDER_AT.0,
            SLIDER_AT.1,
            SLIDER_WIDTH,
            (LEAST, most),
            &mut self.amount,
            SliderStyle::BlueKnob,
        ) {
            self.field.set_text(&self.amount.to_string());
        }
        let look = TextLook::ascii(FIELD_FONT, FIELD_HUE);
        let typed = g.text_box(
            "digits",
            FIELD_AT.0,
            FIELD_AT.1,
            FIELD_SIZE.0,
            FIELD_SIZE.1,
            &mut self.field,
            &look,
        );
        // Asked for after the field draws, so an unseen first frame keeps
        // them.
        if !self.focused {
            g.focus("digits");
            self.focused = true;
        }
        if typed.changed {
            match typed_amount(self.field.text(), most) {
                Some(amount) => self.amount = amount,
                None if self.field.text().is_empty() => self.amount = LEAST,
                None => self.field.set_text(&self.amount.to_string()),
            }
        }
        if g.button("okay", OKAY_AT.0, OKAY_AT.1, OKAY) || typed.submitted {
            self.take(cx);
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame
            .containers
            .iter()
            .flat_map(|container| &container.items)
            .any(|item| item.serial == self.item.serial)
            || frame
                .items
                .iter()
                .any(|item| item.serial == self.item.serial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_digits_are_held_to_the_pile() {
        assert_eq!(typed_amount("12", 50), Some(12));
        assert_eq!(typed_amount("80", 50), Some(50));
        assert_eq!(typed_amount("0", 50), Some(LEAST));
        assert_eq!(typed_amount("", 50), None);
    }

    #[test]
    fn the_menu_opens_under_the_mouse_with_the_whole_pile() {
        let item = WatchPackItem {
            serial: 9,
            amount: 30,
            ..WatchPackItem::default()
        };
        let menu = SplitMenu::new(item, Vec2::ZERO, Pos2::new(200.0, 100.0));
        assert_eq!(menu.place, Some(Pos2::new(120.0, 60.0)));
        assert_eq!(menu.amount, 30);
        assert_eq!(menu.field.text(), "30");
    }
}
