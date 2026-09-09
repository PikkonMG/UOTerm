use crate::buf::PacketReader;
use crate::error::{ProtocolError, Result};
use crate::types::*;
use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};
use std::io::Read;

const SKILL_TYPE_LIST: u8 = 0x00;
const SKILL_TYPE_LIST_CAP: u8 = 0x02;
const SKILL_TYPE_UPDATE_CAP: u8 = 0xDF;
const SKILL_CAP_DEFAULT: u16 = 1000;
const STATUS_FLAG_ML: u8 = 5;
const COMPRESSED_LEN_HEADER: usize = 4;
/// The id and the big-endian total length that open a self-describing packet.
const VARIABLE_HEADER_LEN: usize = 3;
/// Width of a word the reference client steps over in an old-mode context menu
/// entry without reading it.
const CONTEXT_MENU_SPARE_WORD_LEN: usize = 2;
/// A server writes a constant flags byte at the head of a `0x54`. The reference
/// client skips it and no source says what it selects.
const SOUND_EFFECT_FLAGS_LEN: usize = 1;
/// Head of one `0xDF` effect: a word the reference client calls the source type
/// and two bytes it never reads. Both server sources write zeroes here.
const BUFF_ENTRY_SOURCE_LEN: usize = 4;
/// A word the reference client calls the queue index and four bytes it never
/// reads.
const BUFF_ENTRY_QUEUE_LEN: usize = 6;
/// Padding between the duration and the clilocs of one `0xDF` effect.
const BUFF_ENTRY_TIMER_PAD: usize = 3;
/// A third cliloc the reference client appends to the tooltip and a marker
/// word. Both server sources write zero and one, and neither says what they
/// mean.
const BUFF_ARGUMENT_HEADER_LEN: usize = 6;
/// Code units of the fixed field that opens a `0xDF` argument block.
const BUFF_ARGUMENT_PREFIX_UNITS: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerEntry {
    pub index: u16,
    pub name: String,
    pub percent_full: u8,
    pub timezone: u8,
    pub ip: [u8; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterSlot {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeechLine {
    pub serial: Serial,
    pub graphic: u16,
    pub kind: u8,
    pub hue: u16,
    pub name: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MobileView {
    pub serial: Serial,
    pub body: u16,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub direction: u8,
    pub hue: u16,
    pub flags: u8,
    pub notoriety: u8,
    pub hits: Option<u16>,
    pub hits_max: Option<u16>,
    pub equipment: Vec<EquipItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquipItem {
    pub serial: Serial,
    pub graphic: u16,
    pub layer: u8,
    pub hue: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroundItem {
    pub serial: Serial,
    pub graphic: u16,
    pub amount: u16,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub hue: u16,
    /// The item is a building or a boat, not an ordinary object.
    ///
    /// The two world item packets say so in different places: the older one
    /// sets a bit on the graphic, and the newer one carries a type byte. Only
    /// the parser sees both, so it decides here. A reader that had to answer
    /// this from the packet bytes could not answer it at all for an item that
    /// arrived inside a bundle, because a bundle keeps no per-entry bytes.
    pub multi: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerItem {
    pub serial: Serial,
    pub graphic: u16,
    pub amount: u16,
    pub x: u16,
    pub y: u16,
    pub grid: u8,
    pub container: Serial,
    pub hue: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetCursor {
    pub kind: u8,
    pub id: u32,
    pub flags: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenGump {
    pub serial: Serial,
    pub gump_id: u32,
    pub x: i32,
    pub y: i32,
    pub layout: String,
    pub text: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillEntry {
    pub id: u16,
    pub value: u16,
    pub base: u16,
    pub lock: u8,
    pub cap: u16,
}

/// One line of a `0xD6` object property list. The first line of a list is the
/// name of the object, so `arguments` holds the tab-separated name fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectProperty {
    pub cliloc: u32,
    pub arguments: String,
}

/// One colour of a `0x17` health bar status update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthBarStatus {
    /// `HEALTH_BAR_POISON`, `HEALTH_BAR_YELLOW`, or a colour no source names.
    pub kind: u16,
    /// The colour is on. The reference client treats any non-zero status byte
    /// as on.
    pub enabled: bool,
    /// Poison level, for clients from 7.0.0.0 up, where the status byte holds
    /// `level + 1`. `None` for every other colour, for an off bar, and for
    /// older clients, whose status byte is only a flag.
    pub poison_level: Option<u8>,
}

impl HealthBarStatus {
    fn new(kind: u16, status: u8, version: ClientVersion) -> Self {
        let enabled = status != HEALTH_BAR_OFF;
        let poison_level = if kind == HEALTH_BAR_POISON && enabled && version.has_sa_poison_level()
        {
            Some(status - HEALTH_BAR_POISON_LEVEL_BIAS)
        } else {
            None
        };
        Self {
            kind,
            enabled,
            poison_level,
        }
    }
}

/// One effect from a `0xDF` buff and debuff bar update. `arguments` holds the
/// tab-separated cliloc arguments that fill in the title and the description.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuffEntry {
    pub icon: u16,
    pub duration_secs: u16,
    pub title_cliloc: u32,
    pub description_cliloc: u32,
    pub arguments: String,
}

/// One line of a `0x74` buy list. The list has no serials of its own: the
/// items themselves arrive in the `0x3C` contents of the shop container, where
/// a server writes them backwards and gives the item at forward place `n` the
/// x of `n`. So the entry at index `i` here belongs to the `0x3C` item whose
/// `x` is `i + 1`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VendorBuyEntry {
    pub price: u32,
    pub description: String,
}

/// One line of a `0x9E` sell list. `price` is what the shopkeeper pays.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VendorSellEntry {
    pub serial: Serial,
    pub graphic: u16,
    pub hue: u16,
    pub amount: u16,
    pub price: u16,
    pub name: String,
}

/// One entry of a `0x7C` old-style menu. A question menu leaves `graphic` and
/// `hue` at zero; an item list menu fills them in.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MenuEntry {
    pub graphic: u16,
    pub hue: u16,
    pub name: String,
}

/// One entry of a `0xBF` `0x14` context menu. `index` is what the answer
/// carries back, and `cliloc` names the words of the entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextMenuEntry {
    pub index: u16,
    pub cliloc: u32,
    pub flags: u16,
    /// The colour of the entry, present only when [`CME_FLAG_COLORED`] is set.
    pub colour: Option<u16>,
}

impl ContextMenuEntry {
    pub fn enabled(&self) -> bool {
        self.flags & CME_FLAG_DISABLED == 0
    }
}

/// One `0x6F` secure trade update. What the two values hold depends on `kind`,
/// which the reference client and the server agree on:
///
/// - [`TRADE_DISPLAY`]: `serial` is the other player, `first` and `second` are
///   the two trade containers, and `name` is the other player's name.
/// - [`TRADE_CLOSE`]: `serial` is a trade container. Both values are zero.
/// - [`TRADE_UPDATE`]: `first` is this player's accept flag, `second` the
///   other player's.
/// - [`TRADE_UPDATE_GOLD`]: the other player's gold and platinum.
/// - [`TRADE_UPDATE_LEDGER`]: this player's gold and platinum.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecureTrade {
    pub kind: u8,
    pub serial: Serial,
    pub first: u32,
    pub second: u32,
    pub name: String,
}

/// One page of a `0x66` book. `number` is the page as the server numbers it,
/// which starts at one.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookPage {
    pub number: u16,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Inbound {
    ServerList {
        flags: u8,
        servers: Vec<ServerEntry>,
    },
    LoginDenied {
        reason: u8,
    },
    PopupMessage {
        reason: u8,
    },
    Relay {
        ip: [u8; 4],
        port: u16,
        auth_id: u32,
    },
    Features {
        flags: u32,
    },
    CharacterList {
        characters: Vec<CharacterSlot>,
    },
    LoginConfirm {
        serial: Serial,
        body: u16,
        x: u16,
        y: u16,
        z: i8,
        direction: u8,
        map_width: u16,
        map_height: u16,
    },
    LoginComplete,
    DrawPlayer {
        serial: Serial,
        body: u16,
        hue: u16,
        flags: u8,
        x: u16,
        y: u16,
        direction: u8,
        z: i8,
    },
    MoveReject {
        sequence: u8,
        x: u16,
        y: u16,
        direction: u8,
        z: i8,
    },
    MoveAck {
        sequence: u8,
        notoriety: u8,
    },
    Speech(SpeechLine),
    Delete(Serial),
    MobileMoving(MobileView),
    MobileIncoming(MobileView),
    WorldItem(GroundItem),
    AddItem(ContainerItem),
    OpenContainer {
        serial: Serial,
        gump: u16,
    },
    ContainerContents {
        items: Vec<ContainerItem>,
    },
    UpdateHealth {
        serial: Serial,
        current: u16,
        max: u16,
    },
    UpdateMana {
        serial: Serial,
        current: u16,
        max: u16,
    },
    UpdateStam {
        serial: Serial,
        current: u16,
        max: u16,
    },
    Status {
        serial: Serial,
        name: String,
        hits: u16,
        hits_max: u16,
        female: bool,
        str_: u16,
        dex: u16,
        int_: u16,
        stam: u16,
        stam_max: u16,
        mana: u16,
        mana_max: u16,
        gold: u32,
        weight: u16,
        weight_max: Option<u16>,
    },
    Skills {
        skills: Vec<SkillEntry>,
    },
    WarMode(bool),
    Ping(u8),
    Target(TargetCursor),
    Gump(OpenGump),
    Death {
        serial: Serial,
        corpse: Serial,
    },
    Season {
        season: u8,
        play_sound: bool,
    },
    VersionRequest,
    Equipped(EquipItem),
    Swing {
        flag: u8,
        attacker: Serial,
        defender: Serial,
    },
    /// `0xAA`. A live serial is the mobile the server has us fighting. All
    /// zeroes means the fight ended.
    CombatantChanged {
        serial: Serial,
    },
    /// `0x27`. The server refused a lift. `reason` is one of the
    /// `LIFT_REJECT_*` values.
    LiftRejected {
        reason: u8,
    },
    Paperdoll {
        serial: Serial,
        text: String,
        flags: u8,
    },
    /// `0xBF` `0x01`. The whole fastwalk key stack, which replaces every key
    /// the session holds.
    FastwalkKeys([u32; FASTWALK_KEY_COUNT]),
    /// `0xBF` `0x02`. One more fastwalk key, which the reference client puts
    /// in the first free slot of the stack and leaves the other slots alone.
    /// A consumer must append this key, not replace the stack with it, or the
    /// session spends one key on every step and then walks with none.
    FastwalkKeyAdd(u32),
    Damage {
        serial: Serial,
        amount: u16,
    },
    Trade(SecureTrade),
    MapChange {
        map: u8,
    },
    /// `0x74`. The prices of a buy window. `container` is the shop container
    /// whose `0x3C` contents carry the items themselves.
    VendorBuyList {
        container: Serial,
        entries: Vec<VendorBuyEntry>,
    },
    /// `0x9E`. Everything the player may sell to `vendor`.
    VendorSellList {
        vendor: Serial,
        entries: Vec<VendorSellEntry>,
    },
    /// `0x3B` from the server, which shuts a buy or sell window.
    VendorClose {
        vendor: Serial,
    },
    /// `0x7C`. An old-style menu, which is not a gump. Answer it with
    /// [`crate::encode::menu_response`].
    OpenMenu {
        serial: Serial,
        menu_id: u16,
        question: String,
        entries: Vec<MenuEntry>,
    },
    /// `0xBF` `0x14`. The context menu of one object, opened by a single right
    /// click. Answer it with [`crate::encode::context_menu_response`].
    ContextMenu {
        serial: Serial,
        entries: Vec<ContextMenuEntry>,
    },
    /// `0x66`. The pages of an open book.
    BookContent {
        serial: Serial,
        pages: Vec<BookPage>,
    },
    /// `0x93` or `0xD4`. The cover of a book the player just opened.
    BookHeader {
        serial: Serial,
        writable: bool,
        page_count: u16,
        title: String,
        author: String,
    },
    ObjectPropertyList {
        serial: Serial,
        hash: u32,
        properties: Vec<ObjectProperty>,
    },
    OplInfo {
        serial: Serial,
        hash: u32,
    },
    ResurrectPrompt {
        option: u8,
    },
    /// `0x17`. An empty `bars` list is a server that sent no colour at all.
    HealthBarUpdate {
        serial: Serial,
        bars: Vec<HealthBarStatus>,
    },
    /// `0x6E`. `action` is the animation group the mobile plays, which tells a
    /// caller a swing from a cast from a bow from a death.
    CharacterAnimation {
        serial: Serial,
        action: u16,
        frame_count: u16,
        repeat_count: u16,
        forward: bool,
        repeat: bool,
        delay: u8,
    },
    /// `0x54`.
    SoundEffect {
        sound: u16,
        volume: u16,
        x: u16,
        y: u16,
        z: i16,
    },
    /// `0x6D`. `stop` marks the index that silences the music.
    Music {
        index: u16,
        stop: bool,
    },
    /// `0xDF`. An empty `effects` list takes `icon` off the bar.
    BuffDebuff {
        serial: Serial,
        icon: u16,
        effects: Vec<BuffEntry>,
    },
    Extended {
        sub: u16,
        payload: Vec<u8>,
    },
    /// `0xF7`. Several packets that arrived bundled in one. Every entry is the
    /// decoded form that embedded packet carries when it arrives on its own,
    /// so a consumer handles the list by handling each entry in order.
    PacketList(Vec<Inbound>),
    Unknown {
        id: u8,
        payload: Vec<u8>,
    },
}

pub fn parse(packet: &[u8]) -> Result<Inbound> {
    parse_with_version(packet, ClientVersion::MODERN)
}

pub fn parse_with_version(packet: &[u8], version: ClientVersion) -> Result<Inbound> {
    if packet.is_empty() {
        return Err(ProtocolError::Truncated { needed: 1, had: 0 });
    }
    let id = packet[0];
    match id {
        PKT_SERVER_LIST => parse_server_list(packet),
        PKT_LOGIN_DENIED => parse_login_denied(packet),
        PKT_POPUP_MESSAGE => parse_popup_message(packet),
        PKT_RELAY => parse_relay(packet),
        PKT_FEATURES => parse_features(packet),
        PKT_CHARACTER_LIST => parse_character_list(packet),
        PKT_LOGIN_CONFIRM => parse_login_confirm(packet),
        PKT_LOGIN_COMPLETE => Ok(Inbound::LoginComplete),
        PKT_DRAW_PLAYER => parse_draw_player(packet),
        PKT_MOVE_REJECT => parse_move_reject(packet),
        PKT_MOVE_ACK => parse_move_ack(packet),
        PKT_ASCII_MESSAGE => parse_ascii_message(packet),
        PKT_UNICODE_MESSAGE => parse_unicode_message(packet),
        PKT_DELETE => parse_delete(packet),
        PKT_MOBILE_MOVING => parse_mobile_moving(packet),
        PKT_MOBILE_INCOMING => parse_mobile_incoming(packet, version),
        PKT_WORLD_ITEM => parse_world_item(packet),
        PKT_WORLD_ITEM_SA => parse_world_item_sa(packet),
        PKT_PACKET_LIST => parse_packet_list(packet, version),
        PKT_ADD_ITEM => parse_add_item(packet),
        PKT_OPEN_CONTAINER => parse_open_container(packet),
        PKT_CONTAINER_CONTENTS => parse_container_contents(packet),
        PKT_UPDATE_HEALTH => parse_stat_bar(packet, StatKind::Health),
        PKT_UPDATE_MANA => parse_stat_bar(packet, StatKind::Mana),
        PKT_UPDATE_STAM => parse_stat_bar(packet, StatKind::Stam),
        PKT_STATUS => parse_status(packet),
        PKT_SKILLS => parse_skills(packet),
        PKT_WAR_MODE => parse_war_mode(packet),
        PKT_PING => parse_ping(packet),
        PKT_TARGET => parse_target(packet),
        PKT_GUMP | PKT_COMPRESSED_GUMP => parse_gump(packet),
        PKT_DEATH => parse_death(packet),
        PKT_SEASON => parse_season(packet),
        PKT_CLIENT_VERSION => Ok(Inbound::VersionRequest),
        PKT_EQUIPPED => parse_equipped(packet),
        PKT_SWING => parse_swing(packet),
        PKT_COMBATANT => parse_combatant(packet),
        PKT_LIFT_REJECT => parse_lift_reject(packet),
        PKT_PAPERDOLL => parse_paperdoll(packet),
        PKT_EXTENDED => parse_extended(packet),
        PKT_BATCH_QUERY_PROPERTIES => parse_object_property_list(packet),
        PKT_OPL_INFO => parse_opl_info(packet),
        PKT_DEATH_MENU => parse_resurrect(packet),
        PKT_DAMAGE => parse_damage(packet),
        PKT_CLILOC | PKT_CLILOC_AFFIX => parse_cliloc(packet),
        PKT_SECURE_TRADE => parse_trade(packet),
        PKT_VENDOR_BUY_LIST => parse_vendor_buy_list(packet),
        PKT_VENDOR_SELL_LIST => parse_vendor_sell_list(packet),
        PKT_VENDOR_BUY => parse_vendor_close(packet),
        PKT_OPEN_MENU => parse_open_menu(packet),
        PKT_BOOK_CONTENT => parse_book_content(packet),
        PKT_BOOK_HEADER => parse_book_header(packet),
        PKT_BOOK_HEADER_OLD => parse_book_header_old(packet),
        PKT_HEALTH_BAR_STATUS => parse_health_bar_update(packet, version),
        PKT_CHARACTER_ANIMATION => parse_character_animation(packet),
        PKT_SOUND_EFFECT => parse_sound_effect(packet),
        PKT_MUSIC => parse_music(packet),
        PKT_BUFF_DEBUFF => parse_buff_debuff(packet),
        _ => Ok(Inbound::Unknown {
            id,
            payload: packet.to_vec(),
        }),
    }
}

fn parse_server_list(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let flags = r.u8()?;
    let count = r.u16()? as usize;
    let mut servers = Vec::with_capacity(count);
    for _ in 0..count {
        let index = r.u16()?;
        let name = r.ascii_fixed(32)?;
        let percent_full = r.u8()?;
        let timezone = r.u8()?;
        let ip_raw = r.u32()?;
        let ip = ip_raw.to_le_bytes();
        servers.push(ServerEntry {
            index,
            name,
            percent_full,
            timezone,
            ip,
        });
    }
    Ok(Inbound::ServerList { flags, servers })
}

fn parse_login_denied(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::LoginDenied { reason: r.u8()? })
}

fn parse_popup_message(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::PopupMessage { reason: r.u8()? })
}

