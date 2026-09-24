//! Gumps that snap together, as the reference client joins health
//! bars: a gump dropped on another of its anchor group takes the free side
//! of it that it came from, and the joined gumps move as one. Each member
//! has a cell in the grid of its group. The profile keeps the cells, so a
//! group joins again when its gumps open at the next start.

use super::registry::GumpId;
use crate::window::settings::AnchorCell;
use eframe::egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;

/// A side of a gump: left, up, right or down, as a step in the grid.
type Step = (i32, i32);

#[derive(Default)]
pub struct Anchors {
    /// The group and the cell of each joined gump.
    cells: HashMap<GumpId, (u32, (i32, i32))>,
    next_group: u32,
}

/// The side of `host` a gump dropped at `dragged` joins, as the classic
/// client picks it: the axis on which the two are further apart, as a
/// share of the host's size.
fn side(dragged: Rect, host: Rect) -> Step {
    let across = (dragged.left() - host.left()).abs() / host.width();
    let down = (dragged.top() - host.top()).abs() / host.height();
    if across > down {
        if dragged.left() > host.left() {
            (1, 0)
        } else {
            (-1, 0)
        }
    } else if dragged.top() > host.top() {
        (0, 1)
    } else {
        (0, -1)
    }
}

impl Anchors {
    /// True when another gump is joined to this one. A gump put back in
    /// its kept cell is alone until a partner opens.
    pub fn is_joined(&self, id: &GumpId) -> bool {
        !self.partners(id).is_empty()
    }

    /// The group and the cell of a gump in a group, to keep.
    pub fn cell(&self, id: &GumpId) -> Option<AnchorCell> {
        self.cells.get(id).map(|(group, (column, row))| AnchorCell {
            group: *group,
            column: *column,
            row: *row,
        })
    }

    /// Puts a gump back in the cell the profile kept for it.
    pub fn restore(&mut self, id: GumpId, cell: AnchorCell) {
        self.next_group = self.next_group.max(cell.group);
        self.cells.insert(id, (cell.group, (cell.column, cell.row)));
    }

    /// The other gumps joined to this one.
    pub fn partners(&self, id: &GumpId) -> Vec<GumpId> {
        let Some((group, _)) = self.cells.get(id) else {
            return Vec::new();
        };
        self.cells
            .iter()
            .filter(|(other, (their, _))| *other != id && their == group)
            .map(|(other, _)| *other)
            .collect()
    }

    /// Where a free gump dropped at `dragged` goes to join `host`, or None
    /// when that side of the host is taken.
    pub fn drop_place(&self, dragged: (&GumpId, Rect), host: (&GumpId, Rect)) -> Option<Pos2> {
        if self.is_joined(dragged.0) {
            return None;
        }
        let step = side(dragged.1, host.1);
        if let Some((group, (x, y))) = self.cells.get(host.0) {
            let target = (x + step.0, y + step.1);
            let taken = self
                .cells
                .values()
                .any(|(their, cell)| their == group && *cell == target);
            if taken {
                return None;
            }
        }
        // A gump to the right or below moves by the host's size; one to the
        // left or above by its own.
        let size = if step.0 > 0 || step.1 > 0 {
            host.1.size()
        } else {
            dragged.1.size()
        };
        Some(host.1.min + Vec2::new(step.0 as f32 * size.x, step.1 as f32 * size.y))
    }

    /// Joins a free gump to `host`. Gives where it goes, or None when it
    /// cannot join.
    pub fn join(&mut self, dragged: (&GumpId, Rect), host: (&GumpId, Rect)) -> Option<Pos2> {
        let place = self.drop_place(dragged, host)?;
        let step = side(dragged.1, host.1);
        let (group, (x, y)) = match self.cells.get(host.0) {
            Some(known) => *known,
            None => {
                self.next_group += 1;
                let fresh = (self.next_group, (0, 0));
                self.cells.insert(*host.0, fresh);
                fresh
            }
        };
        self.cells
            .insert(*dragged.0, (group, (x + step.0, y + step.1)));
        Some(place)
    }

    /// Takes a gump out of its group. A group of one left is no group.
    pub fn leave(&mut self, id: &GumpId) {
        let Some((group, _)) = self.cells.remove(id) else {
            return;
        };
        let left: Vec<GumpId> = self
            .cells
            .iter()
            .filter(|(_, (their, _))| *their == group)
            .map(|(other, _)| *other)
            .collect();
        if left.len() == 1 {
            self.cells.remove(&left[0]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAR: Vec2 = Vec2::new(120.0, 40.0);
    const A: GumpId = GumpId::of("health_bar", 1);
    const B: GumpId = GumpId::of("health_bar", 2);
    const C: GumpId = GumpId::of("health_bar", 3);

    fn at(x: f32, y: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), BAR)
    }

    #[test]
    fn a_gump_dropped_on_the_right_part_joins_the_right_side() {
        let mut anchors = Anchors::default();
        let place = anchors.join((&B, at(190.0, 105.0)), (&A, at(100.0, 100.0)));
        assert_eq!(place, Some(Pos2::new(220.0, 100.0)));
        assert!(anchors.is_joined(&A) && anchors.is_joined(&B));
        assert_eq!(anchors.partners(&A), vec![B]);
    }

    #[test]
    fn a_gump_dropped_above_moves_by_its_own_height_and_a_taken_side_refuses() {
        let mut anchors = Anchors::default();
        let place = anchors.join((&B, at(100.0, 80.0)), (&A, at(100.0, 100.0)));
        assert_eq!(place, Some(Pos2::new(100.0, 60.0)));
        assert_eq!(
            anchors.join((&C, at(105.0, 90.0)), (&A, at(100.0, 100.0))),
            None
        );
        assert!(anchors
            .join((&C, at(100.0, 130.0)), (&A, at(100.0, 100.0)))
            .is_some());
        assert_eq!(anchors.partners(&C).len(), 2);
    }

    #[test]
    fn a_kept_group_joins_again_and_a_new_group_takes_a_new_number() {
        let mut kept = Anchors::default();
        kept.join((&B, at(190.0, 100.0)), (&A, at(100.0, 100.0)));
        let (a, b) = (kept.cell(&A).unwrap(), kept.cell(&B).unwrap());
        let mut anchors = Anchors::default();
        anchors.restore(A, a);
        assert!(!anchors.is_joined(&A), "alone until B opens");
        anchors.restore(B, b);
        assert_eq!(anchors.partners(&A), vec![B]);
        const D: GumpId = GumpId::of("health_bar", 4);
        anchors.join((&D, at(100.0, 400.0)), (&C, at(100.0, 300.0)));
        assert_ne!(anchors.cell(&C).unwrap().group, a.group);
        assert_eq!(anchors.cell(&A), Some(a));
    }

    #[test]
    fn leaving_a_group_of_two_ends_the_group() {
        let mut anchors = Anchors::default();
        anchors.join((&B, at(190.0, 100.0)), (&A, at(100.0, 100.0)));
        anchors.leave(&B);
        assert!(!anchors.is_joined(&A) && !anchors.is_joined(&B));
        assert!(anchors
            .drop_place((&A, at(0.0, 0.0)), (&B, at(0.0, 60.0)))
            .is_some());
    }
}
