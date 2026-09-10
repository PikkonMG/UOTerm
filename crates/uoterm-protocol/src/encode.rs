use crate::buf::PacketWriter;
use crate::decode::{PromptRequest, TextEntryDialog};
use crate::types::*;

fn var_bytes(w: PacketWriter) -> Vec<u8> {
    w.finish_variable()
        .expect("encoded packet length fits in u16")
}

pub fn seed(seed: u32) -> Vec<u8> {
    seed.to_be_bytes().to_vec()
}

pub fn seed_ext(seed: u32, version: ClientVersion) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_SEED);
    w.u32(seed)
        .u32(version.major)
        .u32(version.minor)
        .u32(version.revision)
        .u32(version.patch);
    w.finish()
}

pub fn login_request(account: &str, password: &str, next_key: u8) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_LOGIN_REQUEST);
    w.ascii_fixed(account, 30)
        .ascii_fixed(password, 30)
        .u8(next_key);
    w.finish()
}

pub fn select_server(index: u16) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_SELECT_SERVER);
    w.u16(index);
    w.finish()
}

pub fn game_login(auth_id: u32, account: &str, password: &str) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_GAME_LOGIN);
    w.u32(auth_id)
        .ascii_fixed(account, 30)
        .ascii_fixed(password, 30);
    w.finish()
}

pub fn play_character(slot: u32, name: &str, client_flags: u32, client_ip: u32) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_PLAY_CHARACTER);
    w.u32(PLAY_CHAR_PATTERN)
        .ascii_fixed(name, 30)
        .u16(0)
        .u32(client_flags)
        .pad(24)
        .u32(slot)
        .u32(client_ip);
    w.finish()
}

pub fn client_version(version: ClientVersion) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_CLIENT_VERSION);
    w.ascii_z(&version.as_string());
    var_bytes(w)
}

pub fn client_type(version: ClientVersion) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_CLIENT_TYPE);
    w.u16(CLIENT_TYPE_CMD)
        .u16(CLIENT_TYPE_CLASSIC)
        .ascii_z(&version.as_string());
    var_bytes(w)
}

pub fn client_info() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_CLIENT_INFO);
    w.u8(CLIENT_INFO_TYPE_NEW)
        .u32(0)
        .u32(0)
        .u32(0)
        .u32(0)
        .u8(0)
        .u32(0)
        .u32(0)
        .u32(0)
        .u8(0)
        .u32(0)
        .u32(0)
        .u32(0)
        .u32(0)
        .u16(0)
        .u16(0)
        .ascii_fixed("", CLIENT_INFO_VIDEO_DESC_LEN)
        .u32(0)
        .u32(0)
        .u32(0)
        .u8(0)
        .u8(CLIENT_INFO_CLIENTS_RUNNING)
        .u8(CLIENT_INFO_CLIENTS_INSTALLED)
        .u8(0)
        .bytes(&LANGUAGE_ENU);
    let mut packet = w.finish();
    packet.resize(CLIENT_INFO_LEN, 0);
    packet
}

/// Classic Client 7.x / server movement request (`0x02`, 7 bytes):
/// `cmd`, `direction` (nibble 0-7, run bit `0x80`), `sequence`, fastwalk `u32` BE.
/// Sequence `0` is valid; this encoder does not wrap or skip it.
pub fn move_request(direction: Direction, running: bool, sequence: u8, fastwalk: u32) -> Vec<u8> {
    let mut dir = direction as u8;
    if running {
        dir |= DIR_RUNNING;
    }
    let mut w = PacketWriter::new(PKT_MOVE);
    w.u8(dir).u8(sequence).u32(fastwalk);
    w.finish()
}

pub fn ascii_speech(kind: u8, hue: u16, text: &str) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASCII_SPEECH);
    w.u8(kind).u16(hue).u16(DEFAULT_FONT).ascii_z(text);
    var_bytes(w)
}

pub fn unicode_speech(kind: u8, hue: u16, text: &str) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_UNICODE_SPEECH);
    w.u8(kind)
        .u16(hue)
        .u16(DEFAULT_FONT)
        .bytes(&LANGUAGE_ENU)
        .utf16be_z(text);
    var_bytes(w)
}

/// One keyword id, and the keyword count that opens the block, is a 12-bit
/// field, which is this many nibbles.
const KEYWORD_FIELD_NIBBLES: u32 = 3;
const NIBBLE_BITS: u32 = 4;
const NIBBLE_MASK: u16 = 0x000F;
const NIBBLES_PER_BYTE: usize = 2;

/// Packs the keyword count and the keyword ids of an encoded `0xAD` into a
/// big-endian stream of 12-bit fields, zero padded to a whole byte. An id
/// wider than 12 bits keeps its low 12 bits, as in the reference client.
fn keyword_block(keywords: &[u16]) -> Vec<u8> {
    let sent = &keywords[..keywords.len().min(SPEECH_KEYWORD_MAX)];
    let fields = std::iter::once(sent.len() as u16).chain(sent.iter().copied());
    let mut nibbles: Vec<u8> = Vec::new();
    for field in fields {
        for place in (0..KEYWORD_FIELD_NIBBLES).rev() {
            nibbles.push(((field >> (place * NIBBLE_BITS)) & NIBBLE_MASK) as u8);
        }
    }
    if nibbles.len() % NIBBLES_PER_BYTE != 0 {
        nibbles.push(0);
    }
    nibbles
        .chunks_exact(NIBBLES_PER_BYTE)
        .map(|pair| (pair[0] << NIBBLE_BITS) | pair[1])
        .collect()
}

/// The Classic Client unicode speech request (`0xAD`) carrying the numbered
/// keywords the words matched. A shard reacts to set phrases, such as the one
/// that renounces young player status, by keyword number and not by the words,
/// so speech that matches a keyword must go out this way. Plain speech of the
/// same words is chatter to the server.
///
/// After the usual `0xAD` header of kind, hue, font and language, the body is:
/// the keyword block from [`keyword_block`], then the text as UTF-8 with a NUL
/// terminator, in place of the plain packet's UTF-16BE text. The kind byte
/// gains [`SPEECH_ENCODED`]. `ascii_z` writes exactly those UTF-8 bytes and
/// that terminator, because a Rust `&str` is already UTF-8.
///
/// An empty `keywords` gives the plain [`unicode_speech`] packet, which is what
/// the reference client sends when no phrase matches. Keyword speech has no
/// ASCII form: the `0x03` body has no room for a keyword block.
pub fn keyword_speech(kind: u8, hue: u16, keywords: &[u16], text: &str) -> Vec<u8> {
    if keywords.is_empty() {
        return unicode_speech(kind, hue, text);
    }
    let mut w = PacketWriter::with_variable(PKT_UNICODE_SPEECH);
    w.u8(kind | SPEECH_ENCODED)
        .u16(hue)
        .u16(DEFAULT_FONT)
        .bytes(&LANGUAGE_ENU)
        .bytes(&keyword_block(keywords))
        .ascii_z(text);
    var_bytes(w)
}

pub fn say(text: &str, unicode: bool) -> Vec<u8> {
    if unicode {
        unicode_speech(SPEECH_REGULAR, DEFAULT_SPEECH_HUE, text)
    } else {
        ascii_speech(SPEECH_REGULAR, DEFAULT_SPEECH_HUE, text)
    }
}

pub fn whisper(text: &str, unicode: bool) -> Vec<u8> {
    if unicode {
        unicode_speech(SPEECH_WHISPER, DEFAULT_SPEECH_HUE, text)
    } else {
        ascii_speech(SPEECH_WHISPER, DEFAULT_SPEECH_HUE, text)
    }
}

