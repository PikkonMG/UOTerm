# Protocol

UOTerm speaks the Ultima Online login and game streams the way a Classic Client does. It does not special-case a shard by name. Target shards: private shards that accept a Classic Client.

## Era

`--era` selects the base packet length table and the default version:

| Value | Default version | Typical use |
| --- | --- | --- |
| `t2a` | `2.0.7.0` | 1.26–2.0.x style. The mock shard uses this. |
| `modern` (default) | `7.0.102.3` | Classic Client 7.x: 32-bit `0xB9` features, container grid, `0xF3` world item |

`--version` is the string sent as packet `0xBD`. If you omit it, a `modern` session sends the version of `client.exe` in the `uopath` folder. Many shards compare the version with their own copy of `client.exe` and kick an older client. With no `client.exe`, or in the `t2a` era, the session uses the default for the era.

The version, not the era, picks the rest, the same way a Classic Client does:

- Login seed: packet `0xEF` (seed and version) from 6.0.4.0. Older versions send four bare bytes.
- Packet lengths: each packet that changed size takes the length of the version, for example `0x0B` and `0x16` at 5.0.0a, `0x08` and `0x25` at 6.0.1.7, `0xB9` at 6.0.14.2, and `0x24` and `0xBA` at 7.0.9.0.
- Character list: start towns carry a place from 7.0.13.0. Empty character slots are kept, so a pick sends the right slot.
- Character creation: the race and sex value and the expansion flags follow the version.

- `0x78` equipment: hue on every item when version is 7.0.33.1 or later. Older versions read hue only when the graphic high bit is set.
- `0x78` framing: length prefix when version is 7.0.0.0 or later. Older versions scan to a serial-0 terminator.
- Container grid on drop when version is 6.0.1.7 or later.

## Encryption

`--encryption`:

| Value | Meaning |
| --- | --- |
| `none` (default) | No stream cipher. Usual unencrypted freeshard mode. |
| `osi` | Classic Client login XOR, then Twofish+MD5 on the game socket. Keys come from client version. |

Huffman is separate. After the client sends `0x91`, inbound game bytes are Huffman-compressed. Outbound game packets are not.

This repository does not ship `client.exe` or official-server keys.

## Login

Login server:

```
seed (t2a: 4 bytes) or 0xEF (modern) → 0x80 account login → 0xA8 server list → 0xA0 select → 0x8C relay
```

After `0x8C` the client opens a new TCP connection to the game server (the Classic Client reconnect-to-relay behaviour) and seeds it with the relay key. It never stays on the login socket: one server family closes that socket after `0xA0`, and the other reads the seed as a packet and drops the client. If the relay IP is `0.0.0.0`, the reconnect uses `--host`.

Game server:

```
seed (if a new socket) → 0x91 game login → Huffman on inbound → 0xB9 features → 0xA9 character list → 0x5D play → 0x1B login confirm → 0x55 login complete
```

The mock shard accepts both paths after the seed: `0x80` account login, or `0x91` game login.

The character is in the world when the server has sent `0x1B`.

## Entering the world

UOTerm is a headless Classic Client. As the character enters the world it
tells the shard what the Classic Client tells it, in the same order and from
the same client versions as the reference client:

| When | Packets, in order | From version |
| --- | --- | --- |
| `0x1B` login confirm | `0xBF` `0x05` game view size, `0xBF` `0x0B` language `ENU` | 2.0.0 |
| | `0xBD` client version, `0x09` click on the character, `0x34` skill request | every version |
| | `0xFB` public house content, off | 7.0.79.6 |
| `0x55` login complete, the first one | `0x34` status request, `0xB5` chat under no name, `0x34` skill request | every version |
| | `0xBF` `0x0F` client type | 3.0.0e |
| | `0xC8` view range | 3.0.5d |
| one second after `0x55` | `0x06` double click on the character with bit `0x80000000`, which opens the paperdoll | every version |

The version gates are the numbers the reference client compares: it writes the
client type gate as 3.0.0 with the letter e. A shard that asks for the version
(`0xBD`) gets it at once, at login and in the world.

- Game view size: width and height, 32 bits each. A session with no window
  tells the size the reference client opens its game window at, 600 by 480.
  The watch window tells the size of the game view it draws (`game_view` in
  [AGENT_API.md](AGENT_API.md)), and the shard hears a new size when it has
  held for half a second, as the reference client tells it when its game
  window is resized.
- Client type: the byte `0x0A`, then 32 flag bits. The reference client sets
  one bit for each step up to the number its expansion bits make, and the
  shift wraps at 32 bits: 2.0.0 sends `0x00000001`, and every version from
  Age of Shadows up sends `0xFFFFFFFF`. Both server families ignore it.
- View range: the largest, 24 tiles, until the shard names one with its own
  `0xC8`; the range goes out inside 5 to 24 tiles.

UOTerm never claims to be the Enhanced or the Kingdom Reborn client: it refuses
a version with a major of 66 or more, and the expansion bits of `0x5D` keep the
3D bits clear.

