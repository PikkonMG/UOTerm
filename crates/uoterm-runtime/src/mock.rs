//! Local unencrypted shard for tests and `uoterm mock-shard`.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::task::AbortHandle;
use uoterm_protocol::buf::PacketWriter;
use uoterm_protocol::crypto::{for_mode, EncryptionMode, StreamCipher};
use uoterm_protocol::frame::compress_packet;
use uoterm_protocol::huffman::Huffman;
use uoterm_protocol::lengths::{PacketLen, PacketTable};
use uoterm_protocol::types::*;

/// The era the mock shard falls back to when no login handshake stated one.
/// The client defaults to the same era.
pub const MOCK_ERA: Era = Era::Modern;
pub const MOCK_CHAR: &str = "Mara";
pub const MOCK_SHARD: &str = "Test Shard";
pub const MOCK_X: u16 = 1424;
pub const MOCK_Y: u16 = 1693;
pub const MOCK_Z: i8 = 0;
pub const MOCK_PLAYER: u32 = 0x0000_00AA;
pub const MOCK_BACKPACK: u32 = 0x4000_0002;
pub const MOCK_HATCHET: u32 = 0x4000_0001;
pub const MOCK_TREE: u32 = 0x4000_0010;
pub const MOCK_LOGS: u32 = 0x4000_0020;
pub const MOCK_TREE_GRAPHIC: u16 = 0x0CCA;

const EXT_SEED_REST: usize = 20;
const RAW_SEED_REST: usize = 3;
const LOGIN_REQUEST_LEN: usize = 62;
const SELECT_SERVER_LEN: usize = 3;
const GAME_SEED_LEN: usize = 4;
const GAME_LOGIN_LEN: usize = 65;
const PLAY_CHAR_REST: usize = 72;
/// Where the character name starts in a play request after its id, and in a
/// create request after its id.
const PLAY_NAME_AT: usize = 4;
const CREATE_NAME_AT: usize = 9;
const CHARACTER_NAME_LEN: usize = 30;
/// A delete request after its id: the password field, the slot, the address.
const DELETE_REST: usize = 38;
const DELETE_SLOT_AT: usize = 30;
/// The refusal the mock gives a new character whose name is taken:
/// "the shard could not carry out the request".
const REFUSED_NAME_TAKEN: u8 = 5;
/// A logout request after its id, and the answer that grants it.
const LOGOUT_REST: usize = 1;
const LOGOUT_GRANTED: u8 = 1;
/// How many character slots the mock account has.
const MOCK_SLOTS: usize = 5;
const MOVE_REST: usize = 6;
const SERIAL_LEN: usize = 4;
const TARGET_REST: usize = 18;
const PING_REST: usize = 1;
const WAR_MODE_REST: usize = 4;
/// The paperdoll flags of a player who may lift the items on it.
const PAPERDOLL_FLAGS_OWN: u8 = 0x02;
const QUERY_REST: usize = 9;
const SPEECH_HEADER_SKIP: usize = 5;
const VAR_LEN_HEADER: usize = 3;
const DISCONNECT: u8 = 0x01;
const WAR_MODE_UNKNOWN: u8 = 0x32;
const MOCK_AUTH_ID: u32 = 0xAC1C_A001;
/// Client feature flags the mock advertises. The mock needs none of them.
const MOCK_FEATURE_FLAGS: u32 = 0;
/// Added to the graphic id of a container item. A server writes zero.
const GRAPHIC_INCREMENT_NONE: u8 = 0;
/// Container gump slot. The mock stacks every item on the origin.
const CONTAINER_SLOT_ORIGIN: u16 = 0;
/// Grid slot of a container item, present from client 6.0.1.7.
const CONTAINER_GRID_INDEX: u8 = 0;
const HUE_NONE: u16 = 0;
const HATCHET_AMOUNT: u16 = 1;
const LOGS_AMOUNT: u16 = 10;
const CONTENTS_ITEM_COUNT: u16 = 1;

/// The era whose packet shapes the mock writes, per listener.
///
/// The opening seed states it: a modern client announces itself with `0xEF`
/// and its version, an older client sends four raw seed bytes. A game
/// connection made after a `0x8C` relay repeats only the auth id, so it takes
/// the era of the login handshake that sent it there, or the configured era
/// when the shard never saw that handshake.
#[derive(Clone)]
struct ClientEra(Arc<Mutex<Era>>);

impl ClientEra {
    fn new(configured: Era) -> Self {
        Self(Arc::new(Mutex::new(configured)))
    }

    fn on_login(&self, extended_seed: bool) -> Era {
        let era = if extended_seed { Era::Modern } else { Era::T2a };
        *self.0.lock() = era;
        era
    }

    fn current(&self) -> Era {
        *self.0.lock()
    }
}

struct SessionCrypt {
    cipher: Box<dyn StreamCipher>,
    game: bool,
}

impl SessionCrypt {
    fn login(seed: u32, version: ClientVersion) -> Self {
        Self {
            cipher: for_mode(EncryptionMode::Osi, seed, version),
            game: false,
        }
    }

    fn game(seed: u32, version: ClientVersion) -> Self {
        let mut cipher = for_mode(EncryptionMode::Osi, seed, version);
        cipher.reset_for_game(seed);
        Self { cipher, game: true }
    }

    fn unwrap_in(&mut self, buf: &mut [u8]) {
        if self.game {
            self.cipher.encrypt(buf);
        } else {
            self.cipher.decrypt(buf);
        }
    }

    fn wrap_out(&mut self, buf: &mut [u8]) {
        if self.game {
            self.cipher.decrypt(buf);
        }
    }

    fn enter_game(&mut self, seed: u32) {
        self.cipher.reset_for_game(seed);
        self.game = true;
    }
}

/// One client connection as the shard reads it: the cipher, when the link
/// is encrypted, and the bytes of the packet being read.
#[derive(Default)]
struct Wire {
    crypt: Option<SessionCrypt>,
    packet: Vec<u8>,
}

/// Every packet the shard heard in the world loop of its clients, from the
/// play request on, oldest first. It keeps the newest [`HEARD_KEPT`], so a
/// shard that runs for long does not grow.
#[derive(Clone, Default)]
struct Heard(Arc<Mutex<VecDeque<Vec<u8>>>>);