pub fn emote(text: &str, unicode: bool) -> Vec<u8> {
    if unicode {
        unicode_speech(SPEECH_EMOTE, DEFAULT_SPEECH_HUE, text)
    } else {
        ascii_speech(SPEECH_EMOTE, DEFAULT_SPEECH_HUE, text)
    }
}

pub fn double_click(serial: Serial) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_DOUBLE_CLICK);
    w.serial(serial);
    w.finish()
}

/// The mobile query (`0x34`) asks for the skill list.
const QUERY_SKILLS: u8 = 5;
/// The fixed word the query carries before its kind.
const QUERY_PATTERN: u32 = 0xEDED_EDED;

/// The Classic Client mobile query (`0x34`) for the character's skills. The
/// shard answers with the full skill list (`0x3A`).
pub fn query_skills(me: Serial) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_QUERY);
    w.u32(QUERY_PATTERN).u8(QUERY_SKILLS).serial(me);
    w.finish()
}

pub fn single_click(serial: Serial) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_SINGLE_CLICK);
    w.serial(serial);
    w.finish()
}

/// The Classic Client object property list request (`0xD6`): id, framed length,
/// then at most [`BATCH_QUERY_PROPERTIES_MAX`] big-endian serials. The server
/// answers each serial with one `0xD6` object property list.
pub fn batch_query_properties(serials: &[Serial]) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_BATCH_QUERY_PROPERTIES);
    for serial in serials.iter().take(BATCH_QUERY_PROPERTIES_MAX) {
        w.serial(*serial);
    }
    var_bytes(w)
}

pub fn attack(serial: Serial) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_ATTACK);
    w.serial(serial);
    w.finish()
}

pub fn lift(serial: Serial, amount: u16) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_LIFT);
    w.serial(serial).u16(amount);
    w.finish()
}

pub fn drop(serial: Serial, x: u16, y: u16, z: i8, dest: Serial, grid: Option<u8>) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_DROP);
    w.serial(serial).u16(x).u16(y).i8(z);
    if let Some(g) = grid {
        w.u8(g);
    }
    w.serial(dest);
    w.finish()
}

/// Drop into a container so the server auto-places and stacks.
pub fn drop_into_container(serial: Serial, dest: Serial, grid: Option<u8>) -> Vec<u8> {
    drop(serial, DROP_CONTAINER_XY, DROP_CONTAINER_XY, 0, dest, grid)
}

pub fn equip(item: Serial, layer: u8, mobile: Serial) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_EQUIP);
    w.serial(item).u8(layer).serial(mobile);
    w.finish()
}

pub fn war_mode(on: bool) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_WAR_MODE);
    w.u8(u8::from(on)).u8(0x00).u8(0x32).u8(0x00);
    w.finish()
}

pub fn ping(seq: u8) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_PING);
    w.u8(seq);
    w.finish()
}

pub fn text_command(kind: u8, command: &str) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_TEXT_COMMAND);
    w.u8(kind).ascii_z(command);
    var_bytes(w)
}

pub fn use_skill(skill_id: u16) -> Vec<u8> {
    text_command(TEXT_CMD_USE_SKILL, &format!("{skill_id} 0"))
}

pub fn cast_spell(spell_id: u16) -> Vec<u8> {
    text_command(TEXT_CMD_CAST_SPELL, &spell_id.to_string())
}

pub fn open_door() -> Vec<u8> {
    text_command(TEXT_CMD_OPEN_DOOR, "")
}

/// The client's word that it has the shard's assistant feature list. A shard
/// that asks and hears nothing back can warn the player and then disconnect.
pub fn assistant_ack() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASSISTANT);
    w.u8(ASSIST_CMD_ACK);
    var_bytes(w)
}

/// The name of the assistant beside the client, in answer to the shard.
pub fn assistant_version(name: &str) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASSIST_VERSION);
    w.ascii_z(name);
    var_bytes(w)
}

pub fn target_object(
    cursor_id: u32,
    serial: Serial,
    x: u16,
    y: u16,
    z: i8,
    graphic: u16,
) -> Vec<u8> {
    target_answer(TARGET_OBJECT, cursor_id, serial, x, y, z, graphic)
}

/// The serial a ground target carries: the ground is no object.
const GROUND_IS_NO_OBJECT: Serial = Serial(0);

/// Answers a target cursor with a spot on the ground: a tile to dig, fish or
/// cast at. `graphic` names the static on that tile, or zero for bare land.
pub fn target_ground(cursor_id: u32, x: u16, y: u16, z: i8, graphic: u16) -> Vec<u8> {
    target_answer(
        TARGET_GROUND,
        cursor_id,
        GROUND_IS_NO_OBJECT,
        x,
        y,
        z,
        graphic,
    )
}

/// One target answer. An object answer and a ground answer share the layout
/// and differ only in the kind byte and the serial.
fn target_answer(
    kind: u8,
    cursor_id: u32,
    serial: Serial,
    x: u16,
    y: u16,
    z: i8,
    graphic: u16,
) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_TARGET);
    w.u8(kind)
        .u32(cursor_id)
        .u8(TARGET_FLAG_NONE)
        .serial(serial)
        .u16(x)
        .u16(y)
        .u8(0)
        .i8(z)
        .u16(graphic);
    w.finish()
}

pub fn cancel_target(cursor_id: u32) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_TARGET);
    w.u8(TARGET_OBJECT)
        .u32(cursor_id)
        .u8(TARGET_FLAG_CANCEL)
        .u32(0)
        .u16(0xFFFF)
        .u16(0xFFFF)
        .u8(0)
        .i8(0)
        .u16(0);
    w.finish()
}

pub fn gump_response(serial: Serial, gump_id: u32, button: u32, switches: &[u32]) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_GUMP_RESPONSE);
    w.serial(serial)
        .u32(gump_id)
        .u32(button)
        .u32(switches.len() as u32);
    for s in switches {
        w.u32(*s);
    }
    w.u32(0);
    var_bytes(w)
}

pub fn gump_close(serial: Serial, gump_id: u32) -> Vec<u8> {
    gump_response(serial, gump_id, 0, &[])
}

pub fn trade_start(mobile: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_SECURE_TRADE);
    w.u8(0).serial(mobile).u32(0).u32(0);
    var_bytes(w)
}

/// The Classic Client trade response with code one: give up the trade whose
/// container is `container`. The server's secure trade case one cancels it.
pub fn trade_cancel(container: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_SECURE_TRADE);
    w.u8(TRADE_CLOSE).serial(container);
    var_bytes(w)
}

/// The Classic Client trade response with code two: tick or untick the accept
/// box of the trade whose container is `container`. The server's secure trade
/// case two reads the value as a 32-bit flag.
pub fn trade_accept(container: Serial, accepted: bool) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_SECURE_TRADE);
    w.u8(TRADE_UPDATE)
        .serial(container)
        .u32(u32::from(accepted));
    var_bytes(w)
}

/// The Classic Client trade gold update: put gold and platinum into the trade
/// whose container is `container`. The server's secure trade case three reads
/// both.
pub fn trade_gold(container: Serial, gold: u32, platinum: u32) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_SECURE_TRADE);
    w.u8(TRADE_UPDATE_GOLD)
        .serial(container)
        .u32(gold)
        .u32(platinum);
    var_bytes(w)
}

