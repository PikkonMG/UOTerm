use crate::buf::PacketWriter;
use crate::decode::{PromptRequest, TextEntryDialog};
use crate::types::*;

/// A framed packet, or nothing when it would not fit its length word. Only
/// text far past what any shard reads can make a packet that long, and such
/// a packet cannot go on the wire, so it is dropped here rather than stop the
/// session.
fn var_bytes(w: PacketWriter) -> Vec<u8> {
    w.finish_variable().unwrap_or_else(|error| {
        tracing::warn!(%error, "a packet too long to send was dropped");
        Vec::new()
    })
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

/// Play the character in `slot`. `client_flags` are the expansion bits, which
/// [`ClientVersion::expansion_flags`] builds. They tell the shard which maps
/// and rules the client has, and their clear 3D bits tell it the session is
/// a Classic Client.
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

/// `0xBF` `0x05`: the width and the height in pixels of the game view.
pub fn game_window_size(width: u32, height: u32) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_GAME_WINDOW_SIZE).u32(width).u32(height);
    var_bytes(w)
}

/// `0xBF` `0x0B`: the language the client speaks, three letters and a zero.
pub fn language(code: [u8; LANGUAGE_LEN]) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_LANGUAGE).bytes(&code);
    var_bytes(w)
}

/// `0xBF` `0x0F`: the client type, with the flags of this version.
pub fn client_type(version: ClientVersion) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_CLIENT_TYPE)
        .u8(CLIENT_TYPE_MARK)
        .u32(version.client_type_flags());
    var_bytes(w)
}

/// `0xC8`: the view range the client asks for, inside the range a client
/// may ask for.
pub fn view_range(tiles: u8) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_VIEW_RANGE);
    w.u8(tiles.clamp(CLIENT_VIEW_RANGE_MIN, CLIENT_VIEW_RANGE_MAX));
    w.finish()
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
    if !nibbles.len().is_multiple_of(NIBBLES_PER_BYTE) {
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

/// The mobile query (`0x34`) asks for the status of a mobile.
const QUERY_STATUS: u8 = 4;

/// The Classic Client mobile query (`0x34`) for the character's skills. The
/// shard answers with the full skill list (`0x3A`).
pub fn query_skills(me: Serial) -> Vec<u8> {
    query(QUERY_SKILLS, me)
}

/// The Classic Client mobile query (`0x34`) for the status of a mobile, as a
/// click on its health bar asks. The shard answers with its hits (`0x11`),
/// and with every stat when it is the character or a pet he owns.
pub fn query_status(mobile: Serial) -> Vec<u8> {
    query(QUERY_STATUS, mobile)
}

fn query(kind: u8, serial: Serial) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_QUERY);
    w.u32(QUERY_PATTERN).u8(kind).serial(serial);
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

/// Casts a spell by its number with the `0x12` text command, the form of
/// clients older than 6.0.14.2.
pub fn cast_spell(spell_id: u16) -> Vec<u8> {
    text_command(TEXT_CMD_CAST_SPELL, &spell_id.to_string())
}

/// `0xBF` `0x1C`: casts a spell by its number with no spellbook named, the
/// form of clients from 6.0.14.2.
pub fn cast_spell_extended(spell_id: u16) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_CAST_SPELL).u16(CAST_WITHOUT_BOOK).u16(spell_id);
    var_bytes(w)
}

/// Casts a spell in the form the client version sends.
pub fn cast(spell_id: u16, version: ClientVersion) -> Vec<u8> {
    if version.has_extended_cast() {
        cast_spell_extended(spell_id)
    } else {
        cast_spell(spell_id)
    }
}

/// `0x12` `0x27`: casts a spell from one spellbook, as a click on the spell
/// in an open book does. The book goes as its serial in decimal.
pub fn cast_from_book(spell_id: u16, book: Serial) -> Vec<u8> {
    text_command(TEXT_CMD_CAST_FROM_BOOK, &format!("{spell_id} {}", book.0))
}

/// `0x12` `0x43`: opens the spellbook of one kind, see the `SPELLBOOK_*`
/// kinds. Both server families read the kind as a number in text, so it
/// goes as text.
pub fn open_spellbook(kind: u8) -> Vec<u8> {
    text_command(TEXT_CMD_OPEN_SPELLBOOK, &kind.to_string())
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
        .i16(i16::from(z))
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
        .i16(0)
        .u16(0);
    w.finish()
}

/// `texts` is what the player typed in each text field: the field id and
/// the words. A field holds at most [`GUMP_TEXT_MAX_CHARS`] characters.
/// The longest text a gump field sends.
pub const GUMP_TEXT_MAX_CHARS: usize = 239;

pub fn gump_response(
    serial: Serial,
    gump_id: u32,
    button: u32,
    switches: &[u32],
    texts: &[(u16, String)],
) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_GUMP_RESPONSE);
    w.serial(serial)
        .u32(gump_id)
        .u32(button)
        .u32(switches.len() as u32);
    for s in switches {
        w.u32(*s);
    }
    w.u32(texts.len() as u32);
    for (id, words) in texts {
        let units: Vec<u16> = words.encode_utf16().take(GUMP_TEXT_MAX_CHARS).collect();
        w.u16(*id).u16(units.len() as u16);
        for unit in units {
            w.u16(unit);
        }
    }
    var_bytes(w)
}

pub fn gump_close(serial: Serial, gump_id: u32) -> Vec<u8> {
    gump_response(serial, gump_id, 0, &[], &[])
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

/// `0xBF` `0x1E`: ask for the design of a custom house, which the shard
/// sends as `0xD8`.
pub fn house_design_request(house: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_HOUSE_DESIGN_REQUEST).serial(house);
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

/// `0xD7` `0x1E`: asks the shard to wear the last weapon the character held.
pub fn equip_last_weapon(player: Serial) -> Vec<u8> {
    encoded_button(player, ENCODED_EQUIP_LAST_WEAPON, ENCODED_END)
}

/// `0x98`: asks for the name of a mobile. The shard answers with `0x98`.
pub fn name_request(mobile: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_UPDATE_NAME);
    w.serial(mobile);
    var_bytes(w)
}

/// The way a `0xA7` turns the tips: back or on.
const TIP_PREVIOUS: u8 = 0;
const TIP_NEXT: u8 = 1;

/// `0xA7`: asks for the tip after, or before, the tip with this number.
pub fn tip_request(tip: u16, next: bool) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_TIP_REQUEST);
    w.u16(tip).u8(if next { TIP_NEXT } else { TIP_PREVIOUS });
    w.finish()
}

/// `0xBF` `0x07`: the player clicked the quest arrow, with the right button
/// or the left.
pub fn quest_arrow_click(right_button: bool) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_QUEST_ARROW_CLICK).u8(u8::from(right_button));
    var_bytes(w)
}

/// The looks a character picks in the race change window. A style of zero
/// is none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NewLooks {
    pub skin_hue: u16,
    pub hair: u16,
    pub hair_hue: u16,
    pub beard: u16,
    pub beard_hue: u16,
}