const HEARD_KEPT: usize = 4096;

impl Heard {
    /// Files the packet the wire has read, once it has one.
    fn file(&self, wire: &mut Wire) {
        if wire.packet.is_empty() {
            return;
        }
        let mut heard = self.0.lock();
        if heard.len() == HEARD_KEPT {
            heard.pop_front();
        }
        heard.push_back(std::mem::take(&mut wire.packet));
    }
}

/// Test-owned mock shard. Abort the accept loop when the last handle drops.
pub struct MockServer {
    pub addr: SocketAddr,
    abort: Option<AbortHandle>,
    shutdown: broadcast::Sender<()>,
    heard: Heard,
}

impl MockServer {
    pub async fn start() -> std::io::Result<Self> {
        Self::start_era(MOCK_ERA).await
    }

    /// Serve a client of this era when no login handshake states one. A
    /// login handshake overrides it, because the shard must build and frame
    /// every packet the way the client on the other end reads it.
    pub async fn start_era(era: Era) -> std::io::Result<Self> {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        spawn_listener(addr, true, None, era).await
    }

    pub async fn start_osi(version: ClientVersion) -> std::io::Result<Self> {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        spawn_listener(addr, true, Some(version), MOCK_ERA).await
    }

    /// Every packet the clients sent in the world loop so far, from the play
    /// request on, oldest first.
    pub fn heard(&self) -> Vec<Vec<u8>> {
        self.heard.0.lock().iter().cloned().collect()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(abort) = self.abort.take() {
            let _ = self.shutdown.send(());
            abort.abort();
        }
    }
}

pub async fn serve(addr: SocketAddr) -> std::io::Result<SocketAddr> {
    let mut server = spawn_listener(addr, false, None, MOCK_ERA).await?;
    let addr = server.addr;
    server.abort.take();
    Ok(addr)
}

async fn spawn_listener(
    addr: SocketAddr,
    with_shutdown: bool,
    osi: Option<ClientVersion>,
    era: Era,
) -> std::io::Result<MockServer> {
    let listener = TcpListener::bind(addr).await?;
    let local = listener.local_addr()?;
    let (shutdown, _) = broadcast::channel::<()>(1);
    let shutdown_tx = shutdown.clone();
    let client_era = ClientEra::new(era);
    let heard = Heard::default();
    let heard_by_clients = heard.clone();
    let task = tokio::spawn(async move {
        if with_shutdown {
            let mut sd = shutdown.subscribe();
            loop {
                tokio::select! {
                    acc = listener.accept() => {
                        match acc {
                            Ok((stream, _)) => {
                                let mut client_sd = sd.resubscribe();
                                let era = client_era.clone();
                                let heard = heard_by_clients.clone();
                                tokio::spawn(async move {
                                    tokio::select! {
                                        _ = handle_client(stream, osi, &era, &heard) => {}
                                        _ = client_sd.recv() => {}
                                    }
                                });
                            }
                            Err(_) => break,
                        }
                    }
                    _ = sd.recv() => break,
                }
            }
        } else {
            while let Ok((stream, _)) = listener.accept().await {
                let era = client_era.clone();
                let heard = heard_by_clients.clone();
                tokio::spawn(async move {
                    let _ = handle_client(stream, osi, &era, &heard).await;
                });
            }
        }
    });
    Ok(MockServer {
        addr: local,
        abort: Some(task.abort_handle()),
        shutdown: shutdown_tx,
        heard,
    })
}

fn send_raw(tx: &mpsc::UnboundedSender<Vec<u8>>, pkt: Vec<u8>) -> std::io::Result<()> {
    tx.send(pkt)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "mock writer closed"))
}

/// Compare one outgoing packet with the length table the client frames by.
/// A fixed-length id must match the table byte for byte. A variable-length id
/// must declare its own true byte length in bytes 1..3.
fn check_packet_shape(table: &PacketTable, pkt: &[u8]) -> Result<(), String> {
    let era = table.era();
    let Some(&id) = pkt.first() else {
        return Err(format!("mock built an empty packet for {era}"));
    };
    match PacketLen::from_table(table.get(id)) {
        PacketLen::Fixed(len) if pkt.len() == len as usize => Ok(()),
        PacketLen::Fixed(len) => Err(format!(
            "mock packet 0x{id:02X} is {} bytes but the {era} table says {len}",
            pkt.len()
        )),
        PacketLen::Variable if pkt.len() < VAR_LEN_HEADER => Err(format!(
            "mock packet 0x{id:02X} is {} bytes and has no {era} length field",
            pkt.len()
        )),
        PacketLen::Variable => {
            let declared = u16::from_be_bytes([pkt[1], pkt[2]]) as usize;
            if declared == pkt.len() {
                Ok(())
            } else {
                Err(format!(
                    "mock packet 0x{id:02X} declares {declared} bytes but is {} for {era}",
                    pkt.len()
                ))
            }
        }
        PacketLen::Unknown => Err(format!("mock packet 0x{id:02X} is unknown to {era}")),
    }
}

/// Catch a shape mistake in test and debug builds before it reaches the wire.
fn debug_assert_shape(table: &PacketTable, pkt: &[u8]) {
    if cfg!(debug_assertions) {
        check_packet_shape(table, pkt).expect("mock packet shape");
    }
}

/// Send a packet the client reads plain, before the game stream starts.
fn send_plain(
    tx: &mpsc::UnboundedSender<Vec<u8>>,
    table: &PacketTable,
    pkt: Vec<u8>,
) -> std::io::Result<()> {
    debug_assert_shape(table, &pkt);
    send_raw(tx, pkt)
}

fn send_h(
    tx: &mpsc::UnboundedSender<Vec<u8>>,
    h: &Huffman,
    wire: &mut Wire,
    table: &PacketTable,
    pkt: &[u8],
) -> std::io::Result<()> {
    debug_assert_shape(table, pkt);
    let mut bytes = compress_packet(h, pkt);
    if let Some(c) = &mut wire.crypt {
        c.wrap_out(&mut bytes);
    }
    send_raw(tx, bytes)
}

