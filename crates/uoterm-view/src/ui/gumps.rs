//! The rules of the gumps and the item panels: the boxes a player ticks in
//! a gump of the shard, the pages, the hues of its pictures and words, the
//! wheel over a list, and a click on an item that asks its name or uses
//! it.

use crate::act::Act;
use crate::frame::{WatchFrame, WatchGump};
use crate::model::clicks::ClickDelay;
use std::collections::{HashMap, HashSet};
use uoterm_world::{GumpLayout, GumpPiece, GumpPieceKind};

/// The side of a cell of an item panel, and the room between two cells.
pub const CELL: f32 = 46.0;
pub const CELL_GAP: f32 = 4.0;
pub const FIRST_PAGE: u32 = 1;
/// The page of a gump whose pieces show on every page.
pub const EVERY_PAGE: u32 = 0;
/// A picture hue this low or lower shows the picture as it is.
const PLAIN_HUE_MAX: u16 = 2;

/// The boxes that are ticked now: the ticks the gump came with, with the
/// human's changes on top. A tick on a radio box clears the others of its page.
pub fn ticked(gump: &WatchGump, flipped: &HashSet<u32>) -> Vec<u32> {
    gump.choices
        .iter()
        .filter(|c| c.on != flipped.contains(&c.switch))
        .map(|c| c.switch)
        .collect()
}

pub fn flip(gump: &WatchGump, flipped: &mut HashSet<u32>, switch: u32) {
    let Some(clicked) = gump.choices.iter().find(|c| c.switch == switch) else {
        return;
    };
    let now_on = ticked(gump, flipped);
    let toggle = |flipped: &mut HashSet<u32>, switch: u32| {
        if !flipped.remove(&switch) {
            flipped.insert(switch);
        }
    };
    if !clicked.radio {
        toggle(flipped, switch);
        return;
    }
    if now_on.contains(&switch) {
        return;
    }
    let rivals = gump
        .choices
        .iter()
        .filter(|c| c.radio && c.page == clicked.page && now_on.contains(&c.switch));
    for rival in rivals {
        toggle(flipped, rival.switch);
    }
    toggle(flipped, switch);
}

/// The first row after the mouse wheel turned `turned` over a list: one
/// row down for a turn down, one up for a turn up, and never past
/// `last_first_row`.
pub fn next_first_row(first_row: usize, turned: f32, last_first_row: usize) -> usize {
    let next = if turned < 0.0 {
        first_row + 1
    } else if turned > 0.0 {
        first_row.saturating_sub(1)
    } else {
        first_row
    };
    next.min(last_first_row)
}

pub fn on_page(item_page: u32, shown: u32) -> bool {
    item_page == EVERY_PAGE || item_page == shown
}

/// A click on an item of a panel: under a target cursor a click targets
/// the item, and else a single click asks its name once the double click
/// time is over. Gives true on a double click, whose act is the panel's,
/// and the act of a single click.
pub fn single_or_double(
    (clicked, double_clicked): (bool, bool),
    clicks: &mut ClickDelay,
    frame: &WatchFrame,
    serial: u32,
    time: f64,
) -> (bool, Option<Act>) {
    if double_clicked {
        clicks.double_clicked();
        return (true, None);
    }
    let act = if clicked {
        clicks.single_click(frame, serial, time)
    } else {
        None
    };
    (false, act)
}

/// The look of the item whose single click waited long enough, and true
/// while a click still waits, so the window keeps drawing.
pub fn ask_waiting_name(clicks: &mut ClickDelay, time: f64) -> (Option<Act>, bool) {
    let act = clicks.due_look(time);
    (act, clicks.is_waiting())
}

/// The hue a gump picture or a line of words takes, from the number the
/// shard sent: the classic client counts one more.
pub fn shown_hue(hue: u16) -> u16 {
    hue.wrapping_add(1)
}

/// The hue of a gump picture, where the lowest hues show no hue.
pub fn picture_hue(hue: u16) -> u16 {
    let hue = shown_hue(hue);
    if hue <= PLAIN_HUE_MAX {
        0
    } else {
        hue
    }
}

/// The boxes of a gump layout that are ticked now: the ticks the gump came
/// with, with the ticks of the human on top.
pub fn layout_ticked(layout: &GumpLayout, ticks: &HashMap<u32, bool>) -> Vec<u32> {
    layout
        .pieces
        .iter()
        .filter_map(|piece| match piece.what {
            GumpPieceKind::Choice { switch, ticked, .. } => ticks
                .get(&switch)
                .copied()
                .unwrap_or(ticked)
                .then_some(switch),
            _ => None,
        })
        .collect()
}