/// `0xBF` `0x2A`: the answer to the race change of the shard. With looks
/// the character takes the race; with none he says no, and the packet ends
/// after the sub-command, as the shard reads a refusal.
pub fn race_change_answer(looks: Option<NewLooks>) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_RACE_CHANGE);
    if let Some(looks) = looks {
        w.u16(looks.skin_hue)
            .u16(looks.hair)
            .u16(looks.hair_hue)
            .u16(looks.beard)
            .u16(looks.beard_hue);
    }
    var_bytes(w)
}

/// `0xBF` `0x0C`: the client shut the status bar of this mobile, so the
/// shard may stop sending its hits.
pub fn close_status_bar(mobile: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_CLOSE_STATUS_BAR).serial(mobile);
    var_bytes(w)
}

/// `0xBF` `0x33`: steers the boat the character pilots, in a direction at
/// one of the `BOAT_SPEED_*` speeds. The direction goes twice, as the
/// reference client writes it; a shard reads the first.
pub fn boat_move(player: Serial, direction: Direction, speed: u8) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_BOAT_MOVE)
        .serial(player)
        .u8(direction as u8)
        .u8(direction as u8)
        .u8(speed);
    var_bytes(w)
}

/// `0xBF` `0x10`: asks for the property list of one object, the way clients
/// older than 5.0.9.0 ask. The shard answers with a `0xD6` list.
pub fn query_properties_old(serial: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_EXTENDED);
    w.u16(EXT_QUERY_PROPERTIES).serial(serial);
    var_bytes(w)
}

/// `0xF0` `0x00`: asks where the party members out of sight stand.
pub fn query_party_positions() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASSISTANT);
    w.u8(ASSIST_CMD_QUERY_PARTY);
    var_bytes(w)
}

/// The guild query asks for the places, not the names alone.
const GUILD_QUERY_WITH_PLACES: u8 = 1;

/// `0xF0` `0x01`: asks where the guild members out of sight stand.
pub fn query_guild_positions() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASSISTANT);
    w.u8(ASSIST_CMD_QUERY_GUILD).u8(GUILD_QUERY_WITH_PLACES);
    var_bytes(w)
}

/// `0xFB`: whether the client shows what stands inside public houses.
pub fn public_house_content(show: bool) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_PUBLIC_HOUSE_CONTENT);
    w.u8(u8::from(show));
    w.finish()
}

/// The fields of a `0x3F` hash answer: six bytes nobody reads, then the
/// command byte of a hash query.
const LIVE_ANSWER_SPARE: usize = 6;
const LIVE_HASH_COMMAND: u8 = 0xFF;
/// A hash answer holds the checksums of the 5 by 5 blocks around one block.
pub const LIVE_HASH_COUNT: usize = 25;

/// `0x3F`: the checksums of the 5 by 5 blocks around `block`, column by
/// column, answering an UltimaLive hash query.
pub fn ultima_live_hashes(block: u32, map: u8, hashes: &[u16; LIVE_HASH_COUNT]) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ULTIMA_LIVE);
    w.u32(block)
        .pad(LIVE_ANSWER_SPARE)
        .u8(LIVE_HASH_COMMAND)
        .u8(map);
    for hash in hashes {
        w.u16(*hash);
    }
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

/// The first two bytes of a `0x93` cover change, as the reference client
/// writes them.
const BOOK_HEADER_OLD_MARK: [u8; 2] = [0, 1];

/// The old book header change (`0x93`): rename a book and its author at the
/// fixed widths, for a shard that opened the book with a `0x93` cover.
pub fn book_header_old(serial: Serial, title: &str, author: &str) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_BOOK_HEADER_OLD);
    w.serial(serial)
        .bytes(&BOOK_HEADER_OLD_MARK)
        .u16(0)
        .ascii_fixed(title, BOOK_TITLE_LEN)
        .ascii_fixed(author, BOOK_AUTHOR_LEN);
    w.finish()
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

const BULLETIN_ASK_MESSAGE: u8 = 3;
const BULLETIN_ASK_SUMMARY: u8 = 4;
const BULLETIN_POST: u8 = 5;
const BULLETIN_REMOVE: u8 = 6;
/// The length byte of a line counts the zero at its end.
const BULLETIN_LINE_MAX_BYTES: usize = u8::MAX as usize - 1;

fn bulletin(kind: u8, board: Serial, message: Serial) -> PacketWriter {
    let mut w = PacketWriter::with_variable(PKT_BULLETIN_BOARD);
    w.u8(kind).serial(board).serial(message);
    w
}

/// `0x71`: ask for one message of a bulletin board. `full` asks for its
/// lines; otherwise the shard sends the one line for the list.
pub fn bulletin_ask(board: Serial, message: Serial, full: bool) -> Vec<u8> {
    let kind = if full {
        BULLETIN_ASK_MESSAGE
    } else {
        BULLETIN_ASK_SUMMARY
    };
    var_bytes(bulletin(kind, board, message))
}

/// Words with their length in one byte before them and a zero at their
/// end. Words that are too long are cut at a whole character.
fn counted_words(w: &mut PacketWriter, words: &str) {
    let mut end = words.len().min(BULLETIN_LINE_MAX_BYTES);
    while !words.is_char_boundary(end) {
        end -= 1;
    }
    w.u8(end as u8 + 1).bytes(&words.as_bytes()[..end]).u8(0);
}

/// `0x71`: post a message. `reply_to` is the message it answers, or zero
/// for a new one.
pub fn bulletin_post(board: Serial, reply_to: Serial, subject: &str, lines: &[&str]) -> Vec<u8> {
    let mut w = bulletin(BULLETIN_POST, board, reply_to);
    counted_words(&mut w, subject);
    let lines = &lines[..lines.len().min(usize::from(u8::MAX))];
    w.u8(lines.len() as u8);
    for line in lines {
        counted_words(&mut w, line);
    }
    var_bytes(w)
}

/// `0x71`: remove a message the character posted.
pub fn bulletin_remove(board: Serial, message: Serial) -> Vec<u8> {
    var_bytes(bulletin(BULLETIN_REMOVE, board, message))
}

/// The `0x56` actions a client sends about an open map.
const MAP_ADD_PIN: u8 = 1;
const MAP_MOVE_PIN: u8 = 3;
const MAP_REMOVE_PIN: u8 = 4;
const MAP_CLEAR_PINS: u8 = 5;
const MAP_TOGGLE_EDIT: u8 = 6;
/// The pin byte of an action that names no pin.
const NO_PIN: u8 = 0;

fn map_message(serial: Serial, action: u8, pin: u8, x: u16, y: u16) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_MAP_MESSAGE);
    w.serial(serial).u8(action).u8(pin).u16(x).u16(y);
    w.finish()
}

/// `0x56`: put a pin on an open map. `x` and `y` are pixels of the map
/// picture, not tiles of the world.
pub fn map_add_pin(serial: Serial, x: u16, y: u16) -> Vec<u8> {
    map_message(serial, MAP_ADD_PIN, NO_PIN, x, y)
}