fn parse_relay(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let ip = [r.u8()?, r.u8()?, r.u8()?, r.u8()?];
    let port = r.u16()?;
    let auth_id = r.u32()?;
    Ok(Inbound::Relay { ip, port, auth_id })
}

fn parse_features(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let flags = if packet.len() >= 5 {
        r.u32()?
    } else {
        u32::from(r.u16()?)
    };
    Ok(Inbound::Features { flags })
}

fn parse_character_list(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let count = r.u8()? as usize;
    let mut characters = Vec::new();
    for _ in 0..count {
        let name = r.ascii_fixed(30)?;
        r.ascii_fixed(30)?;
        if !name.is_empty() {
            characters.push(CharacterSlot { name });
        }
    }
    Ok(Inbound::CharacterList { characters })
}

fn parse_login_confirm(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let serial = r.serial()?;
    r.u32()?;
    let body = r.u16()?;
    let x = r.u16()?;
    let y = r.u16()?;
    let z = r.i16()? as i8;
    let direction = r.u8()?;
    r.u8()?;
    r.u32()?;
    r.u32()?;
    let map_width = r.u16().unwrap_or(MAP_DEFAULT_WIDTH);
    let map_height = r.u16().unwrap_or(MAP_DEFAULT_HEIGHT);
    Ok(Inbound::LoginConfirm {
        serial,
        body,
        x,
        y,
        z,
        direction,
        map_width,
        map_height,
    })
}

fn parse_draw_player(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let serial = r.serial()?;
    let body = r.u16()?;
    r.u8()?;
    let hue = r.u16()?;
    let flags = r.u8()?;
    let x = r.u16()?;
    let y = r.u16()?;
    r.u16()?;
    let direction = r.u8()?;
    let z = r.i8()?;
    Ok(Inbound::DrawPlayer {
        serial,
        body,
        hue,
        flags,
        x,
        y,
        direction,
        z,
    })
}

fn parse_move_reject(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::MoveReject {
        sequence: r.u8()?,
        x: r.u16()?,
        y: r.u16()?,
        direction: r.u8()?,
        z: r.i8()?,
    })
}

fn parse_move_ack(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::MoveAck {
        sequence: r.u8()?,
        notoriety: r.u8()?,
    })
}

fn parse_ascii_message(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let serial = r.serial()?;
    let graphic = r.u16()?;
    let kind = r.u8()?;
    let hue = r.u16()?;
    r.u16()?;
    let name = r.ascii_fixed(30)?;
    let text = r.ascii_z()?;
    Ok(Inbound::Speech(SpeechLine {
        serial,
        graphic,
        kind,
        hue,
        name,
        text,
    }))
}

fn parse_unicode_message(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let serial = r.serial()?;
    let graphic = r.u16()?;
    let kind = r.u8()?;
    let hue = r.u16()?;
    r.u16()?;
    r.skip(4)?;
    let name = r.ascii_fixed(30)?;
    let text = r.utf16be_z().unwrap_or_default();
    Ok(Inbound::Speech(SpeechLine {
        serial,
        graphic,
        kind,
        hue,
        name,
        text,
    }))
}

fn parse_delete(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::Delete(r.serial()?))
}

fn parse_mobile_moving(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::MobileMoving(MobileView {
        serial: r.serial()?,
        body: r.u16()?,
        x: r.u16()?,
        y: r.u16()?,
        z: r.i8()?,
        direction: r.u8()?,
        hue: r.u16()?,
        flags: r.u8()?,
        notoriety: r.u8()?,
        hits: None,
        hits_max: None,
        equipment: Vec::new(),
    }))
}

fn parse_mobile_incoming(packet: &[u8], version: ClientVersion) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    if version.has_prefixed_mobile_incoming() {
        r.u16()?;
    }
    let serial = r.serial()?;
    let body = r.u16()?;
    let x = r.u16()?;
    let y = r.u16()?;
    let z = r.i8()?;
    let direction = r.u8()?;
    let hue = r.u16()?;
    let flags = r.u8()?;
    let notoriety = r.u8()?;
    let always_hue = version.has_incoming_equip_hue();
    let mut equipment = Vec::new();
    loop {
        if r.remaining() < 4 {
            break;
        }
        let item_serial = r.serial()?;
        if item_serial.0 == 0 {
            break;
        }
        let graphic = r.u16()?;
        let layer = r.u8()?;
        let item_hue = if always_hue || graphic & 0x8000 != 0 {
            r.u16().unwrap_or(0)
        } else {
            0
        };
        equipment.push(EquipItem {
            serial: item_serial,
            graphic: graphic & 0x7FFF,
            layer,
            hue: item_hue,
        });
    }
    Ok(Inbound::MobileIncoming(MobileView {
        serial,
        body,
        x,
        y,
        z,
        direction,
        hue,
        flags,
        notoriety,
        hits: None,
        hits_max: None,
        equipment,
    }))
}

fn parse_world_item(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let mut serial = r.u32()?;
    let mut graphic = r.u16()?;
    let mut amount = 1u16;
    if serial & 0x8000_0000 != 0 {
        serial &= 0x7FFF_FFFF;
        amount = r.u16().unwrap_or(1);
    }
    graphic &= 0x7FFF;
    let multi = graphic & ITEM_GRAPHIC_MULTI != 0;
    let mut x = r.u16()?;
    let mut y = r.u16()?;
    if x & 0x8000 != 0 {
        let _dir = r.u8();
        x &= 0x7FFF;
    }
    let z = r.i8()?;
    let mut hue = 0u16;
    if y & 0x8000 != 0 {
        hue = r.u16().unwrap_or(0);
        y &= 0x7FFF;
    }
    if y & 0x4000 != 0 {
        let _flags = r.u8();
        y &= !0x4000;
    }
    Ok(Inbound::WorldItem(GroundItem {
        serial: Serial(serial),
        graphic,
        amount,
        x,
        y: y & 0x3FFF,
        z,
        hue,
        multi,
    }))
}

fn parse_world_item_sa(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let kind = r.u8()?;
    let serial = r.serial()?;
    let graphic = r.u16()?;
    r.u8()?;
    let amount = r.u16()?;
    r.u16()?;
    let x = r.u16()?;
    let y = r.u16()?;
    let z = r.i8()?;
    r.u8()?;
    let hue = r.u16()?;
    r.u8()?;
    Ok(Inbound::WorldItem(GroundItem {
        serial,
        graphic,
        amount,
        x,
        y,
        z,
        hue,
        multi: kind == WORLD_ITEM_SA_TYPE_MULTI,
    }))
}

/// Width of one `0xF3` on the wire. High Seas closed the packet with a word
/// that a client below 7.0.9.0 never gets, so the size follows the version.
fn world_item_sa_len(version: ClientVersion) -> usize {
    if version.has_high_seas() {
        WORLD_ITEM_SA_LEN
    } else {
        WORLD_ITEM_SA_LEN_PRE_HIGH_SEAS
    }
}