/// The Classic Client buy request (`0x3B`): buy `items` from `vendor`, each as
/// a serial from the shop container and the amount wanted. A server acts only
/// on a basket opened by [`VENDOR_BUY_WITH_ITEMS`] and drops one that holds
/// more than [`VENDOR_BUY_ITEM_MAX`] items, so this encoder sends no more than
/// that. An empty `items` gives the empty basket, which buys nothing and shuts
/// the window.
pub fn vendor_buy(vendor: Serial, items: &[(Serial, u16)]) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_VENDOR_BUY);
    w.serial(vendor);
    if items.is_empty() {
        w.u8(VENDOR_BUY_EMPTY);
    } else {
        w.u8(VENDOR_BUY_WITH_ITEMS);
        for (serial, amount) in items.iter().take(VENDOR_BUY_ITEM_MAX) {
            w.u8(VENDOR_BUY_ITEM_LAYER).serial(*serial).u16(*amount);
        }
    }
    var_bytes(w)
}

/// The Classic Client sell request (`0x9F`): sell `items` to `vendor`, each as
/// a serial from the `0x9E` list and the amount to part with. A server drops a
/// list of [`VENDOR_SELL_ITEM_MAX`] items or more, so this encoder stops one
/// short of it.
pub fn vendor_sell(vendor: Serial, items: &[(Serial, u16)]) -> Vec<u8> {
    let sent = &items[..items.len().min(VENDOR_SELL_ITEM_MAX - 1)];
    let mut w = PacketWriter::with_variable(PKT_VENDOR_SELL);
    w.serial(vendor).u16(sent.len() as u16);
    for (serial, amount) in sent {
        w.serial(*serial).u16(*amount);
    }
    var_bytes(w)
}

/// The Classic Client menu response (`0x7D`): pick entry `index` of the `0x7C`
/// menu `menu_id` on `serial`. Entries are numbered from
/// [`MENU_FIRST_INDEX`], which a server takes one off again.
/// `graphic` and `hue` come from the entry that was picked.
pub fn menu_response(serial: Serial, menu_id: u16, index: u16, graphic: u16, hue: u16) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_MENU_RESPONSE);
    w.serial(serial)
        .u16(menu_id)
        .u16(index)
        .u16(graphic)
        .u16(hue);
    w.finish()
}

/// Walk away from a `0x7C` menu without picking anything. The reference client
/// writes the serial and the menu id and pads the rest of the fixed packet
/// with zeroes.
pub fn menu_cancel(serial: Serial, menu_id: u16) -> Vec<u8> {
    menu_response(serial, menu_id, MENU_CANCEL_INDEX, 0, 0)
}

/// The Classic Client popup menu request (`0xBF` `0x13`): ask for the context
/// menu of `serial`, which is what a single right click does. A server answers
/// with a `0xBF` `0x14`.
pub fn context_menu_request(serial: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_CONTEXT_MENU_REQUEST).serial(serial);
    var_bytes(w)
}

/// The Classic Client popup menu selection (`0xBF` `0x15`): pick the entry of
/// the context menu on `serial` whose index is `index`. A server reads the
/// index as a word.
pub fn context_menu_response(serial: Serial, index: u16) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_CONTEXT_MENU_RESPONSE).serial(serial).u16(index);
    var_bytes(w)
}

/// Use a bandage on `target` with no cursor. Pass the character's own serial
/// to heal yourself.
pub fn bandage_target(bandage: Serial, target: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_BANDAGE_TARGET).serial(bandage).serial(target);
    var_bytes(w)
}

/// A prompt answer. Accept sends the text; cancel sends none and tells the
/// shard to give up.
pub fn prompt_response(prompt: PromptRequest, text: &str, accept: bool) -> Vec<u8> {
    let id = if prompt.unicode {
        PKT_UNICODE_PROMPT
    } else {
        PKT_ASCII_PROMPT
    };
    let mut w = PacketWriter::with_variable(id);
    w.serial(prompt.serial)
        .u32(prompt.id)
        .u32(u32::from(accept));
    if prompt.unicode {
        w.ascii_fixed(PROMPT_LANGUAGE, PROMPT_LANGUAGE_FIELD)
            .utf16le(if accept { text } else { "" });
    } else {
        w.ascii_z(if accept { text } else { "" });
    }
    var_bytes(w)
}

/// The language field of a Unicode prompt answer: three letters and a zero.
const PROMPT_LANGUAGE: &str = "ENU";
const PROMPT_LANGUAGE_FIELD: usize = 4;

/// The answer to a one-field dialog. `accept` false is its cancel button.
pub fn text_entry_response(dialog: &TextEntryDialog, text: &str, accept: bool) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_TEXT_ENTRY_RESPONSE);
    w.serial(dialog.serial)
        .u8(dialog.parent)
        .u8(dialog.button)
        .u8(u8::from(accept))
        .u16((text.len() + 1) as u16)
        .ascii_z(text);
    var_bytes(w)
}

/// The colour picked for a dye tub.
pub fn dye_response(tub: Serial, hue: u16) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_DYE);
    w.serial(tub).u16(0).u16(hue);
    w.finish()
}

/// Asks the shard to rename a pet the character owns.
pub fn rename(serial: Serial, name: &str) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_RENAME);
    w.serial(serial).ascii_fixed(name, RENAME_NAME_FIELD);
    w.finish()
}

const RENAME_NAME_FIELD: usize = 30;

/// Arms a weapon special move by its number, or clears the armed move with
/// [`NO_ABILITY`]. The move number goes in the encoded form: a zero type byte,
/// then the number as a 32-bit word, then the closing byte the reference
/// client sends.
pub fn set_ability(player: Serial, ability: u8) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ENCODED);
    w.serial(player)
        .u16(ENCODED_SET_ABILITY)
        .u8(ENCODED_INT_TYPE)
        .u32(u32::from(ability))
        .u8(ENCODED_END);
    var_bytes(w)
}

/// The ability number that clears an armed move.
pub const NO_ABILITY: u8 = 0;
const ENCODED_INT_TYPE: u8 = 0;
const ENCODED_END: u8 = 0x0A;

/// The pre-AOS stun and disarm requests of bare hands.
pub fn stun_request() -> Vec<u8> {
    extended(EXT_STUN)
}

pub fn disarm_request() -> Vec<u8> {
    extended(EXT_DISARM)
}

/// A gargoyle takes off or lands.
pub fn toggle_flying() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_TOGGLE_FLYING).u16(TOGGLE_FLYING_ON).u32(0);
    var_bytes(w)
}

const TOGGLE_FLYING_ON: u16 = 1;

/// An extended packet with no body.
fn extended(sub: u16) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(sub);
    var_bytes(w)
}

/// Uses a tool on the resource it gathers, with no cursor: ore, sand, wood,
/// graves or red mushrooms by number, or a shard's own number.
pub fn resource_target(tool: Serial, resource: u16) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_RESOURCE_TARGET).serial(tool).u16(resource);
    var_bytes(w)
}

/// Sets the lock of one stat: 0 strength, 1 dexterity, 2 intelligence.
pub fn stat_lock(stat: u8, lock: u8) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_STAT_LOCK).u8(stat).u8(lock);
    var_bytes(w)
}

fn party(command: u8) -> PacketWriter {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_PARTY).u8(command);
    w
}

pub fn party_accept(leader: Serial) -> Vec<u8> {
    let mut w = party(PARTY_ACCEPT);
    w.serial(leader);
    var_bytes(w)
}

pub fn party_decline(leader: Serial) -> Vec<u8> {
    let mut w = party(PARTY_DECLINE);
    w.serial(leader);
    var_bytes(w)
}

/// Asks to add a member. With no serial the shard gives a target cursor.
pub fn party_invite(member: Option<Serial>) -> Vec<u8> {
    let mut w = party(PARTY_ADD);
    w.serial(member.unwrap_or(Serial(0)));
    var_bytes(w)
}