/// `0x56`: move one pin of an open map, by its place in the list from 0,
/// to pixels of the map picture.
pub fn map_move_pin(serial: Serial, pin: u8, x: u16, y: u16) -> Vec<u8> {
    map_message(serial, MAP_MOVE_PIN, pin, x, y)
}

/// `0x56`: take one pin off an open map, by its place in the list from 0.
pub fn map_remove_pin(serial: Serial, pin: u8) -> Vec<u8> {
    map_message(serial, MAP_REMOVE_PIN, pin, 0, 0)
}

/// `0x56`: take every pin off an open map.
pub fn map_clear_pins(serial: Serial) -> Vec<u8> {
    map_message(serial, MAP_CLEAR_PINS, NO_PIN, 0, 0)
}

/// `0x56`: ask to draw on the map, or to stop drawing on it. The shard
/// answers with the state it allows.
pub fn map_toggle_edit(serial: Serial) -> Vec<u8> {
    map_message(serial, MAP_TOGGLE_EDIT, NO_PIN, 0, 0)
}

const PROFILE_READ: u8 = 0;
const PROFILE_WRITE: u8 = 1;
/// The word before the words of a profile a player writes.
const PROFILE_WRITE_MARK: u16 = 1;

/// `0xB8`: ask for the profile a player wrote about his character.
pub fn profile_request(serial: Serial) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_PROFILE);
    w.u8(PROFILE_READ).serial(serial);
    var_bytes(w)
}

/// `0xB8`: write the profile of the character. Only his own profile.
pub fn profile_write(serial: Serial, words: &str) -> Vec<u8> {
    let units: Vec<u16> = words.encode_utf16().collect();
    let mut w = PacketWriter::with_variable(PKT_PROFILE);
    w.u8(PROFILE_WRITE)
        .serial(serial)
        .u16(PROFILE_WRITE_MARK)
        .u16(units.len() as u16);
    for unit in units {
        w.u16(unit);
    }
    var_bytes(w)
}

/// The `0xD7` commands that design a house. The reference client sends
/// each one as the serial of the player, the command, its numbers, and a
/// `0x0A` at the end.
const HOUSE_BACKUP: u16 = 0x02;
const HOUSE_RESTORE: u16 = 0x03;
const HOUSE_COMMIT: u16 = 0x04;
const HOUSE_REMOVE: u16 = 0x05;
const HOUSE_ADD: u16 = 0x06;
const HOUSE_EXIT: u16 = 0x0C;
const HOUSE_SYNC: u16 = 0x0E;
const HOUSE_ADD_STAIR: u16 = 0x0D;
const HOUSE_CLEAR: u16 = 0x10;
const HOUSE_GO_TO_FLOOR: u16 = 0x12;
const HOUSE_ADD_ROOF: u16 = 0x13;
const HOUSE_REMOVE_ROOF: u16 = 0x14;
const HOUSE_REVERT: u16 = 0x1A;
/// Each number of a command comes after a zero byte.
const AOS_NUMBER_MARK: u8 = 0x00;
const AOS_END: u8 = 0x0A;

/// What a designer command does to a house.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseEdit {
    /// Puts a wall, a floor, a door or an archway on a tile of the house.
    Add {
        graphic: u16,
        x: i32,
        y: i32,
    },
    Remove {
        graphic: u16,
        x: i32,
        y: i32,
        z: i32,
    },
    AddStair {
        graphic: u16,
        x: i32,
        y: i32,
    },
    AddRoof {
        graphic: u16,
        x: i32,
        y: i32,
        z: i32,
    },
    RemoveRoof {
        graphic: u16,
        x: i32,
        y: i32,
        z: i32,
    },
    /// The level the designer works on, from 1.
    GoToFloor(u8),
    /// Takes every part off the house.
    Clear,
    /// Keeps the design as it was when the designing began.
    Revert,
    /// Saves the design, and the shard charges for it.
    Commit,
    /// Leaves the designer with no change.
    Exit,
    Backup,
    Restore,
    /// Asks the shard to send the design being worked on again.
    Sync,
}

fn aos_command(player: Serial, command: u16) -> PacketWriter {
    let mut w = PacketWriter::with_variable(PKT_AOS_COMMAND);
    w.serial(player).u16(command);
    w
}

fn aos_number(w: &mut PacketWriter, number: i32) {
    w.u8(AOS_NUMBER_MARK).u32(number as u32);
}

/// `0xD7`: one command of the house designer.
pub fn house_edit(player: Serial, edit: HouseEdit) -> Vec<u8> {
    let (command, numbers): (u16, Vec<i32>) = match edit {
        HouseEdit::Add { graphic, x, y } => (HOUSE_ADD, vec![i32::from(graphic), x, y]),
        HouseEdit::AddStair { graphic, x, y } => (HOUSE_ADD_STAIR, vec![i32::from(graphic), x, y]),
        HouseEdit::Remove { graphic, x, y, z } => (HOUSE_REMOVE, vec![i32::from(graphic), x, y, z]),
        HouseEdit::AddRoof { graphic, x, y, z } => {
            (HOUSE_ADD_ROOF, vec![i32::from(graphic), x, y, z])
        }
        HouseEdit::RemoveRoof { graphic, x, y, z } => {
            (HOUSE_REMOVE_ROOF, vec![i32::from(graphic), x, y, z])
        }
        HouseEdit::GoToFloor(floor) => (HOUSE_GO_TO_FLOOR, vec![0, i32::from(floor)]),
        HouseEdit::Clear => (HOUSE_CLEAR, Vec::new()),
        HouseEdit::Revert => (HOUSE_REVERT, Vec::new()),
        HouseEdit::Commit => (HOUSE_COMMIT, Vec::new()),
        HouseEdit::Exit => (HOUSE_EXIT, Vec::new()),
        HouseEdit::Backup => (HOUSE_BACKUP, Vec::new()),
        HouseEdit::Restore => (HOUSE_RESTORE, Vec::new()),
        HouseEdit::Sync => (HOUSE_SYNC, Vec::new()),
    };
    let mut w = aos_command(player, command);
    // The floor command writes its numbers with no mark before the last one.
    if let HouseEdit::GoToFloor(floor) = edit {
        w.u32(0).u8(floor);
    } else {
        for number in numbers {
            aos_number(&mut w, number);
        }
    }
    w.u8(AOS_END);
    var_bytes(w)
}

/// `0x9B`: ask the shard for help. The shard answers with its help menu.
pub fn help_request() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_HELP_REQUEST);
    w.bytes(&[0; HELP_REQUEST_BYTES]);
    w.finish()
}

/// The empty block a help request carries.
const HELP_REQUEST_BYTES: usize = 257;

const CHAT_JOIN: u16 = 0x62;
const CHAT_SAY: u16 = 0x61;
const CHAT_LEAVE: u16 = 0x43;
const CHAT_CREATE: u16 = 0x63;
/// The marks a password of a new channel is written between.
const CHAT_PASSWORD_OPEN: u16 = 0x7B;
const CHAT_PASSWORD_CLOSE: u16 = 0x7D;
/// The quote mark that a channel name is written between.
const CHAT_QUOTE: u16 = 0x22;
/// The space between a channel name and its password.
const CHAT_SPACE: u16 = 0x20;
/// The language the client says it speaks.
const CHAT_LANGUAGE: &str = "ENU";

