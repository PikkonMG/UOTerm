//! Per-session world model. Packets and local map data are the only writers.

mod addressed;
mod assist;
mod events;
mod journal;
mod names;
mod observe;
mod radar;
mod state;

pub use addressed::{
    asks_if_bot, names_character, Channel, ChannelGroup, SpokenTo, SpokenToLog, CHAT_MODE_BASIC,
    CHAT_MODE_PLAY_ALONG, SPOKEN_TO_FRESH_MS, SPOKEN_TO_KEEP,
};
pub use assist::{AssistFeature, AssistRules};
pub use events::{unix_now_ms, Event, EventKind, EVENT_LOG_CAP};
pub use journal::{
    Journal, JournalEntry, JOURNAL_CAP, JOURNAL_DEFAULT_WINDOW, JOURNAL_RECENT_LINES,
};
pub use names::{display_name, display_title, NameBook};
pub use observe::{
    BankView, ContainedItem, NearbyDoor, NearbyItem, Observe, OpenContainer, PlayingAlong,
    TradeView, OBSERVE_CONTAINER_CAP, OBSERVE_CONTAINER_ITEM_CAP, OBSERVE_DOOR_RADIUS,
    OBSERVE_FACT_CAP, OBSERVE_ITEM_CAP, OBSERVE_MOBILE_CAP,
};
pub use radar::{
    default_tile, legend, render_radar, RadarOptions, TileKind, RADAR_DEFAULT, RADAR_SIZE,
};
pub use state::{
    facet_free_movement, facet_rules, is_ghost_body, Buff, Container, DoorItem, DoorUpdate, Harm,
    Item, Mobile, MultiItem, MultiUpdate, SelfState, SkillValue, Trade, World,
    BODY_GHOST_ELF_FEMALE, BODY_GHOST_ELF_MALE, BODY_GHOST_FEMALE, BODY_GHOST_MALE,
    FACET_RULES_FELUCCA, FACET_RULES_TRAMMEL, MAP_RULE_FREE_MOVEMENT, SPEECH_KIND_PARTY,
    SPEECH_KIND_PARTY_PRIVATE,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use uoterm_protocol::{
        ContainerItem, EquipItem, GroundItem, Inbound, MobileView, ObjectProperty, OpenGump,
        PartyEvent, Point3, Serial, SpeechLine, TargetCursor, FLAG_FROZEN, FLAG_HIDDEN, FLAG_WAR,
        GRAPHIC_BACKPACK, LAYER_BACKPACK, LAYER_BANK, NOTO_INNOCENT,
    };

    const LEADER: Serial = Serial(0x0000_0042);

    fn me() -> World {
        let mut w = World::new();
        w.self_state.serial = Serial(0x0000_0001);
        w
    }

    #[test]
    fn a_buff_is_kept_until_the_shard_removes_it() {
        const BLESS_ICON: u16 = 1048;
        const BLESS_CLILOC: u32 = 1_075_847;
        let mut w = me();
        let serial = w.self_state.serial;
        w.apply(&Inbound::BuffDebuff {
            serial,
            icon: BLESS_ICON,
            effects: vec![uoterm_protocol::BuffEntry {
                icon: BLESS_ICON,
                duration_secs: 60,
                title_cliloc: BLESS_CLILOC,
                description_cliloc: 0,
                arguments: String::new(),
            }],
        });
        assert_eq!(w.buffs[&BLESS_ICON].title_cliloc, BLESS_CLILOC);
        w.apply(&Inbound::BuffDebuff {
            serial,
            icon: BLESS_ICON,
            effects: Vec::new(),
        });
        assert!(w.buffs.is_empty());
    }

    #[test]
    fn a_buff_on_someone_else_is_not_ours() {
        const OTHER: Serial = Serial(0x0000_0099);
        let mut w = me();
        w.apply(&Inbound::BuffDebuff {
            serial: OTHER,
            icon: 1,
            effects: vec![uoterm_protocol::BuffEntry {
                icon: 1,
                duration_secs: 0,
                title_cliloc: 0,
                description_cliloc: 0,
                arguments: String::new(),
            }],
        });
        assert!(w.buffs.is_empty());
    }

    #[test]
    fn a_party_invite_is_kept_and_joining_clears_it() {
        let mut w = me();
        w.apply(&Inbound::Party(PartyEvent::Invite { leader: LEADER }));
        assert_eq!(w.party_invite, Some(LEADER));
        assert!(w.events.iter().any(|e| e.kind == EventKind::PartyInvite));
        let members = vec![LEADER, w.self_state.serial];
        w.apply(&Inbound::Party(PartyEvent::Members(members.clone())));
        assert_eq!(w.party, members);
        assert_eq!(w.party_invite, None);
        w.apply(&Inbound::Party(PartyEvent::Removed {
            who: LEADER,
            members: Vec::new(),
        }));
        assert!(w.party.is_empty(), "the party is over");
    }

    #[test]
    fn party_chat_goes_in_the_journal_under_the_speaker_name() {
        let mut w = me();
        w.mobiles.insert(
            LEADER,
            Mobile {
                serial: LEADER,
                name: "Rowan".into(),
                title: String::new(),
                body: 0x0190,
                hue: 0,
                location: Point3::new(0, 0, 0),
                direction: 0,
                running: false,
                notoriety: NOTO_INNOCENT,
                flags: 0,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            },
        );
        w.apply(&Inbound::Party(PartyEvent::Message {
            from: LEADER,
            text: "heal me".into(),
            private: false,
        }));
        let line = w.journal.after(0).last().expect("a party line");
        assert_eq!(line.name, "Rowan");
        assert_eq!(line.kind, SPEECH_KIND_PARTY);
    }

    fn draw_me(w: &mut World, flags: u8) {
        let serial = w.self_state.serial;
        w.apply(&Inbound::DrawPlayer {
            serial,
            body: 0x0190,
            hue: 0,
            flags,
            x: 0,
            y: 0,
            direction: 0,
            z: 0,
        });
    }

    fn poison_bar(serial: Serial, enabled: bool) -> Inbound {
        Inbound::HealthBarUpdate {
            serial,
            bars: vec![uoterm_protocol::HealthBarStatus {
                kind: uoterm_protocol::HEALTH_BAR_POISON,
                enabled,
                poison_level: None,
            }],
        }
    }

    /// Measured against the shard source: for a client from 7.0.0.0 up the
    /// flag bit is flying, and poison comes only on the health bar.
    #[test]
    fn a_modern_client_reads_flying_from_the_flag_and_poison_from_the_bar() {
        const FLYING_BIT: u8 = 0x04;
        let mut w = me();
        w.flags_mean_flying = true;
        let serial = w.self_state.serial;
        draw_me(&mut w, FLYING_BIT);
        assert!(w.self_state.flying);
        assert!(!w.self_state.poisoned, "flying is not poison");
        w.apply(&poison_bar(serial, true));
        assert!(w.is_poisoned(serial));
        w.apply(&poison_bar(serial, false));
        assert!(!w.is_poisoned(serial));
    }

    #[test]
    fn steps_are_counted_while_hidden_and_reset_on_showing() {
        const HIDDEN: u8 = 0x80;
        let mut w = me();
        draw_me(&mut w, HIDDEN);
        for sequence in 0..3 {
            w.apply(&Inbound::MoveAck {
                sequence,
                notoriety: NOTO_INNOCENT,
            });
        }
        assert_eq!(w.self_state.stealth_steps, 3);
        draw_me(&mut w, 0);
        assert_eq!(w.self_state.stealth_steps, 0);
    }

    #[test]
    fn an_old_client_reads_poison_from_the_flag() {
        const POISON_BIT: u8 = 0x04;
        let mut w = me();
        draw_me(&mut w, POISON_BIT);
        assert!(w.self_state.poisoned);
        assert!(!w.self_state.flying);
    }

    #[test]
    fn the_yellow_bar_is_read_from_the_flag_on_any_client() {
        const BLESSED: u8 = 0x08;
        let mut w = me();
        draw_me(&mut w, BLESSED);
        assert!(w.has_yellow_bar(w.self_state.serial));
        draw_me(&mut w, 0);
        assert!(!w.has_yellow_bar(w.self_state.serial));
    }

    #[test]
    fn a_party_list_of_one_ends_the_party() {
        let mut w = me();
        let me = w.self_state.serial;
        w.apply(&Inbound::Party(PartyEvent::Members(vec![LEADER, me])));
        w.apply(&Inbound::Party(PartyEvent::Members(vec![me])));
        assert!(w.party.is_empty());
    }

    #[test]
    fn another_mobile_is_poisoned_by_its_bar() {
        let mut w = me();
        w.flags_mean_flying = true;
        w.apply(&poison_bar(LEADER, true));
        assert!(w.is_poisoned(LEADER));
        w.apply(&Inbound::Delete(LEADER));
        assert!(
            !w.is_poisoned(LEADER),
            "a mobile gone takes its bars with it"
        );
    }

    fn item_in(serial: u32, parent: Option<Serial>) -> Item {
        Item {
            serial: Serial(serial),
            graphic: 0x0E21,
            amount: 1,
            hue: 0,
            location: Point3::new(10, 10, 0),
            parent,
            layer: None,
            grid: 0,
            name: String::new(),
        }
    }

    #[test]
    fn an_item_in_a_bag_in_the_pack_is_inside_the_pack() {
        const PACK: Serial = Serial(0x4000_0001);
        const BAG: Serial = Serial(0x4000_0002);
        const BANDAGE: Serial = Serial(0x4000_0003);
        let mut w = me();
        w.self_state.location = Point3::new(5, 5, 0);
        w.items
            .insert(PACK, item_in(PACK.0, Some(w.self_state.serial)));
        w.items.insert(BAG, item_in(BAG.0, Some(PACK)));
        w.items.insert(BANDAGE, item_in(BANDAGE.0, Some(BAG)));
        assert!(w.is_inside(BANDAGE, PACK));
        assert!(!w.is_inside(PACK, BAG));
        assert_eq!(w.items_inside(PACK, false).len(), 1);
        assert_eq!(w.items_inside(PACK, true).len(), 2);
        assert_eq!(w.map_location(BANDAGE), Some(Point3::new(5, 5, 0)));
    }

    #[test]
    fn an_object_keeps_its_property_list_until_it_is_gone() {
        const RING: Serial = Serial(0x4000_0020);
        const FASTER_CASTING: u32 = 1_060_413;
        let mut w = me();
        w.apply(&Inbound::ObjectPropertyList {
            serial: RING,
            hash: 1,
            properties: vec![ObjectProperty {
                cliloc: FASTER_CASTING,
                arguments: "1".into(),
            }],
        });
        assert_eq!(w.properties[&RING][0].cliloc, FASTER_CASTING);
        w.apply(&Inbound::Delete(RING));
        assert!(!w.properties.contains_key(&RING));
    }

    #[test]
    fn a_broken_parent_chain_does_not_hang() {
        const A: Serial = Serial(0x4000_0010);
        const B: Serial = Serial(0x4000_0011);
        let mut w = me();
        w.items.insert(A, item_in(A.0, Some(B)));
        w.items.insert(B, item_in(B.0, Some(A)));
        assert!(!w.is_inside(A, Serial(0x4000_0099)));
        assert_eq!(w.map_location(A), None);
    }

    #[test]
    fn a_prompt_and_a_text_dialog_wait_for_an_answer() {
        let mut w = me();
        let prompt = uoterm_protocol::PromptRequest {
            serial: LEADER,
            id: 3,
            unicode: true,
        };
        w.apply(&Inbound::Prompt(prompt));
        assert_eq!(w.prompt, Some(prompt));
        assert!(w.observe_default().prompt);
    }

    #[test]
    fn observe_names_the_forbidden_features() {
        const AUTO_OPEN_DOORS_BIT: u64 = 1 << 4;
        let mut w = World::new();
        assert!(w.observe_default().forbidden.is_empty());
        w.assist = AssistRules::from_bits(AUTO_OPEN_DOORS_BIT);
        assert_eq!(w.observe_default().forbidden, vec!["auto_open_doors"]);
    }

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

    /// A banker is a person called Kate to the eye; "banker" is only in her
    /// title, so the title must stay, survive a move, and be searchable.
    #[test]
    fn a_banker_is_found_by_her_title() {
        const KATE: Serial = Serial(0x61);
        let mut w = World::new();
        login(&mut w);
        let kate = |x: u16| {
            Inbound::MobileIncoming(MobileView {
                serial: KATE,
                body: 0x191,
                x,
                y: 20,
                z: 1,
                direction: 0,
                hue: 0,
                flags: 0,
                notoriety: 1,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            })
        };
        w.apply(&kate(11));
        w.apply(&Inbound::ObjectPropertyList {
            serial: KATE,
            hash: NAME_REVISION,
            properties: vec![ObjectProperty {
                cliloc: CLILOC_NAME_WITH_AFFIX,
                arguments: "\tKate\t the banker".into(),
            }],
        });
        w.apply(&kate(12));
        let found = w.find_mobiles(Some("Banker"), None, None);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Kate");
        assert_eq!(found[0].title, "the banker");
        assert_eq!(w.find_mobiles(Some("kat"), None, None).len(), 1);
        assert!(w.find_mobiles(Some("healer"), None, None).is_empty());
    }

    /// A gate on the same map sends no deletes. What is left behind is out of
    /// view and must go, with what it holds; the character's own pack stays.
    #[test]
    fn a_gate_jump_forgets_what_is_left_behind() {
        const COW: Serial = Serial(0x71);
        const NEAR_COW: Serial = Serial(0x72);
        const CHEST: Serial = Serial(0x4000_0301);
        const IN_CHEST: Serial = Serial(0x4000_0302);
        const IN_PACK: Serial = Serial(0x4000_0303);
        const FAR_X: u16 = 3000;
        let mut w = World::new();
        login(&mut w);
        let mobile_at = |serial: Serial, x: u16| {
            Inbound::MobileIncoming(MobileView {
                serial,
                body: 0xD8,
                x,
                y: 20,
                z: 1,
                direction: 0,
                hue: 0,
                flags: 0,
                notoriety: 1,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            })
        };
        w.apply(&mobile_at(COW, 12));
        w.apply(&named(COW, "a cow"));
        w.apply(&mobile_at(NEAR_COW, FAR_X + 2));
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: CHEST,
            graphic: 0x0E43,
            amount: 1,
            x: 13,
            y: 20,
            z: 1,
            hue: 0,
            multi: false,
        }));
        w.apply(&Inbound::AddItem(in_container(CHEST, IN_CHEST, GRAPHIC_GOLD, GOLD_AMOUNT)));
        w.apply(&Inbound::AddItem(in_container(BACKPACK, IN_PACK, GRAPHIC_BONE, ONE_OF_IT)));
        w.apply(&Inbound::DrawPlayer {
            serial: Serial(0xAB),
            body: 0x190,
            hue: 0,
            flags: 0,
            x: FAR_X,
            y: 20,
            direction: 4,
            z: 1,
        });
        assert!(!w.mobiles.contains_key(&COW), "the cow is out of view");
        assert!(w.mobiles.contains_key(&NEAR_COW), "the cow here stays");
        assert!(!w.items.contains_key(&CHEST));
        assert!(!w.items.contains_key(&IN_CHEST), "the chest takes its gold");
        assert!(w.items.contains_key(&IN_PACK), "her own pack stays");
        assert_eq!(w.name_of(COW), "a cow", "the name is kept");
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
    fn bank_box_is_the_worn_container_on_the_bank_layer() {
        let mut w = World::new();
        login(&mut w);
        assert_eq!(w.bank_box(), None, "no bank box before the server sends it");
        const BANK: Serial = Serial(0x4000_0B0B);
        w.apply(&Inbound::Equipped(EquipItem {
            serial: BANK,
            graphic: 0x2436,
            layer: LAYER_BANK,
            hue: 0,
        }));
        assert_eq!(
            w.bank_box(),
            Some(BANK),
            "the item on the bank layer is the bank box"
        );
    }

    /// Lifting an item takes it off the map, and the shard says so with a
    /// delete. That delete is the lift working, not the item being lost: the
    /// item is on the character's cursor until he drops it. Forgetting it
    /// there left 1063 gold stuck on the cursor on a live shard, because the
    /// drop that should have followed was never sent.
    #[test]
    fn a_lifted_item_stays_held_when_the_shard_takes_it_off_the_map() {
        let mut w = World::new();
        let item = in_container(CORPSE, GOLD, GRAPHIC_GOLD, ONE_OF_IT);
        w.apply(&Inbound::AddItem(item));
        w.holding = Some(GOLD);
        w.apply(&Inbound::Delete(GOLD));
        assert_eq!(w.holding, Some(GOLD), "the gold is still on the cursor");
    }

    /// The hold ends when the item lands: the shard puts it in a container.
    #[test]
    fn a_held_item_is_let_go_when_it_lands_in_a_container() {
        let mut w = World::new();
        w.holding = Some(GOLD);
        w.apply(&Inbound::AddItem(in_container(
            CORPSE,
            GOLD,
            GRAPHIC_GOLD,
            ONE_OF_IT,
        )));
        assert_eq!(w.holding, None);
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
        // The pack keeps the tile it was sent at; the character walks on.
        const STEPS_WALKED: u16 = 8;
        w.self_state.location.x += STEPS_WALKED;
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

    /// The server sends `0x2F` only to the attacker. Checking the defender
    /// side never fires, so our own swings were discarded.
    #[test]
    fn a_swing_is_recorded_when_we_are_the_attacker() {
        const ENEMY: Serial = Serial(0x0000_1234);
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::Swing {
            flag: 0,
            attacker: ENEMY,
            defender: w.self_state.serial,
        });
        assert!(w.last_swing.is_none(), "a swing at us is not our swing");
        w.apply(&Inbound::Swing {
            flag: 0,
            attacker: w.self_state.serial,
            defender: ENEMY,
        });
        assert!(
            w.last_swing.is_some(),
            "the server sent this swing to us as the attacker"
        );
    }

    #[test]
    fn combatant_changed_tracks_the_fight_and_its_end() {
        const ENEMY: Serial = Serial(0x0000_1234);
        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::CombatantChanged { serial: ENEMY });
        assert_eq!(w.combatant, Some(ENEMY));
        assert!(w.fighting());
        assert!(w
            .events
            .iter()
            .any(|e| e.kind == EventKind::CombatantChanged && e.serial == Some(ENEMY)));
        w.apply(&Inbound::Swing {
            flag: 0,
            attacker: w.self_state.serial,
            defender: ENEMY,
        });
        assert!(w.last_swing.is_some());
        w.apply(&Inbound::CombatantChanged {
            serial: Serial::INVALID,
        });
        assert_eq!(w.combatant, None);
        assert!(!w.fighting());
        assert!(
            w.last_swing.is_none(),
            "the fight ended, so the last swing is no longer live"
        );
    }

    /// What the character already wore at login reaches the client in the
    /// draw-mobile packet and nowhere else: no equip packet is ever sent for
    /// it. Copying those rows onto the paperdoll alone leaves the items out of
    /// the item map, and then a lookup by serial or by graphic finds nothing.
    ///
    /// Measured on a live shard: her dagger showed on layer one, and
    /// `items.get(&serial)` found nothing.
    #[test]
    fn what_she_already_wears_at_login_is_an_item_as_well_as_a_row() {
        const DAGGER: Serial = Serial(0x4000_0301);
        const GRAPHIC_DAGGER: u16 = 0x0F52;
        const LAYER_ONE_HANDED: u8 = 1;

        let mut w = World::new();
        login(&mut w);
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: w.self_state.serial,
            body: 0x190,
            x: 10,
            y: 20,
            z: 1,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: NOTO_INNOCENT,
            hits: None,
            hits_max: None,
            equipment: vec![EquipItem {
                serial: DAGGER,
                graphic: GRAPHIC_DAGGER,
                layer: LAYER_ONE_HANDED,
                hue: 0,
            }],
        }));
        assert!(
            w.self_state
                .equipment
                .iter()
                .any(|eq| eq.serial == DAGGER && eq.layer == LAYER_ONE_HANDED),
            "the paperdoll row is kept"
        );
        let held = w.items.get(&DAGGER).expect("the dagger is an item too");
        assert_eq!(held.graphic, GRAPHIC_DAGGER);
        assert_eq!(held.layer, Some(LAYER_ONE_HANDED));
        assert_eq!(
            held.parent,
            Some(w.self_state.serial),
            "and it hangs on her, not on the ground"
        );
        assert!(
            w.find_item_graphic(GRAPHIC_DAGGER).is_some(),
            "so a search by graphic finds it"
        );
    }

    /// The swing packet is the only one that names an attacker. Every packet
    /// that takes health away names the one who loses it and nobody else, so
    /// without the swing the character never learns who is hitting her.
    #[test]
    fn a_swing_at_her_names_her_attacker_and_the_memory_ends() {
        const A_BEAST: Serial = Serial(0x0000_2222);
        const WITHIN: Duration = Duration::from_secs(30);
        const LONG_AFTER: Duration = Duration::from_secs(60);

        let mut w = World::new();
        login(&mut w);
        assert!(w.recent_attacker(Instant::now(), WITHIN).is_none());
        w.apply(&Inbound::Swing {
            flag: 0,
            attacker: A_BEAST,
            defender: w.self_state.serial,
        });
        let now = Instant::now();
        assert_eq!(
            w.recent_attacker(now, WITHIN),
            Some(A_BEAST),
            "the swing at her names the one swinging"
        );
        assert!(
            w.last_swing.is_none(),
            "and it is not one of her own swings"
        );
        assert_eq!(
            w.recent_attacker(now + LONG_AFTER, WITHIN),
            None,
            "the memory ends by itself, or she answers a thing that has walked away"
        );
    }

    #[test]
    fn lift_rejected_clears_holding() {
        const GOLD: Serial = Serial(0x4000_0201);
        let mut w = World::new();
        w.holding = Some(GOLD);
        w.apply(&Inbound::LiftRejected {
            reason: uoterm_protocol::LIFT_REJECT_RANGE,
        });
        assert!(w.holding.is_none());
        assert!(w.events.iter().any(|e| e.kind == EventKind::LiftRejected));
    }

    #[test]
    fn an_item_arriving_in_a_container_clears_holding() {
        const GOLD: Serial = Serial(0x4000_0201);
        const PACK: Serial = Serial(0x4000_0100);
        let mut w = World::new();
        w.holding = Some(GOLD);
        w.apply(&Inbound::AddItem(in_container(
            PACK,
            GOLD,
            GRAPHIC_GOLD,
            GOLD_AMOUNT,
        )));
        assert!(w.holding.is_none());
        assert_eq!(w.items[&GOLD].parent, Some(PACK));
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

    const TALKER: Serial = Serial(0x0000_0E01);
    const ME_TALKING: Serial = Serial(0x0000_0E02);

    /// A world where the character is Mara and Ann stands in sight.
    fn mara_and_ann(answer: bool) -> World {
        let mut w = World::new();
        w.answer_when_named = answer;
        w.self_state.serial = ME_TALKING;
        w.self_state.name = "Mara".into();
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: TALKER,
            body: 0x191,
            x: 1,
            y: 1,
            z: 0,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: NOTO_INNOCENT,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }));
        w
    }

    fn say(w: &mut World, serial: Serial, kind: u8, text: &str) {
        w.apply(&Inbound::Speech(SpeechLine {
            serial,
            graphic: 0x191,
            kind,
            hue: 0,
            name: "Ann".into(),
            text: text.into(),
        }));
    }

    fn spoken_to_events(w: &World) -> usize {
        w.events
            .iter()
            .filter(|e| e.kind == EventKind::SpokenTo)
            .count()
    }

    #[test]
    fn a_line_with_the_characters_name_tells_the_agent() {
        let mut w = mara_and_ann(true);
        say(&mut w, TALKER, 0, "hey Mara, are you a bot?");
        assert_eq!(spoken_to_events(&w), 1);
        let obs = w.observe_default();
        assert_eq!(obs.spoken_to.len(), 1);
        assert_eq!(obs.spoken_to[0].name, "Ann");
        assert!(obs.spoken_to[0].asks_if_bot);
    }

    #[test]
    fn other_lines_do_not_count_as_spoken_to() {
        const NOBODY: Serial = Serial(0x0000_0E03);
        let mut w = mara_and_ann(true);
        say(&mut w, TALKER, 0, "Tamara went north");
        say(&mut w, ME_TALKING, 0, "Mara here");
        say(
            &mut w,
            TALKER,
            uoterm_protocol::SPEECH_SYSTEM,
            "Mara has logged in",
        );
        say(&mut w, NOBODY, 0, "Mara, your shop is open");
        assert_eq!(spoken_to_events(&w), 0);
        assert!(w.observe_default().spoken_to.is_empty());
    }

    #[test]
    fn party_and_guild_lines_count_from_out_of_sight() {
        const FAR: Serial = Serial(0x0000_0E04);
        let mut w = mara_and_ann(true);
        w.apply(&Inbound::Party(PartyEvent::Message {
            from: FAR,
            text: "mara heal me".into(),
            private: false,
        }));
        w.apply(&Inbound::Party(PartyEvent::Message {
            from: FAR,
            text: "mara, just you".into(),
            private: true,
        }));
        say(
            &mut w,
            FAR,
            uoterm_protocol::SPEECH_GUILD,
            "mara, guild meeting",
        );
        say(
            &mut w,
            FAR,
            uoterm_protocol::SPEECH_ALLIANCE,
            "mara, allies up",
        );
        say(&mut w, FAR, 0, "mara, can you hear me?");
        let channels: Vec<Channel> = w
            .observe_default()
            .spoken_to
            .iter()
            .map(|l| l.channel)
            .collect();
        assert_eq!(
            channels,
            vec![
                Channel::Party,
                Channel::PartyPrivate,
                Channel::Guild,
                Channel::Alliance
            ],
            "a spoken line from out of sight does not count"
        );
    }

    /// A party member who walks out of sight keeps the name the character
    /// learned, so a party line from far away still names the speaker.
    #[test]
    fn a_name_learned_stays_known_out_of_sight() {
        let mut w = mara_and_ann(true);
        say(&mut w, TALKER, 0, "hello all");
        w.apply(&Inbound::Delete(TALKER));
        assert_eq!(w.name_of(TALKER), "Ann");
        w.apply(&Inbound::Party(PartyEvent::Message {
            from: TALKER,
            text: "mara, where are you?".into(),
            private: false,
        }));
        assert_eq!(w.observe_default().spoken_to[0].name, "Ann");
    }

    /// A combat target that is gone from the world is no fight: a slain
    /// cow leaves a corpse and its mobile is deleted, and a character that
    /// still counts it as a foe never walks again.
    #[test]
    fn a_deleted_combatant_ends_the_fight() {
        let mut w = mara_and_ann(true);
        w.apply(&Inbound::CombatantChanged { serial: TALKER });
        assert!(w.fighting());
        w.apply(&Inbound::Delete(TALKER));
        assert!(!w.fighting(), "the foe is gone");
        assert_eq!(w.combatant, None);
    }

    /// Some doors open without leaving their tile: only the graphic moves
    /// to the open one of its pair, one higher. That open door must not
    /// block a route, and the same door shut again must.
    #[test]
    fn a_door_that_opens_in_place_stops_blocking() {
        const DOOR: Serial = Serial(0x4002_0AF4);
        const SHUT: u16 = 1665;
        const OPEN: u16 = 1666;
        let at = Point3::new(1429, 1684, 0);
        let mut w = World::new();
        w.note_door(DOOR, SHUT, at);
        assert_eq!(w.door_tiles(), vec![at], "a shut door blocks");
        w.note_door(DOOR, OPEN, at);
        assert!(
            w.door_tiles().is_empty(),
            "open in place, it lets him through"
        );
        w.note_door(DOOR, SHUT, at);
        assert_eq!(w.door_tiles(), vec![at], "shut again, it blocks again");
    }

    /// A moongate to another map: the shard sends no delete for what was
    /// in sight on the old one, so the world drops all she does not carry.
    /// Her pack stays, and so do the names she learned.
    #[test]
    fn a_map_change_drops_what_she_left_behind() {
        const OTHER_MAP: u8 = 1;
        let mut w = World::new();
        corpse_and_pack(&mut w);
        w.apply(&Inbound::MobileIncoming(MobileView {
            serial: TALKER,
            body: 0x191,
            x: 11,
            y: 20,
            z: 0,
            direction: 0,
            hue: 0,
            flags: 0,
            notoriety: NOTO_INNOCENT,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }));
        w.mobiles.get_mut(&TALKER).expect("a mobile").name = "Ann".into();
        let map = w.self_state.map;
        w.apply(&Inbound::MapChange { map });
        assert!(
            w.items.contains_key(&CORPSE),
            "the same map keeps the world"
        );

        w.apply(&Inbound::MapChange { map: OTHER_MAP });
        assert!(w.mobiles.is_empty());
        assert!(!w.items.contains_key(&CORPSE), "the corpse stayed behind");
        assert!(w.items.contains_key(&BACKPACK), "her pack goes with her");
        assert!(
            w.items.values().any(|i| i.parent == Some(BACKPACK)),
            "and what is in it"
        );
        assert_eq!(w.name_of(TALKER), "Ann");
        assert!(w.events.iter().any(|e| e.kind == EventKind::MapChanged));
    }

    #[test]
    fn the_switch_off_tells_the_agent_nothing() {
        let mut w = mara_and_ann(false);
        say(&mut w, TALKER, 0, "hey Mara");
        assert_eq!(spoken_to_events(&w), 0);
        assert!(w.observe_default().spoken_to.is_empty());
    }
}