## Inbound packets the decoder handles

| Id | Name | Notes |
| --- | --- | --- |
| `0x53` `0x82` `0x85` `0x86` `0x8C` `0xA8` `0xA9` `0xB9` | Login | Popup, denial, character refusal, character list (with the account flags), relay, server list, features |
| `0x1B` / `0x55` | Login confirm / complete | Serial, body, x/y/z, direction, map size |
| `0x20` / `0x21` / `0x22` / `0x97` | Draw player / walk reject / walk ack / forced walk | See Movement |
| `0x1C` / `0xAE` / `0xC1` / `0xCC` | Speech and cliloc lines | Journal. Label speech names the item it is about. Affix: type byte, 30-byte name, UTF-16BE args |
| `0x11` / `0xA1`–`0xA3` / `0x2D` / `0x17` | Status and stat bars | `weight_max` only when flag ≥ 5 |
| `0x3A` | Skills | Type `0x00` has no cap word. Types `0x02` and `0xDF` have cap |
| `0x1A` / `0xF3` | World item | The graphic step byte and the item flags (movable, hidden) are read |
| `0x24` / `0x25` / `0x3C` / `0x2E` / `0x89` / `0x1D` / `0x29` / `0x27` | Containers, worn items, delete, drop and lift answers | |
| `0x77` / `0x78` / `0xD2` / `0xD3` / `0x98` | Mobiles | `0x78` has a framed length on modern and equipment hue by version |
| `0xF6` | Boat moving | The boat, and each rider and item it carries |
| `0xF7` | Packet list | Each world item it holds |
| `0xB0` / `0xDD` | Gump / compressed gump | Layout and text lines. `0xDD` inflates both, and a count of 0 lines ends the list |
| `0x7C` / `0x9A` / `0xC2` / `0xAB` | Old menu, prompts, text entry | |
| `0x74` / `0x9E` / `0x3B` / `0x6F` | Shop buy and sell lists, shop close, secure trade | |
| `0xD6` / `0xDC` | Property list and its revision | |
| `0x6C` / `0x99` | Target cursor / multi placement | |
| `0x2F` / `0xAA` / `0x0B` / `0xAF` / `0xDF` | Combat, damage, death, buffs | |
| `0x2C` | Death screen | One action byte. Every action but `1` is a death: the world marks the character dead, ends war mode and the weather, cues the death screen (`watch` cue `death_screen`) and files a `died` event when the death is new. The session then sends `0x72` peace, as the reference client does |
| `0x66` / `0x93` / `0xD4` / `0x71` | Books and bulletin boards | |
| `0x56` / `0x90` / `0xF5` / `0xBA` / `0xE5` / `0xE6` | Maps, quest arrow and waypoints | |
| `0x6E` / `0xE2` / `0x70` / `0xC0` / `0xC7` / `0x54` / `0x6D` | Animations, effects, sound and music | |
| `0x4E` / `0x4F` / `0x65` / `0xBC` / `0x5B` | Light, weather, season and time | |
| `0xD8` / `0xB2` / `0xB8` / `0x88` / `0xA5` / `0xA6` / `0x95` / `0x38` | Custom house, chat, profile, paperdoll, web link, tip, dye, pathfind | |
| `0x73` / `0xBD` / `0xBE` | Ping, version request, assistant version | The echo of the last ping gives the round trip: `latency_ms` in `observe` and `watch` |
| `0xC8` | View range | The range the shard keeps; the next login complete asks for it |
| `0xF0` | Assistant and tracking | `0xFE` forbidden features. `0x00` tracking accepted. `0x01` party places and `0x02` guild places: serial, x, y, map (and a hits share for the guild), up to a zero serial. The guild list opens with a byte that says whether places follow. The world keeps them as `tracked_members` |
| `0x3F` | UltimaLive | Block at byte 3, a count of seven-byte units at 7, the command at 13, the map at 14, the body from 15. `0xFF` hash query, `0x00` statics of one block, `0x01` map definitions (nine bytes each), `0x02` login with the shard name. Nothing is answered or changed before the login. See UltimaLive below |
| `0x40` | UltimaLive land | Block, the 192 bytes of land in the layout of the map file, and the map at byte 200 |
| `0xBF` | Extended | Sub-commands: `0x01` / `0x02` fastwalk keys, `0x04` close gump, `0x06` party, `0x08` map change, `0x10` equip info (crafter, unidentified, attributes), `0x14` context menu, `0x16` close window, `0x18` map patches, `0x19` bonded pets and stat locks, `0x1B` spellbook content, `0x1D` house revision, `0x20` house designer, `0x22` damage, `0x26` speed mode, `0x0C` close status bar (`watch` cue `status_bar_closed`), `0x21` clear the armed weapon move, `0x25` a spell or stance on or off (`abilities` in `observe` and `watch`), `0x2A` race change: the sex and the race from 1, any other race closes it (`race_change` in `observe` and `watch`) |
| other | | Log and skip. The session does not panic |

Packet lengths live in era tables in `uoterm-protocol`. Unknown ids with a plausible variable length are skipped.