/// `0xF7`. A count, then that many whole packets one after the other, each
/// still carrying its own id byte. Only `0xF3` is known to ride inside, and
/// the reference client stops at any other id because nothing then says how
/// wide that entry is. Every entry goes through [`parse_with_version`], so an
/// item bundled here decodes exactly as one that arrives on its own.
fn parse_packet_list(packet: &[u8], version: ClientVersion) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let count = r.u16()?;
    let entry_len = world_item_sa_len(version);
    let mut packets = Vec::new();
    for _ in 0..count {
        let head = r.rest().first().copied();
        if head.is_some_and(|id| id != PKT_WORLD_ITEM_SA) {
            return Ok(Inbound::Unknown {
                id: PKT_PACKET_LIST,
                payload: packet.to_vec(),
            });
        }
        // An empty rest falls through, so a container that promises more
        // entries than it carries reports the truncation it really is.
        packets.push(parse_with_version(r.take(entry_len)?, version)?);
    }
    Ok(Inbound::PacketList(packets))
}

fn parse_add_item(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let serial = r.serial()?;
    let graphic = r.u16()?;
    r.u8()?;
    let amount = r.u16()?;
    let x = r.u16()?;
    let y = r.u16()?;
    let mut grid = 0u8;
    if packet.len() >= 21 {
        grid = r.u8().unwrap_or(0);
    }
    let container = r.serial()?;
    let hue = r.u16()?;
    Ok(Inbound::AddItem(ContainerItem {
        serial,
        graphic,
        amount,
        x,
        y,
        grid,
        container,
        hue,
    }))
}

fn parse_open_container(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::OpenContainer {
        serial: r.serial()?,
        gump: r.u16()?,
    })
}

fn parse_container_contents(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let count = r.u16()? as usize;
    let item_size = if packet.len() >= 5 + count * 20 {
        20
    } else {
        19
    };
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let serial = r.serial()?;
        let graphic = r.u16()?;
        r.u8()?;
        let amount = r.u16()?;
        let x = r.u16()?;
        let y = r.u16()?;
        let grid = if item_size == 20 {
            r.u8().unwrap_or(0)
        } else {
            0
        };
        let container = r.serial()?;
        let hue = r.u16()?;
        items.push(ContainerItem {
            serial,
            graphic,
            amount,
            x,
            y,
            grid,
            container,
            hue,
        });
    }
    Ok(Inbound::ContainerContents { items })
}

enum StatKind {
    Health,
    Mana,
    Stam,
}

fn parse_stat_bar(packet: &[u8], kind: StatKind) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let serial = r.serial()?;
    let max = r.u16()?;
    let current = r.u16()?;
    Ok(match kind {
        StatKind::Health => Inbound::UpdateHealth {
            serial,
            current,
            max,
        },
        StatKind::Mana => Inbound::UpdateMana {
            serial,
            current,
            max,
        },
        StatKind::Stam => Inbound::UpdateStam {
            serial,
            current,
            max,
        },
    })
}

fn parse_status(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let serial = r.serial()?;
    let name = r.ascii_fixed(30)?;
    let hits = r.u16()?;
    let hits_max = r.u16()?;
    r.u8()?;
    let flag = r.u8()?;
    if flag == 0 {
        return Ok(Inbound::Status {
            serial,
            name,
            hits,
            hits_max,
            female: false,
            str_: 0,
            dex: 0,
            int_: 0,
            stam: 0,
            stam_max: 0,
            mana: 0,
            mana_max: 0,
            gold: 0,
            weight: 0,
            weight_max: None,
        });
    }
    let female = r.u8()? != 0;
    let str_ = r.u16()?;
    let dex = r.u16()?;
    let int_ = r.u16()?;
    let stam = r.u16()?;
    let stam_max = r.u16()?;
    let mana = r.u16()?;
    let mana_max = r.u16()?;
    let gold = r.u32()?;
    r.u16()?;
    let weight = r.u16()?;
    let weight_max = if flag >= STATUS_FLAG_ML && r.remaining() >= 2 {
        Some(r.u16()?)
    } else {
        None
    };
    Ok(Inbound::Status {
        serial,
        name,
        hits,
        hits_max,
        female,
        str_,
        dex,
        int_,
        stam,
        stam_max,
        mana,
        mana_max,
        gold,
        weight,
        weight_max,
    })
}

fn parse_skills(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let kind = r.u8()?;
    let mut skills = Vec::new();
    let with_cap = kind == SKILL_TYPE_LIST_CAP || kind == SKILL_TYPE_UPDATE_CAP;
    if kind == SKILL_TYPE_LIST || kind == SKILL_TYPE_LIST_CAP {
        loop {
            if r.remaining() < 2 {
                break;
            }
            let id = r.u16()?;
            if id == 0 {
                break;
            }
            let value = r.u16().unwrap_or(0);
            let base = r.u16().unwrap_or(value);
            let lock = r.u8().unwrap_or(0);
            let cap = if with_cap {
                r.u16().unwrap_or(SKILL_CAP_DEFAULT)
            } else {
                SKILL_CAP_DEFAULT
            };
            skills.push(SkillEntry {
                id,
                value,
                base,
                lock,
                cap,
            });
        }
    } else {
        let id = r.u16()?;
        let value = r.u16()?;
        let base = r.u16()?;
        let lock = r.u8()?;
        let cap = if with_cap {
            r.u16().unwrap_or(SKILL_CAP_DEFAULT)
        } else {
            SKILL_CAP_DEFAULT
        };
        skills.push(SkillEntry {
            id,
            value,
            base,
            lock,
            cap,
        });
    }
    Ok(Inbound::Skills { skills })
}

fn parse_war_mode(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::WarMode(r.u8()? != 0))
}

fn parse_ping(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::Ping(r.u8()?))
}

fn parse_target(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::Target(TargetCursor {
        kind: r.u8()?,
        id: r.u32()?,
        flags: r.u8()?,
    }))
}

fn inflate_zlib(src: &[u8], dest_len: usize) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(src).take(dest_len as u64);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|_| ProtocolError::Inflate)?;
    Ok(out)
}

fn parse_gump_text_block(r: &mut PacketReader<'_>) -> Result<Vec<String>> {
    if r.remaining() < 2 {
        return Ok(Vec::new());
    }
    let count = r.u16()? as usize;
    let mut text = Vec::with_capacity(count);
    for _ in 0..count {
        let units = r.u16()? as usize;
        let mut buf = Vec::with_capacity(units);
        for _ in 0..units {
            buf.push(r.u16()?);
        }
        text.push(String::from_utf16(&buf).unwrap_or_default());
    }
    Ok(text)
}

fn parse_gump(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    let id = r.u8()?;
    r.u16()?;
    if id == PKT_COMPRESSED_GUMP {
        return parse_compressed_gump(&mut r);
    }
    let serial = r.serial()?;
    let gump_id = r.u32()?;
    let x = r.u32()? as i32;
    let y = r.u32()? as i32;
    let layout_len = r.u16()? as usize;
    let layout_bytes = r.take(layout_len).unwrap_or(&[]);
    let layout = String::from_utf8_lossy(layout_bytes).into_owned();
    let text = parse_gump_text_block(&mut r)?;
    Ok(Inbound::Gump(OpenGump {
        serial,
        gump_id,
        x,
        y,
        layout,
        text,
    }))
}

fn parse_compressed_gump(r: &mut PacketReader<'_>) -> Result<Inbound> {
    let serial = r.serial()?;
    let gump_id = r.u32()?;
    let x = r.u32()? as i32;
    let y = r.u32()? as i32;
    let layout_packed = r.u32()? as usize;
    let layout_plain = r.u32()? as usize;
    let layout_src_len = layout_packed.saturating_sub(COMPRESSED_LEN_HEADER);
    let layout_src = r.take(layout_src_len)?;
    let layout_bytes = inflate_zlib(layout_src, layout_plain)?;
    let layout = String::from_utf8_lossy(&layout_bytes).into_owned();
    let _lines = r.u32()?;
    let text_packed = r.u32()? as usize;
    let text_plain = r.u32()? as usize;
    let text_src_len = text_packed.saturating_sub(COMPRESSED_LEN_HEADER);
    let text_src = r.take(text_src_len)?;
    let text_bytes = inflate_zlib(text_src, text_plain)?;
    let mut text_reader = PacketReader::new(&text_bytes);
    let text = parse_gump_text_block(&mut text_reader)?;
    Ok(Inbound::Gump(OpenGump {
        serial,
        gump_id,
        x,
        y,
        layout,
        text,
    }))
}

fn parse_death(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::Death {
        serial: r.serial()?,
        corpse: r.serial()?,
    })
}

fn parse_season(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::Season {
        season: r.u8()?,
        play_sound: r.u8()? != 0,
    })
}

fn parse_equipped(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let serial = r.serial()?;
    let graphic = r.u16()?;
    let _unk = r.u8()?;
    let layer = r.u8()?;
    let _parent = r.serial()?;
    let hue = r.u16().unwrap_or(0);
    Ok(Inbound::Equipped(EquipItem {
        serial,
        graphic,
        layer,
        hue,
    }))
}

fn parse_extended(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let sub = r.u16()?;
    match sub {
        EXT_FASTWALK => {
            let mut keys = [0u32; FASTWALK_KEY_COUNT];
            for key in &mut keys {
                *key = r.u32()?;
            }
            Ok(Inbound::FastwalkKeys(keys))
        }
        EXT_FASTWALK_ADD => Ok(Inbound::FastwalkKeyAdd(r.u32()?)),
        EXT_MAP_CHANGE => Ok(Inbound::MapChange { map: r.u8()? }),
        EXT_CONTEXT_MENU_DISPLAY => parse_context_menu(&mut r),
        _ => Ok(Inbound::Extended {
            sub,
            payload: r.rest().to_vec(),
        }),
    }
}

fn parse_object_property_list(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    if r.u16()? > OPL_LIST_FORMAT {
        return Ok(Inbound::Unknown {
            id: PKT_BATCH_QUERY_PROPERTIES,
            payload: packet.to_vec(),
        });
    }
    let serial = r.serial()?;
    r.u16()?;
    let hash = r.u32()?;
    let mut properties = Vec::new();
    while r.remaining() > 0 {
        let cliloc = r.u32()?;
        if cliloc == OPL_LIST_TERMINATOR {
            break;
        }
        let len_bytes = r.u16()? as usize;
        properties.push(ObjectProperty {
            cliloc,
            arguments: r.utf16le_fixed(len_bytes)?,
        });
    }
    Ok(Inbound::ObjectPropertyList {
        serial,
        hash,
        properties,
    })
}

fn parse_opl_info(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::OplInfo {
        serial: r.serial()?,
        hash: r.u32()?,
    })
}

fn parse_damage(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    if packet.len() >= 7 {
        return Ok(Inbound::Damage {
            serial: r.serial()?,
            amount: r.u16()?,
        });
    }
    Ok(Inbound::Unknown {
        id: PKT_DAMAGE,
        payload: packet.to_vec(),
    })
}

fn parse_cliloc(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    let id = r.u8()?;
    r.u16()?;
    let serial = r.serial()?;
    let graphic = r.u16().unwrap_or(0);
    let kind = r.u8().unwrap_or(SPEECH_SYSTEM);
    let hue = r.u16().unwrap_or(0);
    let _font = r.u16().unwrap_or(DEFAULT_FONT);
    let number = r.u32().unwrap_or(0);
    let (name, args) = if id == PKT_CLILOC_AFFIX {
        let _affix_type = r.u8().unwrap_or(0);
        let name = r.ascii_fixed(30).unwrap_or_default();
        let _affix = r.ascii_z().unwrap_or_default();
        let args = r.utf16be_z().unwrap_or_default();
        (name, args)
    } else {
        let name = r.ascii_fixed(30).unwrap_or_default();
        let args = r.utf16le_z().unwrap_or_default();
        (name, args)
    };
    let text = if args.is_empty() {
        format!("#{number}")
    } else {
        format!("#{number} {args}")
    };
    Ok(Inbound::Speech(SpeechLine {
        serial,
        graphic,
        kind,
        hue,
        name,
        text,
    }))
}

/// Opens a self-describing packet at its body and refuses a buffer shorter
/// than the length the packet gives itself.
///
/// A packet that ends in a counted list or a NUL-terminated string cannot tell
/// a cut from a short list on its own: the reader would run out of bytes and
/// call the list finished. Reading the framed length first turns every cut
/// into an error, which is what the decoders below need.
fn framed_body(packet: &[u8]) -> Result<PacketReader<'_>> {
    if packet.len() < VARIABLE_HEADER_LEN {
        return Err(ProtocolError::Truncated {
            needed: VARIABLE_HEADER_LEN,
            had: packet.len(),
        });
    }
    let framed = usize::from(u16::from_be_bytes([packet[1], packet[2]]));
    if packet.len() < framed {
        return Err(ProtocolError::Truncated {
            needed: framed,
            had: packet.len(),
        });
    }
    let mut r = PacketReader::new(&packet[..framed]);
    r.skip(VARIABLE_HEADER_LEN)?;
    Ok(r)
}

