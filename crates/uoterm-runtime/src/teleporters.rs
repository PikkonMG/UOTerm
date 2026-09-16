//! What the character has learned about teleporter pads: stepping onto one
//! tile puts her on another, a jump no walk crosses. The map files hold no
//! teleporters, and dungeons like Despise join their chambers only through
//! them, so a pad she cannot cross leaves a goal unreachable on foot.
//!
//! She learns a pad the honest way: by using it. When a step onto a tile is
//! answered by the server moving her a long way on the same map, that tile is
//! a pad and where she landed is its far side. Once learned, a walk can leg
//! through it: reach the pad, step on, and plan on from the far side.

use uoterm_nav::same_spot;
use uoterm_protocol::Point3;

/// One learned pad: stepping onto `from` on map `map` lands her on `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeleportEdge {
    pub map: u8,
    pub from: Point3,
    pub to: Point3,
}

/// Every pad the character has learned this session.
#[derive(Clone, Debug, Default)]
pub struct TeleportMemory {
    edges: Vec<TeleportEdge>,
}

impl TeleportMemory {
    /// Records that stepping onto `from` lands on `to`. A pad's far side is
    /// fixed, so a second landing from the same tile updates the one edge
    /// rather than adding another.
    pub fn learn(&mut self, map: u8, from: Point3, to: Point3) {
        if let Some(edge) = self
            .edges
            .iter_mut()
            .find(|e| e.map == map && same_spot(e.from, from))
        {
            edge.to = to;
            return;
        }
        self.edges.push(TeleportEdge { map, from, to });
    }

    /// The learned pads on one map.
    pub fn on_map(&self, map: u8) -> impl Iterator<Item = &TeleportEdge> {
        self.edges.iter().filter(move |e| e.map == map)
    }

    /// How many pads are learned.
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// True when no pad is learned.
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: u16, y: u16, z: i8) -> Point3 {
        Point3::new(x, y, z)
    }

    #[test]
    fn a_learned_pad_is_kept_and_found_by_map() {
        let mut mem = TeleportMemory::default();
        mem.learn(0, p(5588, 632, 30), p(5503, 570, 51));
        let found: Vec<_> = mem.on_map(0).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].from, p(5588, 632, 30));
        assert_eq!(found[0].to, p(5503, 570, 51));
        assert_eq!(mem.on_map(1).count(), 0, "not on another map");
    }

    #[test]
    fn learning_the_same_pad_again_updates_the_far_side() {
        let mut mem = TeleportMemory::default();
        mem.learn(0, p(10, 10, 0), p(90, 90, 0));
        mem.learn(0, p(10, 10, 0), p(90, 91, 0));
        assert_eq!(mem.len(), 1, "one pad, not two");
        assert_eq!(mem.on_map(0).next().unwrap().to, p(90, 91, 0));
    }
}