fn chat_command(command: u16) -> PacketWriter {
    let mut w = PacketWriter::with_variable(PKT_CHAT_COMMAND);
    w.ascii_fixed(CHAT_LANGUAGE, 4).u16(command);
    w
}

fn utf16be(w: &mut PacketWriter, text: &str) {
    for unit in text.encode_utf16() {
        w.u16(unit);
    }
}

/// `0xB3`: join a chat channel, with its password when it has one.
pub fn chat_join(channel: &str, password: Option<&str>) -> Vec<u8> {
    let mut w = chat_command(CHAT_JOIN);
    w.u16(CHAT_QUOTE);
    utf16be(&mut w, channel);
    w.u16(CHAT_QUOTE);
    w.u16(CHAT_SPACE);
    if let Some(password) = password {
        utf16be(&mut w, password);
    }
    w.u16(0);
    var_bytes(w)
}

/// `0xB3`: say words in the chat channel the character is in.
pub fn chat_say(words: &str) -> Vec<u8> {
    let mut w = chat_command(CHAT_SAY);
    utf16be(&mut w, words);
    w.u16(0);
    var_bytes(w)
}

/// `0xB3`: make a chat channel and join it. A password goes between braces
/// after the name, as the reference client writes it.
pub fn chat_create(channel: &str, password: Option<&str>) -> Vec<u8> {
    let mut w = chat_command(CHAT_CREATE);
    utf16be(&mut w, channel);
    w.u16(0);
    if let Some(password) = password {
        w.u16(CHAT_PASSWORD_OPEN);
        utf16be(&mut w, password);
        w.u16(0);
        w.u16(CHAT_PASSWORD_CLOSE);
    }
    var_bytes(w)
}

/// `0xB3`: leave the chat channel.
pub fn chat_leave() -> Vec<u8> {
    var_bytes(chat_command(CHAT_LEAVE))
}

/// `0xB5`: open the chat of the shard under this name. The packet has a
/// fixed size, and both server families read it so: a zero, the name, and
/// zeros to the end.
pub fn chat_open(name: &str) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_OPEN_CHAT);
    w.u8(0);
    for unit in name.encode_utf16().take(CHAT_NAME_MAX_UNITS) {
        w.u16(unit);
    }
    let mut packet = w.finish();
    packet.resize(OPEN_CHAT_LEN, 0);
    packet
}

/// The most UTF-16 units of a name the chat takes.
const CHAT_NAME_MAX_UNITS: usize = 30;
/// The fixed size of `0xB5`.
const OPEN_CHAT_LEN: usize = 0x40;

/// The bytes a delete request pads before the slot.
const DELETE_PAD: usize = 30;
/// The fixed width of a name in a character request.
const CHARACTER_NAME_LEN: usize = 30;
/// The two words a create request starts with. A shard reads them to know
/// the request is whole.
const CREATE_PATTERN: u32 = 0xEDED_EDED;
const CREATE_PATTERN_END: u32 = 0xFFFF_FFFF;
/// The block of zeroes between the profession and the sex.
const CREATE_SPARE: usize = 15;
/// From 7.0.16.0 a new character starts with four skills; before it, three.
const CREATE_SKILLS_NEW: usize = 4;
const CREATE_SKILLS_OLD: usize = 3;
const CREATE_MARK: u32 = 0x01;
/// From 4.0.11d the race and the sex share one byte; before it the byte is
/// the sex alone.
const CREATE_RACE_BYTE: ClientVersion = ClientVersion::new(4, 0, 11, b'd' as u32);
/// From 7.0.0.0 the race in that byte counts from one, so a human is two or
/// three; before it a human is zero or one.
const CREATE_RACE_FROM_ONE: ClientVersion = ClientVersion::new(7, 0, 0, 0);

/// `0x83`: delete the character in this slot of the account.
pub fn delete_character(slot: u32) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_DELETE_CHARACTER);
    w.bytes(&[0; DELETE_PAD]).u32(slot).u32(0);
    w.finish()
}

/// What a player picks for a new character. The rest of the parts a shard
/// checks are written for him.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewCharacter<'a> {
    pub name: &'a str,
    pub female: bool,
    /// 0 human, 1 elf, 2 gargoyle.
    pub race: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    /// Each starting skill and how high it starts.
    pub skills: Vec<(u8, u8)>,
    pub skin_hue: u16,
    pub hair: u16,
    pub hair_hue: u16,
    /// Zero is no beard.
    pub beard: u16,
    pub beard_hue: u16,
    /// The hues of the shirt and the trousers he starts in.
    pub shirt_hue: u16,
    pub pants_hue: u16,
    /// The number of the profession of the client's list, or zero when the
    /// numbers above say what he is.
    pub profession: u8,
    /// The town he starts in, and the slot he takes.
    pub start_city: u16,
    pub slot: u16,
}

/// `0x00`, or `0xF8` for a newer client: make a new character. The shard
/// answers with the list of characters, or with `0x85` and a reason.
pub fn create_character(new: &NewCharacter<'_>, version: ClientVersion) -> Vec<u8> {
    let newer = version.has_three_starting_skills();
    let id = if newer {
        PKT_CREATE_CHARACTER_NEW
    } else {
        PKT_CREATE_CHARACTER
    };
    let skills = if newer {
        CREATE_SKILLS_NEW
    } else {
        CREATE_SKILLS_OLD
    };
    let mut w = PacketWriter::new(id);
    w.u32(CREATE_PATTERN)
        .u32(CREATE_PATTERN_END)
        .u8(0)
        .ascii_fixed(new.name, CHARACTER_NAME_LEN)
        .u16(0)
        .u32(version.expansion_flags())
        .u32(CREATE_MARK)
        .u32(0)
        .u8(new.profession)
        .bytes(&[0; CREATE_SPARE])
        .u8(race_and_sex(new, version))
        .u8(new.strength)
        .u8(new.dexterity)
        .u8(new.intelligence);
    for place in 0..skills {
        let (skill, value) = new.skills.get(place).copied().unwrap_or((0, 0));
        w.u8(skill).u8(value);
    }
    w.u16(new.skin_hue)
        .u16(new.hair)
        .u16(new.hair_hue)
        .u16(new.beard)
        .u16(new.beard_hue)
        .u16(new.start_city)
        .u16(0)
        .u16(new.slot)
        .u32(0)
        .u16(new.shirt_hue)
        .u16(new.pants_hue);
    w.finish()
}