/// A click on a box of a gump layout. A radio box clears the other radio
/// boxes of its group.
pub fn click_box(layout: &GumpLayout, ticks: &mut HashMap<u32, bool>, clicked: &GumpPiece) {
    let GumpPieceKind::Choice {
        switch,
        radio,
        group,
        ..
    } = clicked.what
    else {
        return;
    };
    let on = layout_ticked(layout, ticks).contains(&switch);
    if !radio {
        ticks.insert(switch, !on);
        return;
    }
    for piece in &layout.pieces {
        if let GumpPieceKind::Choice {
            switch: rival,
            radio: true,
            group: rival_group,
            ..
        } = piece.what
        {
            if rival_group == group {
                ticks.insert(rival, rival == switch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::WatchGumpChoice;

    fn choice(switch: u32, radio: bool, on: bool) -> WatchGumpChoice {
        WatchGumpChoice {
            switch,
            radio,
            on,
            page: FIRST_PAGE,
            label: String::new(),
        }
    }

    #[test]
    fn a_tick_on_a_radio_box_clears_its_rival() {
        let gump = WatchGump {
            choices: vec![
                choice(1, true, true),
                choice(2, true, false),
                choice(3, false, false),
            ],
            ..WatchGump::default()
        };
        let mut flipped = HashSet::new();
        assert_eq!(ticked(&gump, &flipped), vec![1]);
        flip(&gump, &mut flipped, 2);
        assert_eq!(ticked(&gump, &flipped), vec![2]);
        flip(&gump, &mut flipped, 2);
        assert_eq!(ticked(&gump, &flipped), vec![2], "a radio box stays on");
        flip(&gump, &mut flipped, 3);
        assert_eq!(ticked(&gump, &flipped), vec![2, 3]);
        flip(&gump, &mut flipped, 3);
        assert_eq!(ticked(&gump, &flipped), vec![2]);
    }

    #[test]
    fn the_wheel_turns_a_container_one_row_and_stops_at_its_ends() {
        assert_eq!(next_first_row(0, -1.0, 2), 1);
        assert_eq!(next_first_row(2, -1.0, 2), 2);
        assert_eq!(next_first_row(0, 1.0, 2), 0);
        assert_eq!(next_first_row(2, 0.0, 1), 1, "the container lost rows");
    }

    #[test]
    fn page_zero_shows_on_each_page() {
        assert!(on_page(EVERY_PAGE, 3));
        assert!(on_page(2, 2));
        assert!(!on_page(2, FIRST_PAGE));
    }

    #[test]
    fn a_double_click_is_the_panels_and_no_click_asks_nothing() {
        let mut clicks = ClickDelay::default();
        let frame = WatchFrame::default();
        assert_eq!(
            single_or_double((true, true), &mut clicks, &frame, 7, 0.0),
            (true, None)
        );
        assert_eq!(
            single_or_double((false, false), &mut clicks, &frame, 7, 0.0),
            (false, None)
        );
        assert_eq!(ask_waiting_name(&mut clicks, 0.0), (None, false));
    }

    fn layout_choice(page: u32, switch: u32, radio: bool, ticked: bool, group: u32) -> GumpPiece {
        GumpPiece {
            page,
            x: 0,
            y: 0,
            what: GumpPieceKind::Choice {
                off: 210,
                on: 211,
                switch,
                radio,
                ticked,
                group,
            },
            tooltip: None,
            property: None,
        }
    }

    #[test]
    fn a_radio_box_clears_the_radios_of_its_group_and_a_check_box_flips() {
        let layout = GumpLayout {
            pieces: vec![
                layout_choice(1, 1, true, true, 0),
                layout_choice(2, 2, true, false, 0),
                layout_choice(1, 3, false, false, 0),
                layout_choice(1, 4, true, true, 1),
            ],
            ..GumpLayout::default()
        };
        let mut ticks = HashMap::new();
        assert_eq!(layout_ticked(&layout, &ticks), vec![1, 4]);
        click_box(&layout, &mut ticks, &layout.pieces[1]);
        assert_eq!(
            layout_ticked(&layout, &ticks),
            vec![2, 4],
            "group 1 keeps its tick"
        );
        click_box(&layout, &mut ticks, &layout.pieces[2]);
        assert_eq!(layout_ticked(&layout, &ticks), vec![2, 3, 4]);
        click_box(&layout, &mut ticks, &layout.pieces[2]);
        assert_eq!(layout_ticked(&layout, &ticks), vec![2, 4]);
    }

    #[test]
    fn hues_count_one_more_and_low_picture_hues_show_none() {
        assert_eq!(shown_hue(0), 1);
        assert_eq!(shown_hue(0x0481), 0x0482);
        assert_eq!(picture_hue(0), 0);
        assert_eq!(picture_hue(1), 0);
        assert_eq!(picture_hue(2), 3);
    }
}
