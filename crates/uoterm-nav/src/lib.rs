//! Client-side map query and A* navigation.

mod cliloc;
mod mul;
mod multi;
mod path;
mod speech;
mod step;
mod tiles;
mod uop;

pub use cliloc::ClilocData;
pub use mul::{
    client_data_dir_from_env, infer_mul_blocks, map_block_dims, DoorTile, MapError, MapFiles,
    MulMap, ENV_TEST_UOPATH, TILEDATA_NAME,
};
pub use multi::{MultiData, MultiFiles, MultiPiece};
pub use path::{pathfind, pathfind_flat, Obstacles, Path, PathError, Step};
pub use speech::{SpeechData, KEYWORD_SPEECH_MIN_VERSION};
pub use step::{
    is_standing_surface, LandCorners, TileColumn, TilePiece, PERSON_HEIGHT, STEP_HEIGHT,
};
pub use tiles::{
    z_reachable, MockMap, StaticView, TileInfo, TileQuery, TILE_BRIDGE, TILE_DOOR, TILE_IMPASSABLE,
    TILE_SURFACE, TILE_WET,
};
pub use uop::{hash_filename, map_uop_name};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use uoterm_protocol::{ClientVersion, Direction, Point3};

    use crate::cliloc::{
        CLILOC_COMPRESSED_MARK, CLILOC_COMPRESSED_MARK_AT, CLILOC_ENU_NAMES, CLILOC_HEADER,
        CLILOC_RECORD_HEADER,
    };
    use crate::mul::{
        block_index, tiledata_is_hs, BLOCK_BYTES, BLOCK_HEADER, CELL_BYTES, COLUMN_CACHE_CAP,
        GROUP_HEADER, IDX_EMPTY, LAND_COUNT, LAND_GROUP, LAND_NAME_AFTER_FLAGS, LAND_RECORD_HS,
        LAND_RECORD_OLD, MAP_MALAS_BLOCKS_H, MAP_MALAS_BLOCKS_W, STAIDX_RECORD,
        STATIC_HEIGHT_BYTES_AFTER_FLAGS, STATIC_RECORD, STATIC_RECORD_OLD, TILEDATA_FLAGS_OLD,
        TILEDATA_HS_LEN, TILE_NAME_LEN,
    };
    use crate::multi::{
        multi_is_hs, MULTI_IDX_NAME, MULTI_IDX_RECORD, MULTI_MUL_LOOSE, MULTI_MUL_NAME,
        MULTI_RECORD_HS, MULTI_RECORD_OLD, MULTI_UOP_CLILOC, MULTI_UOP_HEADER, MULTI_UOP_LOOSE,
        MULTI_UOP_NAMES, MULTI_UOP_RECORD,
    };
    use crate::path::DIRS;
    use crate::speech::{SPEECH_MUL_NAMES, SPEECH_RECORD_HEADER};
    use crate::step::{footing, land_is_ignored, Footing};
    use crate::uop::multi_uop_name;
    use crate::uop::{UOP_COMPRESS_NONE, UOP_MAGIC};

    #[test]
    fn radar_walkable_floor_over_wet_is_dot() {
        let mut map = MockMap::new(3, 3);
        map.set_wet(1, 1, true);
        assert_eq!(map.tile(1, 1).radar_char(), '.');
        map.set_door(2, 1, true);
        assert_eq!(map.tile(2, 1).radar_char(), '+');
        map.set_block(0, 0, true);
        map.set_wet(0, 0, true);
        assert_eq!(map.tile(0, 0).radar_char(), '~');
    }

    #[test]
    fn path_around_wall() {
        let mut map = MockMap::new(16, 16);
        for y in 0..16 {
            map.set_block(5, y, true);
        }
        map.set_block(5, 8, false);
        let path = pathfind(
            &map,
            Point3::new(1, 8, 0),
            Point3::new(10, 8, 0),
            &Obstacles::NONE,
        )
        .unwrap();
        assert!(path.steps.iter().any(|s| s.x == 5 && s.y == 8));
        assert_eq!(path.steps.last().unwrap().x, 10);
    }

    #[test]
    fn pathfind_flat_goes_around_block() {
        const FLOOR_Z: i8 = 27;
        let mut map = MockMap::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                map.set_z(x, y, FLOOR_Z);
            }
        }
        map.set_block(1, 0, true);
        let path = pathfind_flat(
            &map,
            Point3::new(0, 0, FLOOR_Z),
            Point3::new(2, 0, FLOOR_Z),
            &Obstacles::NONE,
        )
        .unwrap();
        assert!(
            path.steps.iter().any(|s| s.x == 1 && s.y == 1),
            "{:?}",
            path.steps
        );
        assert_eq!(path.steps.last().map(|s| (s.x, s.y)), Some((2, 0)));
    }

    /// The flat search reads every tile from the height the walker starts at.
    /// A map with one floor answers that question the same way as a question
    /// with no height at all, so the route over the mock grid must not change.
    #[test]
    fn flat_path_on_one_floor_agrees_with_the_height_less_answer() {
        const WALKER_Z: i8 = 0;
        const CLIFF_X: u16 = 1;
        const CLIFF_Y: u16 = 1;
        /// Higher than one step climbs and lower than one storey, so the
        /// height-aware search refuses it and the flat search crosses it.
        const CLIFF_Z: i8 = 12;
        const BLOCK_X: u16 = 1;
        const BLOCK_Y: u16 = 0;
        let mut map = MockMap::new(3, 2);
        map.set_block(BLOCK_X, BLOCK_Y, true);
        map.set_z(CLIFF_X, CLIFF_Y, CLIFF_Z);
        let start = Point3::new(0, 0, WALKER_Z);
        let goal = Point3::new(2, 0, WALKER_Z);

        assert_eq!(
            pathfind(&map, start, goal, &Obstacles::NONE).unwrap_err(),
            PathError::Unreachable,
            "the height-aware search has no way past the block and up the cliff"
        );
        let path = pathfind_flat(&map, start, goal, &Obstacles::NONE)
            .expect("the flat search ignores the cliff");
        assert!(
            path.steps.iter().any(|s| s.x == CLIFF_X && s.y == CLIFF_Y),
            "the flat search still crosses the cliff: {:?}",
            path.steps
        );
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y)),
            Some((goal.x, goal.y)),
            "{:?}",
            path.steps
        );
        for step in &path.steps {
            assert_eq!(
                map.can_walk_from(WALKER_Z, step.x, step.y),
                map.can_walk(step.x, step.y),
                "one floor has one answer at {},{}",
                step.x,
                step.y
            );
            assert_eq!(
                step.z,
                map.tile_from(WALKER_Z, step.x, step.y).z,
                "the flat search records the ground of each step: {:?}",
                path.steps
            );
        }
    }

    #[test]
    fn pathfind_flat_ignores_z_cliff() {
        let mut map = MockMap::new(3, 1);
        map.set_z(1, 0, STEP_HEIGHT.saturating_add(4));
        let err = pathfind(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(2, 0, 0),
            &Obstacles::NONE,
        )
        .unwrap_err();
        assert_eq!(err, PathError::Unreachable);
        let path = pathfind_flat(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(2, 0, 0),
            &Obstacles::NONE,
        )
        .unwrap();
        assert_eq!(path.steps.last().unwrap().x, 2);
    }

    #[test]
    fn blocked_goal_fails() {
        let mut map = MockMap::new(8, 8);
        map.set_block(3, 3, true);
        let err = pathfind(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(3, 3, 0),
            &Obstacles::NONE,
        )
        .unwrap_err();
        assert!(matches!(err, PathError::Unreachable | PathError::BadGoal));
    }

    #[test]
    fn mobile_avoidance() {
        let map = MockMap::new(8, 8);
        let somebody = [Point3::new(2, 0, 0)];
        let path = pathfind(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(4, 0, 0),
            &Obstacles {
                soft: &somebody,
                hard: &[],
            },
        )
        .unwrap();
        assert!(!path.steps.iter().any(|s| s.x == 2 && s.y == 0));
    }

    /// Open ground the two kinds of obstacle are told apart on.
    const OBSTACLE_GRID: u16 = 8;
    const WALK_FROM: Point3 = Point3 { x: 0, y: 0, z: 0 };
    const WALK_TO: Point3 = Point3 { x: 4, y: 0, z: 0 };

    /// The whole of the difference between the two kinds is what each one
    /// means at the destination. A mobile standing where the walk ends has
    /// stepped off it by the time the walker arrives, so the walk is planned.
    /// A tile the server has already refused, or a wall of a building, is
    /// proven shut and stays shut, so the walk is refused and the caller is
    /// free to choose somewhere else. Without that the walker is sent into the
    /// very tile his own memory says will not take him.
    #[test]
    fn a_hard_obstacle_on_the_destination_refuses_the_walk_and_a_soft_one_does_not() {
        let map = MockMap::new(OBSTACLE_GRID, OBSTACLE_GRID);
        let on_the_spot = [WALK_TO];
        let path = pathfind(
            &map,
            WALK_FROM,
            WALK_TO,
            &Obstacles {
                soft: &on_the_spot,
                hard: &[],
            },
        )
        .expect("a person standing there is no reason to refuse the walk");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y)),
            Some((WALK_TO.x, WALK_TO.y)),
            "and the walk still ends on that tile: {:?}",
            path.steps
        );
        assert_eq!(
            pathfind(
                &map,
                WALK_FROM,
                WALK_TO,
                &Obstacles {
                    soft: &[],
                    hard: &on_the_spot,
                },
            )
            .unwrap_err(),
            PathError::BlockedGoal,
            "a tile proven shut is shut whether the walk crosses it or ends on it"
        );
    }

    /// The walker stands on the tile he starts from, whatever anything else
    /// says about it: the server refused him there a moment ago and has put
    /// him on it since, or a mobile is reported on the tile he is standing on.
    /// Neither may hold him where he is.
    #[test]
    fn the_tile_the_walker_stands_on_is_exempt_from_both_kinds() {
        let map = MockMap::new(OBSTACLE_GRID, OBSTACLE_GRID);
        let under_his_feet = [WALK_FROM];
        for obstacles in [
            Obstacles {
                soft: &under_his_feet,
                hard: &[],
            },
            Obstacles {
                soft: &[],
                hard: &under_his_feet,
            },
        ] {
            let path = pathfind(&map, WALK_FROM, WALK_TO, &obstacles)
                .expect("he walks off the tile he is standing on");
            assert_eq!(
                path.steps.last().map(|s| (s.x, s.y)),
                Some((WALK_TO.x, WALK_TO.y)),
                "{obstacles:?} left him with nowhere to go: {:?}",
                path.steps
            );
        }
    }

    /// On the way there the two kinds are the same thing: a corridor one tile
    /// wide is shut by either of them.
    #[test]
    fn either_kind_of_obstacle_shuts_a_corridor() {
        const CORRIDOR_Y: u16 = 1;
        let mut map = MockMap::new(OBSTACLE_GRID, 3);
        for x in 0..OBSTACLE_GRID {
            map.set_block(x, CORRIDOR_Y - 1, true);
            map.set_block(x, CORRIDOR_Y + 1, true);
        }
        let from = Point3::new(0, CORRIDOR_Y, 0);
        let to = Point3::new(WALK_TO.x, CORRIDOR_Y, 0);
        let midway = [Point3::new(2, CORRIDOR_Y, 0)];
        assert!(
            pathfind(&map, from, to, &Obstacles::NONE).is_ok(),
            "the corridor is open until something stands in it"
        );
        for obstacles in [
            Obstacles {
                soft: &midway,
                hard: &[],
            },
            Obstacles {
                soft: &[],
                hard: &midway,
            },
        ] {
            assert_eq!(
                pathfind(&map, from, to, &obstacles).unwrap_err(),
                PathError::Unreachable,
                "{obstacles:?} stands in the only way through"
            );
        }
    }

    #[test]
    fn step_height_blocks_cliff() {
        let mut map = MockMap::new(3, 1);
        map.set_z(1, 0, STEP_HEIGHT.saturating_add(4));
        let err = pathfind(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(2, 0, 0),
            &Obstacles::NONE,
        )
        .unwrap_err();
        assert_eq!(err, PathError::Unreachable);
    }

    /// A stair rises one [`STEP_HEIGHT`] a tile, and a walk climbs it.
    #[test]
    fn step_height_allows_stair() {
        const STAIR_TILES: u16 = 4;
        let mut map = MockMap::new(8, 8);
        for x in 0..STAIR_TILES {
            map.set_z(x, 0, (x as i8) * STEP_HEIGHT);
        }
        let top = (STAIR_TILES as i8 - 1) * STEP_HEIGHT;
        let path = pathfind(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(STAIR_TILES - 1, 0, top),
            &Obstacles::NONE,
        )
        .unwrap();
        assert_eq!(path.steps.last().unwrap().z, top);
    }

    /// The rise of the ramp measured beside the Britain bank: two height units
    /// for every tile walked.
    const RAMP_RISE: i8 = 2;
    /// A ramp long enough that its top stands further above its foot than one
    /// step, which is the climb the walker could not plan.
    const RAMP_TILES: u16 = 12;
    /// One tile wide, so the only way to the top is straight up it.
    const RAMP_WIDTH: u16 = 1;
    const RAMP_COLUMN: u16 = 0;

    /// The height of the ramp `y` tiles from its foot.
    fn ramp_z(y: u16) -> i8 {
        (y as i16 * i16::from(RAMP_RISE)) as i8
    }

    /// A ramp one tile wide that climbs [`RAMP_RISE`] for every tile south,
    /// the shape measured beside the Britain bank.
    fn ramp() -> MockMap {
        let mut map = MockMap::new(RAMP_WIDTH, RAMP_TILES);
        for y in 0..RAMP_TILES {
            map.set_z(RAMP_COLUMN, y, ramp_z(y));
        }
        map
    }

    /// The rule the goal check turns on. A tile at the top of a ramp is one a
    /// person can walk to, and the same tile asked about from the height the
    /// walker starts at is no ground at all. Ask with the wrong height and the
    /// whole search is refused before it begins.
    #[test]
    fn a_goal_up_a_ramp_is_judged_from_its_own_height() {
        let map = ramp();
        let top_y = RAMP_TILES - 1;
        let start = Point3::new(RAMP_COLUMN, 0, ramp_z(0));
        let goal = Point3::new(RAMP_COLUMN, top_y, ramp_z(top_y));
        assert!(
            goal.z > STEP_HEIGHT,
            "the top must stand more than one step above the foot, or the test proves nothing"
        );
        assert!(
            !map.can_walk_from(start.z, goal.x, goal.y),
            "asked from the walker's own height the top of the ramp is no ground at all"
        );
        assert!(
            map.can_walk_from(goal.z, goal.x, goal.y),
            "asked from its own height it is ground a person stands on"
        );

        let path = pathfind(&map, start, goal, &Obstacles::NONE).expect("a way up the ramp");
        let mut climbed = start.z;
        for step in &path.steps {
            assert_eq!(
                step.z,
                ramp_z(step.y),
                "every step stands on the ground of its own tile: {:?}",
                path.steps
            );
            assert_eq!(
                i16::from(step.z) - i16::from(climbed),
                i16::from(RAMP_RISE),
                "and rises with the ground: {:?}",
                path.steps
            );
            climbed = step.z;
        }
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y, s.z)),
            Some((goal.x, goal.y, goal.z)),
            "{:?}",
            path.steps
        );
    }

    /// The height the goal is judged from is the goal's own, and that is all
    /// that changed: a tile nobody can reach is refused as it always was.
    #[test]
    fn a_goal_nothing_can_reach_is_still_refused() {
        const CLIFF_TILES: u16 = 3;
        const CLIFF_Y: u16 = 2;
        let cliff_z = STEP_HEIGHT.saturating_add(RAMP_RISE);
        let mut steep = MockMap::new(RAMP_WIDTH, CLIFF_TILES);
        steep.set_z(RAMP_COLUMN, CLIFF_Y, cliff_z);
        let start = Point3::new(RAMP_COLUMN, 0, 0);
        let top = Point3::new(RAMP_COLUMN, CLIFF_Y, cliff_z);
        assert!(
            steep.can_walk_from(top.z, top.x, top.y),
            "the top of the cliff is ground, asked about from the top"
        );
        assert_eq!(
            pathfind(&steep, start, top, &Obstacles::NONE).unwrap_err(),
            PathError::Unreachable,
            "and no step of a person's climbs it"
        );

        let mut walled = MockMap::new(RAMP_WIDTH, CLIFF_TILES);
        walled.set_block(RAMP_COLUMN, CLIFF_Y, true);
        let goal = Point3::new(RAMP_COLUMN, CLIFF_Y, 0);
        assert_eq!(
            pathfind(&walled, start, goal, &Obstacles::NONE).unwrap_err(),
            PathError::BadGoal,
            "and a tile nobody can stand on is still no goal"
        );
    }

    #[test]
    fn path_along_platform_keeps_z() {
        const PLATFORM_Z: i8 = 27;
        let mut map = MockMap::new(8, 8);
        for x in 0..8 {
            map.set_z(x, 0, PLATFORM_Z);
        }
        let path = pathfind(
            &map,
            Point3::new(0, 0, PLATFORM_Z),
            Point3::new(4, 0, PLATFORM_Z),
            &Obstacles::NONE,
        )
        .unwrap();
        assert_eq!(path.steps.last().unwrap().z, PLATFORM_Z);
    }

    #[test]
    fn diagonal_does_not_cut_corner() {
        let mut map = MockMap::new(4, 4);
        map.set_block(1, 0, true);
        map.set_block(0, 1, true);
        let err = pathfind(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(1, 1, 0),
            &Obstacles::NONE,
        )
        .unwrap_err();
        assert_eq!(err, PathError::Unreachable);
    }

    /// A shut door fills the space a body would, and a person still walks
    /// through it: this client opens a door instead of going round it. The
    /// same tile with a wall on it, and no door, refuses the walk.
    #[test]
    fn a_door_leaf_is_opened_and_not_walked_around() {
        const DOORWAY_X: u16 = 1;
        const GOAL_X: u16 = 2;
        let start = Point3::new(0, 0, 0);
        let goal = Point3::new(GOAL_X, 0, 0);

        let mut walled = MockMap::new(GOAL_X + 1, 1);
        walled.set_block(DOORWAY_X, 0, true);
        assert_eq!(
            pathfind(&walled, start, goal, &Obstacles::NONE).unwrap_err(),
            PathError::Unreachable,
            "a wall across the only way refuses the walk"
        );

        let mut with_door = MockMap::new(GOAL_X + 1, 1);
        with_door.set_door(DOORWAY_X, 0, true);
        let path = pathfind(&with_door, start, goal, &Obstacles::NONE)
            .expect("a doorway is a way through");
        assert!(path.steps.iter().any(|s| s.x == DOORWAY_X && s.y == 0));
    }

    #[test]
    fn infer_felucca_and_old_width() {
        let full = u64::from(896u32) * 512 * BLOCK_BYTES as u64;
        assert_eq!(infer_mul_blocks(full, 896, 512), (896, 512));
        let old = u64::from(768u32) * 512 * BLOCK_BYTES as u64;
        assert_eq!(infer_mul_blocks(old, 896, 512), (768, 512));
        assert_eq!(map_block_dims(3), (MAP_MALAS_BLOCKS_W, MAP_MALAS_BLOCKS_H));
    }

    #[test]
    fn old_tiledata_larger_than_hs_land_block_is_not_hs() {
        let mut old = mini_tiledata();
        let hs_land = LAND_COUNT * LAND_RECORD_HS;
        while old.len() <= hs_land {
            old.extend_from_slice(&0u32.to_le_bytes());
            old.extend_from_slice(&[0u8; STATIC_RECORD_OLD * LAND_GROUP]);
        }
        assert!(old.len() > hs_land);
        assert!(!tiledata_is_hs(old.len()));
        assert!(!tiledata_is_hs(hs_land));
        assert!(tiledata_is_hs(TILEDATA_HS_LEN));
    }

    #[test]
    fn block_index_is_column_major() {
        let blocks_h = 512;
        assert_eq!(block_index(blocks_h, 0, 0), 0);
        assert_eq!(block_index(blocks_h, 0, 1), 1);
        assert_eq!(block_index(blocks_h, 1, 0), 512);
        assert_eq!(block_index(blocks_h, 438, 315), 438 * 512 + 315);
    }

    #[test]
    fn mul_fixture_reads_second_column_block() {
        let dir = scratch("mul-col");
        write_mini_client(&dir, false);
        let files = MapFiles {
            map: dir.join("map0.mul"),
            statics: dir.join("statics0.mul"),
            staidx: dir.join("staidx0.mul"),
            tiledata: dir.join("tiledata.mul"),
            blocks_w: 2,
            blocks_h: 2,
            map_index: 0,
        };
        let map = MulMap::from_files(&files).unwrap();
        assert_eq!(map.tile(8, 0).land_id, LAND_ID_BLOCK_1_0);
        assert_eq!(map.tile(0, 8).land_id, LAND_ID_BLOCK_0_1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mul_fixture_wall_and_walk() {
        let dir = scratch("mul-wall");
        write_mini_client(&dir, false);
        let files = MapFiles {
            map: dir.join("map0.mul"),
            statics: dir.join("statics0.mul"),
            staidx: dir.join("staidx0.mul"),
            tiledata: dir.join("tiledata.mul"),
            blocks_w: 2,
            blocks_h: 2,
            map_index: 0,
        };
        let map = MulMap::from_files(&files).unwrap();
        assert!(map.can_walk(0, 0));
        assert!(!map.can_walk(4, 0));
        assert!(map.tile(4, 0).statics_impassable);
        let path = pathfind(
            &map,
            Point3::new(0, 0, 0),
            Point3::new(6, 0, 0),
            &Obstacles::NONE,
        )
        .unwrap();
        assert!(!path.steps.iter().any(|s| s.x == 4 && s.y == 0));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mul_fixture_reports_tile_names() {
        let dir = scratch("mul-names");
        write_mini_client(&dir, false);
        let files = MapFiles {
            map: dir.join("map0.mul"),
            statics: dir.join("statics0.mul"),
            staidx: dir.join("staidx0.mul"),
            tiledata: dir.join("tiledata.mul"),
            blocks_w: 2,
            blocks_h: 2,
            map_index: 0,
        };
        let map = MulMap::from_files(&files).unwrap();
        assert_eq!(map.land_name(0, 0), FIXTURE_LAND_NAMES[0]);
        assert_eq!(
            map.land_name(8, 0),
            FIXTURE_LAND_NAMES[LAND_ID_BLOCK_1_0 as usize]
        );
        assert_eq!(
            map.land_name(0, 8),
            FIXTURE_LAND_NAMES[LAND_ID_BLOCK_0_1 as usize]
        );
        let wall = map.statics_at(FIXTURE_WALL_CX, FIXTURE_WALL_CY);
        assert_eq!(wall.len(), 1, "{wall:?}");
        assert_eq!(wall[0].name, FIXTURE_WALL_NAME);
        assert_eq!(wall[0].graphic, FIXTURE_WALL_GRAPHIC);
        assert_eq!(wall[0].z, FIXTURE_WALL_Z);
        assert_eq!(wall[0].height, FIXTURE_WALL_HEIGHT);
        assert!(wall[0].impassable());
        assert!(!wall[0].surface());
        assert!(!wall[0].bridge());
        assert!(!wall[0].door());
        assert!(map.statics_at(0, 0).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn uop_legacy_map_reads_same_as_mul() {
        let dir = scratch("uop-map");
        write_mini_client(&dir, true);
        let files = MapFiles {
            map: dir.join("map0LegacyMUL.uop"),
            statics: dir.join("statics0.mul"),
            staidx: dir.join("staidx0.mul"),
            tiledata: dir.join("tiledata.mul"),
            blocks_w: 2,
            blocks_h: 2,
            map_index: 0,
        };
        let map = MulMap::from_files(&files).unwrap();
        assert!(map.can_walk(0, 0));
        assert!(!map.can_walk(4, 0));
        assert_eq!(map.tile(0, 0).z, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "uoterm-nav-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const LAND_ID_BLOCK_1_0: u16 = 2;
    const LAND_ID_BLOCK_0_1: u16 = 1;

    /// Every fixture block gets its own land id so a test can prove which block
    /// a coordinate landed in.
    fn fixture_land_id(bx: u16, by: u16) -> u8 {
        (bx * 2 + by) as u8
    }

    fn write_mini_client(dir: &Path, as_uop: bool) {
        let blocks_w = 2u16;
        let blocks_h = 2u16;
        let mut map = vec![0u8; BLOCK_BYTES * (blocks_w * blocks_h) as usize];
        for by in 0..blocks_h {
            for bx in 0..blocks_w {
                let block = block_index(blocks_h, bx, by) as usize;
                let base = block * BLOCK_BYTES + BLOCK_HEADER;
                let land_id = fixture_land_id(bx, by);
                for cell in 0..64 {
                    let o = base + cell * CELL_BYTES;
                    map[o] = land_id;
                    map[o + 1] = 0;
                    map[o + 2] = 0;
                }
            }
        }
        if as_uop {
            fs::write(dir.join("map0LegacyMUL.uop"), pack_legacy_uop(0, &map)).unwrap();
        } else {
            fs::write(dir.join("map0.mul"), &map).unwrap();
        }

        let mut statics = vec![0u8; STATIC_RECORD];
        statics[0..2].copy_from_slice(&FIXTURE_WALL_GRAPHIC.to_le_bytes());
        statics[2] = FIXTURE_WALL_CX as u8;
        statics[3] = FIXTURE_WALL_CY as u8;
        statics[4] = FIXTURE_WALL_Z as u8;
        fs::write(dir.join("statics0.mul"), statics).unwrap();

        let mut staidx = vec![0u8; STAIDX_RECORD * (blocks_w * blocks_h) as usize];
        staidx[0..4].copy_from_slice(&0u32.to_le_bytes());
        staidx[4..8].copy_from_slice(&(STATIC_RECORD as u32).to_le_bytes());
        for block in 1..(blocks_w * blocks_h) as usize {
            let o = block * STAIDX_RECORD;
            staidx[o..o + 4].copy_from_slice(&IDX_EMPTY.to_le_bytes());
            staidx[o + 4..o + 8].copy_from_slice(&IDX_EMPTY.to_le_bytes());
        }
        fs::write(dir.join("staidx0.mul"), staidx).unwrap();
        fs::write(dir.join("tiledata.mul"), mini_tiledata()).unwrap();
    }

    /// The bytes of a tiledata record that follow the flags field.
    const LAND_REST_LEN: usize = LAND_RECORD_OLD - TILEDATA_FLAGS_OLD;
    const STATIC_REST_LEN: usize = STATIC_RECORD_OLD - TILEDATA_FLAGS_OLD;
    const STATIC_NAME_IN_REST: usize = STATIC_REST_LEN - TILE_NAME_LEN;
    /// One fixture name per land id, so a test can prove a name comes from the
    /// record of that id and not from a neighbour.
    const FIXTURE_LAND_NAMES: [&str; 4] = ["dirt", "grass", "cave floor", "forest"];
    /// The fixture holds one static: an impassable wall in the first block.
    const FIXTURE_WALL_GRAPHIC: u16 = 0;
    const FIXTURE_WALL_NAME: &str = "stone wall";
    const FIXTURE_WALL_HEIGHT: u8 = 20;
    const FIXTURE_WALL_Z: i8 = 0;
    const FIXTURE_WALL_CX: u16 = 4;
    const FIXTURE_WALL_CY: u16 = 0;
    /// A second fixture tile, so a test can prove a height and a flag come
    /// from the tiledata record of the graphic it asked about.
    const FIXTURE_FLOOR_GRAPHIC: u16 = 1;
    const FIXTURE_FLOOR_NAME: &str = "wooden floor";
    const FIXTURE_FLOOR_HEIGHT: u8 = 5;

    /// Writes a NUL-padded latin1 name the way a tiledata record stores it.
    fn put_tile_name(rest: &mut [u8], offset: usize, name: &str) {
        let bytes = name.as_bytes();
        assert!(bytes.len() <= TILE_NAME_LEN, "fixture name is too long");
        rest[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    fn mini_tiledata() -> Vec<u8> {
        let land_groups = LAND_COUNT / LAND_GROUP;
        let mut data = Vec::with_capacity(
            land_groups * (GROUP_HEADER + LAND_GROUP * LAND_RECORD_OLD)
                + GROUP_HEADER
                + LAND_GROUP * STATIC_RECORD_OLD,
        );
        for group in 0..land_groups {
            data.extend_from_slice(&0u32.to_le_bytes());
            for slot in 0..LAND_GROUP {
                data.extend_from_slice(&0u32.to_le_bytes());
                let mut rest = [0u8; LAND_REST_LEN];
                if let Some(&name) = FIXTURE_LAND_NAMES.get(group * LAND_GROUP + slot) {
                    put_tile_name(&mut rest, LAND_NAME_AFTER_FLAGS, name);
                }
                data.extend_from_slice(&rest);
            }
        }
        data.extend_from_slice(&0u32.to_le_bytes());
        for i in 0..LAND_GROUP {
            let (flags, height, name) = match i as u16 {
                FIXTURE_WALL_GRAPHIC => (TILE_IMPASSABLE, FIXTURE_WALL_HEIGHT, FIXTURE_WALL_NAME),
                FIXTURE_FLOOR_GRAPHIC => (TILE_SURFACE, FIXTURE_FLOOR_HEIGHT, FIXTURE_FLOOR_NAME),
                _ => (0, 0, ""),
            };
            data.extend_from_slice(&flags.to_le_bytes());
            let mut rest = [0u8; STATIC_REST_LEN];
            rest[STATIC_HEIGHT_BYTES_AFTER_FLAGS] = height;
            put_tile_name(&mut rest, STATIC_NAME_IN_REST, name);
            data.extend_from_slice(&rest);
        }
        data
    }

    /// Trammel, the facet every New Haven test below reads.
    const TRAMMEL_MAP_INDEX: u8 = 1;

    #[test]
    fn new_haven_trammel_inn_is_walkable_bank_is_walkable() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        const FROM: Point3 = Point3 {
            x: 3508,
            y: 2519,
            z: 27,
        };
        const BANK: Point3 = Point3 {
            x: 3488,
            y: 2577,
            z: 8,
        };
        let stand = map.tile(FROM.x, FROM.y);
        let west = map.tile(FROM.x - 1, FROM.y);
        let bank_t = map.tile(BANK.x, BANK.y);
        let inn = map.tile(3507, 2527);
        assert!(stand.walkable(), "inn tile must be walkable");
        assert!(west.walkable(), "west of inn must be walkable");
        assert!(bank_t.walkable(), "bank front must be walkable");
        if inn.walkable() {
            assert_ne!(inn.radar_char(), '~', "inn floor must not draw as water");
        }
        let _doors = map.doors_near(3507, 2527, 12);
    }

    /// Ground truth read by hand from statics1.mul: the New Haven inn has a
    /// table and a chair on these tiles.
    const INN_TABLE_X: u16 = 3503;
    const INN_TABLE_Y: u16 = 2515;
    const INN_CHAIR_X: u16 = 3503;
    const INN_CHAIR_Y: u16 = 2516;
    const INN_FLOOR_X: u16 = 3504;
    const INN_FLOOR_Y: u16 = 2516;
    const INN_TABLE_NAME: &str = "table";
    const INN_CHAIR_NAME: &str = "chair";
    /// Heights measured from the same live tiledata.mul. Reading the height
    /// byte from the wrong place in the record reports 0 for both.
    const INN_TABLE_HEIGHT: u8 = 6;
    const INN_CHAIR_HEIGHT: u8 = 1;

    #[test]
    fn new_haven_inn_statics_carry_names() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        let table = map.statics_at(INN_TABLE_X, INN_TABLE_Y);
        assert!(
            table.iter().any(|s| s.name == INN_TABLE_NAME),
            "no {INN_TABLE_NAME} at {INN_TABLE_X},{INN_TABLE_Y}: {table:?}"
        );
        assert!(
            table.windows(2).all(|pair| pair[0].z <= pair[1].z),
            "statics must be sorted by z: {table:?}"
        );
        let chair = map.statics_at(INN_CHAIR_X, INN_CHAIR_Y);
        assert!(
            chair.iter().any(|s| s.name == INN_CHAIR_NAME),
            "no {INN_CHAIR_NAME} at {INN_CHAIR_X},{INN_CHAIR_Y}: {chair:?}"
        );
        let floor = map.land_name(INN_FLOOR_X, INN_FLOOR_Y);
        assert!(
            !floor.is_empty(),
            "land tile at {INN_FLOOR_X},{INN_FLOOR_Y} must have a name"
        );
    }

    #[test]
    fn new_haven_inn_statics_carry_heights() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        let table = map.statics_at(INN_TABLE_X, INN_TABLE_Y);
        let table = table
            .iter()
            .find(|s| s.name == INN_TABLE_NAME)
            .unwrap_or_else(|| panic!("no {INN_TABLE_NAME} at {INN_TABLE_X},{INN_TABLE_Y}"));
        assert_eq!(table.height, INN_TABLE_HEIGHT, "{table:?}");
        let chair = map.statics_at(INN_CHAIR_X, INN_CHAIR_Y);
        let chair = chair
            .iter()
            .find(|s| s.name == INN_CHAIR_NAME)
            .unwrap_or_else(|| panic!("no {INN_CHAIR_NAME} at {INN_CHAIR_X},{INN_CHAIR_Y}"));
        assert_eq!(chair.height, INN_CHAIR_HEIGHT, "{chair:?}");
    }

    /// A person stands on a surface that is not impassable, and on nothing
    /// else. A table carries both flags, so it is furniture to walk around.
    /// A stair carries the bridge flag beside the surface flag, and the bridge
    /// flag alone holds nobody up.
    #[test]
    fn furniture_is_not_a_floor() {
        const INN_FLOOR_Z: i8 = 27;
        assert!(is_standing_surface(TILE_SURFACE));
        assert!(is_standing_surface(TILE_SURFACE | TILE_BRIDGE));
        assert!(!is_standing_surface(TILE_BRIDGE));
        assert!(!is_standing_surface(TILE_SURFACE | TILE_IMPASSABLE));
        assert!(!is_standing_surface(TILE_IMPASSABLE));

        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        let table_tile = map.tile(INN_TABLE_X, INN_TABLE_Y);
        assert_eq!(
            table_tile.z, INN_FLOOR_Z,
            "a person stands on the inn floor beside the table, not on the table top"
        );
    }

    /// Ground truth measured on a live shard. The New Haven inn has two
    /// floors. A character stood on the ground floor here, and every tile due
    /// south of him on this run of tiles is the same floor. Asked with no
    /// height, the map answers the floor above for some of them.
    const INN_GROUND_X: u16 = 3506;
    const INN_GROUND_Y: u16 = 2521;
    const INN_GROUND_Z: i8 = 27;
    /// The first and last tile of the run due south that the character walked.
    const INN_SOUTH_FIRST_Y: u16 = 2523;
    const INN_SOUTH_LAST_Y: u16 = 2527;
    /// The floor above the character, which a height-less query reports.
    const INN_UPPER_Z: i8 = 47;

    #[test]
    fn inn_tiles_answer_the_floor_the_character_stands_on() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        let mut upper_without_height = 0;
        for y in INN_SOUTH_FIRST_Y..=INN_SOUTH_LAST_Y {
            let mine = map.tile_from(INN_GROUND_Z, INN_GROUND_X, y);
            assert!(
                mine.walkable(),
                "{INN_GROUND_X},{y} must be walkable from z {INN_GROUND_Z}: {mine:?}"
            );
            assert_eq!(
                mine.z, INN_GROUND_Z,
                "{INN_GROUND_X},{y} must answer the ground floor: {mine:?}"
            );
            assert!(
                map.can_walk_from(INN_GROUND_Z, INN_GROUND_X, y),
                "{INN_GROUND_X},{y} must be walkable from z {INN_GROUND_Z}"
            );
            if map.tile(INN_GROUND_X, y).z == INN_UPPER_Z {
                upper_without_height += 1;
            }
        }
        assert!(
            upper_without_height > 0,
            "this run of tiles must hold the floor above, or the test proves nothing"
        );
    }

    /// Ground truth measured on a live shard, map index 1: a walk along the
    /// ground floor of the New Haven inn. Both floors of the inn stand on
    /// these tiles, and the walk must stay on the lower one.
    #[test]
    fn inn_ground_floor_path_never_leaves_the_floor() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        let start = Point3::new(INN_GROUND_X, INN_GROUND_Y, INN_GROUND_Z);
        let goal = Point3::new(INN_GROUND_X, INN_SOUTH_LAST_Y, INN_GROUND_Z);
        assert!(
            map.statics_at(INN_GROUND_X, INN_GROUND_Y)
                .iter()
                .any(|s| s.z == INN_UPPER_Z),
            "the floor above stands over the walk, or the test proves nothing"
        );

        let path =
            pathfind(&map, start, goal, &Obstacles::NONE).expect("a way along the inn floor");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y)),
            Some((goal.x, goal.y)),
            "{:?}",
            path.steps
        );
        let mut at = start;
        for step in &path.steps {
            assert_eq!(
                step.z, INN_GROUND_Z,
                "every step stays on the ground floor and never climbs to {INN_UPPER_Z}: {:?}",
                path.steps
            );
            assert_eq!(
                map.can_step(at, step.x, step.y),
                Some(step.z),
                "and every step is one the shard allows: {:?}",
                path.steps
            );
            at = Point3::new(step.x, step.y, step.z);
        }
    }

    /// A tile of the street outside the inn, and the wall the flat search cut
    /// through on the way to it. Measured from the same client files: asked
    /// with no height the wall tile answers the floor boards above it and reads
    /// as open ground, so the flat search walked the character into the wall
    /// and he stalled against it.
    const STREET_X: u16 = 3520;
    const STREET_Y: u16 = 2540;
    const FLAT_ROUTE_WALL_X: u16 = 3520;
    const FLAT_ROUTE_WALL_Y: u16 = 2551;

    #[test]
    fn a_flat_path_out_of_the_inn_keeps_off_the_walls_of_its_own_floor() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        assert!(
            map.can_walk(FLAT_ROUTE_WALL_X, FLAT_ROUTE_WALL_Y),
            "asked with no height this wall reads as walkable, or the test proves nothing"
        );
        assert!(
            !map.can_walk_from(INN_GROUND_Z, FLAT_ROUTE_WALL_X, FLAT_ROUTE_WALL_Y),
            "a person on the floor of the inn cannot stand on that wall"
        );

        let start = Point3::new(INN_FLOOR_X, INN_FLOOR_Y, INN_GROUND_Z);
        let street = map.tile_from(INN_GROUND_Z, STREET_X, STREET_Y);
        let goal = Point3::new(STREET_X, STREET_Y, street.z);
        assert!(
            map.can_walk_from(INN_GROUND_Z, goal.x, goal.y),
            "the street tile must be one the character can stand on: {street:?}"
        );
        let path = pathfind_flat(&map, start, goal, &Obstacles::NONE)
            .expect("a flat way from the inn to the street");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y)),
            Some((goal.x, goal.y)),
            "{:?}",
            path.steps
        );
        for step in &path.steps {
            assert!(
                map.can_walk_from(INN_GROUND_Z, step.x, step.y),
                "the flat path steps on {},{}, where a person at z {INN_GROUND_Z} cannot stand: {:?}",
                step.x,
                step.y,
                path.steps
            );
        }
    }

    /// Felucca, the facet the Britain bank stands on.
    const FELUCCA_MAP_INDEX: u8 = 0;
    /// Ground truth measured on a live shard beside the Britain bank: a ramp
    /// that climbs two height units for every tile south. The character stood
    /// at its foot and was told there was no way up it, and the person she
    /// followed stood at the top.
    const BANK_RAMP_X: u16 = 1423;
    const BANK_RAMP_FOOT_Y: u16 = 1700;
    const BANK_RAMP_FOOT_Z: i8 = 0;
    /// The heights of the ramp, tile by tile, from her own tile south. These
    /// are the heights a shard works out: the average of the four corners of
    /// each cell, which on a slope is one unit above the corner written in the
    /// cell itself.
    const BANK_RAMP_HEIGHTS: [i8; 7] = [1, 3, 5, 7, 9, 11, 13];
    /// The top of the same ramp, further above her than one step, which is
    /// the tile the search refused.
    const BANK_RAMP_TOP_Y: u16 = 1710;
    const BANK_RAMP_TOP_Z: i8 = 19;
    /// The four tiles around her, which held nothing at all: no static stood
    /// between her and the ramp.
    const AROUND_HER: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    #[test]
    fn the_ramp_beside_the_britain_bank_can_be_walked_up() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        assert!(
            map.statics_at(BANK_RAMP_X, BANK_RAMP_FOOT_Y).is_empty(),
            "she stood on bare ground"
        );
        for (dx, dy) in AROUND_HER {
            let x = (i32::from(BANK_RAMP_X) + dx) as u16;
            let y = (i32::from(BANK_RAMP_FOOT_Y) + dy) as u16;
            assert!(
                map.statics_at(x, y).is_empty(),
                "and nothing physical stood at {x},{y}"
            );
        }
        for (tile, measured) in BANK_RAMP_HEIGHTS.iter().enumerate() {
            let y = BANK_RAMP_FOOT_Y + tile as u16;
            let below = BANK_RAMP_HEIGHTS[tile.saturating_sub(1)];
            assert_eq!(
                map.tile_from(below, BANK_RAMP_X, y).z,
                *measured,
                "the ramp measured on the shard climbs to {measured} at {BANK_RAMP_X},{y}"
            );
        }

        let start = Point3::new(BANK_RAMP_X, BANK_RAMP_FOOT_Y, BANK_RAMP_FOOT_Z);
        let goal = Point3::new(BANK_RAMP_X, BANK_RAMP_TOP_Y, BANK_RAMP_TOP_Z);
        assert!(
            !map.can_walk_from(start.z, goal.x, goal.y),
            "asked from the foot of the ramp the top is no ground at all, which refused every \
             search she made"
        );
        assert!(
            map.can_walk_from(goal.z, goal.x, goal.y),
            "asked from its own height the top of the ramp is ground she can stand on"
        );

        let path = pathfind(&map, start, goal, &Obstacles::NONE).expect("a way up the ramp");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y, s.z)),
            Some((goal.x, goal.y, goal.z)),
            "{:?}",
            path.steps
        );
        let mut climbed = start.z;
        for step in &path.steps {
            assert_eq!(
                step.z,
                map.tile_from(climbed, step.x, step.y).z,
                "every step stands on the ground of its own tile: {:?}",
                path.steps
            );
            assert!(
                step.z >= climbed,
                "and the walk up a ramp never drops: {:?}",
                path.steps
            );
            climbed = step.z;
        }
        let last = BANK_RAMP_HEIGHTS.len() - 1;
        assert!(
            path.steps.iter().any(|s| (s.x, s.y, s.z)
                == (
                    BANK_RAMP_X,
                    BANK_RAMP_FOOT_Y + last as u16,
                    BANK_RAMP_HEIGHTS[last]
                )),
            "the way up runs over the tiles she measured: {:?}",
            path.steps
        );
    }

    // The same rules, read against the client files at positions measured on a
    // live shard.

    /// Ground truth measured on a live shard. A character stood here at height
    /// 35, and the ground the client files draw under him averages 17. Read
    /// from his own height his tile is no ground at all, and every route he
    /// asked for was refused before the search began.
    const ABOVE_THE_GROUND: Point3 = Point3 {
        x: 1449,
        y: 1494,
        z: 35,
    };
    /// The tile west of him, and the height its own ground stands at.
    const WEST_OF_HIM: u16 = 1448;
    const WEST_OF_HIM_Z: i8 = 23;

    #[test]
    fn a_character_above_the_ground_the_map_draws_can_still_leave_his_tile() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        assert!(
            !map.can_walk_from(ABOVE_THE_GROUND.z, ABOVE_THE_GROUND.x, ABOVE_THE_GROUND.y),
            "the map says he cannot stand where he stands, or the test proves nothing"
        );
        assert_eq!(
            footing(
                &map.column(ABOVE_THE_GROUND.x, ABOVE_THE_GROUND.y),
                ABOVE_THE_GROUND.z,
                Direction::West
            )
            .top,
            i16::from(ABOVE_THE_GROUND.z),
            "his own height is his footing, so his step reaches from where he is"
        );

        let goal = Point3::new(WEST_OF_HIM, ABOVE_THE_GROUND.y, WEST_OF_HIM_Z);
        assert_eq!(
            map.can_step(ABOVE_THE_GROUND, goal.x, goal.y),
            Some(WEST_OF_HIM_Z),
            "and the step off his tile onto the ground beside him is legal"
        );
        let path = pathfind(&map, ABOVE_THE_GROUND, goal, &Obstacles::NONE)
            .expect("a route away from the tile he stands on");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y, s.z)),
            Some((goal.x, goal.y, goal.z)),
            "{:?}",
            path.steps
        );
    }

    /// Ground truth measured on a live shard in the woodland. The follower
    /// stayed at height 30 while the person she followed went down a long
    /// slope to height 8.
    const TOP_OF_THE_SLOPE: Point3 = Point3 {
        x: 1420,
        y: 1528,
        z: 30,
    };
    const FOOT_OF_THE_SLOPE: Point3 = Point3 {
        x: 1411,
        y: 1518,
        z: 8,
    };

    #[test]
    fn the_measured_descent_is_walked_and_every_step_follows_the_ground() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        assert_eq!(
            map.tile_from(TOP_OF_THE_SLOPE.z, TOP_OF_THE_SLOPE.x, TOP_OF_THE_SLOPE.y)
                .z,
            TOP_OF_THE_SLOPE.z,
            "the top of the slope is the height she was measured at"
        );
        assert_eq!(
            map.tile_from(
                FOOT_OF_THE_SLOPE.z,
                FOOT_OF_THE_SLOPE.x,
                FOOT_OF_THE_SLOPE.y
            )
            .z,
            FOOT_OF_THE_SLOPE.z,
            "and the foot of it is the height he was measured at"
        );

        let path = pathfind(&map, TOP_OF_THE_SLOPE, FOOT_OF_THE_SLOPE, &Obstacles::NONE)
            .expect("a way down the slope");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y, s.z)),
            Some((
                FOOT_OF_THE_SLOPE.x,
                FOOT_OF_THE_SLOPE.y,
                FOOT_OF_THE_SLOPE.z
            )),
            "the walk ends where he stood: {:?}",
            path.steps
        );
        let mut at = TOP_OF_THE_SLOPE;
        for step in &path.steps {
            assert_eq!(
                map.can_step(at, step.x, step.y),
                Some(step.z),
                "every step is one the shard allows, and lands on the ground of its own tile: \
                 {:?}",
                path.steps
            );
            at = Point3::new(step.x, step.y, step.z);
        }
    }

    /// Ground truth measured on a live shard on the hillside south of the
    /// Britain bank. The follower sat on this tile for two minutes and never
    /// climbed. The shard refused her nothing: she asked for nothing, because
    /// the height she still believed she was at left her with no ground under
    /// her feet and no step she could plan.
    const HILLSIDE_HER_TILE: Point3 = Point3 {
        x: 1419,
        y: 1709,
        z: 1,
    };
    /// The tiles the person she followed stood on, with the height the shard
    /// reported him at on each. These are the shard's own numbers and they are
    /// what our reading of the map files must agree with.
    const HILLSIDE_MEASURED: [(u16, u16, i8); 6] = [
        (1419, 1710, 19),
        (1419, 1717, 20),
        (1419, 1707, 15),
        (1416, 1704, 9),
        (1420, 1705, 11),
        (1418, 1709, 18),
    ];
    /// The height the shard reported on her own tile, which is seventeen units
    /// over the height she still believed she stood at.
    const HILLSIDE_HER_GROUND: i8 = 18;

    /// The heights we read off the client files for that hillside are the
    /// heights the shard reported for the same tiles.
    #[test]
    fn the_measured_hillside_heights_are_the_ones_the_shard_reported() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        for (x, y, measured) in HILLSIDE_MEASURED {
            assert_eq!(
                map.tile_from(measured, x, y).z,
                measured,
                "the shard put him at {measured} on {x},{y}"
            );
        }
        assert_eq!(
            map.tile_from(
                HILLSIDE_HER_GROUND,
                HILLSIDE_HER_TILE.x,
                HILLSIDE_HER_TILE.y
            )
            .z,
            HILLSIDE_HER_GROUND,
            "and her own tile is the ground she was standing on, not the height she held"
        );
    }

    /// A character whose height has fallen behind the ground still walks.
    ///
    /// She holds a height between one word from the server and the next, and
    /// that height had fallen seventeen units under the hillside she was on.
    /// Her own tile holds her up whatever height she believes she is at, so the
    /// ground of that tile is her footing and the hill is hers to climb.
    #[test]
    fn the_follower_stuck_under_the_hillside_climbs_it() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        assert!(
            !map.can_walk_from(
                HILLSIDE_HER_TILE.z,
                HILLSIDE_HER_TILE.x,
                HILLSIDE_HER_TILE.y
            ),
            "read from the height she held, her own tile is no ground at all, or the test              proves nothing"
        );

        for (x, y, measured) in [HILLSIDE_MEASURED[0], HILLSIDE_MEASURED[1]] {
            let goal = Point3::new(x, y, measured);
            let path = pathfind(&map, HILLSIDE_HER_TILE, goal, &Obstacles::NONE)
                .expect("a way up the hill");
            assert_eq!(
                path.steps.last().map(|s| (s.x, s.y, s.z)),
                Some((goal.x, goal.y, goal.z)),
                "the walk ends where he stood: {:?}",
                path.steps
            );
            assert!(
                path.steps
                    .first()
                    .is_some_and(|s| s.z > HILLSIDE_HER_TILE.z),
                "and it climbs off the height she was stuck at: {:?}",
                path.steps
            );
            let mut at = Point3::new(
                HILLSIDE_HER_TILE.x,
                HILLSIDE_HER_TILE.y,
                HILLSIDE_HER_TILE.z,
            );
            for step in &path.steps {
                assert_eq!(
                    map.can_step(at, step.x, step.y),
                    Some(step.z),
                    "every step of it is one the shard allows: {:?}",
                    path.steps
                );
                assert!(
                    step.z >= at.z,
                    "and a walk up a hill never drops: {:?}",
                    path.steps
                );
                at = Point3::new(step.x, step.y, step.z);
            }
            assert_eq!(
                at.z, measured,
                "she ends the walk at the height he stood at"
            );
        }
    }

    /// The tree that stands north of the top of that slope, and the tile north
    /// west of it that a diagonal step past the tree would reach.
    const TREE_TILE: (u16, u16) = (1420, 1527);
    const PAST_THE_TREE: (u16, u16) = (1419, 1527);
    const TREE_NAME: &str = "tree";

    /// A diagonal step past a blocked corner is illegal. Both tiles beside the
    /// corner must pass the whole test, and the tree fails it.
    #[test]
    fn a_diagonal_past_the_tree_is_refused() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        let statics = map.statics_at(TREE_TILE.0, TREE_TILE.1);
        assert!(
            statics
                .iter()
                .any(|s| s.name == TREE_NAME && s.impassable()),
            "a tree stands at {},{}: {statics:?}",
            TREE_TILE.0,
            TREE_TILE.1
        );
        assert_eq!(
            map.can_step(TOP_OF_THE_SLOPE, TREE_TILE.0, TREE_TILE.1),
            None,
            "the tile the tree stands on is no step of its own"
        );
        assert!(
            map.can_step(TOP_OF_THE_SLOPE, PAST_THE_TREE.0, TOP_OF_THE_SLOPE.y)
                .is_some(),
            "the tile west of her is one she can step onto, or the test proves nothing"
        );
        assert_eq!(
            map.can_step(TOP_OF_THE_SLOPE, PAST_THE_TREE.0, PAST_THE_TREE.1),
            None,
            "so the diagonal north west, which passes the tree's corner, is refused"
        );
    }

    /// A map keeps the last tiles it read, so a search does not read the same
    /// cell of the file over and over. A tile answered out of that store must
    /// answer exactly as one read afresh, and a run of tiles longer than the
    /// store holds must push the oldest out without spoiling one answer.
    #[test]
    fn a_tile_answers_the_same_whether_it_is_kept_or_read_again() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        let run = COLUMN_CACHE_CAP as u16 * 2;
        let read = |d: u16| {
            let tile = map.tile(BANK_RAMP_X, BANK_RAMP_FOOT_Y + d);
            (tile.z, tile.land_z, tile.flags, tile.walkable())
        };
        let first: Vec<_> = (0..run).map(read).collect();
        assert!(
            first.iter().any(|t| t.0 != first[0].0),
            "the run must cross ground of more than one height, or the test proves nothing"
        );
        for (d, was) in first.iter().enumerate() {
            assert_eq!(
                read(d as u16),
                *was,
                "{BANK_RAMP_X},{} answers alike both times",
                BANK_RAMP_FOOT_Y + d as u16
            );
        }
    }

    #[test]
    fn height_less_and_height_aware_agree_on_a_single_floor_map() {
        const CLIFF_X: u16 = 2;
        /// On the walker's own storey, where one floor is all there is to
        /// find and both questions must find it.
        const CLIFF_Z: i8 = 12;
        const WALKER_Z: i8 = 0;
        let mut map = MockMap::new(4, 4);
        map.set_block(1, 1, true);
        map.set_door(0, 1, true);
        map.set_z(CLIFF_X, 1, CLIFF_Z);
        for y in 0..map.height() {
            for x in 0..map.width() {
                assert_eq!(
                    map.tile_from(WALKER_Z, x, y).z,
                    map.tile(x, y).z,
                    "one floor has one answer at {x},{y}"
                );
                assert_eq!(
                    map.can_walk_from(WALKER_Z, x, y),
                    map.can_walk(x, y),
                    "one floor has one answer at {x},{y}"
                );
            }
        }
    }

    // One test for each rule the shard applies to a step.

    /// A map built column by column, so a test can put a floor, a table or a
    /// ceiling exactly where it wants one. [`MockMap`] holds bare ground and
    /// nothing else, so it cannot show the rules that turn on the items on a
    /// tile.
    struct ColumnMap {
        width: u16,
        height: u16,
        columns: Vec<TileColumn>,
    }

    /// The land id of plain ground, which draws something and holds a person
    /// up.
    const GROUND_LAND_ID: u16 = 3;

    impl ColumnMap {
        /// Flat ground at `land_z` with nothing standing on it.
        fn flat(width: u16, height: u16, land_z: i8) -> Self {
            let ground = TileColumn {
                land_id: GROUND_LAND_ID,
                land_flags: TILE_SURFACE,
                land: LandCorners::flat(land_z),
                pieces: Vec::new(),
            };
            Self {
                width,
                height,
                columns: vec![ground; width as usize * height as usize],
            }
        }

        fn at(&mut self, x: u16, y: u16) -> &mut TileColumn {
            let i = y as usize * self.width as usize + x as usize;
            &mut self.columns[i]
        }

        /// Raises the land of one tile, the way one cell of a slope stands
        /// above its neighbour.
        fn raise(&mut self, x: u16, y: u16, land_z: i8) -> &mut Self {
            self.at(x, y).land = LandCorners::flat(land_z);
            self
        }

        fn put(&mut self, x: u16, y: u16, piece: TilePiece) -> &mut Self {
            self.at(x, y).pieces.push(piece);
            self
        }
    }

    impl TileQuery for ColumnMap {
        fn column(&self, x: u16, y: u16) -> TileColumn {
            if !self.in_bounds(x, y) {
                return TileColumn::off_map();
            }
            self.columns[y as usize * self.width as usize + x as usize].clone()
        }

        fn width(&self) -> u16 {
            self.width
        }

        fn height(&self) -> u16 {
            self.height
        }
    }

    /// A floor a person stands on.
    fn floor_at(z: i8) -> TilePiece {
        TilePiece {
            z,
            height: 0,
            flags: TILE_SURFACE,
        }
    }

    /// A wall or a table: it fills space and holds nobody up.
    fn solid_at(z: i8, height: u8) -> TilePiece {
        TilePiece {
            z,
            height,
            flags: TILE_IMPASSABLE,
        }
    }

    const EAST_OF_START: u16 = 1;

    /// A person climbs [`STEP_HEIGHT`] in one step and no more.
    #[test]
    fn a_step_climbs_two_height_units_and_no_more() {
        /// The number itself, read out of the shard's own movement code. It is
        /// written here and not taken from [`STEP_HEIGHT`], so that the wrong
        /// number in the crate cannot make this test agree with it.
        const HIGHEST_CLIMB: i8 = 2;
        let start = Point3::new(0, 0, 0);

        let mut level = ColumnMap::flat(2, 1, 0);
        level.raise(EAST_OF_START, 0, HIGHEST_CLIMB);
        assert_eq!(
            level.can_step(start, EAST_OF_START, 0),
            Some(HIGHEST_CLIMB),
            "a rise of exactly one step is climbed"
        );

        let mut steep = ColumnMap::flat(2, 1, 0);
        steep.raise(EAST_OF_START, 0, HIGHEST_CLIMB + 1);
        assert_eq!(
            steep.can_step(start, EAST_OF_START, 0),
            None,
            "and one height unit more is refused"
        );
    }

    /// There is no such limit downwards. A person walks off a ledge of any
    /// depth, and a walk that refused to would strand him wherever he fell.
    #[test]
    fn a_step_down_has_no_limit() {
        const DEEP: i8 = -100;
        let mut map = ColumnMap::flat(2, 1, 0);
        map.raise(EAST_OF_START, 0, DEEP);
        assert_eq!(
            map.can_step(Point3::new(0, 0, 0), EAST_OF_START, 0),
            Some(DEEP)
        );
    }

    /// A person is [`PERSON_HEIGHT`] tall, and that height is about whether
    /// his body FITS, not about what he climbs. A ceiling lower than he is
    /// shuts him out of the tile under it.
    #[test]
    fn a_ceiling_lower_than_a_person_shuts_him_out() {
        let start = Point3::new(0, 0, 0);
        let mut low = ColumnMap::flat(2, 1, 0);
        low.put(EAST_OF_START, 0, floor_at(PERSON_HEIGHT - 1));
        assert_eq!(
            low.can_step(start, EAST_OF_START, 0),
            None,
            "a floor lower than a person is a ceiling he does not fit under"
        );

        let mut high = ColumnMap::flat(2, 1, 0);
        high.put(EAST_OF_START, 0, floor_at(PERSON_HEIGHT));
        assert_eq!(
            high.can_step(start, EAST_OF_START, 0),
            Some(0),
            "a floor exactly a person's height above him leaves him room"
        );
    }

    /// A surface that is also impassable is furniture. A person walks to the
    /// floor beside a table, never onto the table top.
    #[test]
    fn a_table_is_not_a_floor_to_step_onto() {
        const TABLE_Z: i8 = 0;
        const TABLE_HEIGHT: u8 = 6;
        let start = Point3::new(0, 0, 0);
        let mut map = ColumnMap::flat(2, 1, 0);
        map.put(
            EAST_OF_START,
            0,
            TilePiece {
                z: TABLE_Z,
                height: TABLE_HEIGHT,
                flags: TILE_SURFACE | TILE_IMPASSABLE,
            },
        );
        assert_eq!(
            map.can_step(start, EAST_OF_START, 0),
            None,
            "the table fills the tile and its top is no floor"
        );
    }

    /// A step past a corner needs the two tiles beside that corner to pass the
    /// whole step test, not merely to be tiles a person could stand on. A
    /// corner tile a person cannot CLIMB refuses the diagonal although he
    /// could stand on it.
    #[test]
    fn a_diagonal_needs_both_corners_to_pass_the_whole_check() {
        const CORNER_X: u16 = 1;
        const CORNER_Y: u16 = 0;
        const DIAGONAL_X: u16 = 1;
        const DIAGONAL_Y: u16 = 1;
        let start = Point3::new(0, 0, 0);
        let ledge = STEP_HEIGHT + 1;

        let mut map = ColumnMap::flat(2, 2, 0);
        map.raise(CORNER_X, CORNER_Y, ledge);
        assert!(
            map.can_walk_from(start.z, CORNER_X, CORNER_Y),
            "the corner tile is ground a person can stand on, or the test proves nothing"
        );
        assert_eq!(
            map.can_step(start, CORNER_X, CORNER_Y),
            None,
            "and it is one he cannot step onto from here"
        );
        assert_eq!(
            map.can_step(start, DIAGONAL_X, DIAGONAL_Y),
            None,
            "so the step past that corner is refused"
        );

        let level = ColumnMap::flat(2, 2, 0);
        assert_eq!(
            level.can_step(start, DIAGONAL_X, DIAGONAL_Y),
            Some(0),
            "with both corners level the same step is made"
        );
    }

    /// A cell whose corners rise from north west to south east, so that every
    /// corner and every edge of it is a different height. Nothing here is
    /// symmetric, so a rule that reads the wrong corner cannot pass by luck.
    const SLOPE: LandCorners = LandCorners {
        north_west: 0,
        north_east: 4,
        south_west: 8,
        south_east: 20,
    };

    /// Standing in the middle of a cell a person is half way along the shared
    /// edge of the two triangles it is drawn as, which is the pair of opposite
    /// corners that differs least. The average is rounded down below sea level
    /// as well as above it.
    #[test]
    fn the_middle_of_a_cell_is_the_pair_of_corners_that_differs_least() {
        assert_eq!(LandCorners::flat(4).center(), 4, "flat ground");
        assert_eq!(
            LandCorners {
                north_west: 0,
                north_east: 0,
                south_west: 2,
                south_east: 2,
            }
            .center(),
            1,
            "a slope stands half way up the pair that differs least"
        );
        let beside_the_bank = LandCorners {
            north_west: 18,
            north_east: 15,
            south_west: 22,
            south_east: 17,
        };
        assert_eq!(
            (beside_the_bank.low(), beside_the_bank.center()),
            (15, 17),
            "the cell the character stood on beside the Britain bank"
        );
        assert_eq!(
            LandCorners {
                north_west: -3,
                north_east: -4,
                south_west: -3,
                south_east: -4,
            }
            .center(),
            -4,
            "and below sea level the average is rounded down, not toward zero"
        );
        assert_eq!(
            (SLOPE.low(), SLOPE.center()),
            (0, 6),
            "the lowest corner of the slope, and the middle of it"
        );
    }

    /// The ground a person leaves a cell over is not the middle of the cell
    /// and not its highest corner: it is the corner he walks out over, or the
    /// average of the two corners of the edge he crosses. Four of the eight
    /// ways out read one corner and no average at all.
    #[test]
    fn the_ground_a_cell_is_left_over_is_the_corner_or_the_edge_crossed() {
        assert_eq!(
            SLOPE.toward(Direction::Northwest),
            0,
            "the north west corner"
        );
        assert_eq!(
            SLOPE.toward(Direction::Northeast),
            4,
            "the north east corner"
        );
        assert_eq!(
            SLOPE.toward(Direction::Southwest),
            8,
            "the south west corner"
        );
        assert_eq!(
            SLOPE.toward(Direction::Southeast),
            20,
            "the south east corner"
        );
        assert_eq!(SLOPE.toward(Direction::North), 2, "the north edge, 0 and 4");
        assert_eq!(SLOPE.toward(Direction::East), 12, "the east edge, 4 and 20");
        assert_eq!(
            SLOPE.toward(Direction::South),
            14,
            "the south edge, 8 and 20"
        );
        assert_eq!(SLOPE.toward(Direction::West), 4, "the west edge, 0 and 8");
        for direction in DIRS {
            assert_eq!(
                LandCorners::flat(7).toward(direction),
                7,
                "flat ground is the same height whichever way it is left"
            );
        }
    }

    /// How far a step may climb is measured from the edge the walker leaves
    /// over, so one tile lets him climb south and refuses the same climb
    /// north. The highest corner of his own cell does not lift him.
    #[test]
    fn how_far_a_step_climbs_is_measured_from_the_edge_it_leaves_over() {
        /// A step reaches this far above the ground it starts from.
        const CLIMB: i8 = 2;
        /// The two north corners of the walker's cell, which are where he
        /// leaves it walking north.
        const NORTH_EDGE: i8 = 0;
        /// The two south corners, which are where he leaves it walking south.
        /// They are also its highest corners.
        const SOUTH_EDGE: i8 = 8;
        /// The middle of that cell, where he stands while he is on it.
        const MIDDLE: i8 = 4;
        const NORTH_OF_START: u16 = 0;
        const START_Y: u16 = 1;
        const SOUTH_OF_START: u16 = 2;
        /// As high as a tile can stand and still be one step off the south
        /// edge.
        const OFF_THE_SOUTH_EDGE: i8 = SOUTH_EDGE + CLIMB;
        /// As high as a tile can stand and still be one step off his own feet,
        /// which is all the north edge gives him.
        const OFF_HIS_FEET: i8 = MIDDLE + CLIMB;

        let mut map = ColumnMap::flat(1, 3, MIDDLE);
        // His cell rises to the south: its north corners are on the floor and
        // its south corners at the top of the slope, so its middle is where he
        // stands.
        map.at(0, START_Y).land = LandCorners {
            north_west: NORTH_EDGE,
            north_east: NORTH_EDGE,
            south_west: SOUTH_EDGE,
            south_east: SOUTH_EDGE,
        };
        assert_eq!(
            map.column(0, START_Y).land.center(),
            i16::from(MIDDLE),
            "he stands in the middle of his own cell, or the test proves nothing"
        );
        let walker = Point3::new(0, START_Y, MIDDLE);

        map.raise(0, SOUTH_OF_START, OFF_THE_SOUTH_EDGE);
        assert_eq!(
            map.can_step(walker, 0, SOUTH_OF_START),
            Some(OFF_THE_SOUTH_EDGE),
            "walking south he starts the climb at the south edge, four over his feet"
        );

        map.raise(0, NORTH_OF_START, OFF_THE_SOUTH_EDGE);
        assert_eq!(
            map.can_step(walker, 0, NORTH_OF_START),
            None,
            "walking north he starts it at the north edge, so the same height is out of reach"
        );

        map.raise(0, NORTH_OF_START, OFF_HIS_FEET);
        assert_eq!(
            map.can_step(walker, 0, NORTH_OF_START),
            Some(OFF_HIS_FEET),
            "and north he still climbs the step his own feet give him"
        );
    }

    /// Land that draws nothing holds nobody up. Only the items on such a cell
    /// can.
    #[test]
    fn land_that_draws_nothing_holds_nobody_up() {
        const NODRAW_LAND_ID: u16 = 2;
        const PLATFORM_Z: i8 = 0;
        assert!(land_is_ignored(NODRAW_LAND_ID));
        assert!(!land_is_ignored(GROUND_LAND_ID));

        let start = Point3::new(0, 0, 0);
        let mut empty = ColumnMap::flat(2, 1, 0);
        empty.at(EAST_OF_START, 0).land_id = NODRAW_LAND_ID;
        assert_eq!(
            empty.can_step(start, EAST_OF_START, 0),
            None,
            "no ground and nothing on it is nowhere to step"
        );

        let mut planked = ColumnMap::flat(2, 1, 0);
        planked.at(EAST_OF_START, 0).land_id = NODRAW_LAND_ID;
        planked.put(EAST_OF_START, 0, floor_at(PLATFORM_Z));
        assert_eq!(
            planked.can_step(start, EAST_OF_START, 0),
            Some(PLATFORM_Z),
            "a plank laid over it is a floor again"
        );
    }

    /// A bridge is a ramp: a person meets it half way up, and it walls nothing
    /// off at its own foot.
    #[test]
    fn a_bridge_is_climbed_half_way_up() {
        const BRIDGE_HEIGHT: u8 = 4;
        let start = Point3::new(0, 0, 0);
        let mut map = ColumnMap::flat(2, 1, 0);
        map.put(
            EAST_OF_START,
            0,
            TilePiece {
                z: 0,
                height: BRIDGE_HEIGHT,
                flags: TILE_SURFACE | TILE_BRIDGE,
            },
        );
        assert_eq!(
            map.can_step(start, EAST_OF_START, 0),
            Some(BRIDGE_HEIGHT as i8 / 2),
            "he stands half way up the bridge, which is one step's climb"
        );
    }

    /// A tile with two floors gives the walker the one nearest his own height,
    /// and the lower of two equally near. That is what keeps him on the floor
    /// of a building he is on instead of the one over his head.
    #[test]
    fn the_floor_nearest_the_walker_wins_when_a_tile_holds_two() {
        const GROUND_FLOOR_Z: i8 = 0;
        const UPPER_FLOOR_Z: i8 = 20;
        let mut map = ColumnMap::flat(2, 1, GROUND_FLOOR_Z);
        map.put(EAST_OF_START, 0, floor_at(GROUND_FLOOR_Z));
        map.put(EAST_OF_START, 0, floor_at(UPPER_FLOOR_Z));
        assert_eq!(
            map.can_step(Point3::new(0, 0, GROUND_FLOOR_Z), EAST_OF_START, 0),
            Some(GROUND_FLOOR_Z),
            "a walker on the ground floor stays on it"
        );
    }

    /// A person is on the tile he is on, whatever the map believes about it.
    /// When nothing on it can be found to hold him up, his own height is his
    /// footing, and he walks away from the tile instead of standing frozen on
    /// it.
    #[test]
    fn a_person_stands_where_he_is_when_nothing_holds_him_up() {
        const HIS_HEIGHT: i8 = 35;
        const WALLED_LAND: u32 = TILE_IMPASSABLE;
        let column = TileColumn {
            land_id: GROUND_LAND_ID,
            land_flags: WALLED_LAND,
            ..TileColumn::default()
        };
        assert_eq!(
            footing(&column, HIS_HEIGHT, Direction::East),
            Footing {
                low: i16::from(HIS_HEIGHT),
                center: i16::from(HIS_HEIGHT),
                top: i16::from(HIS_HEIGHT),
            },
        );
    }

    /// A walker whose height has fallen under the ground of his own tile
    /// stands on that ground.
    ///
    /// The height a client holds for a character is its own guess between one
    /// word from the server and the next. A guess that has fallen behind a
    /// hill leaves nothing under his feet, and taking it for his footing
    /// leaves him unable to plan one step in any direction. He is on the tile,
    /// so the ground of the tile is where he stands.
    #[test]
    fn a_walker_under_the_ground_of_his_own_tile_stands_on_it() {
        const HILL_Z: i8 = 20;
        /// The height he still holds, far enough under the hill that no step
        /// measured from it could reach any tile beside him.
        const STALE_Z: i8 = 1;
        let map = ColumnMap::flat(2, 1, HILL_Z);
        let start = Point3::new(0, 0, STALE_Z);
        assert!(
            !map.can_walk_from(STALE_Z, start.x, start.y),
            "the map says he cannot be where he believes he is, or the test proves nothing"
        );
        assert_eq!(
            footing(&map.column(start.x, start.y), STALE_Z, Direction::East),
            Footing {
                low: i16::from(HILL_Z),
                center: i16::from(HILL_Z),
                top: i16::from(HILL_Z),
            },
            "the ground of his own tile is his footing, not the height he holds"
        );
        assert_eq!(
            map.can_step(start, EAST_OF_START, 0),
            Some(HILL_Z),
            "so the tile beside him is one step away and he walks onto the hill"
        );
    }

    /// And a route away from that tile is planned, because the tile a walker
    /// stands on is never the reason a walk cannot start.
    #[test]
    fn a_route_leaves_the_tile_the_walker_stands_on() {
        const HIS_HEIGHT: i8 = 35;
        const GROUND_Z: i8 = 0;
        let mut map = ColumnMap::flat(2, 1, GROUND_Z);
        map.put(0, 0, solid_at(GROUND_Z, PERSON_HEIGHT as u8));
        let start = Point3::new(0, 0, HIS_HEIGHT);
        assert!(
            !map.can_walk_from(start.z, start.x, start.y),
            "the map says he cannot be where he is, or the test proves nothing"
        );
        let path = pathfind(
            &map,
            start,
            Point3::new(EAST_OF_START, 0, GROUND_Z),
            &Obstacles::NONE,
        )
        .expect("a walk away from his own tile");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y)),
            Some((EAST_OF_START, 0))
        );
    }

    /// Only a start off the map refuses a walk.
    #[test]
    fn a_start_off_the_map_is_the_one_start_refused() {
        let map = ColumnMap::flat(2, 1, 0);
        assert_eq!(
            pathfind(
                &map,
                Point3::new(9, 9, 0),
                Point3::new(EAST_OF_START, 0, 0),
                &Obstacles::NONE
            )
            .unwrap_err(),
            PathError::BadStart
        );
    }

    // Multis: the player houses and boats the map files know nothing about.

    /// One component the fixture writes into a multi file.
    #[derive(Clone, Copy)]
    struct FixtureComponent {
        graphic: u16,
        dx: i16,
        dy: i16,
        dz: i16,
        /// Part of the building, and not a door or a sign the shard places as
        /// a loose item of its own.
        built: bool,
    }

    /// A `multi.mul` component that is part of the building carries this flag.
    /// A real file holds no other value but zero.
    const FIXTURE_MULTI_BUILT: u32 = 1;
    /// A UOP component that is part of the building has the loose bit clear.
    const FIXTURE_UOP_BUILT: u16 = 0;
    const FIXTURE_HOUSE_ID: u16 = 3;
    const FIXTURE_BOAT_ID: u16 = 9;
    /// The fixture gives every UOP component this many cliloc ids, so a test
    /// proves the reader steps over them and stays in step with the record
    /// that follows.
    const FIXTURE_CLILOCS: usize = 2;
    /// Five components: 12-byte records make 60 bytes, which 16 does not
    /// divide, so the index alone tells the two record sizes apart.
    fn fixture_house() -> Vec<FixtureComponent> {
        vec![
            FixtureComponent {
                graphic: FIXTURE_FLOOR_GRAPHIC,
                dx: 0,
                dy: 0,
                dz: 0,
                built: true,
            },
            FixtureComponent {
                graphic: FIXTURE_WALL_GRAPHIC,
                dx: -1,
                dy: 0,
                dz: 0,
                built: true,
            },
            FixtureComponent {
                graphic: FIXTURE_WALL_GRAPHIC,
                dx: 1,
                dy: -2,
                dz: 7,
                built: true,
            },
            FixtureComponent {
                graphic: FIXTURE_WALL_GRAPHIC,
                dx: 0,
                dy: 2,
                dz: 0,
                built: false,
            },
            FixtureComponent {
                graphic: FIXTURE_FLOOR_GRAPHIC,
                dx: 1,
                dy: 1,
                dz: 0,
                built: true,
            },
        ]
    }

    fn fixture_boat() -> Vec<FixtureComponent> {
        vec![FixtureComponent {
            graphic: FIXTURE_WALL_GRAPHIC,
            dx: 2,
            dy: -3,
            dz: 1,
            built: true,
        }]
    }

    /// What the fixture tiledata says about a graphic: its height, whether it
    /// blocks a person, and whether a person can stand on it.
    fn fixture_tile(graphic: u16) -> (u8, bool, bool) {
        match graphic {
            FIXTURE_WALL_GRAPHIC => (FIXTURE_WALL_HEIGHT, true, false),
            FIXTURE_FLOOR_GRAPHIC => (FIXTURE_FLOOR_HEIGHT, false, true),
            _ => (0, false, false),
        }
    }

    /// Proves the reader gives back every built component of the fixture, in
    /// file order, with the height and the flags of its own graphic, and none
    /// of the loose ones.
    fn expect_pieces(data: &MultiData, multi_id: u16, wrote: &[FixtureComponent]) {
        let built: Vec<&FixtureComponent> = wrote.iter().filter(|part| part.built).collect();
        let got = data.pieces(multi_id);
        assert_eq!(
            got.len(),
            built.len(),
            "multi {multi_id} keeps every built component and drops the loose ones: {got:?}"
        );
        for (piece, part) in got.iter().zip(built) {
            let (height, blocks, surface) = fixture_tile(part.graphic);
            assert_eq!(
                *piece,
                MultiPiece {
                    graphic: part.graphic,
                    dx: part.dx,
                    dy: part.dy,
                    dz: part.dz,
                    height,
                    flags: piece.flags,
                },
                "multi {multi_id} reads back the component it was given"
            );
            assert_eq!(piece.blocks(), blocks, "piece {} blocks", piece.graphic);
            assert_eq!(piece.surface(), surface, "piece {} surface", piece.graphic);
        }
    }

    /// Writes `multi.mul` and `multi.idx` the way a client ships them.
    fn write_multi_mul(dir: &Path, multis: &[(u16, Vec<FixtureComponent>)], record: usize) {
        let top = multis
            .iter()
            .map(|(id, _)| *id as usize)
            .max()
            .unwrap_or_default();
        let mut idx = vec![0u8; MULTI_IDX_RECORD * (top + 1)];
        for slot in 0..=top {
            let at = slot * MULTI_IDX_RECORD;
            idx[at..at + 4].copy_from_slice(&IDX_EMPTY.to_le_bytes());
            idx[at + 4..at + 8].copy_from_slice(&IDX_EMPTY.to_le_bytes());
        }
        let mut mul = Vec::new();
        for (id, parts) in multis {
            let start = mul.len();
            for part in parts {
                let mut bytes = vec![0u8; record];
                bytes[0..2].copy_from_slice(&part.graphic.to_le_bytes());
                bytes[2..4].copy_from_slice(&part.dx.to_le_bytes());
                bytes[4..6].copy_from_slice(&part.dy.to_le_bytes());
                bytes[6..8].copy_from_slice(&part.dz.to_le_bytes());
                let flags = if part.built {
                    FIXTURE_MULTI_BUILT
                } else {
                    MULTI_MUL_LOOSE
                };
                bytes[8..12].copy_from_slice(&flags.to_le_bytes());
                mul.extend_from_slice(&bytes);
            }
            let at = *id as usize * MULTI_IDX_RECORD;
            idx[at..at + 4].copy_from_slice(&(start as u32).to_le_bytes());
            idx[at + 4..at + 8].copy_from_slice(&((mul.len() - start) as u32).to_le_bytes());
        }
        fs::write(dir.join(MULTI_IDX_NAME), idx).unwrap();
        fs::write(dir.join(MULTI_MUL_NAME), mul).unwrap();
    }

    /// One `MultiCollection.uop` entry: the multi id, the number of
    /// components, then the components with their cliloc ids.
    fn multi_uop_entry(id: u16, parts: &[FixtureComponent], clilocs: usize) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&u32::from(id).to_le_bytes());
        out.extend_from_slice(&(parts.len() as u32).to_le_bytes());
        for part in parts {
            out.extend_from_slice(&part.graphic.to_le_bytes());
            out.extend_from_slice(&part.dx.to_le_bytes());
            out.extend_from_slice(&part.dy.to_le_bytes());
            out.extend_from_slice(&part.dz.to_le_bytes());
            let flags = if part.built {
                FIXTURE_UOP_BUILT
            } else {
                MULTI_UOP_LOOSE
            };
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&(clilocs as u32).to_le_bytes());
            for cliloc in 0..clilocs {
                out.extend_from_slice(&(cliloc as u32).to_le_bytes());
            }
        }
        out
    }

    fn write_multi_uop(dir: &Path, multis: &[(u16, Vec<FixtureComponent>)], clilocs: usize) {
        let files: Vec<(String, Vec<u8>)> = multis
            .iter()
            .map(|(id, parts)| {
                (
                    multi_uop_name(u32::from(*id)),
                    multi_uop_entry(*id, parts, clilocs),
                )
            })
            .collect();
        fs::write(dir.join(MULTI_UOP_NAMES[0]), pack_uop(&files)).unwrap();
    }

    #[test]
    fn a_multi_mul_fixture_round_trips() {
        let dir = scratch("multi-mul");
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        let house = fixture_house();
        let boat = fixture_boat();
        write_multi_mul(
            &dir,
            &[
                (FIXTURE_HOUSE_ID, house.clone()),
                (FIXTURE_BOAT_ID, boat.clone()),
            ],
            MULTI_RECORD_HS,
        );
        let data = MultiData::open(&dir).unwrap();
        assert_eq!(data.multi_count(), 2, "the fixture holds two multis");
        expect_pieces(&data, FIXTURE_HOUSE_ID, &house);
        expect_pieces(&data, FIXTURE_BOAT_ID, &boat);
        assert!(
            data.pieces(FIXTURE_HOUSE_ID + 1).is_empty(),
            "an id the index leaves empty has no pieces"
        );
        assert!(
            data.pieces(u16::MAX).is_empty(),
            "an id past the end of the index has no pieces"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// The runtime hands one map to every session through a cache that holds
    /// an `Arc`. A multi reader has to travel the same way, so that a shard
    /// reads these files once and not once per character.
    #[test]
    fn one_multi_reader_serves_more_than_one_session() {
        let dir = scratch("multi-shared");
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        let house = fixture_house();
        write_multi_mul(&dir, &[(FIXTURE_HOUSE_ID, house.clone())], MULTI_RECORD_HS);
        let data = Arc::new(MultiData::open(&dir).unwrap());
        let other = Arc::clone(&data);
        let reader = std::thread::spawn(move || other.pieces(FIXTURE_HOUSE_ID).to_vec());
        assert_eq!(reader.join().unwrap(), data.pieces(FIXTURE_HOUSE_ID));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_multi_collection_uop_wins_over_the_mul_pair() {
        let dir = scratch("multi-uop");
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        let house = fixture_house();
        write_multi_mul(&dir, &[(FIXTURE_HOUSE_ID, fixture_boat())], MULTI_RECORD_HS);
        write_multi_uop(&dir, &[(FIXTURE_HOUSE_ID, house.clone())], FIXTURE_CLILOCS);
        let files = MultiFiles::from_uopath(&dir).unwrap();
        assert!(
            files.index.is_none(),
            "a UOP package carries its own index: {files:?}"
        );
        let data = MultiData::from_files(&files).unwrap();
        expect_pieces(&data, FIXTURE_HOUSE_ID, &house);
        let _ = fs::remove_dir_all(&dir);
    }

    /// The width of the fields every multi record is built from.
    const MULTI_U16_FIELD: usize = 2;
    const MULTI_U32_FIELD: usize = 4;

    #[test]
    fn every_multi_record_is_as_wide_as_the_fields_it_holds() {
        // A component names a graphic and an x, a y and a z offset, then says
        // whether it is part of the building.
        let offsets = 4 * MULTI_U16_FIELD;
        assert_eq!(MULTI_RECORD_OLD, offsets + MULTI_U32_FIELD);
        // High Seas puts one more field after the flags of a component.
        assert_eq!(MULTI_RECORD_HS, MULTI_RECORD_OLD + MULTI_U32_FIELD);
        // A UOP component holds a narrower flags field, then the number of
        // cliloc ids that follow it.
        assert_eq!(
            MULTI_UOP_RECORD,
            offsets + MULTI_U16_FIELD + MULTI_U32_FIELD
        );
        assert_eq!(MULTI_UOP_CLILOC, MULTI_U32_FIELD);
        // A UOP entry starts with the multi id and the component count.
        assert_eq!(MULTI_UOP_HEADER, 2 * MULTI_U32_FIELD);
        // An index record is a lookup, a length, and an extra field.
        assert_eq!(MULTI_IDX_RECORD, 3 * MULTI_U32_FIELD);
    }

    #[test]
    fn the_length_of_the_blocks_tells_the_two_record_sizes_apart() {
        assert!(multi_is_hs(&[
            MULTI_RECORD_HS as u32,
            2 * MULTI_RECORD_HS as u32
        ]));
        assert!(!multi_is_hs(&[5 * MULTI_RECORD_OLD as u32]));
        let dir = scratch("multi-old");
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        let house = fixture_house();
        write_multi_mul(&dir, &[(FIXTURE_HOUSE_ID, house.clone())], MULTI_RECORD_OLD);
        let data = MultiData::open(&dir).unwrap();
        expect_pieces(&data, FIXTURE_HOUSE_ID, &house);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn multi_files_that_are_not_there_are_refused() {
        let dir = scratch("multi-missing");
        assert!(
            matches!(MultiData::open(&dir), Err(MapError::Missing(_))),
            "no tiledata means no multis"
        );
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        assert!(matches!(MultiData::open(&dir), Err(MapError::Missing(_))));
        fs::write(dir.join(MULTI_MUL_NAME), Vec::new()).unwrap();
        assert!(
            matches!(MultiData::open(&dir), Err(MapError::Missing(_))),
            "a multi.mul without a multi.idx is refused"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_truncated_multi_file_gives_back_only_whole_records() {
        let dir = scratch("multi-cut");
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        let house = fixture_house();
        write_multi_mul(&dir, &[(FIXTURE_HOUSE_ID, house.clone())], MULTI_RECORD_HS);
        let whole = fs::read(dir.join(MULTI_MUL_NAME)).unwrap();
        fs::write(dir.join(MULTI_MUL_NAME), &whole[..whole.len() / 2]).unwrap();
        let data = MultiData::open(&dir).unwrap();
        let pieces = data.pieces(FIXTURE_HOUSE_ID);
        assert!(
            pieces.len() < house.len(),
            "half a file cannot hold the whole house: {pieces:?}"
        );
        assert_eq!(
            pieces.first().map(|piece| piece.graphic),
            Some(house[0].graphic),
            "what is left of the file still reads right"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_index_that_claims_more_than_the_file_holds_is_capped() {
        let dir = scratch("multi-wild");
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        write_multi_mul(
            &dir,
            &[(FIXTURE_HOUSE_ID, fixture_house())],
            MULTI_RECORD_HS,
        );
        let mut idx = fs::read(dir.join(MULTI_IDX_NAME)).unwrap();
        let at = FIXTURE_HOUSE_ID as usize * MULTI_IDX_RECORD;
        idx[at..at + 4].copy_from_slice(&FIXTURE_WILD_LOOKUP.to_le_bytes());
        idx[at + 4..at + 8].copy_from_slice(&FIXTURE_WILD_LENGTH.to_le_bytes());
        fs::write(dir.join(MULTI_IDX_NAME), idx).unwrap();
        let data = MultiData::open(&dir).unwrap();
        assert!(
            data.pieces(FIXTURE_HOUSE_ID).is_empty(),
            "a block that starts past the end of the file holds nothing"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// An index entry that points far past the end of the file it names.
    const FIXTURE_WILD_LOOKUP: u32 = 0x7FFF_FFFF;
    const FIXTURE_WILD_LENGTH: u32 = 0x7FFF_FFF0;
    /// Bytes that are no multi file at all.
    const FIXTURE_RUBBISH: [u8; 40] = [0xA5; 40];

    #[test]
    fn rubbish_in_the_multi_files_is_read_without_a_panic() {
        let dir = scratch("multi-rubbish");
        fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
        fs::write(dir.join(MULTI_MUL_NAME), FIXTURE_RUBBISH).unwrap();
        fs::write(dir.join(MULTI_IDX_NAME), FIXTURE_RUBBISH).unwrap();
        let data = MultiData::open(&dir).unwrap();
        for id in 0..=FIXTURE_BOAT_ID {
            let _ = data.pieces(id);
        }
        fs::write(dir.join(MULTI_UOP_NAMES[0]), FIXTURE_RUBBISH).unwrap();
        assert!(
            matches!(MultiData::open(&dir), Err(MapError::BadUop)),
            "a package that is not a package is refused"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// A small stone house, and the first multi of the file, a small boat.
    /// Measured on the UOAlive client files: the house holds 148 components,
    /// 145 of them part of the building.
    const HOUSE_MULTI_ID: u16 = 100;
    const BOAT_MULTI_ID: u16 = 0;
    /// A house is more than a handful of tiles.
    const HOUSE_PIECE_FLOOR: usize = 20;
    /// No multi in the client files reaches this far from its own position.
    const MULTI_SPAN_LIMIT: i16 = 64;
    /// A building covers more than one tile.
    const MULTI_SPAN_FLOOR: i16 = 2;

    #[test]
    fn a_house_multi_and_a_boat_multi_load_from_the_client_files() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let data = MultiData::open(&dir).expect("open the multi files");
        assert!(data.multi_count() > 0, "the client files describe multis");
        let pieces = data.pieces(HOUSE_MULTI_ID);
        assert!(
            pieces.len() >= HOUSE_PIECE_FLOOR,
            "multi {HOUSE_MULTI_ID} has only {} pieces",
            pieces.len()
        );
        let west = pieces.iter().map(|piece| piece.dx).min().unwrap();
        let east = pieces.iter().map(|piece| piece.dx).max().unwrap();
        let north = pieces.iter().map(|piece| piece.dy).min().unwrap();
        let south = pieces.iter().map(|piece| piece.dy).max().unwrap();
        assert!(
            west > -MULTI_SPAN_LIMIT && east < MULTI_SPAN_LIMIT,
            "multi {HOUSE_MULTI_ID} spans x {west}..{east}"
        );
        assert!(
            north > -MULTI_SPAN_LIMIT && south < MULTI_SPAN_LIMIT,
            "multi {HOUSE_MULTI_ID} spans y {north}..{south}"
        );
        assert!(
            east - west >= MULTI_SPAN_FLOOR && south - north >= MULTI_SPAN_FLOOR,
            "multi {HOUSE_MULTI_ID} is a building, not a tile: x {west}..{east} y {north}..{south}"
        );
        assert!(
            pieces.iter().any(|piece| piece.blocks()),
            "a house has a wall a person cannot walk through"
        );
        assert!(
            pieces.iter().any(|piece| piece.surface()),
            "a house has a floor a person can stand on"
        );
        let boat = data.pieces(BOAT_MULTI_ID);
        assert!(
            boat.iter().any(|piece| piece.blocks()),
            "a boat has a hull: {boat:?}"
        );
    }

    #[test]
    fn the_mul_pair_and_the_uop_package_describe_the_same_multis() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let uop = MultiData::open(&dir).expect("open the multi files");
        let pair = MultiData::from_files(&MultiFiles {
            multi: dir.join(MULTI_MUL_NAME),
            index: Some(dir.join(MULTI_IDX_NAME)),
            tiledata: dir.join(TILEDATA_NAME),
        })
        .expect("open multi.mul and multi.idx");
        assert_eq!(pair.pieces(HOUSE_MULTI_ID), uop.pieces(HOUSE_MULTI_ID));
        assert_eq!(pair.pieces(BOAT_MULTI_ID), uop.pieces(BOAT_MULTI_ID));
        assert!(
            uop.multi_count() >= pair.multi_count(),
            "the package holds at least what the pair holds: {} and {}",
            uop.multi_count(),
            pair.multi_count()
        );
    }

    /// The header a UOP package starts with: magic, version, timestamp, the
    /// offset of the first block, the block size, and the file count.
    const UOP_HEADER_LEN: u64 = 28;
    const UOP_VERSION: u32 = 5;
    const UOP_BLOCK_SIZE: u32 = 100;
    /// One entry of a block: offset, header length, compressed length,
    /// decompressed length, name hash, data hash, compression.
    const UOP_ENTRY_LEN: usize = 34;
    const UOP_NO_HEADER: i32 = 0;
    const UOP_NO_HASH: u32 = 0;
    const UOP_LAST_BLOCK: i64 = 0;

    /// Packs files into a UOP package the way a client ships one: a header,
    /// one block that names every file by the hash of its name, then the
    /// bytes of each file, uncompressed.
    fn pack_uop(files: &[(String, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&UOP_MAGIC.to_le_bytes());
        out.extend_from_slice(&UOP_VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&UOP_HEADER_LEN.to_le_bytes());
        out.extend_from_slice(&UOP_BLOCK_SIZE.to_le_bytes());
        out.extend_from_slice(&(files.len() as i32).to_le_bytes());
        out.extend_from_slice(&(files.len() as i32).to_le_bytes());
        out.extend_from_slice(&UOP_LAST_BLOCK.to_le_bytes());
        let table_at = out.len();
        for (name, bytes) in files {
            out.extend_from_slice(&0i64.to_le_bytes());
            out.extend_from_slice(&UOP_NO_HEADER.to_le_bytes());
            out.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
            out.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
            out.extend_from_slice(&hash_filename(name).to_le_bytes());
            out.extend_from_slice(&UOP_NO_HASH.to_le_bytes());
            out.extend_from_slice(&UOP_COMPRESS_NONE.to_le_bytes());
        }
        for (slot, (_, bytes)) in files.iter().enumerate() {
            let offset_at = table_at + slot * UOP_ENTRY_LEN;
            let data_at = out.len() as i64;
            out[offset_at..offset_at + 8].copy_from_slice(&data_at.to_le_bytes());
            out.extend_from_slice(bytes);
        }
        out
    }

    fn pack_legacy_uop(map_index: u8, mul_bytes: &[u8]) -> Vec<u8> {
        pack_uop(&[(map_uop_name(map_index, 0), mul_bytes.to_vec())])
    }

    // The client text database, and the speech keywords a shard listens by.

    /// Three message numbers a shard sent to a character in one evening, and
    /// the sentences the client files hold for them. The first is the refusal
    /// that reached her journal as `System: #1001018`, a number she could not
    /// read while the server was stating the reason plainly.
    const REFUSAL_NUMBER: u32 = 1001018;
    const REFUSAL_TEXT: &str = "You cannot perform negative acts on your target.";
    const FAME_NUMBER: u32 = 1019051;
    const FAME_TEXT: &str = "You have gained a little fame.";
    const KARMA_NUMBER: u32 = 1019059;
    const KARMA_TEXT: &str = "You have gained a little karma.";

    #[test]
    fn the_text_database_reads_the_messages_the_shard_sent() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let raw = fs::read(dir.join(CLILOC_ENU_NAMES[0])).expect("the client text database");
        assert_eq!(
            raw.get(CLILOC_COMPRESSED_MARK_AT),
            Some(&CLILOC_COMPRESSED_MARK),
            "this client ships the compressed form, or the reader is proving nothing"
        );

        let text = ClilocData::open(&dir).expect("read the client text database");
        assert_eq!(text.text(REFUSAL_NUMBER), Some(REFUSAL_TEXT));
        assert_eq!(text.text(FAME_NUMBER), Some(FAME_TEXT));
        assert_eq!(text.text(KARMA_NUMBER), Some(KARMA_TEXT));
        assert_eq!(
            text.render(REFUSAL_NUMBER, ""),
            Some(REFUSAL_TEXT.to_string()),
            "a message with no blanks reads the same rendered"
        );
        assert!(
            text.message_count() > MANY_MESSAGES,
            "the whole database was read: {}",
            text.message_count()
        );
    }

    /// The client files hold far more messages than this. The test only has to
    /// prove the reader did not stop after the first records.
    const MANY_MESSAGES: usize = 100_000;

    /// The header of a written text database: a version field and a language
    /// field, neither of which any reader uses.
    const FIXTURE_CLILOC_VERSION: u32 = 2;
    const FIXTURE_CLILOC_LANGUAGE: u16 = 1;
    /// Every record of the client files carries this flag byte.
    const FIXTURE_CLILOC_FLAG: u8 = 0;
    /// A message with one blank in it, and one with two.
    const ONE_BLANK_NUMBER: u32 = 1042971;
    const ONE_BLANK_TEXT: &str = "~1_NOTHING~";
    const TWO_BLANK_NUMBER: u32 = 1060658;
    const TWO_BLANK_TEXT: &str = "~1_val~: ~2_val~";
    /// A message an argument can name instead of spelling its text out.
    const NAMED_NUMBER: u32 = 3000000;
    const NAMED_TEXT: &str = "gold coin";
    /// A number the fixture deliberately leaves out.
    const UNKNOWN_NUMBER: u32 = 9_999_999;

    const FIXTURE_MESSAGES: [(u32, &str); 4] = [
        (REFUSAL_NUMBER, REFUSAL_TEXT),
        (ONE_BLANK_NUMBER, ONE_BLANK_TEXT),
        (TWO_BLANK_NUMBER, TWO_BLANK_TEXT),
        (NAMED_NUMBER, NAMED_TEXT),
    ];

    /// Writes a plain text database the way the client files lay one out, and
    /// gives back the bytes so a test can measure them.
    fn write_cliloc(path: &Path, messages: &[(u32, &str)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FIXTURE_CLILOC_VERSION.to_le_bytes());
        data.extend_from_slice(&FIXTURE_CLILOC_LANGUAGE.to_le_bytes());
        for (number, text) in messages {
            data.extend_from_slice(&number.to_le_bytes());
            data.push(FIXTURE_CLILOC_FLAG);
            data.extend_from_slice(&(text.len() as u16).to_le_bytes());
            data.extend_from_slice(text.as_bytes());
        }
        fs::write(path, &data).unwrap();
        data
    }

    fn fixture_cliloc(dir: &Path) -> ClilocData {
        let path = dir.join(CLILOC_ENU_NAMES[0]);
        let written = write_cliloc(&path, &FIXTURE_MESSAGES);
        let expected: usize = CLILOC_HEADER
            + FIXTURE_MESSAGES
                .iter()
                .map(|(_, text)| CLILOC_RECORD_HEADER + text.len())
                .sum::<usize>();
        assert_eq!(
            written.len(),
            expected,
            "the written database is header plus one record for each message"
        );
        assert_ne!(
            written.get(CLILOC_COMPRESSED_MARK_AT),
            Some(&CLILOC_COMPRESSED_MARK),
            "a plain database must not read as a compressed one"
        );
        ClilocData::open(dir).expect("read the written text database")
    }

    #[test]
    fn a_written_text_database_pins_the_record_layout() {
        let dir = scratch("cliloc-layout");
        let text = fixture_cliloc(&dir);
        assert_eq!(text.message_count(), FIXTURE_MESSAGES.len());
        for (number, written) in FIXTURE_MESSAGES {
            assert_eq!(text.text(number), Some(written));
        }
        assert_eq!(text.text(UNKNOWN_NUMBER), None);
        assert_eq!(text.render(UNKNOWN_NUMBER, ""), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn arguments_fill_the_blanks_of_a_message() {
        let dir = scratch("cliloc-arguments");
        let text = fixture_cliloc(&dir);

        // The shard wraps an item's name in an empty prefix and an empty
        // suffix, so the tabs in front of it belong to the format.
        assert_eq!(
            text.render(ONE_BLANK_NUMBER, "\ta wooden chair\t"),
            Some("a wooden chair".to_string())
        );
        assert_eq!(
            text.render(TWO_BLANK_NUMBER, "Weight\t14 stones"),
            Some("Weight: 14 stones".to_string())
        );
        // A blank no argument reaches is filled with nothing.
        assert_eq!(
            text.render(TWO_BLANK_NUMBER, "Weight"),
            Some("Weight: ".to_string()),
            "the second blank has no argument to fill it"
        );
        assert_eq!(
            text.render(TWO_BLANK_NUMBER, ""),
            Some(": ".to_string()),
            "no arguments at all leaves both blanks empty"
        );
        // An argument may name a message instead of spelling it out: with a
        // hash always, and bare only when the shard sent more than one.
        assert_eq!(
            text.render(ONE_BLANK_NUMBER, "#3000000"),
            Some(NAMED_TEXT.to_string())
        );
        assert_eq!(
            text.render(TWO_BLANK_NUMBER, "3000000\t3000000"),
            Some(format!("{NAMED_TEXT}: {NAMED_TEXT}"))
        );
        assert_eq!(
            text.render(ONE_BLANK_NUMBER, "3000000"),
            Some("3000000".to_string()),
            "one bare number is an argument of its own, not a message"
        );
        assert_eq!(
            text.render(ONE_BLANK_NUMBER, "#9999999"),
            Some("#9999999".to_string()),
            "an argument naming a message the files do not hold keeps its number"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// A blank numbered zero, which counts from one and so reaches no
    /// argument, and text between two marks that is no blank at all.
    const ODD_BLANK_NUMBER: u32 = 3000001;
    const ODD_BLANK_TEXT: &str = "[~0_val~][~x~][~1_val~]";

    #[test]
    fn a_blank_no_argument_answers_is_filled_with_nothing() {
        let dir = scratch("cliloc-odd-blanks");
        let path = dir.join(CLILOC_ENU_NAMES[0]);
        write_cliloc(&path, &[(ODD_BLANK_NUMBER, ODD_BLANK_TEXT)]);
        let text = ClilocData::open(&dir).expect("read the written text database");
        assert_eq!(
            text.render(ODD_BLANK_NUMBER, "here"),
            Some("[][~x~][here]".to_string()),
            "a blank numbered zero empties, and text that is not a blank stays"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_text_database_is_refused() {
        let dir = scratch("cliloc-damaged");
        let path = dir.join(CLILOC_ENU_NAMES[0]);
        let whole = write_cliloc(&path, &FIXTURE_MESSAGES);

        for cut in [0, CLILOC_HEADER, CLILOC_HEADER + CLILOC_RECORD_HEADER - 1] {
            fs::write(&path, &whole[..cut]).unwrap();
            assert!(
                ClilocData::open(&dir).is_err(),
                "a database of {cut} bytes holds no message"
            );
        }
        // A record whose text runs past the end of the file.
        fs::write(&path, &whole[..whole.len() - 1]).unwrap();
        assert!(
            ClilocData::open(&dir).is_err(),
            "the last record is cut short"
        );
        // The compressed form, with nothing behind the mark that says so.
        let mut rubbish = vec![0u8; CLILOC_HEADER + CLILOC_RECORD_HEADER];
        rubbish[CLILOC_COMPRESSED_MARK_AT] = CLILOC_COMPRESSED_MARK;
        fs::write(&path, &rubbish).unwrap();
        assert!(
            ClilocData::open(&dir).is_err(),
            "a compressed database with no body is refused, not read"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// The phrase a character said out loud, correctly, and the keyword the
    /// shard listens for. A server handles that keyword in its own keyword
    /// table, and the client files hold the phrase against the same number.
    const RENOUNCE_PHRASE: &str = "i renounce my young player status";
    const RENOUNCE_KEYWORD: u16 = 0x0035;
    /// A phrase that reads almost the same and matches nothing: the keyword
    /// has to end a word, and `statuses` carries on.
    const NEARLY_RENOUNCE_PHRASE: &str = "i renounce my young player statuses";

    #[test]
    fn the_speech_file_holds_the_keyword_the_shard_listens_for() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let speech = SpeechData::open(&dir).expect("read the client speech file");
        assert!(
            speech.phrase_count() > MANY_PHRASES,
            "the whole file was read: {}",
            speech.phrase_count()
        );

        let said = speech.keywords(ClientVersion::MODERN, RENOUNCE_PHRASE);
        assert_eq!(
            said.first(),
            Some(&RENOUNCE_KEYWORD),
            "the keyword the shard listens for comes first: {said:?}"
        );
        assert!(
            said.windows(2).all(|pair| pair[0] <= pair[1]),
            "the numbers go out lowest first: {said:?}"
        );
        assert!(
            speech
                .keywords(ClientVersion::MODERN, NEARLY_RENOUNCE_PHRASE)
                .is_empty(),
            "a word that only starts with the phrase is not the phrase"
        );
        assert_eq!(
            speech.keywords(
                ClientVersion::MODERN,
                "  I RENOUNCE MY YOUNG PLAYER STATUS  "
            ),
            said,
            "spaces around it and the case it was typed in change nothing"
        );
    }

    /// The client files hold thousands of phrases. The test only has to prove
    /// the reader did not stop after the first records.
    const MANY_PHRASES: usize = 1_000;

    /// A written keyword file, one phrase for each shape a phrase has, with
    /// the numbers the client files really give them. The bank phrase is
    /// written twice because the client files write it twice, and a phrase
    /// with no text at all is a record the reader must step over.
    const FIXTURE_PHRASES: [(u16, &str); 6] = [
        (BANK_KEYWORD, "*bank*"),
        (BANK_KEYWORD, "*bank*"),
        (RENOUNCE_KEYWORD, "i renounce my young player status*"),
        (STATUS_KEYWORD, "*status"),
        (TRAINER_KEYWORD, "*where is the *trainer*"),
        (EMPTY_KEYWORD, ""),
    ];
    const BANK_KEYWORD: u16 = 0x0002;
    const STATUS_KEYWORD: u16 = 0x0174;
    const TRAINER_KEYWORD: u16 = 0x00E3;
    const EMPTY_KEYWORD: u16 = 0x0001;
    /// How many of those records carry a phrase.
    const FIXTURE_PHRASES_WITH_TEXT: usize = FIXTURE_PHRASES.len() - 1;

    fn write_speech(path: &Path, phrases: &[(u16, &str)]) -> Vec<u8> {
        let mut data = Vec::new();
        for (keyword, text) in phrases {
            data.extend_from_slice(&keyword.to_be_bytes());
            data.extend_from_slice(&(text.len() as u16).to_be_bytes());
            data.extend_from_slice(text.as_bytes());
        }
        fs::write(path, &data).unwrap();
        data
    }

    fn fixture_speech(dir: &Path) -> SpeechData {
        let path = dir.join(SPEECH_MUL_NAMES[0]);
        let written = write_speech(&path, &FIXTURE_PHRASES);
        let expected: usize = FIXTURE_PHRASES
            .iter()
            .map(|(_, text)| SPEECH_RECORD_HEADER + text.len())
            .sum();
        assert_eq!(
            written.len(),
            expected,
            "the written file is one record for each phrase and nothing else"
        );
        SpeechData::open(dir).expect("read the written speech file")
    }

    #[test]
    fn a_written_speech_file_pins_the_record_layout() {
        let dir = scratch("speech-layout");
        let speech = fixture_speech(&dir);
        assert_eq!(
            speech.phrase_count(),
            FIXTURE_PHRASES_WITH_TEXT,
            "a record with no text carries no phrase"
        );
        assert_eq!(
            speech.keywords(ClientVersion::MODERN, RENOUNCE_PHRASE),
            vec![RENOUNCE_KEYWORD, STATUS_KEYWORD],
            "both phrases the words match, lowest number first"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn wildcards_match_at_the_start_the_end_and_the_middle() {
        let dir = scratch("speech-wildcards");
        let speech = fixture_speech(&dir);

        // A wildcard at each end: the words may sit anywhere in the sentence.
        assert_eq!(
            speech.keywords(ClientVersion::MODERN, "deposit that at the bank please"),
            vec![BANK_KEYWORD, BANK_KEYWORD],
            "the file writes that phrase twice, so it counts twice"
        );
        // A wildcard at the end only: the words have to open the sentence.
        assert_eq!(
            speech.keywords(
                ClientVersion::MODERN,
                "i renounce my young player status now"
            ),
            vec![RENOUNCE_KEYWORD]
        );
        assert_eq!(
            speech.keywords(
                ClientVersion::MODERN,
                "now i renounce my young player status"
            ),
            vec![STATUS_KEYWORD],
            "with no wildcard in front that phrase has to open what was said, so only the \
             phrase that does carry one in front matches"
        );
        // A wildcard at the start only: the words have to close the sentence.
        assert_eq!(
            speech.keywords(ClientVersion::MODERN, "tell me my status"),
            vec![STATUS_KEYWORD]
        );
        assert!(
            speech
                .keywords(ClientVersion::MODERN, "my status is fine")
                .is_empty(),
            "with no wildcard behind it, the phrase has to close what was said"
        );
        // A wildcard in the middle splits the phrase, and either half matching
        // is enough.
        assert_eq!(
            speech.keywords(ClientVersion::MODERN, "the trainer is over there"),
            vec![TRAINER_KEYWORD],
            "the half behind the middle wildcard matches on its own"
        );
        // A phrase that must not match: the letters are there, the word is not.
        assert!(
            speech
                .keywords(ClientVersion::MODERN, "i walked along the embankment")
                .is_empty(),
            "a keyword inside a longer word is not a keyword"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// The oldest version that sends keyword numbers, with the letter dropped
    /// off it. The reference client sends none below `3.0.5d`.
    const BELOW_THE_KEYWORD_GATE: ClientVersion = ClientVersion {
        major: 3,
        minor: 0,
        revision: 5,
        patch: 0,
    };

    #[test]
    fn an_old_client_sends_no_keyword_numbers_at_all() {
        let dir = scratch("speech-version");
        let speech = fixture_speech(&dir);
        for old in [ClientVersion::T2A, BELOW_THE_KEYWORD_GATE] {
            assert!(
                speech.keywords(old, RENOUNCE_PHRASE).is_empty(),
                "a client of {old} sends the words with no numbers beside them"
            );
        }
        for new in [KEYWORD_SPEECH_MIN_VERSION, ClientVersion::MODERN] {
            assert!(
                !speech.keywords(new, RENOUNCE_PHRASE).is_empty(),
                "a client of {new} sends the numbers"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_speech_file_is_refused() {
        let dir = scratch("speech-damaged");
        let path = dir.join(SPEECH_MUL_NAMES[0]);
        let whole = write_speech(&path, &FIXTURE_PHRASES);

        for cut in [0, SPEECH_RECORD_HEADER - 1] {
            fs::write(&path, &whole[..cut]).unwrap();
            assert!(
                SpeechData::open(&dir).is_err(),
                "a file of {cut} bytes holds no phrase"
            );
        }
        fs::write(&path, &whole[..whole.len() - 1]).unwrap();
        assert!(
            SpeechData::open(&dir).is_err(),
            "the last record is cut short"
        );
        write_speech(&path, &[(EMPTY_KEYWORD, "")]);
        assert!(
            SpeechData::open(&dir).is_err(),
            "a file of records with no text in them leaves nothing to match"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn one_text_and_speech_reader_serve_more_than_one_session() {
        let dir = scratch("shared-readers");
        let text = Arc::new(fixture_cliloc(&dir));
        let speech = Arc::new(fixture_speech(&dir));
        let other_text = Arc::clone(&text);
        let other_speech = Arc::clone(&speech);
        let reader = std::thread::spawn(move || {
            (
                other_text.text(REFUSAL_NUMBER).map(str::to_string),
                other_speech.keywords(ClientVersion::MODERN, RENOUNCE_PHRASE),
            )
        });
        let (read_text, read_keywords) = reader.join().unwrap();
        assert_eq!(read_text.as_deref(), text.text(REFUSAL_NUMBER));
        assert_eq!(
            read_keywords,
            speech.keywords(ClientVersion::MODERN, RENOUNCE_PHRASE)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// The heights of the three tiles the flat route crosses. The middle one
    /// stands a whole step above what a walk climbs, so only the flat search
    /// crosses it, and the route it gives back has to say so.
    const FLAT_GROUND_Z: i8 = 4;
    const FLAT_CLIFF_Z: i8 = 12;

    #[test]
    fn a_flat_route_records_the_ground_it_lands_on() {
        let mut map = MockMap::new(3, 1);
        map.set_z(0, 0, FLAT_GROUND_Z);
        map.set_z(1, 0, FLAT_CLIFF_Z);
        map.set_z(2, 0, FLAT_GROUND_Z);
        let start = Point3::new(0, 0, FLAT_GROUND_Z);
        let goal = Point3::new(2, 0, FLAT_GROUND_Z);
        assert_eq!(
            pathfind(&map, start, goal, &Obstacles::NONE).unwrap_err(),
            PathError::Unreachable,
            "the cliff is higher than one step, so the height-aware search refuses it"
        );

        let path = pathfind_flat(&map, start, goal, &Obstacles::NONE)
            .expect("the flat search crosses the cliff");
        assert_eq!(
            path.steps
                .iter()
                .map(|s| (s.x, s.y, s.z))
                .collect::<Vec<_>>(),
            vec![(1, 0, FLAT_CLIFF_Z), (2, 0, FLAT_GROUND_Z)],
            "each step carries the height of the ground it lands on"
        );
    }

    #[test]
    fn a_flat_route_over_the_client_files_lands_on_real_ground() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        let last = BANK_RAMP_HEIGHTS.len() - 1;
        let start = Point3::new(BANK_RAMP_X, BANK_RAMP_FOOT_Y, BANK_RAMP_FOOT_Z);
        let top_y = BANK_RAMP_FOOT_Y + last as u16;
        let goal = Point3::new(BANK_RAMP_X, top_y, BANK_RAMP_HEIGHTS[last]);
        let path = pathfind_flat(&map, start, goal, &Obstacles::NONE)
            .expect("a flat way up the ramp beside the bank");
        assert_eq!(
            path.steps.last().map(|s| (s.x, s.y, s.z)),
            Some((goal.x, goal.y, goal.z)),
            "the route ends on the ground the shard measured: {:?}",
            path.steps
        );
        for step in &path.steps {
            assert_eq!(
                step.z,
                map.tile_from(start.z, step.x, step.y).z,
                "every step of the flat route stands on the ground of its own tile: {:?}",
                path.steps
            );
        }
        assert!(
            path.steps.iter().any(|step| step.z != start.z),
            "the ramp climbs, so the route cannot hold one height: {:?}",
            path.steps
        );
    }
}
