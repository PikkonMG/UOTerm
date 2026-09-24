# Protocol

UOTerm speaks the Ultima Online login and game streams the way a Classic Client does. It does not special-case a shard by name. Target shards: private shards that accept a Classic Client.

## Era

`--era` selects the base packet length table and the default version:

| Value | Default version | Typical use |
| --- | --- | --- |
| `t2a` | `2.0.7.0` | 1.26–2.0.x style. The mock shard uses this. |
| `modern` (default) | `7.0.102.3` | Classic Client 7.x: 32-bit `0xB9` features, container grid, `0xF3` world item |

`--version` is the string sent as packet `0xBD`. If you omit it, the session uses the default for the era.

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
| `0x2F` / `0xAA` / `0x0B` / `0xAF` / `0x2C` / `0xDF` | Combat, damage, death, death menu, buffs | |
| `0x66` / `0x93` / `0xD4` / `0x71` | Books and bulletin boards | |
| `0x56` / `0x90` / `0xF5` / `0xBA` / `0xE5` / `0xE6` | Maps, quest arrow and waypoints | |
| `0x6E` / `0xE2` / `0x70` / `0xC0` / `0xC7` / `0x54` / `0x6D` | Animations, effects, sound and music | |
| `0x4E` / `0x4F` / `0x65` / `0xBC` / `0x5B` | Light, weather, season and time | |
| `0xD8` / `0xB2` / `0xB8` / `0x88` / `0xA5` / `0xA6` / `0x95` / `0x38` | Custom house, chat, profile, paperdoll, web link, tip, dye, pathfind | |
| `0x73` / `0xBD` / `0xBE` / `0xF0` | Ping, version request, assistant version and assistant features | |
| `0xBF` | Extended | Sub-commands: `0x01` / `0x02` fastwalk keys, `0x04` close gump, `0x06` party, `0x08` map change, `0x10` equip info (crafter, unidentified, attributes), `0x14` context menu, `0x16` close window, `0x18` map patches, `0x19` bonded pets and stat locks, `0x1B` spellbook content, `0x1D` house revision, `0x20` house designer, `0x22` damage, `0x26` speed mode |
| other | | Log and skip. The session does not panic |

Packet lengths live in era tables in `uoterm-protocol`. Unknown ids with a plausible variable length are skipped.

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
