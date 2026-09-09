# Protocol

UOTerm speaks the Ultima Online login and game streams the way a Classic Client does. It does not special-case a shard by name. Target shards: private shards that accept a Classic Client.

## Era

`--era` selects the packet length table and seed shape:

| Value | Seed | Typical use |
| --- | --- | --- |
| `t2a` | 4-byte seed | 1.26–2.0.x style. The mock shard uses this. |
| `modern` (default) | packet `0xEF` (seed + version) | Classic Client 7.x: 32-bit `0xB9` features, container grid, `0xF3` world item |

`--version` is the string sent as packet `0xBD`. If you omit it, the session uses the default for the era (`2.0.7.0` for `t2a`, `7.0.102.3` for `modern`).

Version also gates decode, the same way a Classic Client does:

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

After `0x8C` the client either stays on the socket or opens a new TCP connection (the Classic Client reconnect-to-relay behaviour). `--era modern` reconnects. `--era t2a` may stay when `stay_on_socket` is set. If the relay IP is `0.0.0.0`, a reconnect uses `--host`.

Game server:

```
seed (if a new socket) → 0x91 game login → Huffman on inbound → 0xB9 features → 0xA9 character list → 0x5D play → 0x1B login confirm → 0x55 login complete
```

The mock shard accepts both paths after the seed: `0x80` account login, or `0x91` game login.

The character is in the world when the server has sent `0x1B`.

## Inbound packets the decoder handles

| Id | Name | Notes |
| --- | --- | --- |
| `0x1B` | Login confirm | Serial, body, x/y/z, direction, map size |
| `0x1C` / `0xAE` | ASCII / Unicode speech | Journal |
| `0x11` | Status | `weight_max` only when flag ≥ 5 |
| `0x3A` | Skills | Type `0x00` has no cap word. Types `0x02` and `0xDF` have cap |
| `0x3C` / `0x25` | Container contents / add item | |
| `0x78` | Mobile incoming | Framed length on modern; equipment hue by version |
| `0xB0` | Gump | Layout and text lines |
| `0xDD` | Compressed gump | Inflates layout and text. No placeholder string |
| `0xC1` / `0xCC` | Cliloc / cliloc affix | Affix: type byte, 30-byte name, UTF-16BE args |
| `0x02` path | Move | Client `0x02`. Server `0x22` ack or `0x21` reject |
| `0x6C` | Target cursor | |
| other | | Log and skip. The session does not panic |

Packet lengths live in era tables in `uoterm-protocol`. Unknown ids with a plausible variable length are skipped.

## Movement

Client `0x02`: direction nibble 0–7, run bit `0x80`, sequence, fastwalk key. Sequence starts at 0. After 255 it wraps to 1. A reject resets sequence to 0.

The session never predicts position. It may hold a few steps on the wire at
once, so a slow link does not stall the walk, but it writes the character's
tile only when the server confirms a step. A reject snaps the character to the
tile the reject carries. An unanswered step expires and the session asks the
server to resync.

The reason is that a guessed position a shard silently ignores would otherwise
stand as fact, and every route, scene and decision after it would read a tile
the character is not standing on.