async fn read_unwrapped(
    reader: &mut OwnedReadHalf,
    wire: &mut Wire,
    buf: &mut [u8],
) -> std::io::Result<()> {
    reader.read_exact(buf).await?;
    if let Some(c) = &mut wire.crypt {
        c.unwrap_in(buf);
    }
    wire.packet.extend_from_slice(buf);
    Ok(())
}

async fn handle_client(
    stream: TcpStream,
    osi: Option<ClientVersion>,
    client_era: &ClientEra,
    heard: &Heard,
) -> std::io::Result<()> {
    let huff = Huffman::new();
    let port = stream.local_addr()?.port();
    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let write_task = tokio::spawn(async move {
        while let Some(buf) = rx.recv().await {
            if writer.write_all(&buf).await.is_err() {
                break;
            }
        }
    });
    let result = handle_client_io(&mut reader, &tx, &huff, port, osi, client_era, heard).await;
    drop(tx);
    let _ = write_task.await;
    result
}

async fn handle_client_io(
    reader: &mut OwnedReadHalf,
    tx: &mpsc::UnboundedSender<Vec<u8>>,
    huff: &Huffman,
    port: u16,
    osi: Option<ClientVersion>,
    client_era: &ClientEra,
    heard: &Heard,
) -> std::io::Result<()> {
    let mut first = [0u8; 1];
    reader.read_exact(&mut first).await?;
    let extended_seed = first[0] == PKT_SEED;
    let seed = if extended_seed {
        let mut rest = [0u8; EXT_SEED_REST];
        reader.read_exact(&mut rest).await?;
        u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]])
    } else {
        let mut rest = [0u8; RAW_SEED_REST];
        reader.read_exact(&mut rest).await?;
        u32::from_be_bytes([first[0], rest[0], rest[1], rest[2]])
    };
    let mut peek = [0u8; 1];
    reader.read_exact(&mut peek).await?;
    let mut wire = Wire::default();
    if let Some(version) = osi {
        let mut login = SessionCrypt::login(seed, version);
        let mut trial = peek;
        login.unwrap_in(&mut trial);
        if trial[0] == PKT_LOGIN_REQUEST {
            peek = trial;
            wire.crypt = Some(login);
        } else {
            let mut game = SessionCrypt::game(seed, version);
            let mut trial = peek;
            game.unwrap_in(&mut trial);
            if trial[0] != PKT_GAME_LOGIN {
                return Ok(());
            }
            peek = trial;
            wire.crypt = Some(game);
        }
    }
    let era = if peek[0] == PKT_LOGIN_REQUEST {
        client_era.on_login(extended_seed)
    } else {
        client_era.current()
    };
    let table = PacketTable::for_era(era);
    match peek[0] {
        PKT_LOGIN_REQUEST => {
            let mut rest = [0u8; LOGIN_REQUEST_LEN - 1];
            read_unwrapped(reader, &mut wire, &mut rest).await?;
            send_plain(tx, &table, server_list())?;
            let mut sel = [0u8; SELECT_SERVER_LEN];
            read_unwrapped(reader, &mut wire, &mut sel).await?;
            send_plain(tx, &table, relay(port))?;
            let mut seedb = [0u8; GAME_SEED_LEN];
            if reader.read_exact(&mut seedb).await.is_err() {
                return Ok(());
            }
            let auth = u32::from_be_bytes(seedb);
            if let Some(c) = wire.crypt.as_mut() {
                c.enter_game(auth);
            }
            let mut game = [0u8; GAME_LOGIN_LEN];
            read_unwrapped(reader, &mut wire, &mut game).await?;
            if game[0] != PKT_GAME_LOGIN {
                return Ok(());
            }
        }
        PKT_GAME_LOGIN => {
            let mut rest = [0u8; GAME_LOGIN_LEN - 1];
            read_unwrapped(reader, &mut wire, &mut rest).await?;
        }
        _ => return Ok(()),
    }
    send_h(tx, huff, &mut wire, &table, &features(era))?;
    // The characters of the account, each in its slot. A delete empties its
    // slot and a new one takes the first empty slot.
    let mut characters: Vec<String> = std::iter::once(MOCK_CHAR.to_string())
        .chain(std::iter::repeat_n(String::new(), MOCK_SLOTS - 1))
        .collect();
    send_h(tx, huff, &mut wire, &table, &character_list(&characters))?;
    wire.packet.clear();
    loop {
        // The packet before this one has been read and answered.
        heard.file(&mut wire);
        let mut id = [0u8; 1];
        if read_unwrapped(reader, &mut wire, &mut id).await.is_err() {
            break;
        }
        match id[0] {
            PKT_PLAY_CHARACTER => {
                let mut rest = [0u8; PLAY_CHAR_REST];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
                let name = name_at(&rest, PLAY_NAME_AT);
                for packet in enter_world(era, &name) {
                    send_h(tx, huff, &mut wire, &table, &packet)?;
                }
            }
            PKT_CREATE_CHARACTER | PKT_CREATE_CHARACTER_NEW => {
                let len = match PacketLen::from_table(table.get(id[0])) {
                    PacketLen::Fixed(len) => usize::from(len),
                    PacketLen::Variable | PacketLen::Unknown => return Ok(()),
                };
                let mut rest = vec![0u8; len.saturating_sub(1)];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
                let name = name_at(&rest, CREATE_NAME_AT);
                let taken = characters
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(&name));
                let empty = characters.iter().position(String::is_empty);
                let Some(slot) = empty.filter(|_| !taken) else {
                    send_h(
                        tx,
                        huff,
                        &mut wire,
                        &table,
                        &[PKT_CHARACTER_REJECTED, REFUSED_NAME_TAKEN],
                    )?;
                    send_h(
                        tx,
                        huff,
                        &mut wire,
                        &table,
                        &character_list_update(&characters),
                    )?;
                    continue;
                };
                characters[slot] = name.clone();
                // A shard puts a new character straight into the world.
                for packet in enter_world(era, &name) {
                    send_h(tx, huff, &mut wire, &table, &packet)?;
                }
            }
            PKT_DELETE_CHARACTER => {
                let mut rest = [0u8; DELETE_REST];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
                let slot = u32::from_be_bytes([
                    rest[DELETE_SLOT_AT],
                    rest[DELETE_SLOT_AT + 1],
                    rest[DELETE_SLOT_AT + 2],
                    rest[DELETE_SLOT_AT + 3],
                ]) as usize;
                if let Some(gone) = characters.get_mut(slot) {
                    gone.clear();
                }
                send_h(
                    tx,
                    huff,
                    &mut wire,
                    &table,
                    &character_list_update(&characters),
                )?;
            }
            PKT_CLIENT_VERSION => {
                eat_var(reader, &mut wire).await?;
            }
            PKT_MOVE => {
                let mut rest = [0u8; MOVE_REST];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
                send_h(
                    tx,
                    huff,
                    &mut wire,
                    &table,
                    &[PKT_MOVE_ACK, rest[1], NOTO_INNOCENT],
                )?;
            }
            PKT_ASCII_SPEECH => {
                let text = read_speech(reader, &mut wire).await?;
                send_h(tx, huff, &mut wire, &table, &echo(&text))?;
            }
            PKT_UNICODE_SPEECH => {
                eat_var(reader, &mut wire).await?;
                send_h(tx, huff, &mut wire, &table, &echo("ok"))?;
            }
            PKT_DOUBLE_CLICK => {
                let mut ser = [0u8; SERIAL_LEN];
                read_unwrapped(reader, &mut wire, &mut ser).await?;
                let serial = u32::from_be_bytes(ser);
                if serial == MOCK_PLAYER | PAPERDOLL_REQUEST_BIT {
                    send_h(tx, huff, &mut wire, &table, &paperdoll())?;
                } else if serial == MOCK_HATCHET || serial == MOCK_PLAYER {
                    send_h(tx, huff, &mut wire, &table, &target_cursor())?;
                }
            }
            PKT_TARGET => {
                let mut rest = [0u8; TARGET_REST];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
                send_h(tx, huff, &mut wire, &table, &chop_msg())?;
                send_h(tx, huff, &mut wire, &table, &add_logs(era))?;
            }
            PKT_PING => {
                let mut v = [0u8; PING_REST];
                read_unwrapped(reader, &mut wire, &mut v).await?;
                send_h(tx, huff, &mut wire, &table, &[PKT_PING, v[0]])?;
            }
            PKT_WAR_MODE => {
                let mut rest = [0u8; WAR_MODE_REST];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
                send_h(
                    tx,
                    huff,
                    &mut wire,
                    &table,
                    &[PKT_WAR_MODE, rest[0], 0, WAR_MODE_UNKNOWN, 0],
                )?;
            }
            PKT_TEXT_COMMAND => {
                eat_var(reader, &mut wire).await?;
            }
            PKT_QUERY => {
                let mut rest = [0u8; QUERY_REST];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
            }
            // A logout is granted at once, as a shard grants it; the client
            // closes the link.
            PKT_LOGOUT => {
                let mut rest = [0u8; LOGOUT_REST];
                read_unwrapped(reader, &mut wire, &mut rest).await?;
                send_h(tx, huff, &mut wire, &table, &[PKT_LOGOUT, LOGOUT_GRANTED])?;
            }
            DISCONNECT => break,
            other => {
                skip_known(reader, &mut wire, other, &table).await?;
            }
        }
    }
    Ok(())
}