pub fn party_remove(member: Serial) -> Vec<u8> {
    let mut w = party(PARTY_REMOVE);
    w.serial(member);
    var_bytes(w)
}

/// Says a line to the whole party, or to one member.
pub fn party_message(to: Option<Serial>, text: &str) -> Vec<u8> {
    let mut w = match to {
        Some(member) => {
            let mut w = party(PARTY_PRIVATE_MESSAGE);
            w.serial(member);
            w
        }
        None => party(PARTY_PUBLIC_MESSAGE),
    };
    w.utf16be_z(text);
    var_bytes(w)
}

pub fn party_can_loot(allow: bool) -> Vec<u8> {
    let mut w = party(PARTY_CAN_LOOT);
    w.u8(u8::from(allow));
    var_bytes(w)
}

/// Invokes a virtue by its number, 1 to 8.
pub fn invoke_virtue(virtue: u8) -> Vec<u8> {
    text_command(TEXT_CMD_INVOKE_VIRTUE, &virtue.to_string())
}

/// Plays an emote animation by name, such as "bow" or "salute".
pub fn emote_animation(action: &str) -> Vec<u8> {
    text_command(TEXT_CMD_EMOTE_ANIMATION, action)
}

/// Opens the guild or the quest menu, as the paperdoll buttons do.
pub fn guild_menu(player: Serial) -> Vec<u8> {
    encoded_button(player, ENCODED_GUILD_MENU, ENCODED_END)
}

pub fn quest_menu(player: Serial) -> Vec<u8> {
    encoded_button(player, ENCODED_QUEST_MENU, ENCODED_QUEST_END)
}

const ENCODED_QUEST_END: u8 = 0;

fn encoded_button(player: Serial, command: u16, end: u8) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ENCODED);
    w.serial(player).u16(command).u8(end);
    var_bytes(w)
}

/// Tells the shard the character logs out.
pub fn logout() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_LOGOUT);
    w.u8(0);
    w.finish()
}

/// The Classic Client skill status change request (`0x3A`): set the lock of
/// one skill to [`SKILL_LOCK_UP`], [`SKILL_LOCK_DOWN`] or
/// [`SKILL_LOCK_LOCKED`]. A server reads the skill number and the lock byte.
pub fn skill_lock(skill_id: u16, lock: u8) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_SKILLS);
    w.u16(skill_id).u8(lock);
    var_bytes(w)
}

/// The Classic Client book page request (`0x66`): ask for one page of a book
/// the client has not read yet. The line count of [`BOOK_PAGE_REQUEST`] is
/// what marks the packet as a request and not an edit.
pub fn book_page_request(serial: Serial, page: u16) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_BOOK_CONTENT);
    w.serial(serial)
        .u16(BOOK_PAGE_COUNT_ONE)
        .u16(page)
        .u16(BOOK_PAGE_REQUEST);
    var_bytes(w)
}

/// The Classic Client book page write (`0x66`): write one page of a book. Each
/// line goes out as UTF-8 with a NUL after it, and a further NUL closes the
/// page. A server drops a page of more than [`BOOK_PAGE_LINE_MAX`] lines, so
/// this encoder sends no more than that.
pub fn book_page(serial: Serial, page: u16, lines: &[&str]) -> Vec<u8> {
    let sent = &lines[..lines.len().min(BOOK_PAGE_LINE_MAX)];
    let mut w = PacketWriter::with_variable(PKT_BOOK_CONTENT);
    w.serial(serial)
        .u16(BOOK_PAGE_COUNT_ONE)
        .u16(page)
        .u16(sent.len() as u16);
    for line in sent {
        w.ascii_z(&line.replace('\n', ""));
    }
    w.u8(0);
    var_bytes(w)
}

/// The Classic Client book header change (`0xD4`): rename a book and its
/// author. Both lengths count the text alone, with no terminator, which is
/// what a server reads.
pub fn book_header(serial: Serial, title: &str, author: &str) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_BOOK_HEADER);
    w.serial(serial)
        .u8(0)
        .u8(0)
        .u16(0)
        .u16(title.len() as u16)
        .bytes(title.as_bytes())
        .u16(author.len() as u16)
        .bytes(author.as_bytes());
    var_bytes(w)
}