fn parse_trade(packet: &[u8]) -> Result<Inbound> {
    let mut r = framed_body(packet)?;
    let kind = r.u8()?;
    let serial = r.serial()?;
    let (first, second) = if kind == TRADE_CLOSE {
        // The reference client reads nothing past the serial of a close, and the
        // server writes two zeroes there.
        (0, 0)
    } else {
        (r.u32()?, r.u32()?)
    };
    let name = if kind == TRADE_DISPLAY && r.u8()? == TRADE_HAS_NAME {
        // The server writes a NUL-padded field of TRADE_NAME_LEN bytes and the
        // reference client reads to the first NUL, which is the same text.
        r.ascii_z()?
    } else {
        String::new()
    };
    Ok(Inbound::Trade(SecureTrade {
        kind,
        serial,
        first,
        second,
        name,
    }))
}

fn parse_vendor_buy_list(packet: &[u8]) -> Result<Inbound> {
    let mut r = framed_body(packet)?;
    let container = r.serial()?;
    let count = usize::from(r.u8()?);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let price = r.u32()?;
        // The server counts the terminator into the length byte and the
        // reference client reads that many bytes, terminator and all.
        let width = usize::from(r.u8()?);
        entries.push(VendorBuyEntry {
            price,
            description: r.ascii_fixed(width)?,
        });
    }
    Ok(Inbound::VendorBuyList { container, entries })
}

fn parse_vendor_sell_list(packet: &[u8]) -> Result<Inbound> {
    let mut r = framed_body(packet)?;
    let vendor = r.serial()?;
    let count = usize::from(r.u16()?);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let serial = r.serial()?;
        let graphic = r.u16()?;
        let hue = r.u16()?;
        let amount = r.u16()?;
        let price = r.u16()?;
        let width = usize::from(r.u16()?);
        entries.push(VendorSellEntry {
            serial,
            graphic,
            hue,
            amount,
            price,
            name: r.ascii_fixed(width)?,
        });
    }
    Ok(Inbound::VendorSellList { vendor, entries })
}

/// The server's own `0x3B`. The server writes a zero byte after the serial and
/// calls it a buy count; the reference client reads only the serial and no
/// source says what a count other than zero would mean, so the byte is left
/// out.
fn parse_vendor_close(packet: &[u8]) -> Result<Inbound> {
    let mut r = framed_body(packet)?;
    Ok(Inbound::VendorClose {
        vendor: r.serial()?,
    })
}

fn parse_open_menu(packet: &[u8]) -> Result<Inbound> {
    let mut r = framed_body(packet)?;
    let serial = r.serial()?;
    let menu_id = r.u16()?;
    let width = usize::from(r.u8()?);
    let question = r.ascii_fixed(width)?;
    let count = usize::from(r.u8()?);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let graphic = r.u16()?;
        let hue = r.u16()?;
        let width = usize::from(r.u8()?);
        entries.push(MenuEntry {
            graphic,
            hue,
            name: r.ascii_fixed(width)?,
        });
    }
    Ok(Inbound::OpenMenu {
        serial,
        menu_id,
        question,
        entries,
    })
}

/// Reads a `0xBF` `0x14` the way the reference client does. From mode two up, an
/// entry carries a whole cliloc; below it, the entry carries a 16-bit offset
/// from [`CONTEXT_MENU_CLILOC_BASE`] and may be followed by words that the
/// reference client steps over without reading.
fn parse_context_menu(r: &mut PacketReader<'_>) -> Result<Inbound> {
    let mode = r.u16()?;
    let serial = r.serial()?;
    let count = usize::from(r.u8()?);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let (cliloc, index, flags) = if mode >= CONTEXT_MENU_MODE_CLILOC {
            (r.u32()?, r.u16()?, r.u16()?)
        } else {
            let index = r.u16()?;
            let cliloc = CONTEXT_MENU_CLILOC_BASE + u32::from(r.u16()?);
            (cliloc, index, r.u16()?)
        };
        let mut colour = None;
        if mode < CONTEXT_MENU_MODE_CLILOC {
            if flags & CME_FLAGS_WITH_SPARE_WORD != 0 {
                r.skip(CONTEXT_MENU_SPARE_WORD_LEN)?;
            }
            if flags & CME_FLAG_WITH_SPARE_WORD != 0 {
                r.skip(CONTEXT_MENU_SPARE_WORD_LEN)?;
            }
            if flags & CME_FLAG_COLORED != 0 {
                colour = Some(r.u16()?);
            }
        }
        entries.push(ContextMenuEntry {
            index,
            cliloc,
            flags,
            colour,
        });
    }
    Ok(Inbound::ContextMenu { serial, entries })
}

fn parse_book_content(packet: &[u8]) -> Result<Inbound> {
    let mut r = framed_body(packet)?;
    let serial = r.serial()?;
    let count = usize::from(r.u16()?);
    let mut pages = Vec::with_capacity(count);
    for _ in 0..count {
        let number = r.u16()?;
        let line_count = usize::from(r.u16()?);
        let mut lines = Vec::with_capacity(line_count);
        for _ in 0..line_count {
            lines.push(r.ascii_z()?);
        }
        pages.push(BookPage { number, lines });
    }
    Ok(Inbound::BookContent { serial, pages })
}

/// `0xD4`. A server counts the terminator into each length word. The byte in
/// front of the writable flag is always one and no source names it, so it is
/// left out.
fn parse_book_header(packet: &[u8]) -> Result<Inbound> {
    let mut r = framed_body(packet)?;
    let serial = r.serial()?;
    r.u8()?;
    let writable = r.u8()? != 0;
    let page_count = r.u16()?;
    let width = usize::from(r.u16()?);
    let title = r.ascii_fixed(width)?;
    let width = usize::from(r.u16()?);
    let author = r.ascii_fixed(width)?;
    Ok(Inbound::BookHeader {
        serial,
        writable,
        page_count,
        title,
        author,
    })
}

/// `0x93`, the fixed-width cover. The reference client reads the first byte of
/// the pair and throws it away, then the title and author at their set widths.
fn parse_book_header_old(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let serial = r.serial()?;
    let writable = r.u8()? != 0;
    r.u8()?;
    let page_count = r.u16()?;
    let title = r.ascii_fixed(BOOK_TITLE_LEN)?;
    let author = r.ascii_fixed(BOOK_AUTHOR_LEN)?;
    Ok(Inbound::BookHeader {
        serial,
        writable,
        page_count,
        title,
        author,
    })
}

fn parse_resurrect(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::ResurrectPrompt {
        option: r.u8().unwrap_or(0),
    })
}

fn parse_swing(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::Swing {
        flag: r.u8()?,
        attacker: r.serial()?,
        defender: r.serial()?,
    })
}

fn parse_combatant(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::CombatantChanged {
        serial: r.serial()?,
    })
}

fn parse_lift_reject(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::LiftRejected { reason: r.u8()? })
}

fn parse_paperdoll(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    Ok(Inbound::Paperdoll {
        serial: r.serial()?,
        text: r.ascii_fixed(60).unwrap_or_default(),
        flags: r.u8().unwrap_or(0),
    })
}

fn parse_health_bar_update(packet: &[u8], version: ClientVersion) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let serial = r.serial()?;
    let count = r.u16()?;
    let mut bars = Vec::new();
    for _ in 0..count {
        let kind = r.u16()?;
        let status = r.u8()?;
        bars.push(HealthBarStatus::new(kind, status, version));
    }
    Ok(Inbound::HealthBarUpdate { serial, bars })
}

fn parse_character_animation(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let serial = r.serial()?;
    let action = r.u16()?;
    let frame_count = r.u16()?;
    let repeat_count = r.u16()?;
    let reverse = r.u8()? != 0;
    let repeat = r.u8()? != 0;
    let delay = r.u8()?;
    Ok(Inbound::CharacterAnimation {
        serial,
        action,
        frame_count,
        repeat_count,
        forward: !reverse,
        repeat,
        delay,
    })
}

fn parse_sound_effect(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.skip(SOUND_EFFECT_FLAGS_LEN)?;
    let sound = r.u16()?;
    let volume = r.u16()?;
    let x = r.u16()?;
    let y = r.u16()?;
    let z = r.i16()?;
    Ok(Inbound::SoundEffect {
        sound,
        volume,
        x,
        y,
        z,
    })
}

fn parse_music(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    let index = r.u16()?;
    Ok(Inbound::Music {
        index,
        stop: index == MUSIC_INDEX_STOP,
    })
}

fn parse_buff_debuff(packet: &[u8]) -> Result<Inbound> {
    let mut r = PacketReader::new(packet);
    r.u8()?;
    r.u16()?;
    let serial = r.serial()?;
    let icon = r.u16()?;
    let count = r.u16()?;
    let mut effects = Vec::new();
    for _ in 0..count {
        r.skip(BUFF_ENTRY_SOURCE_LEN)?;
        let entry_icon = r.u16()?;
        r.skip(BUFF_ENTRY_QUEUE_LEN)?;
        let duration_secs = r.u16()?;
        r.skip(BUFF_ENTRY_TIMER_PAD)?;
        let title_cliloc = r.u32()?;
        let description_cliloc = r.u32()?;
        effects.push(BuffEntry {
            icon: entry_icon,
            duration_secs,
            title_cliloc,
            description_cliloc,
            arguments: read_buff_arguments(&mut r),
        });
    }
    Ok(Inbound::BuffDebuff {
        serial,
        icon,
        effects,
    })
}