/// Consume a packet the shard does not handle, using the client's own length
/// table. Framing it with the wrong era desynchronizes the whole stream.
async fn skip_known(
    reader: &mut OwnedReadHalf,
    wire: &mut Wire,
    id: u8,
    table: &PacketTable,
) -> std::io::Result<()> {
    match PacketLen::from_table(table.get(id)) {
        PacketLen::Fixed(len) => skip_rest(reader, wire, (len as usize).saturating_sub(1)).await,
        PacketLen::Variable => eat_var(reader, wire).await,
        PacketLen::Unknown => eat_unknown(reader, wire).await,
    }
}

/// Body bytes after the length word of a packet no era table lists.
///
/// `FrameDecoder::guess_unknown` frames such a packet by its own length word
/// while that word is credible, so the mock must eat exactly as many bytes.
fn unknown_body_len(declared: usize) -> Option<usize> {
    if (UNKNOWN_VAR_MIN..=UNKNOWN_VAR_MAX).contains(&declared) {
        Some(declared - VAR_LEN_HEADER)
    } else {
        None
    }
}

async fn eat_unknown(reader: &mut OwnedReadHalf, wire: &mut Wire) -> std::io::Result<()> {
    match unknown_body_len(read_var_len(reader, wire).await?) {
        Some(body) => skip_rest(reader, wire, body).await,
        None => Ok(()),
    }
}

async fn eat_var(reader: &mut OwnedReadHalf, wire: &mut Wire) -> std::io::Result<()> {
    let len = read_var_len(reader, wire).await?;
    skip_rest(reader, wire, len.saturating_sub(VAR_LEN_HEADER)).await
}

async fn read_var_len(reader: &mut OwnedReadHalf, wire: &mut Wire) -> std::io::Result<usize> {
    let mut lenb = [0u8; 2];
    read_unwrapped(reader, wire, &mut lenb).await?;
    Ok(u16::from_be_bytes(lenb) as usize)
}

async fn skip_rest(reader: &mut OwnedReadHalf, wire: &mut Wire, len: usize) -> std::io::Result<()> {
    if len > 0 {
        let mut rest = vec![0u8; len];
        read_unwrapped(reader, wire, &mut rest).await?;
    }
    Ok(())
}

async fn read_speech(reader: &mut OwnedReadHalf, wire: &mut Wire) -> std::io::Result<String> {
    let len = read_var_len(reader, wire).await?;
    let mut rest = vec![0u8; len.saturating_sub(VAR_LEN_HEADER)];
    read_unwrapped(reader, wire, &mut rest).await?;
    if rest.len() > SPEECH_HEADER_SKIP {
        let t = &rest[SPEECH_HEADER_SKIP..];
        let end = t.iter().position(|&b| b == 0).unwrap_or(t.len());
        Ok(String::from_utf8_lossy(&t[..end]).into_owned())
    } else {
        Ok(String::new())
    }
}

fn server_list() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_SERVER_LIST);
    w.u8(0x5D)
        .u16(1)
        .u16(0)
        .ascii_fixed(MOCK_SHARD, 32)
        .u8(0)
        .u8(0)
        .u32(0);
    w.finish_variable().expect("mock packet length fits in u16")
}

