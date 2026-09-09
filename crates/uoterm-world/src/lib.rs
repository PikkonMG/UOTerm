//! Per-session world model. Packets and local map data are the only writers.

mod events;
mod journal;
mod names;
mod observe;
mod radar;
mod state;

pub use events::{Event, EventKind, EVENT_LOG_CAP};
pub use journal::{
    Journal, JournalEntry, JOURNAL_CAP, JOURNAL_DEFAULT_WINDOW, JOURNAL_RECENT_LINES,
};
pub use names::{display_name, NameBook};
pub use observe::{
    ContainedItem, NearbyDoor, NearbyItem, Observe, OpenContainer, OBSERVE_CONTAINER_CAP,
    OBSERVE_CONTAINER_ITEM_CAP, OBSERVE_DOOR_RADIUS, OBSERVE_FACT_CAP, OBSERVE_ITEM_CAP,
    OBSERVE_MOBILE_CAP,
};
pub use radar::{
    default_tile, legend, render_radar, RadarOptions, TileKind, RADAR_DEFAULT, RADAR_SIZE,
};
pub use state::{
    facet_free_movement, facet_rules, is_ghost_body, Container, DoorItem, DoorUpdate, Item, Mobile,
    MultiItem, MultiUpdate, SelfState, SkillValue, World, BODY_GHOST_ELF_FEMALE,
    BODY_GHOST_ELF_MALE, BODY_GHOST_FEMALE, BODY_GHOST_MALE, FACET_RULES_FELUCCA,
    FACET_RULES_TRAMMEL, MAP_RULE_FREE_MOVEMENT,
};

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::{
        ContainerItem, EquipItem, GroundItem, Inbound, MobileView, ObjectProperty, OpenGump,
        Point3, Serial, SpeechLine, TargetCursor, FLAG_FROZEN, FLAG_HIDDEN, FLAG_WAR,
        GRAPHIC_BACKPACK, LAYER_BACKPACK, NOTO_INNOCENT,
    };

    /// The cliloc a shard uses for a name that has a prefix and a suffix field.
    const CLILOC_NAME_WITH_AFFIX: u32 = 1050045;
    /// One property list revision. Which number it is does not matter; that it
    /// stays the same between two replies does.
    const NAME_REVISION: u32 = 0x00C0_FFEE;
    const GRAPHIC_GOLD: u16 = 0x0EED;
    const GRAPHIC_BONE: u16 = 0x0F7E;
    const BACKPACK: Serial = Serial(0x4000_0100);
    const CORPSE: Serial = Serial(0x4000_0200);
    const GOLD: Serial = Serial(0x4000_0201);
    const BONE: Serial = Serial(0x4000_0202);
    const GOLD_AMOUNT: u16 = 54;
    const ONE_OF_IT: u16 = 1;
    const CORPSE_GUMP: u16 = 9;

    /// One item record of the kind `0x3C` and `0x25` carry.
    fn in_container(container: Serial, serial: Serial, graphic: u16, amount: u16) -> ContainerItem {
        ContainerItem {
            serial,
            graphic,
            amount,
            x: 0,
            y: 0,
            grid: 0,
            container,
            hue: 0,
        }
    }

    /// The `0xD6` reply that tells the client what one object is called.
    fn named(serial: Serial, name: &str) -> Inbound {
        Inbound::ObjectPropertyList {
            serial,
            hash: NAME_REVISION,
            properties: vec![ObjectProperty {
                cliloc: CLILOC_NAME_WITH_AFFIX,
                arguments: format!("\t{name}\t"),
            }],
        }
    }

    fn login(world: &mut World) {
        world.apply(&Inbound::LoginConfirm {
            serial: Serial(0xAB),
            body: 0x190,
            x: 10,
            y: 20,
            z: 1,
            direction: 4,
            map_width: 6144,
            map_height: 4096,
        });
        world.apply(&Inbound::LoginComplete);
    }

    #[test]
    fn login_sets_self_and_event() {
        let mut w = World::new();
        login(&mut w);
        assert!(w.logged_in);
        assert_eq!(w.self_state.serial, Serial(0xAB));
        assert_eq!(w.self_state.location, Point3::new(10, 20, 1));
        assert_eq!(w.map_width, 6144);
        assert!(w.events.iter().any(|e| e.kind == EventKind::LoggedIn));
    }

    #[test]
    fn login_confirm_alone_marks_logged_in() {
        let mut w = World::new();
        w.self_state.name = "Mara".into();
        w.apply(&Inbound::LoginConfirm {
            serial: Serial(0xAA),
            body: 0x190,
            x: 3507,
            y: 2513,
            z: 27,
            direction: 0,
            map_width: 7168,
            map_height: 4096,
        });
        assert!(w.logged_in, "0x1B login confirm must enter the world");
        assert_eq!(w.self_state.serial, Serial(0xAA));
        assert_eq!(
            w.events
                .iter()
                .filter(|e| e.kind == EventKind::LoggedIn)
                .count(),
            1
        );
        w.apply(&Inbound::LoginComplete);
        assert_eq!(
            w.events
                .iter()
                .filter(|e| e.kind == EventKind::LoggedIn)
                .count(),
            1,
            "0x55 must not emit a second logged_in event"
        );
    }

    #[test]
    fn radar_places_self() {
        let mut w = World::new();
        w.self_state.location = Point3::new(50, 50, 0);
        let radar = render_radar(&w, RadarOptions { size: 5 }, default_tile);
        assert!(radar.contains('@'));
        assert!(legend().contains("self"));
        let via_api = w.radar(5, &|_, _| TileKind::Walk);
        assert!(via_api.contains('@'));
    }

    #[test]
    fn observe_is_compact() {
        let mut w = World::new();
        w.self_state.name = "Mara".into();
        w.self_state.location = Point3::new(1, 1, 0);
        let obs = w.observe_default();
        assert!(obs.caption.contains("Mara"));
        assert!(obs.caption.contains("facing"));
        assert_eq!(obs.facing, "north");
        assert_eq!(obs.x, 1);
        assert_eq!(obs.y, 1);
        assert!(obs.radar.contains('@'));
        assert!(obs.nearby_items.is_empty());
        assert!(obs.doors.is_empty());
        let json = w.observe_json(&obs.radar);
        assert_eq!(json["self_state"]["name"], "Mara");
    }

    #[test]
    fn speech_goes_to_journal() {
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::Speech(SpeechLine {
            serial: Serial(0xCD),
            graphic: 0x190,
            kind: 0,
            hue: 0x03B2,
            name: "Aldreth".into(),
            text: "vendor buy".into(),
        }));
        assert_eq!(w.journal.last_lines(1)[0].text, "vendor buy");
        assert!(!w.journal.search("vendor").is_empty());
        assert!(w.events.iter().any(|e| e.kind == EventKind::Speech));
        assert!(w
            .observe_default()
            .journal
            .iter()
            .any(|l| l.contains("vendor")));
    }

    #[test]
    fn delete_removes_mobile_and_item() {
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: Serial(0x50),
            body: 0x190,
            x: 11,
            y: 20,
            z: 1,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: 1,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }));
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: Serial(0x4000_0002),
            graphic: 0x0EED,
            amount: 5,
            x: 12,
            y: 20,
            z: 1,
            hue: 0,
            multi: false,
        }));
        assert_eq!(w.find_mobiles(None, None, None).len(), 1);
        assert_eq!(w.find_items(Some(0x0EED), None, None).len(), 1);
        w.apply(&Inbound::Delete(Serial(0x50)));
        w.apply(&Inbound::Delete(Serial(0x4000_0002)));
        assert!(w.mobiles.is_empty());
        assert!(w.items.is_empty());
    }

    #[test]
    fn target_and_gump_lifecycle() {
        let mut w = World::new();
        w.apply(&Inbound::Target(TargetCursor {
            kind: 0,
            id: 9,
            flags: 0,
        }));
        assert!(w.pending_target.is_some());
        w.clear_target();
        assert!(w.pending_target.is_none());
        w.apply(&Inbound::Gump(OpenGump {
            serial: Serial(0x10),
            gump_id: 0x1B,
            x: 0,
            y: 0,
            layout: "{ button }".into(),
            text: vec!["OK".into()],
        }));
        assert_eq!(w.gumps.len(), 1);
        w.close_gump(0x1B);
        assert!(w.gumps.is_empty());
    }

    #[test]
    fn find_mobiles_filters_name_and_range() {
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: Serial(0x51),
            body: 0x190,
            x: 11,
            y: 20,
            z: 1,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: 1,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }));
        w.apply(&Inbound::Speech(SpeechLine {
            serial: Serial(0x51),
            graphic: 0x190,
            kind: 0,
            hue: 0,
            name: "Cedric".into(),
            text: "hi".into(),
        }));
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: Serial(0x52),
            body: 0x191,
            x: 80,
            y: 80,
            z: 1,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: 1,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }));
        assert_eq!(w.find_mobiles(Some("cedric"), None, Some(5)).len(), 1);
        assert!(w
            .find_mobiles(None, None, Some(5))
            .iter()
            .all(|m| m.serial == Serial(0x51)));
        assert_eq!(w.find_items(None, None, Some("unused")).len(), 0);
    }

    #[test]
    fn frozen_flag_sets_paralyzed() {
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::DrawPlayer {
            serial: Serial(0xAB),
            body: 0x190,
            hue: 0,
            flags: FLAG_FROZEN | FLAG_WAR,
            x: 10,
            y: 20,
            direction: 4,
            z: 1,
        });
        assert!(w.self_state.paralyzed);
        assert!(w.self_state.war);
        assert!(!w.self_state.hidden);
        w.apply(&Inbound::DrawPlayer {
            serial: Serial(0xAB),
            body: 0x190,
            hue: 0,
            flags: FLAG_HIDDEN,
            x: 10,
            y: 20,
            direction: 4,
            z: 1,
        });
        assert!(!w.self_state.paralyzed);
        assert!(w.self_state.hidden);
    }

    #[test]
    fn equipped_attaches_to_self() {
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::Equipped(EquipItem {
            serial: Serial(0x4000_00AA),
            graphic: 0x0F49,
            layer: 1,
            hue: 0,
        }));
        assert_eq!(w.self_state.equipment.len(), 1);
        assert_eq!(w.items[&Serial(0x4000_00AA)].parent, Some(Serial(0xAB)));
        w.apply(&Inbound::Equipped(EquipItem {
            serial: Serial(0x4000_00BB),
            graphic: 0x0F4B,
            layer: 1,
            hue: 0,
        }));
        assert_eq!(w.self_state.equipment.len(), 1);
        assert_eq!(w.self_state.equipment[0].serial, Serial(0x4000_00BB));
    }

    #[test]
    fn container_does_not_duplicate_serial() {
        let mut w = World::new();
        let item = in_container(CORPSE, GOLD, GRAPHIC_GOLD, ONE_OF_IT);
        w.apply(&Inbound::AddItem(item.clone()));
        w.apply(&Inbound::AddItem(item));
        assert_eq!(w.containers[&CORPSE].items.len(), 1);
    }

    /// Puts a corpse on the ground beside the character with gold and a bone
    /// in it, and a pack on the character with a bandage in it, and gives the
    /// server's name to each of the five.
    fn corpse_and_pack(w: &mut World) {
        const BANDAGE: Serial = Serial(0x4000_0101);
        const GRAPHIC_BANDAGE: u16 = 0x0E21;
        login(w);
        w.apply(&Inbound::Equipped(EquipItem {
            serial: BACKPACK,
            graphic: GRAPHIC_BACKPACK,
            layer: LAYER_BACKPACK,
            hue: 0,
        }));
        w.apply(&Inbound::AddItem(in_container(
            BACKPACK,
            BANDAGE,
            GRAPHIC_BANDAGE,
            ONE_OF_IT,
        )));
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: CORPSE,
            graphic: 0x2006,
            amount: ONE_OF_IT,
            x: 11,
            y: 20,
            z: 1,
            hue: 0,
            multi: false,
        }));
        w.apply(&Inbound::OpenContainer {
            serial: CORPSE,
            gump: CORPSE_GUMP,
        });
        w.apply(&Inbound::ContainerContents {
            items: vec![
                in_container(CORPSE, GOLD, GRAPHIC_GOLD, GOLD_AMOUNT),
                in_container(CORPSE, BONE, GRAPHIC_BONE, ONE_OF_IT),
            ],
        });
        w.apply(&named(BACKPACK, "a backpack"));
        w.apply(&named(BANDAGE, "clean bandages"));
        w.apply(&named(CORPSE, "a rotting corpse"));
        w.apply(&named(GOLD, "gold coins"));
        w.apply(&named(BONE, "a bone"));
    }

    #[test]
    fn an_opened_container_shows_what_it_holds() {
        let mut w = World::new();
        corpse_and_pack(&mut w);
        let obs = w.observe_default();

        let corpse = obs
            .containers
            .iter()
            .find(|c| c.serial == CORPSE.to_string())
            .expect("the corpse the character opened must be in the observation");
        assert_eq!(corpse.name, "a rotting corpse");
        assert_eq!(corpse.total, 2);
        assert_eq!(corpse.contents.len(), 2);
        let gold = corpse
            .contents
            .iter()
            .find(|i| i.serial == GOLD.to_string())
            .expect("the gold in the corpse must be in the observation");
        assert_eq!(gold.name, "gold coins", "a name must reach the agent");
        assert_eq!(gold.amount, GOLD_AMOUNT);
        assert_eq!(gold.graphic, GRAPHIC_GOLD);

        // The ground scene stays the ground scene: what is inside a container
        // is not on any tile and must not be reported as if it were.
        assert!(
            !obs.items.iter().any(|i| i.serial == GOLD),
            "contained items must not join the ground scene"
        );
        assert!(!obs
            .nearby_items
            .iter()
            .any(|i| i.serial == GOLD.to_string()));
        assert!(
            obs.nearby_items
                .iter()
                .any(|i| i.serial == CORPSE.to_string()),
            "the corpse itself stands on a tile and stays in the ground scene"
        );
    }

    #[test]
    fn a_carried_container_shows_what_it_holds() {
        let mut w = World::new();
        corpse_and_pack(&mut w);
        let obs = w.observe_default();
        let pack = obs
            .containers
            .iter()
            .find(|c| c.serial == BACKPACK.to_string())
            .expect("the pack the character wears must be in the observation");
        assert_eq!(pack.name, "a backpack");
        assert_eq!(pack.graphic, Some(GRAPHIC_BACKPACK));
        assert_eq!(pack.dist, Some(0), "a worn pack is at the character's tile");
        assert_eq!(pack.total, 1);
        assert_eq!(pack.contents[0].name, "clean bandages");
    }

    #[test]
    fn a_long_container_is_cut_and_says_so_without_crowding_the_scene() {
        const OVERFULL: usize = OBSERVE_CONTAINER_ITEM_CAP + 7;
        const FIRST_BANK_ITEM: u32 = 0x4000_1000;
        let mut w = World::new();
        corpse_and_pack(&mut w);
        let ground_before = w.observe_default().nearby_items.len();

        w.apply(&Inbound::OpenContainer {
            serial: BACKPACK,
            gump: CORPSE_GUMP,
        });
        for step in 0..OVERFULL as u32 {
            w.apply(&Inbound::AddItem(in_container(
                BACKPACK,
                Serial(FIRST_BANK_ITEM + step),
                GRAPHIC_GOLD,
                ONE_OF_IT,
            )));
        }

        let obs = w.observe_default();
        let pack = obs
            .containers
            .iter()
            .find(|c| c.serial == BACKPACK.to_string())
            .expect("the overfull pack must still be in the observation");
        assert_eq!(
            pack.contents.len(),
            OBSERVE_CONTAINER_ITEM_CAP,
            "a long container is cut at the cap"
        );
        assert_eq!(
            pack.total,
            OVERFULL + 1,
            "the count must say how many it really holds"
        );
        assert_eq!(
            obs.nearby_items.len(),
            ground_before,
            "a full container must not push anything out of the ground scene"
        );
        let corpse = obs
            .containers
            .iter()
            .find(|c| c.serial == CORPSE.to_string())
            .expect("the corpse must not be pushed out by the fuller container");
        assert_eq!(corpse.total, 2);
    }

    #[test]
    fn a_search_reaches_inside_a_container() {
        let mut w = World::new();
        corpse_and_pack(&mut w);

        let in_corpse = w.find_items(None, Some(CORPSE), Some("gold"));
        assert_eq!(in_corpse.len(), 1, "part of a name must match");
        assert_eq!(in_corpse[0].serial, GOLD);
        assert_eq!(in_corpse[0].amount, GOLD_AMOUNT);

        assert!(
            w.find_items(None, Some(CORPSE), Some("bandages"))
                .is_empty(),
            "a container search must not read another container"
        );
        assert_eq!(w.find_items(None, None, Some("bandages")).len(), 1);
        assert_eq!(
            w.find_items(Some(GRAPHIC_BONE), Some(CORPSE), None).len(),
            1
        );
        assert_eq!(w.find_items(None, None, Some("0x0EED")).len(), 1);
    }

    #[test]
    fn a_name_survives_the_container_being_sent_again() {
        let mut w = World::new();
        corpse_and_pack(&mut w);
        // A shard sends the whole of a container again every time it is
        // opened. Without the name carried over, looting a corpse twice would
        // leave every item in it nameless.
        w.apply(&Inbound::ContainerContents {
            items: vec![
                in_container(CORPSE, GOLD, GRAPHIC_GOLD, GOLD_AMOUNT),
                in_container(CORPSE, BONE, GRAPHIC_BONE, ONE_OF_IT),
            ],
        });
        assert_eq!(w.items[&GOLD].name, "gold coins");
        let obs = w.observe_default();
        let corpse = obs
            .containers
            .iter()
            .find(|c| c.serial == CORPSE.to_string())
            .expect("the corpse must survive being sent again");
        assert!(corpse.contents.iter().all(|i| !i.name.is_empty()));
    }

    #[test]
    fn death_then_living_draw_resurrects() {
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::Death {
            serial: Serial(0xAB),
            corpse: Serial(0x4000_0099),
        });
        assert!(w.self_state.dead);
        w.apply(&Inbound::DrawPlayer {
            serial: Serial(0xAB),
            body: 0x190,
            hue: 0,
            flags: 0,
            x: 10,
            y: 20,
            direction: 4,
            z: 1,
        });
        assert!(!w.self_state.dead);
        assert!(w.events.iter().any(|e| e.kind == EventKind::Resurrected));
    }

    /// The sequence number of the first step of a walk.
    const FIRST_STEP_SEQUENCE: u8 = 0;
    /// Where the server puts the character when it moves her itself: through a
    /// door, onto a boat, or out of a teleporter.
    const TELEPORTED_TO: Point3 = Point3 {
        x: 3505,
        y: 2525,
        z: 27,
    };
    /// The way she faces when it puts her there.
    const TELEPORTED_FACING: u8 = 2;

    /// The world model never guesses where the character is. Two packets put
    /// her on a tile: the one that places her in the world at login, and the
    /// one the server sends when it moves her itself. A move acknowledgement
    /// is neither: it carries a sequence number and no tile, so it moves
    /// nobody here. The session holds the step that sequence answers, and it
    /// is what walks her on.
    #[test]
    fn only_a_packet_that_carries_a_tile_moves_the_character() {
        let mut w = World::new();
        login(&mut w);
        assert_eq!(w.self_state.location, Point3::new(10, 20, 1));
        w.apply(&Inbound::DrawPlayer {
            serial: w.self_state.serial,
            body: 0x190,
            hue: 0,
            flags: 0,
            x: TELEPORTED_TO.x,
            y: TELEPORTED_TO.y,
            direction: TELEPORTED_FACING,
            z: TELEPORTED_TO.z,
        });
        assert_eq!(
            w.self_state.location, TELEPORTED_TO,
            "the server placed her, so that is where she is"
        );
        assert_eq!(w.self_state.direction, TELEPORTED_FACING);
        w.apply(&Inbound::MoveAck {
            sequence: FIRST_STEP_SEQUENCE,
            notoriety: NOTO_INNOCENT,
        });
        assert_eq!(
            w.self_state.location, TELEPORTED_TO,
            "an acknowledgement says a step was allowed, never where it ended"
        );
        assert_eq!(w.self_state.notoriety, NOTO_INNOCENT);
    }

    #[test]
    fn move_reject_snaps_location_and_set_nav() {
        let mut w = World::new();
        login(&mut w);
        w.self_state.location = Point3::new(11, 20, 1);
        w.apply(&Inbound::MoveReject {
            sequence: 1,
            x: 10,
            y: 20,
            direction: 4,
            z: 1,
        });
        assert_eq!(w.self_state.location, Point3::new(10, 20, 1));
        w.set_nav(Some(Point3::new(40, 40, 0)), 12);
        assert_eq!(w.path_len, 12);
    }

    #[test]
    fn radar_marks_mobile_and_item() {
        let mut w = World::new();
        w.self_state.location = Point3::new(10, 10, 0);
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: Serial(0x70),
            body: 0x190,
            x: 11,
            y: 10,
            z: 0,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: 1,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }));
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: Serial(0x4000_0070),
            graphic: 0x0EED,
            amount: 1,
            x: 9,
            y: 10,
            z: 0,
            hue: 0,
            multi: false,
        }));
        let radar = render_radar(&w, RadarOptions { size: 5 }, default_tile);
        assert!(radar.contains('m'));
        assert!(radar.contains('i'));
    }

    /// Ground truth measured on a live shard: the door of the New Haven inn,
    /// shut in its doorway and open one tile away, with the graphic it carries
    /// in each state.
    const INN_DOOR_SERIAL: Serial = Serial(1_073_744_340);
    const INN_DOOR_SHUT_GRAPHIC: u16 = 1701;
    const INN_DOOR_OPEN_GRAPHIC: u16 = 1702;
    const INN_DOORWAY: Point3 = Point3 {
        x: 3506,
        y: 2526,
        z: 27,
    };
    const INN_DOOR_SWUNG_TO: Point3 = Point3 {
        x: 3505,
        y: 2527,
        z: 27,
    };

    #[test]
    fn a_door_item_blocks_its_own_tile_until_it_swings() {
        let mut w = World::new();
        assert_eq!(
            w.note_door(INN_DOOR_SERIAL, INN_DOOR_SHUT_GRAPHIC, INN_DOORWAY),
            DoorUpdate::Learned
        );
        assert_eq!(
            w.door_tiles(),
            vec![INN_DOORWAY],
            "a shut door stands in its own doorway, so a route must go around it"
        );
        assert_eq!(
            w.note_door(INN_DOOR_SERIAL, INN_DOOR_SHUT_GRAPHIC, INN_DOORWAY),
            DoorUpdate::Unchanged,
            "the server repeats an item it already sent"
        );
        assert_eq!(
            w.note_door(INN_DOOR_SERIAL, INN_DOOR_OPEN_GRAPHIC, INN_DOOR_SWUNG_TO),
            DoorUpdate::Swung,
            "one click moves the door and changes its graphic"
        );
        assert_eq!(
            w.door_tiles(),
            vec![INN_DOOR_SWUNG_TO],
            "the doorway is clear once the leaf has swung off it"
        );
        assert_eq!(w.doors[&INN_DOOR_SERIAL].graphic, INN_DOOR_OPEN_GRAPHIC);
    }

    #[test]
    fn a_door_answers_by_graphic_alone_or_by_tile_alone() {
        let mut w = World::new();
        w.note_door(INN_DOOR_SERIAL, INN_DOOR_SHUT_GRAPHIC, INN_DOORWAY);
        assert_eq!(
            w.note_door(INN_DOOR_SERIAL, INN_DOOR_OPEN_GRAPHIC, INN_DOORWAY),
            DoorUpdate::Swung,
            "a new graphic on the same tile is the door answering"
        );
        let mut w = World::new();
        w.note_door(INN_DOOR_SERIAL, INN_DOOR_SHUT_GRAPHIC, INN_DOORWAY);
        assert_eq!(
            w.note_door(INN_DOOR_SERIAL, INN_DOOR_SHUT_GRAPHIC, INN_DOOR_SWUNG_TO),
            DoorUpdate::Swung,
            "a new tile with the same graphic is the door answering too"
        );
    }

    #[test]
    fn a_door_out_of_sight_blocks_nothing() {
        let mut w = World::new();
        login(&mut w);
        w.note_door(INN_DOOR_SERIAL, INN_DOOR_SHUT_GRAPHIC, INN_DOORWAY);
        w.apply(&Inbound::Delete(INN_DOOR_SERIAL));
        assert!(
            w.door_tiles().is_empty(),
            "a door the server took away blocks no tile"
        );
        w.note_door(INN_DOOR_SERIAL, INN_DOOR_SHUT_GRAPHIC, INN_DOORWAY);
        w.forget_door(INN_DOOR_SERIAL);
        assert!(
            w.door_tiles().is_empty(),
            "and neither does an item whose graphic is no longer a door"
        );
    }

    #[test]
    fn event_seq_keeps_new_events_after_ring_cap() {
        let mut w = World::new();
        for i in 0..(EVENT_LOG_CAP + 50) {
            w.push_event(Event::new(EventKind::Speech, None, format!("{i}")));
        }
        assert_eq!(w.events.len(), EVENT_LOG_CAP);
        assert_eq!(w.event_seq, (EVENT_LOG_CAP + 50) as u64);
        let mut last = 0u64;
        let mut harvested = 0usize;
        for ev in &w.events {
            if ev.seq > last {
                harvested += 1;
                last = ev.seq;
            }
        }
        assert_eq!(harvested, EVENT_LOG_CAP);
        w.push_event(Event::new(EventKind::Speech, None, "after"));
        let extra = w.events.iter().filter(|e| e.seq > last).count();
        assert_eq!(extra, 1);
        assert_eq!(w.events.last().map(|e| e.seq), Some(w.event_seq));
    }

    /// The serial of a house, and the multi id of a small stone house.
    const HOUSE_SERIAL: Serial = Serial(0x4000_0064);
    const STONE_HOUSE_ID: u16 = 0x0064;
    const HOUSE_AT: Point3 = Point3 {
        x: 1442,
        y: 1659,
        z: 0,
    };
    /// Where a boat is after it has sailed one tile.
    const SAILED_TO: Point3 = Point3 {
        x: 1443,
        y: 1659,
        z: 0,
    };
    const FELUCCA_MAP_INDEX: u8 = 0;
    const TRAMMEL_MAP_INDEX: u8 = 1;
    /// The highest facet the client files hold. A shard may register more.
    const LAST_CLIENT_FACET: u8 = 5;
    /// A facet index past the six the client files hold.
    const SHARD_OWN_FACET: u8 = 32;
    const SOMEBODY_ELSE: Serial = Serial(0x0000_0051);
    const STANDING_AT: Point3 = Point3 { x: 11, y: 20, z: 1 };

    /// A building is one record on one serial, and the server keeps it up to
    /// date the way it keeps a door up to date: the same serial again with
    /// what has changed on it, and a delete when it is gone.
    #[test]
    fn a_building_is_recorded_moved_and_dropped_with_its_item() {
        let mut w = World::new();
        assert_eq!(
            w.note_multi(HOUSE_SERIAL, STONE_HOUSE_ID, HOUSE_AT),
            MultiUpdate::Learned
        );
        assert_eq!(
            w.multis_seen(),
            vec![MultiItem {
                serial: HOUSE_SERIAL,
                multi_id: STONE_HOUSE_ID,
                location: HOUSE_AT,
            }]
        );
        assert_eq!(
            w.note_multi(HOUSE_SERIAL, STONE_HOUSE_ID, HOUSE_AT),
            MultiUpdate::Unchanged,
            "the same packet again says nothing new"
        );
        assert_eq!(
            w.note_multi(HOUSE_SERIAL, STONE_HOUSE_ID, SAILED_TO),
            MultiUpdate::Moved,
            "a boat sails, and the record moves with it"
        );
        assert_eq!(w.multis_seen().first().map(|m| m.location), Some(SAILED_TO));
        w.apply(&Inbound::Delete(HOUSE_SERIAL));
        assert!(
            w.multis.is_empty(),
            "the item is gone, so the building is gone with it"
        );
    }

    /// The graphic on the item may stop naming a multi, and the record must go
    /// with it. Nothing else is left holding tiles a route goes around.
    #[test]
    fn a_building_is_forgotten_when_its_item_stops_being_one() {
        let mut w = World::new();
        w.note_multi(HOUSE_SERIAL, STONE_HOUSE_ID, HOUSE_AT);
        w.forget_multi(HOUSE_SERIAL);
        assert!(w.multis_seen().is_empty());
    }

    /// The facet table, read from `map-definitions.json`: only Felucca is
    /// without the free movement rule.
    #[test]
    fn only_felucca_makes_a_route_go_around_people() {
        assert_eq!(facet_rules(FELUCCA_MAP_INDEX), FACET_RULES_FELUCCA);
        assert!(!facet_free_movement(FELUCCA_MAP_INDEX));
        for map in TRAMMEL_MAP_INDEX..=LAST_CLIENT_FACET {
            assert_eq!(facet_rules(map), FACET_RULES_TRAMMEL, "facet {map}");
            assert!(facet_free_movement(map), "facet {map}");
        }
        assert_eq!(
            FACET_RULES_TRAMMEL & MAP_RULE_FREE_MOVEMENT,
            MAP_RULE_FREE_MOVEMENT,
            "the Trammel rules are the ones that carry free movement"
        );
        assert!(
            !facet_free_movement(SHARD_OWN_FACET),
            "a facet the client files do not hold is read as the stricter one"
        );
    }

    /// The measured cost of treating every mobile as a wall: on a
    /// Trammel-ruleset facet the server lets the character walk straight
    /// through anybody, so a route that goes around them is longer than it
    /// needs to be and has him dodge people he could have walked through.
    #[test]
    fn a_player_blocks_a_route_on_felucca_and_not_on_a_free_movement_facet() {
        let mut w = World::new();
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: SOMEBODY_ELSE,
            body: 0x190,
            x: STANDING_AT.x,
            y: STANDING_AT.y,
            z: STANDING_AT.z,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: NOTO_INNOCENT,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }));
        w.apply(&Inbound::MapChange {
            map: FELUCCA_MAP_INDEX,
        });
        assert!(!w.free_movement());
        assert_eq!(
            w.blocking_mobile_tiles(),
            vec![STANDING_AT],
            "on Felucca he is in the way"
        );
        w.apply(&Inbound::MapChange {
            map: TRAMMEL_MAP_INDEX,
        });
        assert!(w.free_movement());
        assert!(
            w.blocking_mobile_tiles().is_empty(),
            "and on Trammel the same player is not"
        );
    }
}