/// The byte that says the race and the sex of a new character, in the form
/// the version writes it.
fn race_and_sex(new: &NewCharacter<'_>, version: ClientVersion) -> u8 {
    let female = u8::from(new.female);
    if !version.at_least(CREATE_RACE_BYTE) {
        return female;
    }
    let race = if version.at_least(CREATE_RACE_FROM_ONE) {
        new.race + 1
    } else {
        new.race
    };
    race * 2 + female
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
    fn a_status_query_and_a_design_request_are_byte_exact() {
        const PET: Serial = Serial(0x0000_0F01);
        const HOUSE: Serial = Serial(0x4000_0F02);
        let status = query_status(PET);
        assert_eq!(status[0], PKT_QUERY);
        assert_eq!(&status[1..5], &QUERY_PATTERN.to_be_bytes());
        assert_eq!(status[5], QUERY_STATUS);
        assert_eq!(&status[6..10], &PET.0.to_be_bytes());
        let design = house_design_request(HOUSE);
        assert_eq!(design[0], PKT_EXTENDED);
        assert_eq!(
            usize::from(u16::from_be_bytes([design[1], design[2]])),
            design.len()
        );
        assert_eq!(&design[3..5], &EXT_HOUSE_DESIGN_REQUEST.to_be_bytes());
        assert_eq!(&design[5..9], &HOUSE.0.to_be_bytes());
    }

    /// Words far past any shard's limit make no packet, and do not stop the
    /// session.
    #[test]
    fn a_packet_too_long_for_its_length_word_is_dropped() {
        let words = "a".repeat(usize::from(u16::MAX));
        assert!(chat_say(&words).is_empty());
        assert!(!chat_say("hail").is_empty());
    }

    #[test]
    fn making_and_deleting_a_character_is_byte_exact() {
        let gone = delete_character(2);
        assert_eq!(gone[0], PKT_DELETE_CHARACTER);
        assert_eq!(
            &gone[1 + DELETE_PAD..1 + DELETE_PAD + 4],
            &2u32.to_be_bytes()
        );
        let new = NewCharacter {
            name: "Mara",
            female: true,
            race: 0,
            strength: 45,
            dexterity: 35,
            intelligence: 10,
            skills: vec![(45, 25), (30, 25)],
            skin_hue: 0x83EA,
            hair: 0x203B,
            hair_hue: 0x044E,
            beard: 0x2040,
            beard_hue: 0x044E,
            shirt_hue: 0x0003,
            pants_hue: 0x0008,
            profession: 1,
            start_city: 0,
            slot: 1,
        };
        let old_client = ClientVersion {
            major: 6,
            minor: 0,
            revision: 0,
            patch: 0,
        };
        let new_client = ClientVersion {
            major: 7,
            minor: 0,
            revision: 16,
            patch: 0,
        };
        let made = create_character(&new, old_client);
        assert_eq!(made[0], PKT_CREATE_CHARACTER);
        assert_eq!(&made[1..5], &CREATE_PATTERN.to_be_bytes());
        // id, two patterns and one zero byte come before the name.
        assert_eq!(&made[10..14], b"Mara");
        // A newer client sends its own packet, with room for one more skill.
        let newer = create_character(&new, new_client);
        assert_eq!(newer[0], PKT_CREATE_CHARACTER_NEW);
        assert_eq!(newer.len(), made.len() + 2);
        // The profession sits before the spare bytes, and the beard, the
        // shirt and the trousers after the hair and the slot.
        assert_eq!(made[RACE_BYTE_AT - CREATE_SPARE - 1], new.profession);
        let word = |bytes: &[u8], at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
        let hair_at = RACE_BYTE_AT + 4 + CREATE_SKILLS_OLD * 2 + 2;
        assert_eq!(word(&made, hair_at + 4), new.beard);
        assert_eq!(word(&made, hair_at + 6), new.beard_hue);
        assert_eq!(word(&made, made.len() - 4), new.shirt_hue);
        assert_eq!(word(&made, made.len() - 2), new.pants_hue);
    }

    /// The offset of the race and sex byte: id, two patterns, a zero byte,
    /// the name, two spare bytes, three words, the profession and fifteen
    /// spare bytes.
    const RACE_BYTE_AT: usize = 1 + 4 + 4 + 1 + CHARACTER_NAME_LEN + 2 + 12 + 1 + CREATE_SPARE;
    const CLIENT_FLAG_AT: usize = 1 + 4 + 4 + 1 + CHARACTER_NAME_LEN + 2;

    /// A shard reads the race from that byte by the version, so an elf must
    /// arrive as an elf on every version, and the flags must be the ones the
    /// version reports everywhere else.
    #[test]
    fn a_new_character_says_its_race_and_flags_as_its_version_does() {
        const ELF: u8 = 1;
        let elf = NewCharacter {
            name: "Lyra",
            female: true,
            race: ELF,
            strength: 45,
            dexterity: 35,
            intelligence: 10,
            skills: vec![(45, 25), (30, 25)],
            skin_hue: 0,
            hair: 0,
            hair_hue: 0,
            beard: 0,
            beard_hue: 0,
            shirt_hue: 0,
            pants_hue: 0,
            profession: 0,
            start_city: 0,
            slot: 0,
        };
        let cases = [
            (ClientVersion::new(4, 0, 0, 0), 1),
            (ClientVersion::new(6, 0, 1, 7), ELF * 2 + 1),
            (ClientVersion::new(7, 0, 16, 0), (ELF + 1) * 2 + 1),
        ];
        for (version, byte) in cases {
            let made = create_character(&elf, version);
            assert_eq!(made[RACE_BYTE_AT], byte, "{version}");
            assert_eq!(
                &made[CLIENT_FLAG_AT..CLIENT_FLAG_AT + 4],
                &version.expansion_flags().to_be_bytes(),
                "{version}"
            );
        }
    }

    /// The lengths a shard of the ModernUO family registers for `0x00` and
    /// `0xF8`. It drops a request of any other length.
    const SHARD_CREATE_LEN_OLD: usize = 104;
    const SHARD_CREATE_LEN_NEW: usize = 106;
    /// The client version of the user's client file.
    const CLIENT_7_0_117: ClientVersion = ClientVersion::new(7, 0, 117, 0);

    /// What such a shard reads from a create request, field by field in its
    /// own order, and how many bytes it read.
    #[derive(Debug, PartialEq, Eq)]
    struct ShardRead {
        name: String,
        flags: u32,
        profession: u8,
        race_and_sex: u8,
        stats: [u8; 3],
        skills: Vec<(u8, u8)>,
        hues_and_hair: [u16; 5],
        city: u8,
        shirt_hue: u16,
        pants_hue: u16,
        read: usize,
    }

    fn shard_reads(packet: &[u8], four_skills: bool) -> ShardRead {
        const ID: usize = 1;
        const PATTERNS_AND_ZERO: usize = 9;
        const NAME_SPARE: usize = 2;
        const LOGIN_COUNT_AND_UNKNOWN: usize = 8;
        const PROFESSION_SPARE: usize = 15;
        const CITY_SPARE: usize = 1;
        const SLOT_AND_ADDRESS: usize = 8;
        let mut r = crate::buf::PacketReader::new(packet);
        r.skip(ID + PATTERNS_AND_ZERO).unwrap();
        let name = r.ascii_fixed(CHARACTER_NAME_LEN).unwrap();
        r.skip(NAME_SPARE).unwrap();
        let flags = r.u32().unwrap();
        r.skip(LOGIN_COUNT_AND_UNKNOWN).unwrap();
        let profession = r.u8().unwrap();
        r.skip(PROFESSION_SPARE).unwrap();
        let race_and_sex = r.u8().unwrap();
        let stats = [r.u8().unwrap(), r.u8().unwrap(), r.u8().unwrap()];
        let count = if four_skills {
            CREATE_SKILLS_NEW
        } else {
            CREATE_SKILLS_OLD
        };
        let skills = (0..count)
            .map(|_| (r.u8().unwrap(), r.u8().unwrap()))
            .collect();
        let hues_and_hair = [
            r.u16().unwrap(),
            r.u16().unwrap(),
            r.u16().unwrap(),
            r.u16().unwrap(),
            r.u16().unwrap(),
        ];
        r.skip(CITY_SPARE).unwrap();
        let city = r.u8().unwrap();
        r.skip(SLOT_AND_ADDRESS).unwrap();
        let shirt_hue = r.u16().unwrap();
        let pants_hue = r.u16().unwrap();
        ShardRead {
            name,
            flags,
            profession,
            race_and_sex,
            stats,
            skills,
            hues_and_hair,
            city,
            shirt_hue,
            pants_hue,
            read: packet.len() - r.remaining(),
        }
    }

    /// The race such a shard makes of the byte: from 7.0.0.0 a human is
    /// two or three and each later race one more pair; before it the races
    /// count from zero.
    fn shard_race(race_and_sex: u8, version: ClientVersion) -> u8 {
        const FIRST_RACE_PAIR: u8 = 4;
        const SEXES: u8 = 2;
        if !version.at_least(CREATE_RACE_FROM_ONE) {
            return race_and_sex / SEXES;
        }
        if race_and_sex < FIRST_RACE_PAIR {
            0
        } else {
            race_and_sex / SEXES - 1
        }
    }

    /// The request the user's client of 7.0.117 sends, and the older one,
    /// read as the shard reads them: every field in its place, every byte
    /// read, and each race and sex as it was picked.
    #[test]
    fn a_create_request_reads_back_as_the_shard_reads_it() {
        let mut new = NewCharacter {
            name: "Lyra",
            female: true,
            race: 0,
            strength: 50,
            dexterity: 30,
            intelligence: 10,
            skills: vec![(40, 30), (27, 30), (17, 30), (5, 30)],
            skin_hue: 0x83EA,
            hair: 0x203B,
            hair_hue: 0x044E,
            beard: 0x2040,
            beard_hue: 0x0455,
            shirt_hue: 0x0009,
            pants_hue: 0x0010,
            profession: 2,
            start_city: 3,
            slot: 1,
        };
        let old_client = ClientVersion::new(6, 0, 1, 7);
        for (version, id, len, four_skills) in [
            (
                CLIENT_7_0_117,
                PKT_CREATE_CHARACTER_NEW,
                SHARD_CREATE_LEN_NEW,
                true,
            ),
            (
                old_client,
                PKT_CREATE_CHARACTER,
                SHARD_CREATE_LEN_OLD,
                false,
            ),
        ] {
            let made = create_character(&new, version);
            assert_eq!(made[0], id, "{version}");
            assert_eq!(made.len(), len, "{version}");
            let read = shard_reads(&made, four_skills);
            let count = if four_skills {
                CREATE_SKILLS_NEW
            } else {
                CREATE_SKILLS_OLD
            };
            assert_eq!(
                read,
                ShardRead {
                    name: new.name.into(),
                    flags: version.expansion_flags(),
                    profession: new.profession,
                    race_and_sex: race_and_sex(&new, version),
                    stats: [new.strength, new.dexterity, new.intelligence],
                    skills: new.skills[..count].to_vec(),
                    hues_and_hair: [
                        new.skin_hue,
                        new.hair,
                        new.hair_hue,
                        new.beard,
                        new.beard_hue
                    ],
                    city: new.start_city as u8,
                    shirt_hue: new.shirt_hue,
                    pants_hue: new.pants_hue,
                    read: len,
                },
                "{version}"
            );
            for race in [0, 1] {
                for female in [false, true] {
                    new.race = race;
                    new.female = female;
                    let byte =
                        shard_reads(&create_character(&new, version), four_skills).race_and_sex;
                    assert_eq!(shard_race(byte, version), race, "{version}");
                    assert_eq!(byte % 2 == 1, female, "{version}");
                }
            }
        }
    }

    #[test]
    fn a_house_designer_command_is_byte_exact() {
        const ME: Serial = Serial(0x0000_00AB);
        const WALL: u16 = 10;
        let add = house_edit(
            ME,
            HouseEdit::Add {
                graphic: WALL,
                x: -3,
                y: 4,
            },
        );
        assert_eq!(add[0], PKT_AOS_COMMAND);
        assert_eq!(usize::from(u16::from_be_bytes([add[1], add[2]])), add.len());
        assert_eq!(&add[3..7], &ME.0.to_be_bytes());
        assert_eq!(&add[7..9], &HOUSE_ADD.to_be_bytes());
        assert_eq!(&add[9..14], &[AOS_NUMBER_MARK, 0, 0, 0, WALL as u8]);
        assert_eq!(&add[14..19], &[AOS_NUMBER_MARK, 0xFF, 0xFF, 0xFF, 0xFD]);
        assert_eq!(add[add.len() - 1], AOS_END);
        assert_eq!(
            house_edit(ME, HouseEdit::Commit)[7..9],
            HOUSE_COMMIT.to_be_bytes()
        );
        let floor = house_edit(ME, HouseEdit::GoToFloor(2));
        assert_eq!(&floor[7..9], &HOUSE_GO_TO_FLOOR.to_be_bytes());
        assert_eq!(&floor[9..14], &[0, 0, 0, 0, 2]);
    }

    #[test]
    fn chat_and_help_packets_are_byte_exact() {
        let help = help_request();
        assert_eq!(help.len(), 1 + HELP_REQUEST_BYTES);
        assert_eq!(help[0], PKT_HELP_REQUEST);
        let say = chat_say("hail");
        assert_eq!(say[0], PKT_CHAT_COMMAND);
        assert_eq!(&say[3..7], b"ENU\0");
        assert_eq!(&say[7..9], &CHAT_SAY.to_be_bytes());
        assert_eq!(&say[9..17], &[0, b'h', 0, b'a', 0, b'i', 0, b'l']);
        let join = chat_join("General", None);
        assert_eq!(&join[7..9], &CHAT_JOIN.to_be_bytes());
        assert_eq!(&join[9..13], &[0, 0x22, 0, b'G']);
        // ServUO registers 0x43 as leave and 0x63 as create in its chat
        // action handlers.
        const SERVER_LEAVE: [u8; 2] = [0x00, 0x43];
        const SERVER_CREATE: [u8; 2] = [0x00, 0x63];
        let leave = chat_leave();
        assert_eq!(leave[7..9], SERVER_LEAVE);
        assert_eq!(leave.len(), 9, "leave carries nothing after its command");
        let create = chat_create("Trade", Some("pw"));
        assert_eq!(create[7..9], SERVER_CREATE);
        assert_eq!(
            &create[9..],
            &[
                0, b'T', 0, b'r', 0, b'a', 0, b'd', 0, b'e', 0, 0, 0, b'{', 0, b'p', 0, b'w', 0, 0,
                0, b'}'
            ]
        );
        assert_eq!(framed_len(&create), create.len());
        assert_eq!(
            &chat_create("Trade", None)[9..],
            &[0, b'T', 0, b'r', 0, b'a', 0, b'd', 0, b'e', 0, 0]
        );
        let open = chat_open("Mara");
        assert_eq!(open.len(), OPEN_CHAT_LEN);
        assert_eq!(
            &open[..10],
            &[PKT_OPEN_CHAT, 0, 0, b'M', 0, b'a', 0, b'r', 0, b'a']
        );
        assert!(open[10..].iter().all(|&byte| byte == 0));
        let unnamed = chat_open("");
        assert_eq!(unnamed.len(), OPEN_CHAT_LEN);
        assert_eq!(unnamed[0], PKT_OPEN_CHAT);
        assert!(unnamed[1..].iter().all(|&byte| byte == 0));
        let long = chat_open(&"x".repeat(CHAT_NAME_MAX_UNITS * 2));
        assert_eq!(long.len(), OPEN_CHAT_LEN);
        assert_eq!(long[2 + CHAT_NAME_MAX_UNITS * 2 - 1], b'x');
        assert!(long[2 + CHAT_NAME_MAX_UNITS * 2..].iter().all(|&b| b == 0));
    }

    /// The layouts of what the client says as it enters the world, as the
    /// reference client writes them.
    #[test]
    fn the_login_talk_packets_are_byte_exact() {
        const WIDTH: u32 = 600;
        const HEIGHT: u32 = 480;
        assert_eq!(
            game_window_size(WIDTH, HEIGHT),
            vec![
                PKT_EXTENDED,
                0x00,
                0x0D,
                0x00,
                0x05,
                0,
                0,
                0x02,
                0x58,
                0,
                0,
                0x01,
                0xE0
            ]
        );
        assert_eq!(
            language(LANGUAGE_ENU),
            vec![PKT_EXTENDED, 0x00, 0x09, 0x00, 0x0B, b'E', b'N', b'U', 0]
        );
        assert_eq!(
            client_type(ClientVersion::MODERN),
            vec![
                PKT_EXTENDED,
                0x00,
                0x0A,
                0x00,
                0x0F,
                0x0A,
                0xFF,
                0xFF,
                0xFF,
                0xFF
            ]
        );
        assert_eq!(
            client_type(ClientVersion::T2A),
            vec![PKT_EXTENDED, 0x00, 0x0A, 0x00, 0x0F, 0x0A, 0, 0, 0, 0x01]
        );
        assert_eq!(view_range(18), vec![PKT_VIEW_RANGE, 18]);
        assert_eq!(
            view_range(CLIENT_VIEW_RANGE_MAX + 1),
            vec![PKT_VIEW_RANGE, CLIENT_VIEW_RANGE_MAX]
        );
        assert_eq!(
            view_range(CLIENT_VIEW_RANGE_MIN - 1),
            vec![PKT_VIEW_RANGE, CLIENT_VIEW_RANGE_MIN]
        );
        assert_eq!(
            client_version(ClientVersion::T2A),
            [&[PKT_CLIENT_VERSION, 0x00, 0x0B][..], b"2.0.7.0\0"].concat()
        );
    }

    #[test]
    fn map_and_profile_packets_are_byte_exact() {
        const MAP: Serial = Serial(0x4000_0060);
        let pin = map_add_pin(MAP, 40, 90);
        assert_eq!(pin.len(), 11);
        assert_eq!(
            (pin[0], pin[5], pin[6]),
            (PKT_MAP_MESSAGE, MAP_ADD_PIN, NO_PIN)
        );
        assert_eq!(&pin[7..11], &[0, 40, 0, 90]);
        assert_eq!(map_clear_pins(MAP)[5], MAP_CLEAR_PINS);
        assert_eq!(map_toggle_edit(MAP)[5], MAP_TOGGLE_EDIT);
        let moved = map_move_pin(MAP, 2, 41, 91);
        assert_eq!((moved[5], moved[6]), (MAP_MOVE_PIN, 2));
        assert_eq!(&moved[7..11], &[0, 41, 0, 91]);
        let removed = map_remove_pin(MAP, 1);
        assert_eq!(
            (removed.len(), removed[5], removed[6]),
            (11, MAP_REMOVE_PIN, 1)
        );
        assert_eq!(&removed[7..11], &[0, 0, 0, 0]);
        let ask = profile_request(MAP);
        assert_eq!((ask[0], ask[3]), (PKT_PROFILE, PROFILE_READ));
        assert_eq!(usize::from(u16::from_be_bytes([ask[1], ask[2]])), ask.len());
        let write = profile_write(MAP, "hi");
        assert_eq!(write[3], PROFILE_WRITE);
        assert_eq!(&write[8..], &[0, 1, 0, 2, 0, b'h', 0, b'i']);
    }

    #[test]
    fn bulletin_packets_are_byte_exact() {
        const BOARD: Serial = Serial(0x4000_0050);
        const MESSAGE: Serial = Serial(0x4000_0051);
        let ask = bulletin_ask(BOARD, MESSAGE, true);
        assert_eq!(ask[0], PKT_BULLETIN_BOARD);
        assert_eq!(usize::from(u16::from_be_bytes([ask[1], ask[2]])), ask.len());
        assert_eq!(ask[3], BULLETIN_ASK_MESSAGE);
        assert_eq!(&ask[4..8], &BOARD.0.to_be_bytes());
        assert_eq!(&ask[8..12], &MESSAGE.0.to_be_bytes());
        assert_eq!(bulletin_ask(BOARD, MESSAGE, false)[3], BULLETIN_ASK_SUMMARY);
        assert_eq!(bulletin_remove(BOARD, MESSAGE)[3], BULLETIN_REMOVE);
        let post = bulletin_post(BOARD, Serial(0), "Hi", &["one", "two"]);
        assert_eq!(post[3], BULLETIN_POST);
        assert_eq!(&post[12..16], &[3, b'H', b'i', 0]);
        assert_eq!(post[16], 2, "the count of lines");
        assert_eq!(&post[17..22], &[4, b'o', b'n', b'e', 0]);
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
        assert_eq!(&p[15..17], &i16::from(Z).to_be_bytes());
        assert_eq!(&p[17..19], &ROCK.to_be_bytes());
    }

    #[test]
    fn a_target_below_sea_level_keeps_its_sign() {
        const WATER_Z: i8 = -5;
        let p = target_ground(1, 100, 100, WATER_Z, 0x1797);
        assert_eq!(&p[15..17], &[0xFF, 0xFB], "z is a signed 16-bit word");
    }

    #[test]
    fn target_object_is_19_bytes() {
        let p = target_object(1, Serial(0xAA), 10, 10, 0, 0);
        assert_eq!(p.len(), 19);
        assert_eq!(p[0], PKT_TARGET);
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
    fn play_character_carries_the_classic_expansion_bits() {
        const FLAGS_AT: usize = 1 + 4 + 30 + 2;
        let flags = ClientVersion::MODERN.expansion_flags();
        let p = play_character(0, "Tester", flags, 0);
        let sent = u32::from_be_bytes([
            p[FLAGS_AT],
            p[FLAGS_AT + 1],
            p[FLAGS_AT + 2],
            p[FLAGS_AT + 3],
        ]);
        assert_eq!(sent, flags);
        assert_eq!(sent & CLIENT_FLAG_UO3D, 0, "never the 3D client");
        assert_eq!(sent & CLIENT_FLAG_UOTD_CLIENT, 0, "never the UO:TD client");
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

    /// The spell packets of each client version, as the reference client
    /// and both server families write and read them.
    #[test]
    fn spell_packets_are_byte_exact() {
        const GREATER_HEAL: u16 = 29;
        const SPELLBOOK: Serial = Serial(0x4000_0001);
        let extended = cast_spell_extended(GREATER_HEAL);
        assert_eq!(
            extended,
            [PKT_EXTENDED, 0x00, 0x09, 0x00, 0x1C, 0x00, 0x02, 0x00, 29]
        );
        assert_eq!(cast(GREATER_HEAL, ClientVersion::MODERN), extended);
        assert_eq!(
            cast(GREATER_HEAL, ClientVersion::T2A),
            cast_spell(GREATER_HEAL)
        );
        assert_eq!(
            cast(GREATER_HEAL, ClientVersion::new(6, 0, 14, 1)),
            cast_spell(GREATER_HEAL)
        );
        let from_book = cast_from_book(GREATER_HEAL, SPELLBOOK);
        assert_eq!(
            &from_book[..4],
            &[PKT_TEXT_COMMAND, 0x00, 0x12, TEXT_CMD_CAST_FROM_BOOK]
        );
        assert_eq!(&from_book[4..], b"29 1073741825\0");
        let open = open_spellbook(SPELLBOOK_NECROMANCY);
        assert_eq!(
            open,
            [
                PKT_TEXT_COMMAND,
                0x00,
                0x06,
                TEXT_CMD_OPEN_SPELLBOOK,
                b'2',
                0
            ]
        );
    }

    /// The small client requests, each at the length its server handler
    /// reads.
    #[test]
    fn small_client_requests_are_byte_exact() {
        const MOBILE: Serial = Serial(0x0000_0102);
        const TIP: u16 = 7;
        assert_eq!(
            name_request(MOBILE),
            [PKT_UPDATE_NAME, 0x00, 0x07, 0, 0, 0x01, 0x02]
        );
        assert_eq!(tip_request(TIP, true), [PKT_TIP_REQUEST, 0x00, 0x07, 0x01]);
        assert_eq!(tip_request(TIP, false), [PKT_TIP_REQUEST, 0x00, 0x07, 0x00]);
        assert_eq!(
            quest_arrow_click(true),
            [PKT_EXTENDED, 0x00, 0x06, 0x00, 0x07, 0x01]
        );
        assert_eq!(
            close_status_bar(MOBILE),
            [PKT_EXTENDED, 0x00, 0x09, 0x00, 0x0C, 0, 0, 0x01, 0x02]
        );
        let looks = NewLooks {
            skin_hue: 0x03EA,
            hair: 0x203B,
            hair_hue: 0x044E,
            beard: 0x2040,
            beard_hue: 0x044F,
        };
        assert_eq!(
            race_change_answer(Some(looks)),
            [
                PKT_EXTENDED,
                0x00,
                0x0F,
                0x00,
                0x2A,
                0x03,
                0xEA,
                0x20,
                0x3B,
                0x04,
                0x4E,
                0x20,
                0x40,
                0x04,
                0x4F
            ]
        );
        assert_eq!(
            race_change_answer(None),
            [PKT_EXTENDED, 0x00, 0x05, 0x00, 0x2A]
        );
        assert_eq!(
            query_properties_old(MOBILE),
            [PKT_EXTENDED, 0x00, 0x09, 0x00, 0x10, 0, 0, 0x01, 0x02]
        );
        assert_eq!(
            boat_move(MOBILE, Direction::East, BOAT_SPEED_FAST),
            [
                PKT_EXTENDED,
                0x00,
                0x0C,
                0x00,
                0x33,
                0,
                0,
                0x01,
                0x02,
                2,
                2,
                2
            ]
        );
        assert_eq!(query_party_positions(), [PKT_ASSISTANT, 0x00, 0x04, 0x00]);
        assert_eq!(
            query_guild_positions(),
            [PKT_ASSISTANT, 0x00, 0x05, 0x01, 0x01]
        );
        assert_eq!(public_house_content(true), [PKT_PUBLIC_HOUSE_CONTENT, 1]);
        assert_eq!(
            equip_last_weapon(MOBILE),
            [PKT_ENCODED, 0x00, 0x0A, 0, 0, 0x01, 0x02, 0x00, 0x1E, 0x0A]
        );
        let sync = house_edit(MOBILE, HouseEdit::Sync);
        assert_eq!(
            sync,
            [
                PKT_AOS_COMMAND,
                0x00,
                0x0A,
                0,
                0,
                0x01,
                0x02,
                0x00,
                0x0E,
                0x0A
            ]
        );
    }

    /// The old cover change is the fixed 99 bytes a server reads for `0x93`.
    #[test]
    fn the_old_book_header_is_fixed_width() {
        const OLD_HEADER_LEN: usize = 99;
        let packet = book_header_old(BOOK, "Tale", "Bob");
        assert_eq!(packet.len(), OLD_HEADER_LEN);
        assert_eq!(
            &packet[..9],
            &[PKT_BOOK_HEADER_OLD, 0x40, 0x00, 0x00, 0xCC, 0, 1, 0, 0]
        );
        assert_eq!(&packet[9..14], b"Tale\0");
        assert_eq!(&packet[69..73], b"Bob\0");
        assert_eq!(
            crate::lengths::PacketTable::t2a().fixed_len(PKT_BOOK_HEADER_OLD),
            Some(OLD_HEADER_LEN as u16)
        );
    }

    /// The hash answer: the block, six spare bytes, the query command, the
    /// map, then 25 checksums, as the reference client writes it.
    #[test]
    fn the_ultima_live_answer_is_byte_exact() {
        const BLOCK: u32 = 0x0001_0203;
        const MAP: u8 = 1;
        let mut hashes = [0u16; LIVE_HASH_COUNT];
        hashes[0] = 0xABCD;
        hashes[LIVE_HASH_COUNT - 1] = 0x0102;
        let packet = ultima_live_hashes(BLOCK, MAP, &hashes);
        assert_eq!(packet.len(), 15 + 2 * LIVE_HASH_COUNT);
        assert_eq!(framed_len(&packet), packet.len());
        assert_eq!(
            &packet[..17],
            &[
                PKT_ULTIMA_LIVE,
                0x00,
                0x41,
                0,
                1,
                2,
                3,
                0,
                0,
                0,
                0,
                0,
                0,
                0xFF,
                MAP,
                0xAB,
                0xCD
            ]
        );
        assert_eq!(&packet[packet.len() - 2..], &[0x01, 0x02]);
    }
}