fn relay(port: u16) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_RELAY);
    w.u8(0).u8(0).u8(0).u8(0).u16(port).u32(MOCK_AUTH_ID);
    w.finish()
}

/// The reference client reads `0xB9` as a u32 from client 6.0.14.2 and as a u16
/// below it. A server writes the same two shapes, 5 bytes and 3 bytes.
fn features(era: Era) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_FEATURES);
    if era.default_version().has_feature_uint32() {
        w.u32(MOCK_FEATURE_FLAGS);
    } else {
        w.u16(MOCK_FEATURE_FLAGS as u16);
    }
    w.finish()
}

/// The name that starts at `at` of a request, up to its first zero.
fn name_at(rest: &[u8], at: usize) -> String {
    let field = rest.get(at..at + CHARACTER_NAME_LEN).unwrap_or_default();
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

/// What the shard sends as a character enters the world.
fn enter_world(era: Era, name: &str) -> Vec<Vec<u8>> {
    vec![
        login_confirm(),
        draw_player(),
        status(name),
        welcome(),
        backpack(),
        hatchet(era),
        tree(),
        vec![PKT_LOGIN_COMPLETE],
    ]
}

/// Each character's slot: its name, and the password field no shard fills.
fn write_slots(w: &mut PacketWriter, characters: &[String]) {
    w.u8(characters.len() as u8);
    for name in characters {
        w.ascii_fixed(name, CHARACTER_NAME_LEN)
            .ascii_fixed("", CHARACTER_NAME_LEN);
    }
}

fn character_list(characters: &[String]) -> Vec<u8> {
    const NO_START_TOWNS: u8 = 0;
    let mut w = PacketWriter::with_variable(PKT_CHARACTER_LIST);
    write_slots(&mut w, characters);
    w.u8(NO_START_TOWNS)
        .u32(ACCOUNT_FLAG_CONTEXT_MENUS | ACCOUNT_FLAG_PROPERTY_LISTS);
    w.finish_variable().expect("mock packet length fits in u16")
}

/// The list again after a character is made or deleted.
fn character_list_update(characters: &[String]) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_CHARACTER_LIST_UPDATE);
    write_slots(&mut w, characters);
    w.finish_variable().expect("mock packet length fits in u16")
}

fn login_confirm() -> Vec<u8> {
    let mut b = vec![0u8; 37];
    b[0] = PKT_LOGIN_CONFIRM;
    b[1..5].copy_from_slice(&MOCK_PLAYER.to_be_bytes());
    b[9..11].copy_from_slice(&0x0190u16.to_be_bytes());
    b[11..13].copy_from_slice(&MOCK_X.to_be_bytes());
    b[13..15].copy_from_slice(&MOCK_Y.to_be_bytes());
    b[17] = Direction::South as u8;
    b[18] = MOCK_Z as u8;
    b[28..30].copy_from_slice(&7168u16.to_be_bytes());
    b[30..32].copy_from_slice(&4096u16.to_be_bytes());
    b
}

fn draw_player() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_DRAW_PLAYER);
    w.u32(MOCK_PLAYER)
        .u16(0x0190)
        .u8(0)
        .u16(0)
        .u8(0)
        .u16(MOCK_X)
        .u16(MOCK_Y)
        .u16(0)
        .u8(Direction::South as u8)
        .i8(MOCK_Z);
    w.finish()
}

fn status(name: &str) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_STATUS);
    w.u32(MOCK_PLAYER)
        .ascii_fixed(name, CHARACTER_NAME_LEN)
        .u16(60)
        .u16(60)
        .u8(0)
        .u8(1)
        .u8(0)
        .u16(40)
        .u16(40)
        .u16(10)
        .u16(40)
        .u16(40)
        .u16(10)
        .u16(10)
        .u32(100)
        .u16(0)
        .u16(50);
    w.finish_variable().expect("mock packet length fits in u16")
}

fn welcome() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASCII_MESSAGE);
    w.u32(0xFFFF_FFFF)
        .u16(0xFFFF)
        .u8(SPEECH_SYSTEM)
        .u16(0x03B2)
        .u16(3)
        .ascii_fixed("System", 30)
        .ascii_z("Welcome to the private shard.");
    w.finish_variable().expect("mock packet length fits in u16")
}

fn backpack() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_EQUIPPED);
    w.u32(MOCK_BACKPACK)
        .u16(GRAPHIC_BACKPACK)
        .u8(0)
        .u8(LAYER_BACKPACK)
        .u32(MOCK_PLAYER)
        .u16(0);
    w.finish()
}

/// One item record of a backpack. `0x3C` repeats it and `0x25` carries one.
/// The reference client skips a grid byte from client 6.0.1.7 in both its
/// container content and its single contained item handler, and a server
/// writes that byte.
fn container_item(w: &mut PacketWriter, era: Era, serial: u32, graphic: u16, amount: u16) {
    w.u32(serial)
        .u16(graphic)
        .u8(GRAPHIC_INCREMENT_NONE)
        .u16(amount)
        .u16(CONTAINER_SLOT_ORIGIN)
        .u16(CONTAINER_SLOT_ORIGIN);
    if era.default_version().has_container_grid() {
        w.u8(CONTAINER_GRID_INDEX);
    }
    w.u32(MOCK_BACKPACK).u16(HUE_NONE);
}

fn hatchet(era: Era) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_CONTAINER_CONTENTS);
    w.u16(CONTENTS_ITEM_COUNT);
    container_item(&mut w, era, MOCK_HATCHET, GRAPHIC_HATCHET, HATCHET_AMOUNT);
    w.finish_variable().expect("mock packet length fits in u16")
}

fn tree() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_WORLD_ITEM);
    w.u32(MOCK_TREE)
        .u16(MOCK_TREE_GRAPHIC)
        .u16(MOCK_X)
        .u16(MOCK_Y + 1)
        .i8(MOCK_Z);
    w.finish_variable().expect("mock packet length fits in u16")
}