pub fn resync() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_MOVE_ACK);
    w.u16(0);
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Inbound;
    use crate::error::ProtocolError;
    use crate::parse;

    const VARIABLE_HEADER_LEN: usize = 3;
    const SERIAL_LEN: usize = 4;

    /// Id, framed length, kind, hue, font and the four language bytes. The
    /// keyword block of an encoded `0xAD` starts right after them.
    const UNICODE_SPEECH_HEADER_LEN: usize = 12;
    const WORD_BYTES: usize = 2;
    const BYTE_BITS: u32 = NIBBLE_BITS * NIBBLES_PER_BYTE as u32;

    /// The words a player says to give up young player status. A server matches
    /// them to this keyword in its own keyword table and answers with the
    /// renounce gump.
    const RENOUNCE_YOUNG_PHRASE: &str = "i renounce my young player status";
    const KEYWORD_RENOUNCE_YOUNG: u16 = 0x0035;

    /// Two ids whose three nibbles all differ and are none of them zero, so a
    /// nibble put in the wrong place shows up in the golden bytes.
    const KEYWORD_ABC: u16 = 0x0ABC;
    const KEYWORD_DEF: u16 = 0x0DEF;

    /// Reads a keyword block the way a server's unicode speech handler does: a
    /// 16-bit word gives the 12-bit count and holds a spare nibble, then the
    /// keywords come in pairs, the first from one byte joined to the held
    /// nibble and the second from a fresh word that holds a nibble again.
    fn keywords_as_a_server_reads(block: &[u8]) -> Vec<u16> {
        let word = |at: usize| u16::from_be_bytes([block[at], block[at + 1]]);
        let mut at = 0;
        let opening = word(at);
        at += WORD_BYTES;
        let count = usize::from(opening >> NIBBLE_BITS);
        let mut hold = opening & NIBBLE_MASK;
        let mut keywords = Vec::with_capacity(count);
        let mut from_word = false;
        for _ in 0..count {
            if from_word {
                let value = word(at);
                at += WORD_BYTES;
                keywords.push(value >> NIBBLE_BITS);
                hold = value & NIBBLE_MASK;
            } else {
                keywords.push((hold << BYTE_BITS) | u16::from(block[at]));
                at += 1;
                hold = 0;
            }
            from_word = !from_word;
        }
        keywords
    }

    fn regular_keyword_speech(keywords: &[u16], text: &str) -> Vec<u8> {
        keyword_speech(SPEECH_REGULAR, DEFAULT_SPEECH_HUE, keywords, text)
    }

    #[test]
    fn keyword_speech_without_keywords_is_the_plain_packet() {
        const PLAIN_HI_PACKET: [u8; 18] = [
            PKT_UNICODE_SPEECH,
            0x00,
            0x12,
            SPEECH_REGULAR,
            0x03,
            0xB2,
            0x00,
            0x03,
            0x45,
            0x4E,
            0x55,
            0x00,
            0x00,
            b'h',
            0x00,
            b'i',
            0x00,
            0x00,
        ];
        let packet = regular_keyword_speech(&[], "hi");
        assert_eq!(packet, PLAIN_HI_PACKET);
        assert_eq!(
            packet,
            unicode_speech(SPEECH_REGULAR, DEFAULT_SPEECH_HUE, "hi")
        );
        assert_eq!(packet, say("hi", true));
    }

    #[test]
    fn keyword_speech_with_one_keyword_packs_twelve_bits() {
        const ONE_KEYWORD_HI_PACKET: [u8; 18] = [
            PKT_UNICODE_SPEECH,
            0x00,
            0x12,
            SPEECH_REGULAR | SPEECH_ENCODED,
            0x03,
            0xB2,
            0x00,
            0x03,
            0x45,
            0x4E,
            0x55,
            0x00,
            0x00,
            0x10,
            0x35,
            b'h',
            b'i',
            0x00,
        ];
        let packet = regular_keyword_speech(&[KEYWORD_RENOUNCE_YOUNG], "hi");
        assert_eq!(packet, ONE_KEYWORD_HI_PACKET);
        assert_eq!(
            keywords_as_a_server_reads(&packet[UNICODE_SPEECH_HEADER_LEN..]),
            vec![KEYWORD_RENOUNCE_YOUNG]
        );
    }

    /// An even keyword count leaves a nibble over and pads the block with a
    /// zero nibble; an odd count ends on a whole byte. Both crossings put a
    /// keyword astride a byte boundary, which is where the packing goes wrong.
    #[test]
    fn keyword_speech_packing_crosses_byte_boundaries_both_ways() {
        const TWO_KEYWORD_HI_PACKET: [u8; 20] = [
            PKT_UNICODE_SPEECH,
            0x00,
            0x14,
            SPEECH_REGULAR | SPEECH_ENCODED,
            0x03,
            0xB2,
            0x00,
            0x03,
            0x45,
            0x4E,
            0x55,
            0x00,
            0x00,
            0x20,
            0x35,
            0xAB,
            0xC0,
            b'h',
            b'i',
            0x00,
        ];
        const THREE_KEYWORD_HI_PACKET: [u8; 21] = [
            PKT_UNICODE_SPEECH,
            0x00,
            0x15,
            SPEECH_REGULAR | SPEECH_ENCODED,
            0x03,
            0xB2,
            0x00,
            0x03,
            0x45,
            0x4E,
            0x55,
            0x00,
            0x00,
            0x30,
            0x35,
            0xAB,
            0xCD,
            0xEF,
            b'h',
            b'i',
            0x00,
        ];
        let even = [KEYWORD_RENOUNCE_YOUNG, KEYWORD_ABC];
        let odd = [KEYWORD_RENOUNCE_YOUNG, KEYWORD_ABC, KEYWORD_DEF];

        let packet = regular_keyword_speech(&even, "hi");
        assert_eq!(packet, TWO_KEYWORD_HI_PACKET);
        assert_eq!(
            keywords_as_a_server_reads(&packet[UNICODE_SPEECH_HEADER_LEN..]),
            even.to_vec()
        );

        let packet = regular_keyword_speech(&odd, "hi");
        assert_eq!(packet, THREE_KEYWORD_HI_PACKET);
        assert_eq!(
            keywords_as_a_server_reads(&packet[UNICODE_SPEECH_HEADER_LEN..]),
            odd.to_vec()
        );
    }

    /// The packet that was missing on the shard: the renounce phrase with the
    /// keyword the server listens for, byte for byte.
    #[test]
    fn renounce_young_speech_is_byte_exact() {
        const RENOUNCE_PACKET_LEN: usize = 49;
        let mut expected: Vec<u8> = vec![
            PKT_UNICODE_SPEECH,
            0x00,
            0x31,
            SPEECH_REGULAR | SPEECH_ENCODED,
            0x03,
            0xB2,
            0x00,
            0x03,
            0x45,
            0x4E,
            0x55,
            0x00,
            0x00,
            0x10,
            0x35,
        ];
        expected.extend_from_slice(RENOUNCE_YOUNG_PHRASE.as_bytes());
        expected.push(0x00);

        let packet = regular_keyword_speech(&[KEYWORD_RENOUNCE_YOUNG], RENOUNCE_YOUNG_PHRASE);
        assert_eq!(packet, expected);
        assert_eq!(packet.len(), RENOUNCE_PACKET_LEN);
        assert_eq!(
            u16::from_be_bytes([packet[1], packet[2]]) as usize,
            packet.len()
        );
        assert_eq!(
            keywords_as_a_server_reads(&packet[UNICODE_SPEECH_HEADER_LEN..]),
            vec![KEYWORD_RENOUNCE_YOUNG]
        );
        assert_ne!(packet, say(RENOUNCE_YOUNG_PHRASE, true));
    }

    #[test]
    fn keyword_speech_takes_malformed_keyword_lists() {
        const OVER_TWELVE_BITS: u16 = 0xFA35;
        const LOW_TWELVE_BITS: u16 = 0x0A35;
        const OVER_KEYWORD_MAX: u16 = SPEECH_KEYWORD_MAX as u16 + 7;

        assert_eq!(
            regular_keyword_speech(&[OVER_TWELVE_BITS], "hi"),
            regular_keyword_speech(&[LOW_TWELVE_BITS], "hi")
        );

        let too_many: Vec<u16> = (0..OVER_KEYWORD_MAX).collect();
        let packet = regular_keyword_speech(&too_many, "hi");
        assert_eq!(
            keywords_as_a_server_reads(&packet[UNICODE_SPEECH_HEADER_LEN..]).as_slice(),
            &too_many[..SPEECH_KEYWORD_MAX]
        );
        assert_eq!(
            u16::from_be_bytes([packet[1], packet[2]]) as usize,
            packet.len()
        );

        let no_text = regular_keyword_speech(&[KEYWORD_RENOUNCE_YOUNG], "");
        assert_eq!(
            u16::from_be_bytes([no_text[1], no_text[2]]) as usize,
            no_text.len()
        );
    }

    #[test]
    fn keyword_speech_bytes_survive_every_cut() {
        let packet = regular_keyword_speech(
            &[KEYWORD_RENOUNCE_YOUNG, KEYWORD_ABC],
            RENOUNCE_YOUNG_PHRASE,
        );
        for cut in 0..=packet.len() {
            match parse(&packet[..cut]) {
                Ok(Inbound::Unknown { id, .. }) => assert_eq!(id, PKT_UNICODE_SPEECH),
                Err(ProtocolError::Truncated { .. }) => assert_eq!(cut, 0),
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    #[test]
    fn login_request_is_62_bytes() {
        let p = login_request("acct", "pw", LOGIN_NEXT_KEY_DEFAULT);
        assert_eq!(p.len(), 62);
        assert_eq!(p[0], PKT_LOGIN_REQUEST);
        assert_eq!(p[61], LOGIN_NEXT_KEY_DEFAULT);
    }

    #[test]
    fn move_sets_run_bit() {
        let p = move_request(Direction::East, true, 1, 0);
        assert_eq!(
            p,
            vec![
                PKT_MOVE,
                (Direction::East as u8) | DIR_RUNNING,
                1,
                0,
                0,
                0,
                0
            ]
        );
        assert_eq!(
            move_request(Direction::South, false, 1, 0),
            vec![PKT_MOVE, Direction::South as u8, 1, 0, 0, 0, 0]
        );
        assert_eq!(
            move_request(Direction::South, true, 1, 0),
            vec![
                PKT_MOVE,
                (Direction::South as u8) | DIR_RUNNING,
                1,
                0,
                0,
                0,
                0
            ]
        );
        assert_eq!(
            move_request(Direction::South, false, 0, 0x0102_0304),
            vec![PKT_MOVE, Direction::South as u8, 0, 0x01, 0x02, 0x03, 0x04]
        );
    }

    #[test]
    fn first_walk_after_login_is_sequence_zero() {
        const MOVE_REQ_LEN: usize = 7;
        let packet = move_request(Direction::North, false, 0, 0);
        assert_eq!(packet.len(), MOVE_REQ_LEN);
        assert_eq!(
            packet,
            vec![PKT_MOVE, Direction::North as u8, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn run_south_direction_byte_is_0x84() {
        const MOVE_REQ_LEN: usize = 7;
        const DIR_SOUTH_RUN: u8 = 0x84;
        let packet = move_request(Direction::South, true, 0, 0);
        assert_eq!(packet.len(), MOVE_REQ_LEN);
        assert_eq!(packet, vec![PKT_MOVE, DIR_SOUTH_RUN, 0, 0, 0, 0, 0]);
        assert_eq!(DIR_SOUTH_RUN, Direction::South as u8 | DIR_RUNNING);
    }

    #[test]
    fn direction_from_name_accepts_aliases() {
        assert_eq!(Direction::from_name("south"), Some(Direction::South));
        assert_eq!(Direction::from_name("S"), Some(Direction::South));
        assert_eq!(Direction::from_name("4"), Some(Direction::South));
        assert_eq!(Direction::from_name("nw"), Some(Direction::Northwest));
        assert_eq!(Direction::from_name("nope"), None);
    }

    #[test]
    fn speech_patches_length() {
        let p = ascii_speech(SPEECH_REGULAR, 0x22, "hi");
        let len = u16::from_be_bytes([p[1], p[2]]);
        assert_eq!(len as usize, p.len());
        assert_eq!(p[0], PKT_ASCII_SPEECH);
    }

    #[test]
    fn play_character_is_73_bytes() {
        let p = play_character(0, "Mara", 0, 0x7F00_0001);
        assert_eq!(p.len(), 73);
        assert_eq!(p[0], PKT_PLAY_CHARACTER);
    }

    #[test]
    fn seed_ext_is_21_bytes() {
        let p = seed_ext(1, ClientVersion::MODERN);
        assert_eq!(p.len(), 21);
        assert_eq!(p[0], PKT_SEED);
    }

    #[test]
    fn drop_with_grid_is_15_bytes() {
        let p = drop(Serial(0x4000_0001), 1, 2, 0, Serial::WORLD, Some(0));
        assert_eq!(p.len(), 15);
    }

    #[test]
    fn drop_into_a_container_uses_auto_place_coordinates() {
        const ITEM: Serial = Serial(0x4000_0001);
        const PACK: Serial = Serial(0x4000_0002);
        let p = drop_into_container(ITEM, PACK, Some(0));
        assert_eq!(p.len(), 15);
        assert_eq!(p[0], PKT_DROP);
        assert_eq!(u16::from_be_bytes([p[5], p[6]]), DROP_CONTAINER_XY);
        assert_eq!(u16::from_be_bytes([p[7], p[8]]), DROP_CONTAINER_XY);
        let dest = u32::from_be_bytes([p[11], p[12], p[13], p[14]]);
        assert_eq!(dest, PACK.0);
    }

    #[test]
    fn bandage_target_is_thirteen_bytes() {
        const BANDAGE: Serial = Serial(0x4000_0101);
        const SELF: Serial = Serial(0x0000_00AB);
        const PACKET: [u8; BANDAGE_TARGET_LEN] = [
            PKT_EXTENDED,
            0x00,
            0x0D,
            0x00,
            0x2C,
            0x40,
            0x00,
            0x01,
            0x01,
            0x00,
            0x00,
            0x00,
            0xAB,
        ];
        let packet = bandage_target(BANDAGE, SELF);
        assert_eq!(packet, PACKET);
        assert_eq!(packet.len(), BANDAGE_TARGET_LEN);
    }

    const PLAYER: Serial = Serial(0x0000_0001);
    const PROMPT: PromptRequest = PromptRequest {
        serial: Serial(0x0000_1234),
        id: 7,
        unicode: false,
    };

    #[test]
    fn a_prompt_answer_names_both_ids_and_the_text() {
        let p = prompt_response(PROMPT, "home", true);
        assert_eq!(p[0], PKT_ASCII_PROMPT);
        assert_eq!(&p[3..7], &PROMPT.serial.0.to_be_bytes());
        assert_eq!(&p[7..11], &PROMPT.id.to_be_bytes());
        assert_eq!(&p[11..15], &1u32.to_be_bytes(), "accepted");
        assert_eq!(&p[15..], b"home\0");
    }

    #[test]
    fn a_unicode_prompt_answer_carries_a_language_and_little_endian_text() {
        let unicode = PromptRequest {
            unicode: true,
            ..PROMPT
        };
        let p = prompt_response(unicode, "hi", true);
        assert_eq!(p[0], PKT_UNICODE_PROMPT);
        assert_eq!(&p[15..19], b"ENU\0");
        assert_eq!(&p[19..], &[b'h', 0, b'i', 0]);
        let cancelled = prompt_response(unicode, "hi", false);
        assert_eq!(&cancelled[11..15], &0u32.to_be_bytes());
        assert_eq!(cancelled.len(), 19, "a cancel sends no text");
    }

    #[test]
    fn a_text_entry_answer_repeats_the_dialog_ids() {
        let dialog = TextEntryDialog {
            serial: PROMPT.serial,
            parent: 1,
            button: 2,
            text: String::new(),
            can_cancel: true,
            style: 1,
            max_len: 20,
            description: String::new(),
        };
        let p = text_entry_response(&dialog, "ok", true);
        assert_eq!(&p[3..7], &PROMPT.serial.0.to_be_bytes());
        assert_eq!(&p[7..10], &[1, 2, 1]);
        assert_eq!(&p[10..12], &3u16.to_be_bytes());
        assert_eq!(&p[12..], b"ok\0");
    }

    #[test]
    fn a_dye_answer_names_the_tub_and_the_colour() {
        const HUE: u16 = 35;
        assert_eq!(
            dye_response(PROMPT.serial, HUE),
            vec![PKT_DYE, 0, 0, 0x12, 0x34, 0, 0, 0, 35]
        );
    }

    #[test]
    fn a_rename_has_the_fixed_size_the_shard_expects() {
        const RENAME_LEN: usize = 35;
        assert_eq!(rename(PROMPT.serial, "Snorlax").len(), RENAME_LEN);
    }

    /// The layout the reference client sends: player, command 0x19, a zero
    /// type byte, the move as a 32-bit word, and a closing 0x0A.
    #[test]
    fn a_special_move_is_armed_by_number() {
        const MORTAL_STRIKE: u8 = 9;
        let p = set_ability(PLAYER, MORTAL_STRIKE);
        assert_eq!(p[0], PKT_ENCODED);
        assert_eq!(u16::from_be_bytes([p[1], p[2]]) as usize, p.len());
        assert_eq!(&p[3..7], &PLAYER.0.to_be_bytes());
        assert_eq!(&p[7..], &[0x00, 0x19, 0, 0, 0, 0, MORTAL_STRIKE, 0x0A]);
    }

    #[test]
    fn party_answers_name_the_leader() {
        const LEADER: Serial = Serial(0x0000_0042);
        let p = party_accept(LEADER);
        assert_eq!(&p[3..], &[0x00, 0x06, PARTY_ACCEPT, 0, 0, 0, 0x42]);
        assert_eq!(party_decline(LEADER)[5], PARTY_DECLINE);
        let chat = party_message(None, "hi");
        assert_eq!(&chat[5..], &[PARTY_PUBLIC_MESSAGE, 0, b'h', 0, b'i', 0, 0]);
    }

    #[test]
    fn logout_and_flying_have_their_fixed_forms() {
        assert_eq!(logout(), vec![PKT_LOGOUT, 0]);
        assert_eq!(&toggle_flying()[3..], &[0x00, 0x32, 0x00, 0x01, 0, 0, 0, 0]);
    }

    #[test]
    fn the_assistant_ack_is_the_four_byte_handshake_answer() {
        assert_eq!(
            assistant_ack(),
            vec![PKT_ASSISTANT, 0x00, 0x04, ASSIST_CMD_ACK]
        );
    }

    #[test]
    fn the_assistant_version_is_a_length_prefixed_name() {
        const NAME: &str = "UOTerm 0.1.0";
        let p = assistant_version(NAME);
        assert_eq!(p[0], PKT_ASSIST_VERSION);
        assert_eq!(u16::from_be_bytes([p[1], p[2]]) as usize, p.len());
        assert_eq!(&p[3..3 + NAME.len()], NAME.as_bytes());
        assert_eq!(p.last(), Some(&0), "the name ends in a zero byte");
    }

    #[test]
    fn a_ground_target_names_a_tile_and_no_object() {
        const CURSOR: u32 = 7;
        const X: u16 = 1400;
        const Y: u16 = 1600;
        const Z: i8 = 5;
        const ROCK: u16 = 0x053B;
        let p = target_ground(CURSOR, X, Y, Z, ROCK);
        assert_eq!(p.len(), 19);
        assert_eq!(p[0], PKT_TARGET);
        assert_eq!(p[1], TARGET_GROUND);
        assert_eq!(&p[2..6], &CURSOR.to_be_bytes());
        assert_eq!(&p[7..11], &[0, 0, 0, 0], "the ground is no object");
        assert_eq!(&p[11..13], &X.to_be_bytes());
        assert_eq!(&p[13..15], &Y.to_be_bytes());
        assert_eq!(p[16] as i8, Z);
        assert_eq!(&p[17..19], &ROCK.to_be_bytes());
    }

    #[test]
    fn target_object_is_19_bytes() {
        let p = target_object(1, Serial(0xAA), 10, 10, 0, 0);
        assert_eq!(p.len(), 19);
        assert_eq!(p[0], PKT_TARGET);
    }

    #[test]
    fn client_type_is_the_modern_layout() {
        let p = client_type(ClientVersion::MODERN);
        assert_eq!(p[0], PKT_CLIENT_TYPE);
        let len = u16::from_be_bytes([p[1], p[2]]);
        assert_eq!(len as usize, p.len());
        assert_eq!(u16::from_be_bytes([p[3], p[4]]), CLIENT_TYPE_CMD);
        assert_eq!(u16::from_be_bytes([p[5], p[6]]), CLIENT_TYPE_CLASSIC);
        assert_eq!(&p[7..], b"7.0.102.3\0");
    }

    #[test]
    fn single_click_is_id_plus_serial() {
        const SINGLE_CLICK_LEN: usize = 1 + SERIAL_LEN;
        let packet = single_click(Serial(0x4000_00AB));
        assert_eq!(packet.len(), SINGLE_CLICK_LEN);
        assert_eq!(packet, vec![PKT_SINGLE_CLICK, 0x40, 0x00, 0x00, 0xAB]);
    }

    #[test]
    fn query_skills_matches_the_client_layout() {
        assert_eq!(
            query_skills(Serial(0x0000_1234)),
            vec![PKT_QUERY, 0xED, 0xED, 0xED, 0xED, 5, 0x00, 0x00, 0x12, 0x34]
        );
    }

    #[test]
    fn batch_query_properties_frames_every_serial() {
        let serials = [Serial(0x0000_1234), Serial(0x4000_00AB)];
        let packet = batch_query_properties(&serials);
        assert_eq!(
            packet,
            vec![
                PKT_BATCH_QUERY_PROPERTIES,
                0x00,
                0x0B,
                0x00,
                0x00,
                0x12,
                0x34,
                0x40,
                0x00,
                0x00,
                0xAB,
            ]
        );
        assert_eq!(
            packet.len(),
            VARIABLE_HEADER_LEN + serials.len() * SERIAL_LEN
        );
        let framed = u16::from_be_bytes([packet[1], packet[2]]);
        assert_eq!(framed as usize, packet.len());
    }

    #[test]
    fn batch_query_properties_stops_at_the_client_limit() {
        let overflow = BATCH_QUERY_PROPERTIES_MAX as u32 + 1;
        let serials: Vec<Serial> = (0..overflow).map(|i| Serial(SERIAL_ITEM_MIN + i)).collect();
        let packet = batch_query_properties(&serials);
        assert_eq!(
            packet.len(),
            VARIABLE_HEADER_LEN + BATCH_QUERY_PROPERTIES_MAX * SERIAL_LEN
        );
        let last = &packet[packet.len() - SERIAL_LEN..];
        assert_eq!(
            u32::from_be_bytes([last[0], last[1], last[2], last[3]]),
            SERIAL_ITEM_MIN + BATCH_QUERY_PROPERTIES_MAX as u32 - 1
        );
    }

    #[test]
    fn client_info_is_classic_hardware_len() {
        let p = client_info();
        assert_eq!(p.len(), CLIENT_INFO_LEN);
        assert_eq!(p[0], PKT_CLIENT_INFO);
        assert_eq!(p[1], CLIENT_INFO_TYPE_NEW);
        assert!(p.windows(4).any(|w| w == LANGUAGE_ENU));
    }

    const VENDOR: Serial = Serial(0x0000_1234);
    const SHOP_ITEM: Serial = Serial(0x4000_00AA);
    const TRADE_CONTAINER: Serial = Serial(0x4000_0001);
    const BOOK: Serial = Serial(0x4000_00CC);
    const MENU_HOLDER: Serial = Serial(0x0000_0099);

    /// The framed length of a self-describing packet must be its own size.
    fn framed_len(packet: &[u8]) -> usize {
        usize::from(u16::from_be_bytes([packet[1], packet[2]]))
    }

    #[test]
    fn vendor_buy_is_byte_exact() {
        const BUY_ONE_PACKET: [u8; 15] = [
            PKT_VENDOR_BUY,
            0x00,
            0x0F,
            0x00,
            0x00,
            0x12,
            0x34,
            VENDOR_BUY_WITH_ITEMS,
            VENDOR_BUY_ITEM_LAYER,
            0x40,
            0x00,
            0x00,
            0xAA,
            0x00,
            0x03,
        ];
        let packet = vendor_buy(VENDOR, &[(SHOP_ITEM, 3)]);
        assert_eq!(packet, BUY_ONE_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    /// An empty basket buys nothing. It is the same eight bytes the server
    /// sends back to shut the window.
    #[test]
    fn vendor_buy_with_no_items_is_the_empty_basket() {
        const BUY_NONE_PACKET: [u8; 8] = [
            PKT_VENDOR_BUY,
            0x00,
            0x08,
            0x00,
            0x00,
            0x12,
            0x34,
            VENDOR_BUY_EMPTY,
        ];
        assert_eq!(vendor_buy(VENDOR, &[]), BUY_NONE_PACKET);
    }

    #[test]
    fn vendor_buy_stops_at_the_server_limit() {
        const ITEM_LEN: usize = 7;
        let overflow = VENDOR_BUY_ITEM_MAX + 1;
        let items: Vec<(Serial, u16)> = (0..overflow)
            .map(|i| (Serial(SERIAL_ITEM_MIN + i as u32), 1))
            .collect();
        let packet = vendor_buy(VENDOR, &items);
        assert_eq!(
            packet.len(),
            VARIABLE_HEADER_LEN + SERIAL_LEN + 1 + VENDOR_BUY_ITEM_MAX * ITEM_LEN
        );
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn vendor_sell_is_byte_exact() {
        const SELL_ONE_PACKET: [u8; 15] = [
            PKT_VENDOR_SELL,
            0x00,
            0x0F,
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
            0x00,
            0x05,
        ];
        let packet = vendor_sell(VENDOR, &[(Serial(0x4000_00BB), 5)]);
        assert_eq!(packet, SELL_ONE_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    /// A server drops a sell list of `VENDOR_SELL_ITEM_MAX` items or more, so
    /// the encoder stops one item short of it.
    #[test]
    fn vendor_sell_stays_under_the_server_limit() {
        const ITEM_LEN: usize = 6;
        const COUNT_LEN: usize = 2;
        let items: Vec<(Serial, u16)> = (0..VENDOR_SELL_ITEM_MAX + 5)
            .map(|i| (Serial(SERIAL_ITEM_MIN + i as u32), 1))
            .collect();
        let packet = vendor_sell(VENDOR, &items);
        let sent = VENDOR_SELL_ITEM_MAX - 1;
        assert_eq!(
            packet.len(),
            VARIABLE_HEADER_LEN + SERIAL_LEN + COUNT_LEN + sent * ITEM_LEN
        );
        assert_eq!(u16::from_be_bytes([packet[7], packet[8]]) as usize, sent);
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn menu_response_is_byte_exact() {
        const MENU_RESPONSE_PACKET: [u8; 13] = [
            PKT_MENU_RESPONSE,
            0x00,
            0x00,
            0x00,
            0x99,
            0x00,
            0x00,
            0x00,
            0x01,
            0x1B,
            0xD7,
            0x00,
            0x00,
        ];
        assert_eq!(
            menu_response(MENU_HOLDER, 0, MENU_FIRST_INDEX, 0x1BD7, 0),
            MENU_RESPONSE_PACKET
        );
    }

    #[test]
    fn menu_cancel_is_byte_exact() {
        const MENU_CANCEL_PACKET: [u8; 13] = [
            PKT_MENU_RESPONSE,
            0x00,
            0x00,
            0x00,
            0x99,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        assert_eq!(menu_cancel(MENU_HOLDER, 0), MENU_CANCEL_PACKET);
        assert_eq!(
            menu_cancel(MENU_HOLDER, 0),
            menu_response(MENU_HOLDER, 0, MENU_CANCEL_INDEX, 0, 0)
        );
    }

    #[test]
    fn context_menu_request_is_byte_exact() {
        const REQUEST_PACKET: [u8; 9] =
            [PKT_EXTENDED, 0x00, 0x09, 0x00, 0x13, 0x00, 0x00, 0x12, 0x34];
        let packet = context_menu_request(VENDOR);
        assert_eq!(packet, REQUEST_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn context_menu_response_is_byte_exact() {
        const RESPONSE_PACKET: [u8; 11] = [
            PKT_EXTENDED,
            0x00,
            0x0B,
            0x00,
            0x15,
            0x00,
            0x00,
            0x12,
            0x34,
            0x00,
            0x01,
        ];
        let packet = context_menu_response(VENDOR, 1);
        assert_eq!(packet, RESPONSE_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn skill_lock_is_byte_exact() {
        const SKILL_LOCK_PACKET: [u8; 6] = [PKT_SKILLS, 0x00, 0x06, 0x00, 0x2C, SKILL_LOCK_LOCKED];
        let packet = skill_lock(SKILL_LUMBERJACKING, SKILL_LOCK_LOCKED);
        assert_eq!(packet, SKILL_LOCK_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
        assert_eq!(
            skill_lock(SKILL_LUMBERJACKING, SKILL_LOCK_UP)[5],
            SKILL_LOCK_UP
        );
        assert_eq!(
            skill_lock(SKILL_LUMBERJACKING, SKILL_LOCK_DOWN)[5],
            SKILL_LOCK_DOWN
        );
    }

    #[test]
    fn trade_cancel_is_byte_exact() {
        const TRADE_CANCEL_PACKET: [u8; 8] = [
            PKT_SECURE_TRADE,
            0x00,
            0x08,
            TRADE_CLOSE,
            0x40,
            0x00,
            0x00,
            0x01,
        ];
        let packet = trade_cancel(TRADE_CONTAINER);
        assert_eq!(packet, TRADE_CANCEL_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn trade_accept_is_byte_exact() {
        const TRADE_ACCEPT_PACKET: [u8; 12] = [
            PKT_SECURE_TRADE,
            0x00,
            0x0C,
            TRADE_UPDATE,
            0x40,
            0x00,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x01,
        ];
        let packet = trade_accept(TRADE_CONTAINER, true);
        assert_eq!(packet, TRADE_ACCEPT_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
        assert_eq!(trade_accept(TRADE_CONTAINER, false)[11], 0);
    }

    #[test]
    fn trade_gold_is_byte_exact() {
        const TRADE_GOLD_PACKET: [u8; 16] = [
            PKT_SECURE_TRADE,
            0x00,
            0x10,
            TRADE_UPDATE_GOLD,
            0x40,
            0x00,
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x64,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        let packet = trade_gold(TRADE_CONTAINER, 100, 0);
        assert_eq!(packet, TRADE_GOLD_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn book_page_request_is_byte_exact() {
        const PAGE_REQUEST_PACKET: [u8; 13] = [
            PKT_BOOK_CONTENT,
            0x00,
            0x0D,
            0x40,
            0x00,
            0x00,
            0xCC,
            0x00,
            0x01,
            0x00,
            0x01,
            0xFF,
            0xFF,
        ];
        let packet = book_page_request(BOOK, 1);
        assert_eq!(packet, PAGE_REQUEST_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn book_page_is_byte_exact() {
        const BOOK_PAGE_PACKET: [u8; 17] = [
            PKT_BOOK_CONTENT,
            0x00,
            0x11,
            0x40,
            0x00,
            0x00,
            0xCC,
            0x00,
            0x01,
            0x00,
            0x01,
            0x00,
            0x01,
            b'h',
            b'i',
            0x00,
            0x00,
        ];
        let packet = book_page(BOOK, 1, &["hi"]);
        assert_eq!(packet, BOOK_PAGE_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }

    /// A line break inside a line would end the line early on the wire, so it
    /// is taken out, as the reference client takes it out. An empty line is one
    /// NUL.
    #[test]
    fn book_page_takes_out_line_breaks_and_stops_at_the_page_limit() {
        let packet = book_page(BOOK, 1, &["a\nb", ""]);
        assert_eq!(&packet[13..], &[b'a', b'b', 0x00, 0x00, 0x00]);
        assert_eq!(framed_len(&packet), packet.len());

        let long: Vec<&str> = std::iter::repeat_n("x", BOOK_PAGE_LINE_MAX + 3).collect();
        let packet = book_page(BOOK, 1, &long);
        assert_eq!(
            u16::from_be_bytes([packet[11], packet[12]]) as usize,
            BOOK_PAGE_LINE_MAX
        );
        assert_eq!(framed_len(&packet), packet.len());
    }

    #[test]
    fn book_header_is_byte_exact() {
        const BOOK_HEADER_PACKET: [u8; 22] = [
            PKT_BOOK_HEADER,
            0x00,
            0x16,
            0x40,
            0x00,
            0x00,
            0xCC,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x04,
            b'T',
            b'a',
            b'l',
            b'e',
            0x00,
            0x03,
            b'B',
            b'o',
            b'b',
        ];
        let packet = book_header(BOOK, "Tale", "Bob");
        assert_eq!(packet, BOOK_HEADER_PACKET);
        assert_eq!(framed_len(&packet), packet.len());
    }
}