/// Reads the cliloc arguments that close one `0xDF` effect the way the
/// reference client does. An effect with no arguments pads the block with
/// zeroes and stops at the fixed field, so a short tail gives empty text
/// instead of an error.
fn read_buff_arguments(r: &mut PacketReader<'_>) -> String {
    if r.skip(BUFF_ARGUMENT_HEADER_LEN).is_err() {
        return String::new();
    }
    let Ok(prefix) = r.utf16le_fixed_z(BUFF_ARGUMENT_PREFIX_UNITS) else {
        return String::new();
    };
    prefix + &r.utf16le_z().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode;

    #[test]
    fn mobile_incoming_reads_framed_length() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_MOBILE_INCOMING);
        w.u32(0x0000_1234)
            .u16(0x0190)
            .u16(100)
            .u16(200)
            .i8(0)
            .u8(0)
            .u16(0)
            .u8(0)
            .u8(1)
            .u32(0);
        let p = w.finish_variable().unwrap();
        match parse(&p).unwrap() {
            Inbound::MobileIncoming(m) => {
                assert_eq!(m.serial.0, 0x0000_1234);
                assert_eq!(m.body, 0x0190);
                assert_eq!(m.x, 100);
                assert_eq!(m.y, 200);
                assert!(m.equipment.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ping_roundtrip() {
        let p = encode::ping(4);
        match parse(&p).unwrap() {
            Inbound::Ping(4) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn server_list_parses() {
        let mut body = vec![PKT_SERVER_LIST, 0x00, 0x2E, 0x5D, 0x00, 0x01];
        body.extend_from_slice(&0u16.to_be_bytes());
        let mut name = b"Test Shard".to_vec();
        name.resize(32, 0);
        body.extend_from_slice(&name);
        body.push(10);
        body.push(0);
        body.extend_from_slice(&0x7F00_0001u32.to_le_bytes());
        let len = body.len() as u16;
        body[1..3].copy_from_slice(&len.to_be_bytes());
        match parse(&body).unwrap() {
            Inbound::ServerList { servers, .. } => {
                assert_eq!(servers.len(), 1);
                assert_eq!(servers[0].name, "Test Shard");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn popup_message_parses_char_in_world() {
        match parse(&[PKT_POPUP_MESSAGE, POPUP_CHAR_IN_WORLD]).unwrap() {
            Inbound::PopupMessage { reason } => assert_eq!(reason, POPUP_CHAR_IN_WORLD),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn login_confirm_parses_serial_and_xy() {
        let mut w = crate::buf::PacketWriter::new(PKT_LOGIN_CONFIRM);
        w.serial(Serial(0xAA))
            .u32(0)
            .u16(0x0190)
            .u16(1425)
            .u16(1680)
            .i16(-5)
            .u8(Direction::South as u8)
            .u8(0)
            .u32(u32::MAX)
            .u32(0)
            .u16(MAP_DEFAULT_WIDTH)
            .u16(MAP_DEFAULT_HEIGHT)
            .pad(6);
        let p = w.finish();
        assert_eq!(p.len(), LOGIN_CONFIRM_LEN);
        match parse(&p).unwrap() {
            Inbound::LoginConfirm {
                serial,
                x,
                y,
                z,
                direction,
                map_width,
                map_height,
                ..
            } => {
                assert_eq!(serial, Serial(0xAA));
                assert_eq!(x, 1425);
                assert_eq!(y, 1680);
                assert_eq!(z, -5);
                assert_eq!(direction, Direction::South as u8);
                assert_eq!(map_width, MAP_DEFAULT_WIDTH);
                assert_eq!(map_height, MAP_DEFAULT_HEIGHT);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn move_ack_and_reject() {
        let ack = vec![PKT_MOVE_ACK, 3, 1];
        match parse(&ack).unwrap() {
            Inbound::MoveAck {
                sequence,
                notoriety,
            } => {
                assert_eq!(sequence, 3);
                assert_eq!(notoriety, 1);
            }
            other => panic!("{other:?}"),
        }
        let mut rej = vec![PKT_MOVE_REJECT, 3, 0, 0, 0, 0, 4, 0];
        rej[2..4].copy_from_slice(&100u16.to_be_bytes());
        rej[4..6].copy_from_slice(&200u16.to_be_bytes());
        match parse(&rej).unwrap() {
            Inbound::MoveReject {
                sequence,
                x,
                y,
                direction,
                z,
            } => {
                assert_eq!(sequence, 3);
                assert_eq!((x, y, direction, z), (100, 200, 4, 0));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn move_ack_and_reject_accept_sequence_zero() {
        match parse(&[PKT_MOVE_ACK, 0, 1]).unwrap() {
            Inbound::MoveAck {
                sequence,
                notoriety,
            } => {
                assert_eq!(sequence, 0);
                assert_eq!(notoriety, 1);
            }
            other => panic!("{other:?}"),
        }
        let mut rej = vec![PKT_MOVE_REJECT, 0, 0, 0, 0, 0, 4, 0];
        rej[2..4].copy_from_slice(&100u16.to_be_bytes());
        rej[4..6].copy_from_slice(&200u16.to_be_bytes());
        match parse(&rej).unwrap() {
            Inbound::MoveReject {
                sequence,
                x,
                y,
                direction,
                z,
            } => {
                assert_eq!(sequence, 0);
                assert_eq!((x, y, direction, z), (100, 200, 4, 0));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn map_change_from_extended() {
        let pkt = vec![PKT_EXTENDED, 0x00, 0x06, 0x00, 0x08, 0x00];
        match parse(&pkt).unwrap() {
            Inbound::MapChange { map } => assert_eq!(map, 0),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fastwalk_keys() {
        let mut pkt = vec![PKT_EXTENDED, 0x00, 0x1D, 0x00, 0x01];
        for i in 0u32..6 {
            pkt.extend_from_slice(&i.to_be_bytes());
        }
        let len = pkt.len() as u16;
        pkt[1..3].copy_from_slice(&len.to_be_bytes());
        match parse(&pkt).unwrap() {
            Inbound::FastwalkKeys(keys) => assert_eq!(keys[5], 5),
            other => panic!("{other:?}"),
        }
    }

    /// Sub-command 2 adds one key to the stack sub-command 1 filled, so it
    /// must decode to its own form and not to a whole new stack.
    #[test]
    fn fastwalk_key_add_is_a_single_key() {
        const ADDED_KEY: u32 = 0xDEAD_BEEF;
        let mut pkt = vec![PKT_EXTENDED, 0x00, 0x00];
        pkt.extend_from_slice(&EXT_FASTWALK_ADD.to_be_bytes());
        pkt.extend_from_slice(&ADDED_KEY.to_be_bytes());
        let len = pkt.len() as u16;
        pkt[1..3].copy_from_slice(&len.to_be_bytes());
        match parse(&pkt).unwrap() {
            Inbound::FastwalkKeyAdd(key) => assert_eq!(key, ADDED_KEY),
            other => panic!("{other:?}"),
        }
    }

    fn world_item_sa_bytes(serial: u32, x: u16, y: u16, version: ClientVersion) -> Vec<u8> {
        world_item_sa_of_kind(WORLD_ITEM_SA_ITEM, serial, x, y, version)
    }

    fn world_item_sa_of_kind(
        kind: u8,
        serial: u32,
        x: u16,
        y: u16,
        version: ClientVersion,
    ) -> Vec<u8> {
        let mut w = crate::buf::PacketWriter::new(PKT_WORLD_ITEM_SA);
        w.u16(WORLD_ITEM_SA_UNKNOWN)
            .u8(kind)
            .u32(serial)
            .u16(0x0EED)
            .u8(0)
            .u16(1)
            .u16(1)
            .u16(x)
            .u16(y)
            .i8(0)
            .u8(0)
            .u16(0x0441)
            .u8(0);
        if version.has_high_seas() {
            w.u16(0);
        }
        let pkt = w.finish();
        assert_eq!(pkt.len(), world_item_sa_len(version));
        pkt
    }

    fn packet_list_bytes(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_PACKET_LIST);
        w.u16(entries.len() as u16);
        for entry in entries {
            w.bytes(entry);
        }
        w.finish_variable().unwrap()
    }

    /// A bundled item must reach a caller as the same `WorldItem` it would be
    /// had it arrived on its own.
    /// One item on the older world item packet, at a fixed spot.
    fn world_item_bytes(serial: u32, graphic: u16) -> Vec<u8> {
        const AT_X: u16 = 1425;
        const AT_Y: u16 = 1680;
        let mut w = crate::buf::PacketWriter::with_variable(PKT_WORLD_ITEM);
        w.u32(serial).u16(graphic).u16(AT_X).u16(AT_Y).i8(0);
        w.finish_variable().unwrap()
    }

    /// The type byte of a mobile and of a damageable item. One server family
    /// writes 1 for a mobile, the other 3 for a damageable item. Neither is a
    /// building.
    const WORLD_ITEM_SA_TYPE_MOBILE: u8 = 0x01;
    const WORLD_ITEM_SA_TYPE_DAMAGEABLE: u8 = 0x03;

    /// The two world item packets say "this is a building" in different
    /// places, and only the decoder sees both. The older one sets a bit on the
    /// graphic. The newer one carries a type byte and reads no bit off the
    /// graphic at all, so a graphic carrying that bit there is still an
    /// ordinary item.
    #[test]
    fn each_world_item_packet_reads_its_own_building_rule() {
        const HOUSE_ID: u16 = 0x0064;
        const AT_X: u16 = 1425;
        const AT_Y: u16 = 1680;
        let version = ClientVersion::MODERN;

        let legacy_multi = world_item_bytes(0x4000_0001, HOUSE_ID | ITEM_GRAPHIC_MULTI);
        match parse(&legacy_multi).unwrap() {
            Inbound::WorldItem(item) => assert!(item.multi, "the bit on the graphic says building"),
            other => panic!("{other:?}"),
        }

        let legacy_plain = world_item_bytes(0x4000_0002, HOUSE_ID);
        match parse(&legacy_plain).unwrap() {
            Inbound::WorldItem(item) => assert!(!item.multi, "no bit, no building"),
            other => panic!("{other:?}"),
        }

        let sa_multi =
            world_item_sa_of_kind(WORLD_ITEM_SA_TYPE_MULTI, 0x4000_0003, AT_X, AT_Y, version);
        match parse(&sa_multi).unwrap() {
            Inbound::WorldItem(item) => assert!(item.multi, "the type byte says building"),
            other => panic!("{other:?}"),
        }

        for kind in [
            WORLD_ITEM_SA_ITEM,
            WORLD_ITEM_SA_TYPE_MOBILE,
            WORLD_ITEM_SA_TYPE_DAMAGEABLE,
        ] {
            let pkt = world_item_sa_of_kind(kind, 0x4000_0004, AT_X, AT_Y, version);
            match parse(&pkt).unwrap() {
                Inbound::WorldItem(item) => assert!(
                    !item.multi,
                    "only type {WORLD_ITEM_SA_TYPE_MULTI} is a building, never {kind}"
                ),
                other => panic!("{other:?}"),
            }
        }
    }

    /// A building sent inside a bundle must still be known as one. Nothing
    /// keeps the bytes of a bundled entry, so an answer worked out from the
    /// packet bytes could not be given here at all.
    #[test]
    fn a_building_inside_a_bundle_is_still_a_building() {
        let version = ClientVersion::MODERN;
        let entries = vec![
            world_item_sa_of_kind(WORLD_ITEM_SA_TYPE_MULTI, 0x4000_0001, 1425, 1680, version),
            world_item_sa_bytes(0x4000_0002, 1426, 1681, version),
        ];
        match parse(&packet_list_bytes(&entries)).unwrap() {
            Inbound::PacketList(items) => {
                let flags: Vec<bool> = items
                    .iter()
                    .map(|entry| match entry {
                        Inbound::WorldItem(ground) => ground.multi,
                        other => panic!("{other:?}"),
                    })
                    .collect();
                assert_eq!(flags, vec![true, false]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn packet_list_unpacks_every_world_item() {
        let version = ClientVersion::MODERN;
        let entries = vec![
            world_item_sa_bytes(0x4000_0001, 1425, 1680, version),
            world_item_sa_bytes(0x4000_0002, 1426, 1681, version),
        ];
        let pkt = packet_list_bytes(&entries);
        match parse(&pkt).unwrap() {
            Inbound::PacketList(items) => {
                assert_eq!(items.len(), 2);
                let serials: Vec<Serial> = items
                    .iter()
                    .map(|item| match item {
                        Inbound::WorldItem(ground) => ground.serial,
                        other => panic!("{other:?}"),
                    })
                    .collect();
                assert_eq!(serials, vec![Serial(0x4000_0001), Serial(0x4000_0002)]);
                match &items[1] {
                    Inbound::WorldItem(ground) => {
                        assert_eq!(ground.x, 1426);
                        assert_eq!(ground.y, 1681);
                        assert_eq!(ground.hue, 0x0441);
                    }
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }

    /// The entries are two bytes narrower below 7.0.9.0, so the version has to
    /// step the reader or the second entry is read from the wrong offset.
    #[test]
    fn packet_list_entry_width_follows_the_version() {
        const VERSION_PRE_HIGH_SEAS: &str = "7.0.8.2";
        let version: ClientVersion = VERSION_PRE_HIGH_SEAS.parse().unwrap();
        let entries = vec![
            world_item_sa_bytes(0x4000_0003, 100, 200, version),
            world_item_sa_bytes(0x4000_0004, 101, 201, version),
        ];
        let pkt = packet_list_bytes(&entries);
        match parse_with_version(&pkt, version).unwrap() {
            Inbound::PacketList(items) => match (&items[0], &items[1]) {
                (Inbound::WorldItem(first), Inbound::WorldItem(second)) => {
                    assert_eq!(first.serial, Serial(0x4000_0003));
                    assert_eq!(second.serial, Serial(0x4000_0004));
                    assert_eq!(second.x, 101);
                    assert_eq!(second.y, 201);
                }
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    /// Nothing says how wide an entry that is not `0xF3` would be, so the
    /// container comes back whole and undecoded rather than half read.
    #[test]
    fn packet_list_with_an_unexpected_entry_stays_undecoded() {
        let mut entry = world_item_sa_bytes(0x4000_0005, 1, 2, ClientVersion::MODERN);
        entry[0] = PKT_ADD_ITEM;
        let pkt = packet_list_bytes(&[entry]);
        match parse(&pkt).unwrap() {
            Inbound::Unknown { id, payload } => {
                assert_eq!(id, PKT_PACKET_LIST);
                assert_eq!(payload, pkt);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn packet_list_short_of_its_count_is_truncated() {
        let entries = vec![world_item_sa_bytes(
            0x4000_0006,
            3,
            4,
            ClientVersion::MODERN,
        )];
        let mut pkt = packet_list_bytes(&entries);
        // Claim two entries but carry one.
        pkt[3..5].copy_from_slice(&2u16.to_be_bytes());
        match parse(&pkt) {
            Err(ProtocolError::Truncated { .. }) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn worn_item_layer_is_fifth_field() {
        let mut w = crate::buf::PacketWriter::new(PKT_EQUIPPED);
        w.u32(0x4000_0001)
            .u16(0x0F43)
            .u8(0)
            .u8(LAYER_ONE_HANDED)
            .u32(0xAA)
            .u16(0);
        let pkt = w.finish();
        match parse(&pkt).unwrap() {
            Inbound::Equipped(eq) => {
                assert_eq!(eq.serial, Serial(0x4000_0001));
                assert_eq!(eq.graphic, 0x0F43);
                assert_eq!(eq.layer, LAYER_ONE_HANDED);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unknown_packet_does_not_fail() {
        match parse(&[0xEE, 1, 2, 3]).unwrap() {
            Inbound::Unknown { id, payload } => {
                assert_eq!(id, 0xEE);
                assert_eq!(payload.len(), 4);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn combatant_changed_reads_a_live_serial() {
        const FIGHTING: Serial = Serial(0x0000_1234);
        let mut w = crate::buf::PacketWriter::new(PKT_COMBATANT);
        w.serial(FIGHTING);
        let packet = w.finish();
        assert_eq!(packet.len(), 5);
        match parse(&packet).unwrap() {
            Inbound::CombatantChanged { serial } => assert_eq!(serial, FIGHTING),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn combatant_changed_all_zeroes_means_the_fight_ended() {
        let mut w = crate::buf::PacketWriter::new(PKT_COMBATANT);
        w.serial(Serial::INVALID);
        match parse(&w.finish()).unwrap() {
            Inbound::CombatantChanged { serial } => assert_eq!(serial, Serial::INVALID),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn lift_reject_reads_the_reason() {
        match parse(&[PKT_LIFT_REJECT, LIFT_REJECT_RANGE]).unwrap() {
            Inbound::LiftRejected { reason } => assert_eq!(reason, LIFT_REJECT_RANGE),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn swing_reads_attacker_then_defender() {
        const ATTACKER: Serial = Serial(0x0000_00AB);
        const DEFENDER: Serial = Serial(0x0000_1234);
        let mut w = crate::buf::PacketWriter::new(PKT_SWING);
        w.u8(0).serial(ATTACKER).serial(DEFENDER);
        let packet = w.finish();
        assert_eq!(packet.len(), 10);
        match parse(&packet).unwrap() {
            Inbound::Swing {
                attacker, defender, ..
            } => {
                assert_eq!(attacker, ATTACKER);
                assert_eq!(defender, DEFENDER);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn world_item_sa_parses_z_and_hue() {
        let mut w = crate::buf::PacketWriter::new(PKT_WORLD_ITEM_SA);
        w.u16(WORLD_ITEM_SA_UNKNOWN)
            .u8(WORLD_ITEM_SA_ITEM)
            .u32(0x4000_0020)
            .u16(0x0EED)
            .u8(Direction::South as u8)
            .u16(5)
            .u16(5)
            .u16(1425)
            .u16(1680)
            .i8(-7)
            .u8(0)
            .u16(0x0441)
            .u8(0)
            .u16(0);
        let pkt = w.finish();
        assert_eq!(pkt.len(), WORLD_ITEM_SA_LEN);
        match parse(&pkt).unwrap() {
            Inbound::WorldItem(item) => {
                assert_eq!(item.serial, Serial(0x4000_0020));
                assert_eq!(item.graphic, 0x0EED);
                assert_eq!(item.amount, 5);
                assert_eq!(item.x, 1425);
                assert_eq!(item.y, 1680);
                assert_eq!(item.z, -7);
                assert_eq!(item.hue, 0x0441);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn world_item_direction_does_not_steal_x() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_WORLD_ITEM);
        w.u32(0x4000_0010)
            .u16(0x0CCA)
            .u16(0x8000 | 1424)
            .u16(1694)
            .u8(Direction::South as u8)
            .i8(0);
        let pkt = w.finish_variable().unwrap();
        match parse(&pkt).unwrap() {
            Inbound::WorldItem(item) => {
                assert_eq!(item.serial, Serial(0x4000_0010));
                assert_eq!(item.graphic, 0x0CCA);
                assert_eq!(item.x, 1424);
                assert_eq!(item.y, 1694);
                assert_eq!(item.z, 0);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn modern_damage_parses_serial_and_amount() {
        let mut w = crate::buf::PacketWriter::new(PKT_DAMAGE);
        w.u32(0xAA).u16(12);
        match parse(&w.finish()).unwrap() {
            Inbound::Damage { serial, amount } => {
                assert_eq!(serial, Serial(0xAA));
                assert_eq!(amount, 12);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn mobile_incoming_equip_hue_depends_on_version() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_MOBILE_INCOMING);
        w.u32(0xAA)
            .u16(0x0190)
            .u16(10)
            .u16(20)
            .i8(0)
            .u8(0)
            .u16(0)
            .u8(0)
            .u8(1)
            .u32(0x4000_0001)
            .u16(0x0F43)
            .u8(1)
            .u16(0x0441)
            .u32(0);
        let p = w.finish_variable().unwrap();
        match parse_with_version(&p, ClientVersion::MODERN).unwrap() {
            Inbound::MobileIncoming(m) => {
                assert_eq!(m.equipment.len(), 1);
                assert_eq!(m.equipment[0].hue, 0x0441);
            }
            other => panic!("{other:?}"),
        }
        let old = ClientVersion {
            major: 7,
            minor: 0,
            revision: 9,
            patch: 0,
        };
        assert!(!old.has_incoming_equip_hue());
        let mut old_w = crate::buf::PacketWriter::with_variable(PKT_MOBILE_INCOMING);
        old_w
            .u32(0xAA)
            .u16(0x0190)
            .u16(10)
            .u16(20)
            .i8(0)
            .u8(0)
            .u16(0)
            .u8(0)
            .u8(1)
            .u32(0x4000_0001)
            .u16(0x0F43)
            .u8(1)
            .u32(0);
        let old_p = old_w.finish_variable().unwrap();
        match parse_with_version(&old_p, old).unwrap() {
            Inbound::MobileIncoming(m) => {
                assert_eq!(m.equipment.len(), 1);
                assert_eq!(m.equipment[0].hue, 0);
                assert_eq!(m.equipment[0].graphic, 0x0F43);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn status_reads_weight_max_when_flag_at_least_five() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_STATUS);
        w.u32(0xAA)
            .ascii_fixed("Mara", 30)
            .u16(10)
            .u16(10)
            .u8(0)
            .u8(STATUS_FLAG_ML)
            .u8(0)
            .u16(20)
            .u16(20)
            .u16(20)
            .u16(10)
            .u16(10)
            .u16(10)
            .u16(10)
            .u32(5)
            .u16(0)
            .u16(40)
            .u16(100);
        let p = w.finish_variable().unwrap();
        match parse(&p).unwrap() {
            Inbound::Status {
                weight_max, weight, ..
            } => {
                assert_eq!(weight, 40);
                assert_eq!(weight_max, Some(100));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn skills_type_zero_has_no_cap_word() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_SKILLS);
        w.u8(SKILL_TYPE_LIST).u16(1).u16(100).u16(100).u8(0).u16(0);
        let p = w.finish_variable().unwrap();
        match parse(&p).unwrap() {
            Inbound::Skills { skills } => {
                assert_eq!(skills.len(), 1);
                assert_eq!(skills[0].id, 1);
                assert_eq!(skills[0].cap, SKILL_CAP_DEFAULT);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn skills_type_two_reads_cap() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_SKILLS);
        w.u8(SKILL_TYPE_LIST_CAP)
            .u16(1)
            .u16(100)
            .u16(100)
            .u8(0)
            .u16(1200)
            .u16(0);
        let p = w.finish_variable().unwrap();
        match parse(&p).unwrap() {
            Inbound::Skills { skills } => {
                assert_eq!(skills[0].cap, 1200);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ascii_and_unicode_speech_parse() {
        let mut ascii = crate::buf::PacketWriter::with_variable(PKT_ASCII_MESSAGE);
        ascii
            .u32(1)
            .u16(0x190)
            .u8(0)
            .u16(0x03B2)
            .u16(3)
            .ascii_fixed("Sys", 30)
            .ascii_z("hello");
        match parse(&ascii.finish_variable().unwrap()).unwrap() {
            Inbound::Speech(s) => assert_eq!(s.text, "hello"),
            other => panic!("{other:?}"),
        }
        let mut uni = crate::buf::PacketWriter::with_variable(PKT_UNICODE_MESSAGE);
        uni.u32(1)
            .u16(0x190)
            .u8(0)
            .u16(0x03B2)
            .u16(3)
            .bytes(b"ENU\0")
            .ascii_fixed("Sys", 30)
            .utf16be_z("hi");
        match parse(&uni.finish_variable().unwrap()).unwrap() {
            Inbound::Speech(s) => assert_eq!(s.text, "hi"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn container_contents_and_add_item() {
        let mut contents = crate::buf::PacketWriter::with_variable(PKT_CONTAINER_CONTENTS);
        contents
            .u16(1)
            .u32(0x4000_0001)
            .u16(0x0EED)
            .u8(0)
            .u16(1)
            .u16(0)
            .u16(0)
            .u8(0)
            .u32(0x4000_0002)
            .u16(0);
        let pkt = contents.finish_variable().unwrap();
        match parse(&pkt) {
            Ok(Inbound::ContainerContents { items }) => {
                assert_eq!(items[0].graphic, 0x0EED);
            }
            other => panic!("{other:?}"),
        }
        let mut add = crate::buf::PacketWriter::new(PKT_ADD_ITEM);
        add.u32(0x4000_0003)
            .u16(0x0EED)
            .u8(0)
            .u16(2)
            .u16(1)
            .u16(1)
            .u32(0x4000_0002)
            .u16(0);
        match parse(&add.finish()).unwrap() {
            Inbound::AddItem(it) => assert_eq!(it.amount, 2),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn gump_parses_text_lines() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_GUMP);
        w.u32(1)
            .u32(2)
            .u32(0)
            .u32(0)
            .u16(2)
            .bytes(b"{}")
            .u16(1)
            .u16(2);
        for c in "ok".encode_utf16() {
            w.u16(c);
        }
        match parse(&w.finish_variable().unwrap()).unwrap() {
            Inbound::Gump(g) => {
                assert_eq!(g.layout, "{}");
                assert_eq!(g.text, vec!["ok".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn compressed_gump_inflates_layout_and_text() {
        fn z(bytes: &[u8]) -> Vec<u8> {
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
            enc.write_all(bytes).unwrap();
            enc.finish().unwrap()
        }
        let layout_z = z(b"{page 0}");
        let mut text_plain = Vec::new();
        text_plain.extend_from_slice(&1u16.to_be_bytes());
        text_plain.extend_from_slice(&1u16.to_be_bytes());
        text_plain.extend_from_slice(&(b'A' as u16).to_be_bytes());
        let text_z = z(&text_plain);
        let mut w = crate::buf::PacketWriter::with_variable(PKT_COMPRESSED_GUMP);
        w.u32(1)
            .u32(9)
            .u32(0)
            .u32(0)
            .u32((layout_z.len() + COMPRESSED_LEN_HEADER) as u32)
            .u32(8)
            .bytes(&layout_z)
            .u32(1)
            .u32((text_z.len() + COMPRESSED_LEN_HEADER) as u32)
            .u32(text_plain.len() as u32)
            .bytes(&text_z);
        match parse(&w.finish_variable().unwrap()).unwrap() {
            Inbound::Gump(g) => {
                assert_eq!(g.layout, "{page 0}");
                assert_eq!(g.text, vec!["A".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }

    /// A server packs an item name as `prefix \t name \t suffix` under cliloc
    /// 1050045.
    const OPL_NAME_CLILOC: u32 = 1_050_045;
    const OPL_WEIGHT_CLILOC: u32 = 1_072_788;
    const OPL_NO_ARGUMENTS: u16 = 0;
    const OPL_SERIAL: u32 = 0x4000_00AB;
    const OPL_HASH: u32 = 0x1234_5678;

    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn object_property_list_parses_name_and_arguments() {
        let name_args = utf16le("\ta wooden chair\t");
        let mut w = crate::buf::PacketWriter::with_variable(PKT_BATCH_QUERY_PROPERTIES);
        w.u16(OPL_LIST_FORMAT)
            .u32(OPL_SERIAL)
            .u16(OPL_LIST_PADDING)
            .u32(OPL_HASH)
            .u32(OPL_NAME_CLILOC)
            .u16(name_args.len() as u16)
            .bytes(&name_args)
            .u32(OPL_WEIGHT_CLILOC)
            .u16(OPL_NO_ARGUMENTS)
            .u32(OPL_LIST_TERMINATOR);
        let packet = w.finish_variable().unwrap();
        let framed = u16::from_be_bytes([packet[1], packet[2]]);
        assert_eq!(framed as usize, packet.len());
        match parse(&packet).unwrap() {
            Inbound::ObjectPropertyList {
                serial,
                hash,
                properties,
            } => {
                assert_eq!(serial, Serial(OPL_SERIAL));
                assert_eq!(hash, OPL_HASH);
                assert_eq!(properties.len(), 2);
                assert_eq!(properties[0].cliloc, OPL_NAME_CLILOC);
                assert_eq!(properties[0].arguments, "\ta wooden chair\t");
                assert_eq!(
                    properties[0].arguments.split('\t').nth(1),
                    Some("a wooden chair")
                );
                assert_eq!(properties[1].cliloc, OPL_WEIGHT_CLILOC);
                assert!(properties[1].arguments.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn object_property_list_ignores_unsupported_format() {
        let mut w = crate::buf::PacketWriter::with_variable(PKT_BATCH_QUERY_PROPERTIES);
        w.u16(OPL_LIST_FORMAT + 1)
            .u32(OPL_SERIAL)
            .u16(OPL_LIST_PADDING)
            .u32(OPL_HASH)
            .u32(OPL_LIST_TERMINATOR);
        match parse(&w.finish_variable().unwrap()).unwrap() {
            Inbound::Unknown { id, .. } => assert_eq!(id, PKT_BATCH_QUERY_PROPERTIES),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn opl_info_parses_serial_and_revision_hash() {
        const OPL_INFO_LEN: usize = 9;
        let packet = vec![PKT_OPL_INFO, 0x40, 0x00, 0x00, 0xAB, 0x12, 0x34, 0x56, 0x78];
        assert_eq!(packet.len(), OPL_INFO_LEN);
        match parse(&packet).unwrap() {
            Inbound::OplInfo { serial, hash } => {
                assert_eq!(serial, Serial(OPL_SERIAL));
                assert_eq!(hash, OPL_HASH);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cliloc_and_affix_parse() {
        let mut c1 = crate::buf::PacketWriter::with_variable(PKT_CLILOC);
        c1.u32(1)
            .u16(0)
            .u8(0)
            .u16(0)
            .u16(3)
            .u32(500119)
            .ascii_fixed("System", 30)
            .u16(0);
        match parse(&c1.finish_variable().unwrap()).unwrap() {
            Inbound::Speech(s) => assert!(s.text.contains("500119")),
            other => panic!("{other:?}"),
        }
        let mut cc = crate::buf::PacketWriter::with_variable(PKT_CLILOC_AFFIX);
        cc.u32(1)
            .u16(0)
            .u8(0)
            .u16(0)
            .u16(3)
            .u32(1)
            .u8(0)
            .ascii_fixed("Npc", 30)
            .ascii_z("!")
            .utf16be_z("arg");
        match parse(&cc.finish_variable().unwrap()).unwrap() {
            Inbound::Speech(s) => {
                assert_eq!(s.name, "Npc");
                assert!(s.text.contains("arg"));
            }
            other => panic!("{other:?}"),
        }
    }

    /// Bytes of a server health bar packet: id, length, serial, one entry count,
    /// colour, status byte.
    fn health_bar_packet(kind: u16, status: u8) -> Vec<u8> {
        let mut p = vec![
            PKT_HEALTH_BAR_STATUS,
            0x00,
            0x0C,
            0x00,
            0x00,
            0x12,
            0x34,
            0x00,
            0x01,
        ];
        p.extend_from_slice(&kind.to_be_bytes());
        p.push(status);
        p
    }

    #[test]
    fn health_bar_poison_carries_a_level_on_a_modern_client() {
        let p = health_bar_packet(HEALTH_BAR_POISON, 3);
        assert_eq!(p.len(), 12);
        match parse_with_version(&p, ClientVersion::MODERN).unwrap() {
            Inbound::HealthBarUpdate { serial, bars } => {
                assert_eq!(serial, Serial(0x0000_1234));
                assert_eq!(bars.len(), 1);
                assert_eq!(bars[0].kind, HEALTH_BAR_POISON);
                assert!(bars[0].enabled);
                assert_eq!(bars[0].poison_level, Some(2));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn health_bar_poison_is_only_a_flag_on_an_old_client() {
        let p = health_bar_packet(HEALTH_BAR_POISON, 3);
        assert!(!ClientVersion::T2A.has_sa_poison_level());
        match parse_with_version(&p, ClientVersion::T2A).unwrap() {
            Inbound::HealthBarUpdate { bars, .. } => {
                assert_eq!(bars[0].kind, HEALTH_BAR_POISON);
                assert!(bars[0].enabled);
                assert_eq!(bars[0].poison_level, None);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn health_bar_yellow_marks_a_blessed_mobile() {
        let p = health_bar_packet(HEALTH_BAR_YELLOW, 1);
        match parse(&p).unwrap() {
            Inbound::HealthBarUpdate { bars, .. } => {
                assert_eq!(bars[0].kind, HEALTH_BAR_YELLOW);
                assert!(bars[0].enabled);
                assert_eq!(bars[0].poison_level, None);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn health_bar_status_zero_turns_the_colour_off() {
        let p = health_bar_packet(HEALTH_BAR_POISON, HEALTH_BAR_OFF);
        match parse(&p).unwrap() {
            Inbound::HealthBarUpdate { bars, .. } => {
                assert!(!bars[0].enabled);
                assert_eq!(bars[0].poison_level, None);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn health_bar_refuses_a_truncated_packet() {
        let p = health_bar_packet(HEALTH_BAR_POISON, 3);
        for cut in 1..p.len() {
            match parse(&p[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server animation packet.
    const ANIMATION_PACKET: [u8; 14] = [
        PKT_CHARACTER_ANIMATION,
        0x00,
        0x00,
        0x12,
        0x34,
        0x00,
        0x0B,
        0x00,
        0x07,
        0x00,
        0x01,
        0x00,
        0x00,
        0x05,
    ];

    #[test]
    fn character_animation_reads_the_action_group() {
        match parse(&ANIMATION_PACKET).unwrap() {
            Inbound::CharacterAnimation {
                serial,
                action,
                frame_count,
                repeat_count,
                forward,
                repeat,
                delay,
            } => {
                assert_eq!(serial, Serial(0x0000_1234));
                assert_eq!(action, 0x000B);
                assert_eq!(frame_count, 7);
                assert_eq!(repeat_count, 1);
                assert!(forward);
                assert!(!repeat);
                assert_eq!(delay, 5);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn character_animation_reverse_byte_clears_forward() {
        let mut p = ANIMATION_PACKET;
        p[11] = 1;
        p[12] = 1;
        match parse(&p).unwrap() {
            Inbound::CharacterAnimation {
                forward, repeat, ..
            } => {
                assert!(!forward);
                assert!(repeat);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn character_animation_refuses_a_truncated_packet() {
        for cut in 1..ANIMATION_PACKET.len() {
            match parse(&ANIMATION_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server sound effect packet at 1425, 1680, -5.
    const SOUND_PACKET: [u8; 12] = [
        PKT_SOUND_EFFECT,
        0x01,
        0x00,
        0x55,
        0x00,
        0x00,
        0x05,
        0x91,
        0x06,
        0x90,
        0xFF,
        0xFB,
    ];

    #[test]
    fn sound_effect_reads_the_id_and_the_place() {
        match parse(&SOUND_PACKET).unwrap() {
            Inbound::SoundEffect {
                sound,
                volume,
                x,
                y,
                z,
            } => {
                assert_eq!(sound, 0x0055);
                assert_eq!(volume, 0);
                assert_eq!((x, y, z), (1425, 1680, -5));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn sound_effect_refuses_a_truncated_packet() {
        for cut in 1..SOUND_PACKET.len() {
            match parse(&SOUND_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    #[test]
    fn music_plays_an_index() {
        match parse(&[PKT_MUSIC, 0x00, 0x13]).unwrap() {
            Inbound::Music { index, stop } => {
                assert_eq!(index, 0x0013);
                assert!(!stop);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn music_stop_uses_the_named_index() {
        match parse(&[PKT_MUSIC, 0x1F, 0xFF]).unwrap() {
            Inbound::Music { index, stop } => {
                assert_eq!(index, MUSIC_INDEX_STOP);
                assert!(stop);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn music_refuses_a_truncated_packet() {
        for cut in 1..3 {
            match parse(&[PKT_MUSIC, 0x1F, 0xFF][..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server add buff packet for night sight, which carries
    /// no cliloc arguments and pads the tail with ten zeroes.
    const BUFF_ADD_PACKET: [u8; 46] = [
        PKT_BUFF_DEBUFF,
        0x00,
        0x2E,
        0x00,
        0x00,
        0x12,
        0x34,
        0x03,
        0xED,
        0x00,
        0x01,
        0x00,
        0x00,
        0x00,
        0x00,
        0x03,
        0xED,
        0x00,
        0x01,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x10,
        0x69,
        0xBB,
        0x00,
        0x10,
        0x69,
        0xBC,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
    ];

    /// Bytes of a server add buff packet for the strength spell, whose
    /// cliloc argument is the percentage the spell adds.
    const BUFF_ADD_WITH_ARGUMENTS_PACKET: [u8; 56] = [
        PKT_BUFF_DEBUFF,
        0x00,
        0x38,
        0x00,
        0x00,
        0x12,
        0x34,
        0x04,
        0x17,
        0x00,
        0x01,
        0x00,
        0x00,
        0x00,
        0x00,
        0x04,
        0x17,
        0x00,
        0x01,
        0x00,
        0x00,
        0x00,
        0x00,
        0x01,
        0x2C,
        0x00,
        0x00,
        0x00,
        0x00,
        0x10,
        0x6A,
        0x85,
        0x00,
        0x10,
        0x6A,
        0x86,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x01,
        0x00,
        0x00,
        0x09,
        0x00,
        0x31,
        0x00,
        0x35,
        0x00,
        0x00,
        0x00,
        0x00,
        0x01,
        0x00,
        0x00,
    ];

    /// Bytes of a server remove buff packet.
    const BUFF_REMOVE_PACKET: [u8; 15] = [
        PKT_BUFF_DEBUFF,
        0x00,
        0x0F,
        0x00,
        0x00,
        0x12,
        0x34,
        0x04,
        0x0E,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
    ];

    #[test]
    fn buff_add_reads_the_icon_and_the_clilocs() {
        match parse(&BUFF_ADD_PACKET).unwrap() {
            Inbound::BuffDebuff {
                serial,
                icon,
                effects,
            } => {
                assert_eq!(serial, Serial(0x0000_1234));
                assert_eq!(icon, 0x03ED);
                assert_eq!(effects.len(), 1);
                assert_eq!(effects[0].icon, 0x03ED);
                assert_eq!(effects[0].duration_secs, 0);
                assert_eq!(effects[0].title_cliloc, 1075643);
                assert_eq!(effects[0].description_cliloc, 1075644);
                assert_eq!(effects[0].arguments, "");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn buff_add_reads_the_cliloc_arguments() {
        match parse(&BUFF_ADD_WITH_ARGUMENTS_PACKET).unwrap() {
            Inbound::BuffDebuff { icon, effects, .. } => {
                assert_eq!(icon, 0x0417);
                assert_eq!(effects.len(), 1);
                assert_eq!(effects[0].duration_secs, 300);
                assert_eq!(effects[0].title_cliloc, 1075845);
                assert_eq!(effects[0].description_cliloc, 1075846);
                assert_eq!(effects[0].arguments, "15");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn buff_remove_carries_no_effect() {
        match parse(&BUFF_REMOVE_PACKET).unwrap() {
            Inbound::BuffDebuff {
                serial,
                icon,
                effects,
            } => {
                assert_eq!(serial, Serial(0x0000_1234));
                assert_eq!(icon, 0x040E);
                assert!(effects.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn buff_refuses_a_truncated_effect() {
        /// Bytes of a `0xDF` effect that must be there. The argument block
        /// after them is padding on a server that sends no arguments, so only
        /// a cut inside this part is a truncated packet.
        const BUFF_EFFECT_MIN_LEN: usize = 36;
        for cut in 1..BUFF_EFFECT_MIN_LEN {
            match parse(&BUFF_ADD_WITH_ARGUMENTS_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
        match parse(&BUFF_ADD_WITH_ARGUMENTS_PACKET[..BUFF_EFFECT_MIN_LEN]).unwrap() {
            Inbound::BuffDebuff { effects, .. } => {
                assert_eq!(effects.len(), 1);
                assert_eq!(effects[0].title_cliloc, 1075845);
                assert_eq!(effects[0].arguments, "");
            }
            other => panic!("{other:?}"),
        }
    }

    /// Bytes of a server vendor buy list for a shop container that holds
    /// two lines, the second with no description at all.
    const VENDOR_BUY_LIST_PACKET: [u8; 30] = [
        PKT_VENDOR_BUY_LIST,
        0x00,
        0x1E,
        0x40,
        0x00,
        0x00,
        0xAA,
        0x02,
        0x00,
        0x00,
        0x00,
        0x2A,
        0x0B,
        b'i',
        b'r',
        b'o',
        b'n',
        b' ',
        b'i',
        b'n',
        b'g',
        b'o',
        b't',
        0x00,
        0x00,
        0x00,
        0x00,
        0x07,
        0x01,
        0x00,
    ];

    #[test]
    fn vendor_buy_list_reads_every_price() {
        match parse(&VENDOR_BUY_LIST_PACKET).unwrap() {
            Inbound::VendorBuyList { container, entries } => {
                assert_eq!(container, Serial(0x4000_00AA));
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].price, 42);
                assert_eq!(entries[0].description, "iron ingot");
                assert_eq!(entries[1].price, 7);
                assert_eq!(entries[1].description, "");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn vendor_buy_list_refuses_a_truncated_packet() {
        for cut in 1..VENDOR_BUY_LIST_PACKET.len() {
            match parse(&VENDOR_BUY_LIST_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server vendor sell list for one stack of arrows.
    const VENDOR_SELL_LIST_PACKET: [u8; 28] = [
        PKT_VENDOR_SELL_LIST,
        0x00,
        0x1C,
        0x00,
        0x00,
        0x12,
        0x34,
        0x00,
        0x01,
        0x40,
        0x00,
        0x00,
        0xBB,
        0x0F,
        0x3F,
        0x00,
        0x00,
        0x00,
        0x05,
        0x00,
        0x03,
        0x00,
        0x05,
        b'a',
        b'r',
        b'r',
        b'o',
        b'w',
    ];

    #[test]
    fn vendor_sell_list_reads_the_price_the_shopkeeper_pays() {
        match parse(&VENDOR_SELL_LIST_PACKET).unwrap() {
            Inbound::VendorSellList { vendor, entries } => {
                assert_eq!(vendor, Serial(0x0000_1234));
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].serial, Serial(0x4000_00BB));
                assert_eq!(entries[0].graphic, 0x0F3F);
                assert_eq!(entries[0].hue, 0);
                assert_eq!(entries[0].amount, 5);
                assert_eq!(entries[0].price, 3);
                assert_eq!(entries[0].name, "arrow");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn vendor_sell_list_refuses_a_truncated_packet() {
        for cut in 1..VENDOR_SELL_LIST_PACKET.len() {
            match parse(&VENDOR_SELL_LIST_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server end vendor buy packet.
    const VENDOR_CLOSE_PACKET: [u8; 8] = [PKT_VENDOR_BUY, 0x00, 0x08, 0x00, 0x00, 0x12, 0x34, 0x00];

    #[test]
    fn vendor_close_names_the_shopkeeper() {
        match parse(&VENDOR_CLOSE_PACKET).unwrap() {
            Inbound::VendorClose { vendor } => assert_eq!(vendor, Serial(0x0000_1234)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn vendor_close_refuses_a_truncated_packet() {
        for cut in 1..VENDOR_CLOSE_PACKET.len() {
            match parse(&VENDOR_CLOSE_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// The `0x24` a shopkeeper sends to open its buy window carries a gump of
    /// its own, which is how a server marks it.
    #[test]
    fn open_container_marks_the_buy_window() {
        let packet = [PKT_OPEN_CONTAINER, 0x00, 0x00, 0x12, 0x34, 0x00, 0x30];
        match parse(&packet).unwrap() {
            Inbound::OpenContainer { serial, gump } => {
                assert_eq!(serial, Serial(0x0000_1234));
                assert_eq!(gump, CONTAINER_GUMP_VENDOR_BUY);
            }
            other => panic!("{other:?}"),
        }
    }

    /// Bytes of a server item list menu with a question and two
    /// entries that carry a graphic each.
    const ITEM_LIST_MENU_PACKET: [u8; 33] = [
        PKT_OPEN_MENU,
        0x00,
        0x21,
        0x00,
        0x00,
        0x00,
        0x99,
        0x00,
        0x00,
        0x06,
        b'C',
        b'h',
        b'o',
        b'o',
        b's',
        b'e',
        0x02,
        0x1B,
        0xD7,
        0x00,
        0x00,
        0x03,
        b'l',
        b'o',
        b'g',
        0x0F,
        0x43,
        0x00,
        0x21,
        0x03,
        b'a',
        b'x',
        b'e',
    ];

    #[test]
    fn open_menu_reads_the_question_and_every_entry() {
        match parse(&ITEM_LIST_MENU_PACKET).unwrap() {
            Inbound::OpenMenu {
                serial,
                menu_id,
                question,
                entries,
            } => {
                assert_eq!(serial, Serial(0x0000_0099));
                assert_eq!(menu_id, 0);
                assert_eq!(question, "Choose");
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].graphic, 0x1BD7);
                assert_eq!(entries[0].hue, 0);
                assert_eq!(entries[0].name, "log");
                assert_eq!(entries[1].graphic, 0x0F43);
                assert_eq!(entries[1].hue, 0x0021);
                assert_eq!(entries[1].name, "axe");
            }
            other => panic!("{other:?}"),
        }
    }

    /// A server question menu writes four zeroes where an item
    /// list menu writes a graphic and a hue, so both read the same way.
    #[test]
    fn open_menu_reads_a_question_menu_with_no_graphics() {
        const QUESTION_MENU_PACKET: [u8; 19] = [
            PKT_OPEN_MENU,
            0x00,
            0x13,
            0x00,
            0x00,
            0x00,
            0x99,
            0x00,
            0x00,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x00,
            0x03,
            b'y',
            b'e',
            b's',
        ];
        match parse(&QUESTION_MENU_PACKET).unwrap() {
            Inbound::OpenMenu {
                question, entries, ..
            } => {
                assert_eq!(question, "");
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].graphic, 0);
                assert_eq!(entries[0].hue, 0);
                assert_eq!(entries[0].name, "yes");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn open_menu_refuses_a_truncated_packet() {
        for cut in 1..ITEM_LIST_MENU_PACKET.len() {
            match parse(&ITEM_LIST_MENU_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server context menu in the mode that carries a
    /// cliloc offset. The second entry is greyed out and carries a colour.
    const CONTEXT_MENU_PACKET: [u8; 26] = [
        PKT_EXTENDED,
        0x00,
        0x1A,
        0x00,
        0x14,
        0x00,
        0x01,
        0x00,
        0x00,
        0x12,
        0x34,
        0x02,
        0x00,
        0x00,
        0x00,
        0x06,
        0x00,
        0x00,
        0x00,
        0x01,
        0x00,
        0x0A,
        0x00,
        0x21,
        0x00,
        0x35,
    ];

    #[test]
    fn context_menu_adds_the_cliloc_base_in_the_old_mode() {
        match parse(&CONTEXT_MENU_PACKET).unwrap() {
            Inbound::ContextMenu { serial, entries } => {
                assert_eq!(serial, Serial(0x0000_1234));
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].index, 0);
                assert_eq!(entries[0].cliloc, CONTEXT_MENU_CLILOC_BASE + 6);
                assert_eq!(entries[0].colour, None);
                assert!(entries[0].enabled());
                assert_eq!(entries[1].index, 1);
                assert_eq!(entries[1].cliloc, CONTEXT_MENU_CLILOC_BASE + 10);
                assert_eq!(entries[1].colour, Some(0x0035));
                assert!(!entries[1].enabled());
            }
            other => panic!("{other:?}"),
        }
    }

    /// From mode two up a server writes the whole cliloc and no colour.
    #[test]
    fn context_menu_reads_a_whole_cliloc_in_the_new_mode() {
        const NEW_MODE_PACKET: [u8; 20] = [
            PKT_EXTENDED,
            0x00,
            0x14,
            0x00,
            0x14,
            0x00,
            0x02,
            0x00,
            0x00,
            0x12,
            0x34,
            0x01,
            0x00,
            0x2D,
            0xC6,
            0xC6,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        match parse(&NEW_MODE_PACKET).unwrap() {
            Inbound::ContextMenu { entries, .. } => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].cliloc, CONTEXT_MENU_CLILOC_BASE + 6);
                assert_eq!(entries[0].index, 0);
                assert_eq!(entries[0].colour, None);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn context_menu_refuses_a_truncated_packet() {
        for cut in 1..CONTEXT_MENU_PACKET.len() {
            match parse(&CONTEXT_MENU_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server secure trade display packet.
    fn trade_display_packet() -> Vec<u8> {
        let mut packet = vec![
            PKT_SECURE_TRADE,
            0x00,
            0x2F,
            TRADE_DISPLAY,
            0x00,
            0x00,
            0x12,
            0x34,
            0x40,
            0x00,
            0x00,
            0x01,
            0x40,
            0x00,
            0x00,
            0x02,
            TRADE_HAS_NAME,
        ];
        let mut name = b"Mara".to_vec();
        name.resize(TRADE_NAME_LEN, 0);
        packet.extend_from_slice(&name);
        packet
    }

    #[test]
    fn trade_display_names_the_partner_and_both_containers() {
        let packet = trade_display_packet();
        assert_eq!(packet.len(), 47);
        match parse(&packet).unwrap() {
            Inbound::Trade(trade) => {
                assert_eq!(trade.kind, TRADE_DISPLAY);
                assert_eq!(trade.serial, Serial(0x0000_1234));
                assert_eq!(trade.first, 0x4000_0001);
                assert_eq!(trade.second, 0x4000_0002);
                assert_eq!(trade.name, "Mara");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn trade_update_reads_both_accept_flags() {
        const TRADE_UPDATE_PACKET: [u8; 17] = [
            PKT_SECURE_TRADE,
            0x00,
            0x11,
            TRADE_UPDATE,
            0x40,
            0x00,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        match parse(&TRADE_UPDATE_PACKET).unwrap() {
            Inbound::Trade(trade) => {
                assert_eq!(trade.kind, TRADE_UPDATE);
                assert_eq!(trade.serial, Serial(0x4000_0001));
                assert_eq!(trade.first, 1);
                assert_eq!(trade.second, 0);
                assert_eq!(trade.name, "");
            }
            other => panic!("{other:?}"),
        }
    }

    /// A server writes two zeroes past the serial of a secure trade close and
    /// the reference client reads none of them, so a close carries no values.
    #[test]
    fn trade_close_carries_no_values() {
        const TRADE_CLOSE_PACKET: [u8; 17] = [
            PKT_SECURE_TRADE,
            0x00,
            0x11,
            TRADE_CLOSE,
            0x40,
            0x00,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        match parse(&TRADE_CLOSE_PACKET).unwrap() {
            Inbound::Trade(trade) => {
                assert_eq!(trade.kind, TRADE_CLOSE);
                assert_eq!(trade.serial, Serial(0x4000_0001));
                assert_eq!((trade.first, trade.second), (0, 0));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn trade_refuses_a_truncated_packet() {
        let packet = trade_display_packet();
        for cut in 1..packet.len() {
            match parse(&packet[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server book content packet for one page of two lines.
    const BOOK_CONTENT_PACKET: [u8; 25] = [
        PKT_BOOK_CONTENT,
        0x00,
        0x19,
        0x40,
        0x00,
        0x00,
        0xCC,
        0x00,
        0x01,
        0x00,
        0x01,
        0x00,
        0x02,
        b'h',
        b'e',
        b'l',
        b'l',
        b'o',
        0x00,
        b'w',
        b'o',
        b'r',
        b'l',
        b'd',
        0x00,
    ];

    #[test]
    fn book_content_reads_every_line_of_a_page() {
        match parse(&BOOK_CONTENT_PACKET).unwrap() {
            Inbound::BookContent { serial, pages } => {
                assert_eq!(serial, Serial(0x4000_00CC));
                assert_eq!(pages.len(), 1);
                assert_eq!(pages[0].number, 1);
                assert_eq!(pages[0].lines, vec!["hello", "world"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn book_content_refuses_a_truncated_packet() {
        for cut in 1..BOOK_CONTENT_PACKET.len() {
            match parse(&BOOK_CONTENT_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// Bytes of a server book cover, whose length words count the
    /// terminator of each field.
    const BOOK_HEADER_PACKET: [u8; 24] = [
        PKT_BOOK_HEADER,
        0x00,
        0x18,
        0x40,
        0x00,
        0x00,
        0xCC,
        0x01,
        BOOK_WRITABLE,
        0x00,
        0x02,
        0x00,
        0x05,
        b'T',
        b'a',
        b'l',
        b'e',
        0x00,
        0x00,
        0x04,
        b'B',
        b'o',
        b'b',
        0x00,
    ];

    #[test]
    fn book_header_reads_the_title_and_the_author() {
        match parse(&BOOK_HEADER_PACKET).unwrap() {
            Inbound::BookHeader {
                serial,
                writable,
                page_count,
                title,
                author,
            } => {
                assert_eq!(serial, Serial(0x4000_00CC));
                assert!(writable);
                assert_eq!(page_count, 2);
                assert_eq!(title, "Tale");
                assert_eq!(author, "Bob");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn book_header_refuses_a_truncated_packet() {
        for cut in 1..BOOK_HEADER_PACKET.len() {
            match parse(&BOOK_HEADER_PACKET[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    /// The old cover puts the title and the author at set widths. The reference
    /// client takes the writable flag from the first of the pair of bytes past
    /// the serial and reads the second without using it.
    fn book_header_old_packet() -> Vec<u8> {
        let mut packet = vec![
            PKT_BOOK_HEADER_OLD,
            0x40,
            0x00,
            0x00,
            0xCC,
            BOOK_WRITABLE,
            0x00,
            0x00,
            0x02,
        ];
        let mut title = b"Tale".to_vec();
        title.resize(BOOK_TITLE_LEN, 0);
        packet.extend_from_slice(&title);
        let mut author = b"Bob".to_vec();
        author.resize(BOOK_AUTHOR_LEN, 0);
        packet.extend_from_slice(&author);
        packet
    }

    #[test]
    fn book_header_old_reads_the_fixed_width_fields() {
        let packet = book_header_old_packet();
        assert_eq!(packet.len(), 99);
        match parse(&packet).unwrap() {
            Inbound::BookHeader {
                serial,
                writable,
                page_count,
                title,
                author,
            } => {
                assert_eq!(serial, Serial(0x4000_00CC));
                assert!(writable);
                assert_eq!(page_count, 2);
                assert_eq!(title, "Tale");
                assert_eq!(author, "Bob");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn book_header_old_refuses_a_truncated_packet() {
        let packet = book_header_old_packet();
        for cut in 1..packet.len() {
            match parse(&packet[..cut]) {
                Err(ProtocolError::Truncated { .. }) => {}
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }
}