/// The paperdoll of the player, as a shard opens it when asked.
fn paperdoll() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_PAPERDOLL);
    w.u32(MOCK_PLAYER)
        .ascii_fixed(MOCK_CHAR, PAPERDOLL_TEXT_LEN)
        .u8(PAPERDOLL_FLAGS_OWN);
    w.finish()
}

fn target_cursor() -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_TARGET);
    w.u8(TARGET_OBJECT)
        .u32(0x0100)
        .u8(TARGET_FLAG_NONE)
        .u32(0)
        .u16(0)
        .u16(0)
        .u8(0)
        .u8(0)
        .u16(0);
    w.finish()
}

fn chop_msg() -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASCII_MESSAGE);
    w.u32(0xFFFF_FFFF)
        .u16(0xFFFF)
        .u8(SPEECH_SYSTEM)
        .u16(0x03B2)
        .u16(3)
        .ascii_fixed("System", 30)
        .ascii_z("You chop some wood and put it in your backpack.");
    w.finish_variable().expect("mock packet length fits in u16")
}

fn add_logs(era: Era) -> Vec<u8> {
    let mut w = PacketWriter::new(PKT_ADD_ITEM);
    container_item(&mut w, era, MOCK_LOGS, GRAPHIC_LOGS, LOGS_AMOUNT);
    w.finish()
}