## Outbound packets on request

Each of these goes out only when a tool asks for it, or as the answer to a
shard. What goes out as the character enters the world is in
[Entering the world](#entering-the-world).

| Id | Name | Sent by | Layout |
| --- | --- | --- | --- |
| `0x12` `0x27` | Cast from a book | `cast` with `book` | Text: the spell number and the book serial in decimal, parted by a space |
| `0x12` `0x43` | Open spellbook | `open_spellbook` | Text: the kind as a number, 1 magery to 7 mysticism. Both server families read the kind as text; the reference client writes a raw byte, which they read as magery |
| `0xBF` `0x1C` | Cast | `cast`, scripts, hotkeys, from client 6.0.14.2 | Word 2 (no book named), then the spell number. Older clients send `0x12` `0x56` |
| `0x98` | Name request | the name pump, for a mobile on a shard with no property lists | The serial. The shard answers with `0x98` |
| `0xA7` | Tip request | `tip` | Fixed 4 bytes: the tip number, then 1 for the next tip or 0 for the one before |
| `0xBF` `0x07` | Quest arrow click | `quest_arrow` | One byte: 1 for the right button |
| `0xBF` `0x0C` | Close status bar | `mobile_status` with `close` | The serial |
| `0xBF` `0x10` | Property request | the name pump and `properties`, for clients older than 5.0.9.0 | One serial. Newer clients send the `0xD6` batch |
| `0xBF` `0x33` | Boat move | `boat_move` | The pilot serial, the direction twice, the speed: 0 stop, 1 slow, 2 fast |
| `0xBF` `0x2A` | Race change answer | `race_change` | Skin hue, hair, hair hue, beard, beard hue, one word each. Nothing after the sub-command says no |
| `0xB3` `0x43` | Chat leave | `chat` action `leave` | Language and the command alone |
| `0xB3` `0x63` | Chat create | `chat` action `create` | The channel name, then the password between `{` and `}` |
| `0xB5` | Chat open | `chat` action `open`, and login complete | Fixed 64 bytes: a zero, the name in UTF-16 (30 units at most), zeros to the end |
| `0xBF` `0x05` | Game view size | login confirm, and `game_view` | Width and height, 32 bits each |
| `0xD7` `0x0E` | House design sync | `house_edit` action `sync` | Player serial, command, `0x0A` |
| `0xD7` `0x1E` | Equip last weapon | `equip` with `who=last` when no weapon went to the pack here | Player serial, command, `0x0A` |
| `0xF0` `0x00` / `0x01` | Party / guild places | `track_members` | The command; the guild query adds 1 to ask for the places |
| `0xFB` | Public house content | `house_content` | Fixed 2 bytes: 1 to show |
| `0x93` | Book cover, old form | `book_write` for a book the shard opened with `0x93` | Fixed 99 bytes: serial, `0`, `1`, a zero word, a 60-byte title and a 30-byte author |
| `0x66` | Book page request | `book_read` for a page the shard has not sent | One page, line count `0xFFFF` |
| `0x3F` | UltimaLive hashes | a hash query | Block, six spare bytes, `0xFF`, the map, then 25 checksum words |
| `0x73` | Ping | every 30 seconds, and the `ping` script command | A number that counts up; its echo times the round trip |

## UltimaLive

A shard that runs UltimaLive changes blocks of the map while the game runs.
After its `0x3F` login the session takes the land (`0x40`) and statics
(`0x3F` `0x00`) of each changed block of the map underfoot. The map files are
shared by every session on the same client directory, so the first change
moves the session to a copy of the map of its own, with every change laid
over the files; walking, the radar and line of sight read it at once. A new
map form (patches) opens the copy again with every change on it.

A hash query asks for the checksums of the 5 by 5 blocks around one block,
column by column. Each checksum is the Fletcher-16 sum of the 192 bytes of
land and then the statics records of the block. The square wraps at the wrap
size of the map definitions while the middle block lies inside it.

`watch` sends the changed blocks within four blocks of the character under
`live_map`, as hex, with a revision that counts the changes, so a window can
lay them over its own map files.

## Movement

Client `0x02`: direction nibble 0–7, run bit `0x80`, sequence, fastwalk key. Sequence starts at 0. After 255 it wraps to 1. A reject resets sequence to 0.

The session never predicts position. It may hold a few steps on the wire at
once, so a slow link does not stall the walk, but it writes the character's
tile only when the server confirms a step. A reject snaps the character to the
tile the reject carries, and the session sends no resync for it. An ack that
matches no step on the wire, or a step that stays unanswered, makes the
session ask the server to resync (`0x22`) once. It then waits for the server's
redraw (`0x20`) before it walks again.

The session never walks faster than the game allows. A step waits for the
pace of the walk, the run and the mount, and for the speed mode the shard
sets (`0xBF` `0x26`). A turn on the spot costs one fast step.

The reason is that a guessed position a shard silently ignores would otherwise
stand as fact, and every route, scene and decision after it would read a tile
the character is not standing on.