fn echo(text: &str) -> Vec<u8> {
    let mut w = PacketWriter::with_variable(PKT_ASCII_MESSAGE);
    w.u32(MOCK_PLAYER)
        .u16(0x0190)
        .u8(SPEECH_REGULAR)
        .u16(DEFAULT_SPEECH_HUE)
        .u16(3)
        .ascii_fixed(MOCK_CHAR, 30)
        .ascii_z(text);
    w.finish_variable().expect("mock packet length fits in u16")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConnectOptions;
    use crate::manager::Runtime;
    use crate::persona::Persona;
    use std::time::Duration;
    use uoterm_protocol::encode;
    use uoterm_protocol::lengths::PacketTable;
    use uoterm_protocol::types::{
        ClientVersion, Era, PKT_ADD_ITEM, PKT_DOUBLE_CLICK, PKT_MOVE, PKT_PING, PKT_PLAY_CHARACTER,
        PKT_QUERY, PKT_TARGET, PKT_WAR_MODE,
    };

    #[test]
    fn sizes() {
        assert_eq!(login_confirm().len(), 37);
        assert_eq!(draw_player().len(), 19);
        assert_eq!(target_cursor().len(), 19);
        assert_eq!(add_logs(Era::T2a).len(), 20);
        assert_eq!(add_logs(Era::Modern).len(), 21);
        assert_eq!(features(Era::T2a).len(), 3);
        assert_eq!(features(Era::Modern).len(), 5);
    }

    #[test]
    fn target_cursor_decodes_after_huffman() {
        use uoterm_protocol::frame::GameDecoder;
        use uoterm_protocol::{parse, Inbound};
        let huff = Huffman::new();
        let mut stream = compress_packet(&huff, &target_cursor());
        stream.extend_from_slice(&compress_packet(&huff, &chop_msg()));
        stream.extend_from_slice(&compress_packet(&huff, &add_logs(Era::T2a)));
        let mut decoder = GameDecoder::new(PacketTable::t2a());
        let packets = decoder.push_compressed(&stream).unwrap();
        assert!(
            packets.iter().any(|p| p.id == PKT_TARGET),
            "ids: {:?}",
            packets.iter().map(|p| p.id).collect::<Vec<_>>()
        );
        assert!(packets.iter().any(|p| p.id == PKT_ADD_ITEM));
        let target = packets.iter().find(|p| p.id == PKT_TARGET).unwrap();
        match parse(&target.bytes).unwrap() {
            Inbound::Target(cursor) => assert_eq!(cursor.id, 0x0100),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_paperdoll_it_sends_names_the_player() {
        use uoterm_protocol::{parse, Inbound};
        match parse(&paperdoll()).unwrap() {
            Inbound::Paperdoll { serial, text, .. } => {
                assert_eq!(serial.0, MOCK_PLAYER);
                assert_eq!(text, MOCK_CHAR);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rest_sizes_match_t2a_table() {
        let table = PacketTable::t2a();
        assert_eq!(
            table.fixed_len(PKT_DOUBLE_CLICK),
            Some((SERIAL_LEN + 1) as u16)
        );
        assert_eq!(table.fixed_len(PKT_TARGET), Some((TARGET_REST + 1) as u16));
        assert_eq!(table.fixed_len(PKT_PING), Some((PING_REST + 1) as u16));
        assert_eq!(table.fixed_len(PKT_MOVE), Some((MOVE_REST + 1) as u16));
        assert_eq!(
            table.fixed_len(PKT_PLAY_CHARACTER),
            Some((PLAY_CHAR_REST + 1) as u16)
        );
        assert_eq!(table.fixed_len(PKT_QUERY), Some((QUERY_REST + 1) as u16));
        assert_eq!(
            table.fixed_len(PKT_WAR_MODE),
            Some((WAR_MODE_REST + 1) as u16)
        );
    }

    /// Every packet the mock can put on the wire, built for one era.
    fn all_mock_packets(era: Era) -> Vec<Vec<u8>> {
        const SAMPLE_PORT: u16 = 2593;
        const SAMPLE_SEQUENCE: u8 = 1;
        const SAMPLE_PING: u8 = 0x22;
        const SAMPLE_WAR_FLAG: u8 = 1;
        vec![
            server_list(),
            relay(SAMPLE_PORT),
            features(era),
            character_list(&[MOCK_CHAR.into()]),
            character_list_update(&[MOCK_CHAR.into()]),
            vec![PKT_CHARACTER_REJECTED, REFUSED_NAME_TAKEN],
            login_confirm(),
            draw_player(),
            status(MOCK_CHAR),
            welcome(),
            backpack(),
            hatchet(era),
            tree(),
            paperdoll(),
            target_cursor(),
            chop_msg(),
            add_logs(era),
            echo("hello"),
            vec![PKT_LOGIN_COMPLETE],
            vec![PKT_MOVE_ACK, SAMPLE_SEQUENCE, NOTO_INNOCENT],
            vec![PKT_PING, SAMPLE_PING],
            vec![PKT_WAR_MODE, SAMPLE_WAR_FLAG, 0, WAR_MODE_UNKNOWN, 0],
        ]
    }

    #[test]
    fn every_mock_packet_matches_its_era_table() {
        for era in [Era::T2a, Era::Modern] {
            let table = PacketTable::for_era(era);
            for pkt in all_mock_packets(era) {
                check_packet_shape(&table, &pkt).unwrap_or_else(|reason| panic!("{reason}"));
            }
        }
    }

    #[test]
    fn every_mock_packet_frames_back_on_its_era_stream() {
        use uoterm_protocol::frame::GameDecoder;
        let huff = Huffman::new();
        for era in [Era::T2a, Era::Modern] {
            let packets = all_mock_packets(era);
            let mut stream = Vec::new();
            for pkt in &packets {
                stream.extend_from_slice(&compress_packet(&huff, pkt));
            }
            let framed = GameDecoder::new(PacketTable::for_era(era))
                .push_compressed(&stream)
                .unwrap();
            let bytes: Vec<Vec<u8>> = framed.into_iter().map(|p| p.bytes).collect();
            assert_eq!(bytes, packets, "{era} stream must frame back byte for byte");
        }
    }

    #[test]
    fn container_items_keep_their_fields_in_both_eras() {
        use uoterm_protocol::{parse, Inbound};
        for era in [Era::T2a, Era::Modern] {
            match parse(&add_logs(era)).unwrap() {
                Inbound::AddItem(item) => {
                    assert_eq!(item.serial, Serial(MOCK_LOGS), "{era}");
                    assert_eq!(item.graphic, GRAPHIC_LOGS, "{era}");
                    assert_eq!(item.amount, LOGS_AMOUNT, "{era}");
                    assert_eq!(item.container, Serial(MOCK_BACKPACK), "{era}");
                }
                other => panic!("{era}: {other:?}"),
            }
            match parse(&hatchet(era)).unwrap() {
                Inbound::ContainerContents { items } => {
                    assert_eq!(items.len(), CONTENTS_ITEM_COUNT as usize, "{era}");
                    assert_eq!(items[0].serial, Serial(MOCK_HATCHET), "{era}");
                    assert_eq!(items[0].graphic, GRAPHIC_HATCHET, "{era}");
                    assert_eq!(items[0].amount, HATCHET_AMOUNT, "{era}");
                    assert_eq!(items[0].container, Serial(MOCK_BACKPACK), "{era}");
                }
                other => panic!("{era}: {other:?}"),
            }
            match parse(&features(era)).unwrap() {
                Inbound::Features { flags } => assert_eq!(flags, MOCK_FEATURE_FLAGS, "{era}"),
                other => panic!("{era}: {other:?}"),
            }
        }
    }

    #[test]
    fn opening_seed_states_the_client_era() {
        let shard = ClientEra::new(MOCK_ERA);
        assert_eq!(shard.on_login(false), Era::T2a);
        assert_eq!(
            shard.current(),
            Era::T2a,
            "a relay game connection adopts the era of its login handshake"
        );
        assert_eq!(shard.on_login(true), Era::Modern);
        assert_eq!(shard.current(), Era::Modern);
        assert_eq!(
            ClientEra::new(MOCK_ERA).current(),
            MOCK_ERA,
            "a game connection with no login handshake keeps the configured era"
        );
    }

    #[test]
    fn unknown_id_is_framed_by_its_own_length_word() {
        let opl_request = uoterm_protocol::encode::batch_query_properties(&[Serial(MOCK_TREE)]);
        assert_eq!(
            unknown_body_len(opl_request.len()),
            Some(opl_request.len() - VAR_LEN_HEADER)
        );
        assert_eq!(unknown_body_len(UNKNOWN_VAR_MIN - 1), None);
        assert_eq!(unknown_body_len(UNKNOWN_VAR_MAX + 1), None);
    }

    #[test]
    fn shape_check_rejects_a_wrong_length() {
        let modern = PacketTable::for_era(Era::Modern);
        assert!(check_packet_shape(&modern, &features(Era::T2a)).is_err());
        let t2a = PacketTable::for_era(Era::T2a);
        assert!(check_packet_shape(&t2a, &features(Era::Modern)).is_err());
        let mut short = status(MOCK_CHAR);
        short.pop();
        assert!(check_packet_shape(&t2a, &short).is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn login_against_mock() {
        let server = MockServer::start().await.unwrap();
        let opts = ConnectOptions {
            host: server.addr.ip().to_string(),
            port: server.addr.port(),
            account: "test".into(),
            password: "test".into(),
            shard: None,
            character: MOCK_CHAR.into(),
            version: ClientVersion::T2A,
            era: Era::T2a,
            uopath: None,
            markers: None,
            persona: Some(Persona::lumberjack_yew()),
            next_login_key: 0xFF,
            encryption: Default::default(),
            obey_shard_rules: crate::config::OBEY_SHARD_RULES_DEFAULT,
            answer_when_named: crate::config::ANSWER_WHEN_NAMED_DEFAULT,
            play_along: crate::config::PLAY_ALONG_DEFAULT,
            picker: None,
            reconnect: false,
            proxy: None,
        };
        let rt = Runtime::new(2);
        let handle = rt.connect(opts).await.unwrap();
        let mut ok = false;
        for _ in 0..25 {
            if handle.world.read().logged_in {
                ok = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ok, "expected login complete against mock shard");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn login_path_game_login_after_seed() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use uoterm_protocol::encode;
        let server = MockServer::start().await.unwrap();
        let mut stream = TcpStream::connect(server.addr).await.unwrap();
        stream.write_all(&[0, 0, 0, 1]).await.unwrap();
        stream
            .write_all(&encode::game_login(0, "test", "test"))
            .await
            .unwrap();
        let mut buf = [0u8; 16];
        let n = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf))
            .await
            .expect("game login path timed out")
            .unwrap();
        assert!(n > 0, "game login after seed must receive a reply");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn login_path_nocrypt_account() {
        let server = MockServer::start().await.unwrap();
        let opts = ConnectOptions {
            host: server.addr.ip().to_string(),
            port: server.addr.port(),
            account: "test".into(),
            password: "test".into(),
            shard: None,
            character: MOCK_CHAR.into(),
            version: ClientVersion::T2A,
            era: Era::T2a,
            uopath: None,
            markers: None,
            persona: Some(Persona::lumberjack_yew()),
            next_login_key: 0xFF,
            encryption: crate::config::EncryptionMode::None,
            obey_shard_rules: crate::config::OBEY_SHARD_RULES_DEFAULT,
            answer_when_named: crate::config::ANSWER_WHEN_NAMED_DEFAULT,
            play_along: crate::config::PLAY_ALONG_DEFAULT,
            picker: None,
            reconnect: false,
            proxy: None,
        };
        let rt = Runtime::new(2);
        let handle = rt.connect(opts).await.unwrap();
        let mut ok = false;
        for _ in 0..25 {
            if handle.world.read().logged_in {
                ok = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ok, "nocrypt account login must enter the world");
    }

    fn connect_opts(
        server: &MockServer,
        encryption: crate::config::EncryptionMode,
    ) -> ConnectOptions {
        ConnectOptions {
            host: server.addr.ip().to_string(),
            port: server.addr.port(),
            account: "test".into(),
            password: "test".into(),
            shard: None,
            character: MOCK_CHAR.into(),
            version: ClientVersion::T2A,
            era: Era::T2a,
            uopath: None,
            markers: None,
            persona: Some(Persona::lumberjack_yew()),
            next_login_key: 0xFF,
            encryption,
            obey_shard_rules: crate::config::OBEY_SHARD_RULES_DEFAULT,
            answer_when_named: crate::config::ANSWER_WHEN_NAMED_DEFAULT,
            play_along: crate::config::PLAY_ALONG_DEFAULT,
            picker: None,
            reconnect: false,
            proxy: None,
        }
    }

    async fn assert_character_list(handle: &crate::session::SessionHandle) {
        for _ in 0..25 {
            if handle.world.read().self_state.name == MOCK_CHAR {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!(
            "did not reach character list; name={}",
            handle.world.read().self_state.name
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn login_path_relay_reconnect() {
        let server = MockServer::start().await.unwrap();
        let opts = connect_opts(&server, crate::config::EncryptionMode::None);
        let handle = Runtime::new(2).connect(opts).await.unwrap();
        assert_character_list(&handle).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn login_path_modern_relay_reconnect() {
        let server = MockServer::start().await.unwrap();
        let mut opts = connect_opts(&server, crate::config::EncryptionMode::None);
        opts.era = Era::Modern;
        opts.version = ClientVersion::MODERN;
        let handle = Runtime::new(2).connect(opts).await.unwrap();
        assert_character_list(&handle).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn login_path_osi_relay() {
        let server = MockServer::start_osi(ClientVersion::T2A).await.unwrap();
        let opts = connect_opts(&server, crate::config::EncryptionMode::Osi);
        let handle = Runtime::new(2).connect(opts).await.unwrap();
        assert_character_list(&handle).await;
    }

    /// How often and how long a test looks for the double click on the
    /// character, which comes a second after login complete.
    const HEARD_POLLS: usize = 40;
    const HEARD_POLL_GAP: Duration = Duration::from_millis(100);

    /// What a client of this version said from its play request on, up to
    /// and with the double click that opens the character's paperdoll.
    async fn heard_at_login(version: ClientVersion, era: Era) -> Vec<Vec<u8>> {
        let server = MockServer::start_era(era).await.unwrap();
        let mut opts = connect_opts(&server, crate::config::EncryptionMode::None);
        opts.version = version;
        opts.era = era;
        let handle = Runtime::new(2).connect(opts).await.unwrap();
        let open_self = encode::double_click(Serial(MOCK_PLAYER | PAPERDOLL_REQUEST_BIT));
        for _ in 0..HEARD_POLLS {
            let heard = server.heard();
            if let Some(at) = heard.iter().position(|pkt| *pkt == open_self) {
                handle.shutdown().await;
                return heard[..=at].to_vec();
            }
            tokio::time::sleep(HEARD_POLL_GAP).await;
        }
        panic!(
            "the character was never double clicked; heard ids {:02X?}",
            server.heard().iter().map(|pkt| pkt[0]).collect::<Vec<_>>()
        );
    }

    /// Checks the play request, then the talk of login confirm and login
    /// complete in order, and the double click that ends it.
    fn assert_login_talk(heard: &[Vec<u8>], talk: &[Vec<u8>]) {
        let ids: Vec<u8> = heard.iter().map(|pkt| pkt[0]).collect();
        assert_eq!(heard[0][0], PKT_PLAY_CHARACTER, "ids {ids:02X?}");
        assert_eq!(&heard[1..=talk.len()], talk, "ids {ids:02X?}");
        assert_eq!(
            heard.last(),
            Some(&encode::double_click(Serial(
                MOCK_PLAYER | PAPERDOLL_REQUEST_BIT
            )))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_modern_client_enters_the_world_as_the_reference_client_does() {
        let me = Serial(MOCK_PLAYER);
        let view = uoterm_world::GameView::default();
        let heard = heard_at_login(ClientVersion::MODERN, Era::Modern).await;
        assert_login_talk(
            &heard,
            &[
                encode::game_window_size(view.width, view.height),
                encode::language(LANGUAGE_ENU),
                encode::client_version(ClientVersion::MODERN),
                encode::single_click(me),
                encode::query_skills(me),
                encode::public_house_content(false),
                encode::query_status(me),
                encode::chat_open(""),
                encode::query_skills(me),
                encode::client_type(ClientVersion::MODERN),
                encode::view_range(CLIENT_VIEW_RANGE_MAX),
            ],
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_t2a_client_enters_the_world_as_the_reference_client_does() {
        let me = Serial(MOCK_PLAYER);
        let view = uoterm_world::GameView::default();
        let heard = heard_at_login(ClientVersion::T2A, Era::T2a).await;
        assert_login_talk(
            &heard,
            &[
                encode::game_window_size(view.width, view.height),
                encode::language(LANGUAGE_ENU),
                encode::client_version(ClientVersion::T2A),
                encode::single_click(me),
                encode::query_skills(me),
                encode::query_status(me),
                encode::chat_open(""),
                encode::query_skills(me),
            ],
        );
    }
}
