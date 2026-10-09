# Agent API

This is the reference for a program that drives a character: the tools, the
events, the HTTP routes and the MCP server.

A session is one character in the world. It starts from `uoterm connect`,
`uoterm populate`, `uoterm play`, a login in the web client of `uoterm web`,
`POST /v1/sessions`, or the runtime tools `connect` and `character_create`.
You drive it with the CLI (see the command reference in the
[README](../README.md#command-reference)), with HTTP, or with `uoterm mcp`.

## How a tool answers

A tool that acts answers at once with an `action_id`. You learn the end of
the act from an event:

- `arrived` or `path_failed` for a walk.
- `job_ended` for a hunt, a walk, a loot, a bank deposit, a script or an
  agent job. Its text is `<job>: done`, `stopped`, or the reason.
- `job_failed` when a job gives up.
- `target_requested`, `gump_opened` and the others for what the shard sends.

A tool call that takes more than 8 seconds fails with `tool timed out`.

## next_event

An agent must not miss what happens between its calls. So it runs one loop
around `next_event`. The [driver playbook](playbooks/driver.md) says in what
order to act on what comes back.

`next_event` answers the moment something important happens. With nothing,
it answers after `timeout_ms` (default 5000, most 7000) with no events. The
answer holds `events` and `state`, and `gumps` when a gump is open.

Events wait in the session for you. When you are slow, the next call gives
you all of them, in order, 50 at most per call. The session keeps the last
256. `missed` counts the events it dropped before you asked. It drops the
busy kinds first.

### Events

| Kind | Events |
| --- | --- |
| Control | `control_taken`: a human took the character in a play window (the native window or the web client). Your goal, job and script are stopped. Each acting tool is refused with `a human has control of the character; look only, and wait for the control_released event`. You may still look (`observe`, `look_around`, `find_*`, `next_event`). `control_released` gives the character back: read `observe` again, because the human may have moved her. `state.human_control` and `observe.human_control` say which it is now. Control also comes back by itself after 90 seconds with no human act. |
| Danger | `died`, `low_health` (health under half), `damaged`, `enemy_near`, `combatant_changed`, `pk_flag`. `died` also comes when another mobile dies; its text is then `corpse <serial>`. |
| Waits for an answer | `target_requested`, `gump_opened`, `prompt_opened` (answer with `prompt_answer`), `trade_opened`, `party_invite` (answer with `party`), `shop_opened`, `context_menu_opened` and `menu_opened` (`observe` holds the goods, the lines or the entries), `race_change_opened` (answer with `race_change`). |
| Chat | `spoken_to`. Also read `state.unanswered`. Answer with `reply`. See [Talking to players](#talking-to-players). |
| Jobs | `job_ended`: a hunt or walk job handed back (`hunt: <reason>` or `walk: <reason>`). See [playbooks/hunt.md](playbooks/hunt.md) and [playbooks/walk.md](playbooks/walk.md). `job_failed`: a loot, deposit, gather, ress or agent job gave up, with the reason. |
| Your own task | `item_added`, `arrived`, `path_failed`, `lift_rejected`, `play_along_ended`, `resurrected`, `gump_closed`. `map_changed`: a moongate or a recall took her to another map. The old map's mobiles and items are gone, and a walk or follow there is dropped. `buff_changed`: a buff came on or went off, by icon. `system_message`: the shard's own words, such as "that is too far away". `skill_changed` (`skill 27 70.3 (+0.1)`: the skill number, its value and the change). `stat_changed` (`strength 51 (+1)`). `ability_changed`: the armed weapon move is spent, or a stance came on or went off. `quest_arrow` (`shown at x,y` or `removed`). `map_opened`: a map item opened; `watch` shows it under `maps`. |
| Link | `disconnected`: the link to the shard dropped. The session logs in again by itself after 5 seconds, then after longer waits, up to 60 seconds. Meanwhile each call is refused with words that say so. The `reconnect` setting (`uoterm.toml`, and the session create body) turns this off. A `logout` ends the session for good. |
| Busy | `sound`, `effect`, `animation` (each swing, cast and bow of every mobile in view), `item_deleted` and `member_positions`. You get them only when you ask with `ambient`: a list of kinds, such as `["sound"]`, or `["all"]`. |

The session also logs `speech`, `container_opened` and `logged_in`, but
`next_event` does not return them.

A death shows the death screen: you get `died`, `state.dead` is true, and
the session leaves war mode.

### state

`state` holds:

- `hits`, `mana` and `stam` with their maximums, `war`, `dead`, `location`,
  `combatant`.
- `enemies_near`: the 5 nearest mobiles you may fight, within 10 tiles, not
  party members or friends.
- `unanswered`, `chat_mode`, `target_cursor`.
- `pack`: `items`, `weight`, `weight_max`.
- `doing`: the `goal`, the tile she is `walking_to`, whom she is `following`
  or `playing_along_with`, the corpse she is `looting`, whether she is
  `banking`, the first `script` that runs and the slots of all of them in
  `scripts`, and `job` (hunt or walk, with its phase).

`job_start` with `replace` true stops the old job first (`job_ended` reason
`stopped`), then starts the new one.

Read `doing` before you answer a player. Her words must match what she does.
When she already follows the player, say "right behind you", not "I will
stay here". When she is already at the cows, do not say "lead the way".
Never say she will do something unless you start it in the same step.

## Talking to players

When another character says your character's name, the session sends a
`spoken_to` event. Each tool result then carries these lines in
`unanswered`, whatever the tool, so a busy agent still sees them. A line
leaves the list when it is answered (see `reply` below), or after three
minutes. `observe` lists the lines of the last three minutes in
`spoken_to`, answered or not, 5 at most.

Each line has the speaker's `serial` and `name`, the `text`, the `channel`
it came in (`say`, `whisper`, `yell`, `party`, `party_private`, `guild` or
`alliance`), and `asks_if_bot`. Party, guild and alliance lines count even
when the speaker is out of sight. The name counts only as a whole word:
"Tamara" does not name "Mara". System lines, spell words and your own lines
never count. The `answer_when_named` setting turns all this off.

To start a line yourself, use `say` with `channel`: `say` (the default),
`yell`, `party`, `guild` or `alliance`.

Answer with `reply`. It sends your text back in the channel the line came
in. A yell gets an answer in a normal voice. A private party line gets an
answer to that member only. Give `to`, a name or a serial, to pick the
speaker when more than one waits. Without it, the newest line is answered.
A reply answers that speaker's lines only. A `say`, `whisper` or `emote`
answers every waiting line said nearby. A party, guild or alliance line
answers the lines of that channel.

A result with `unanswered` lines also has `chat_mode`, which `observe` shows
too: `basic` or `play_along`.

The `listen_range` option ([AGENTS.md](AGENTS.md)) makes a line said aloud
by anyone that many tiles away or nearer count as said to the character.
It is off by default. The `ignore_list` tool (list `journal`) hides lines by
speaker or words: they do not count as spoken to, and `observe`,
`journal_search` and `wait_journal` leave them out.

Answer with `reply` the way a player would. What you may agree to depends
on `chat_mode`:

- `basic` (the `play_along` setting is off): answer in a few friendly words,
  and say no to every plan: hunting, following, a party, a trade, "come
  here". For example "not right now, busy" or "maybe later". Keep doing your
  own task. The client refuses the `follow` tool, and the script command
  `partyaccept`, for a player who asked in chat, unless that player is on
  the friends list.
- `play_along`: you may say yes to the plans the persona lists (see
  [PERSONAS.md](PERSONAS.md)). With `party`, the client joins the party of a
  player who spoke to you. With `follow`, you may `follow` them. With
  `fight`, you may `attack` what they fight. The client refuses a `follow`
  that is not listed. It does not check `fight`. `observe` shows
  `playing_along`: the player and the minutes left. When the persona's time
  is up, or the character is hurt past its play-along risk, the character
  stops following and you get a `play_along_ended` event. Say a short
  goodbye and go back to your own task. Talk the way `reply_style` says,
  when it is given.

In both modes:

- Keep it short and plain. One line is enough.
- Stay in character as a player. Never bring up AI, bots, programs or "a
  game" on your own.
- When `asks_if_bot` is true, laugh it off ("lol, you're funny"), dodge it
  ("too busy chopping to chat"), or just keep working. Never flatly say "I
  am a real person" or "I am not a bot".
- Never say the same line twice. Change the words each time. The session
  refuses a line said a short time before, and a reply that comes too soon
  after the last one.

## Places

### Banks

`observe` shows `nearest_bank`: the town, the `location` where its banker
stands, and the `dist`. It is the nearest bank on this map within 400
tiles. The banks are those of the standard towns on every map. A shard with
its own towns has other banks; find them by their bankers (`find_mobiles`
with `name` `banker`). The `bank` goal walks to `nearest_bank` and ends
there.

### Landmarks

`find_landmarks` reads named places from the marker file set in the config
(`markers`): gates, banks, towns. With no marker file, it fails with words
that say so. You can filter by:

- `name`: part of the place name.
- `kind`: the marker word, such as `bank` or `moongate`. For a file with no
  marker words, part of the name.
- `map`: the one underfoot by default.
- `distance`.
- `closest` (a kind): only the nearest place of that kind.

Each place has its `map`, `location`, `dist` (only on the current map) and
`kind`. When the map underfoot has no match and the call names no map, the
answer is `{places, note, elsewhere}`: the matches on the other maps (10 at
most), so you know to travel there first.

`landmarks_info` says what marker data is loaded: `loaded`, `count`, and the
count on each map and of each kind. `move_to` and `route` take `name` to walk
to the nearest landmark of that name.

### Moongates and teleporter pads

A town moongate is not in the map files. The shard puts it in the world as a
live item. So walk to the landmark, then `find_items` with the gate graphic
(blue `0x0F6C`, red `0x0DDA`) to find the gate that stands there. Step onto
it, and answer the moongate gump.

A moongate fires when you step onto its tile, not on `use`. On most shards a
second `use` while you stand on the gate does nothing. If the gump did not
open, step off the tile and back on.

Dungeon teleporter pads are not in the map files either. The character
learns a pad the first time she uses one: when she steps onto a tile and is
moved at once on the same map, the session keeps that tile and where she
landed. After that, a `move_to` whose goal she can reach only across a
learned pad walks to the pad by itself (`{partial:true, via:"teleporter"}`).
Step on it, and call `move_to` again from the far side. Moongates are not
learned as pads.

## Gumps

A gump is a window the shard opens: a moongate, a bank question, a vendor
menu. `observe` `gumps` and the `next_event` answer show each open gump in
words. The text numbers are read from the client files.

- `texts`: the words that are not a label.
- `buttons`: each `id` with the `label` beside it. A button with `to_page`
  only shows another page.
- `choices`: each round or square button with its `switch`, its `label`, and
  the `section`, the label of the page it is on.

For example, to take the moongate to Moonglow on Trammel, tick the choice
with that label and section, and press OKAY: `gump_respond` with `button` 1
and `switches` [0].

## Trades

When another player opens a secure trade, you get a `trade_opened` event, and
`observe` shows `trade`: the player, what `theirs` and `mine` hold, and
`i_accept` and `they_accept`. Read what they offer before you agree.

- `trade_accept` ticks your accept box. `accept: false` unticks it.
- `trade_cancel` closes the trade.
- `trade_gold` sets the gold and platinum you offer.

A change to either side clears both accept boxes, so accept again after it.

Several trades can be open at once. `observe` `trades` lists each one, the
newest last, with `serial` (the other player) and `box_serial` (your box of
that trade). `trade` is the newest. `trade_accept`, `trade_cancel` and
`trade_gold` take `trade` (the other player, or a box of the trade) to pick
one. Without it they act on the newest.

## Names on a shard with no property lists

A shard says at login whether it sends property lists, the tooltips that
name every object. When it does not, the session reads names the way a
player does, one every half second. It asks for the name of each mobile with
no name, as the client asks for the name it shows over a head. It clicks
each item with no name once, and takes the name the shard shows over it.

What a click tells of an item comes back in `properties`, in the form of the
shard's era. On the oldest shards, these are the label lines, which say it
all in words: for example `a vanquishing katana crafted by Bob`, or a bag's
name and then what it holds. On later shards, it is the click info: its
name, its maker, its quality and magic, whether the magic is known, and its
charges. `properties` asks again with a new click when the last answer is
more than 10 seconds old, and never more than once a second for one object.

## Tools

### Look

| Tool | Needs | Result |
| --- | --- | --- |
| `observe` | a session | `self_state`, the radar, the journal, mobiles, items, `pending_target`, gumps, doors, buffs, party, prompt, `forbidden` (assistant features the shard forbids), `abilities` (the armed weapon move, and the spells and stances that are on), `tracked_members` (party and guild members out of sight, after `track_members`), and `latency_ms` (the last round trip to the shard). `size` (5 to 41, default 21) sets the radar width. `GET /v1/sessions/{id}/state` always uses 21. |
| `find_mobiles` | in the world | Filters: `name`, `graphic`, `distance`, `notoriety` (`innocent`, `friend`, `gray`, `criminal`, `enemy`, `murderer`, `invulnerable` or `any`), `species` (read from the body, so a named orc is still an orc), `in_sight`, `z_min` and `z_max`. `name` also matches the title, so `banker` finds "Kate the banker". On a shard with no property lists the title comes from the label of a click, so `single_click` a mobile first. Each mobile has its title, `species`, `notoriety`, `hits_percent` when known, `war`, `hidden`, `poisoned`, `dead`, `in_sight` (by the `sight_mode` option), `npc` (a guess: no human body, or nobody can harm it), `multi` (the house or boat it stands on), and what it wears (`worn`). |
| `find_items` | in the world | Filters: `graphic`, `graphics` (any of a list), `hue`, `container`, `name`, `distance`, `x` with `y` (the items on one tile), `z_min` and `z_max`. Each item has its location, dist, hue, `movable` (a player may lift it), `is_container`, and `multi` (the house or boat it is, or stands on). |
| `find_tiles` | map files, or the mock grid for the map underfoot | Map tiles of a kind in an area of any map, nearest first, one page at a time. What: `group` (`water`, `trees`, `ore`, `forge`, `anvil`, `loom`, `oven`, `mill`), `graphics` (land ids or static graphics), `flags` (tiledata flag names, all needed: `wet`, `impassable`, `surface`, `wall`, `door`, `roof`, `foliage` and the rest), `name` (a word of the tiledata name). Where: `x`, `y` and `radius` (default: the character and 12, most 64), or the box `x1`, `y1`, `x2`, `y2` (at most 129 tiles on a side); `map`; `layer` `land`, `statics` or `both`; `z_min` and `z_max`. `page` and `page_size` (default 50, most 200). Each tile has `x`, `y`, `z`, `layer`, `graphic`, `name`, `flags` and `dist`, with `total` and `pages`. |
| `map_tile` | map files or the mock grid | Everything on the tile `x`, `y` of any `map`: `land` (id, name, z, flags), each static (graphic, name, z, height, hue, flags), and on the map underfoot the `items` and the `multi_parts` on it. Also `walkable`, `door`, and the standing `z` from the height `z` (default: the character's). |
| `multi_parts` | a house or boat in view | `serial` gives every part of one house or boat. `x` and `y` give the parts of any of them on that tile. A house a player designed gives the parts the shard sent. Each part has `multi`, `graphic`, `name`, `x`, `y`, `z`, `height`, `flags` and `designed`. One page at a time. |
| `find_entrances` | map files | The ways into a dungeon round a spot: each run of stair or ladder statics once (with its `tiles`), the teleporter pads the character learned, and `dungeon` or `cave` landmarks. `x`, `y`, `radius` (default: the character and 32, most 64), `map`. Nearest first. |
| `find_landmarks` | in the world | Named places from the marker file. See [Landmarks](#landmarks). |
| `landmarks_info` | a session | The marker data loaded: `loaded`, `count`, `maps` and `kinds` with their counts. |
| `skill_gains` | a session | The skills gained this session. For each one: `gained` (points), `changes`, `value` and `per_hour`, and the newest changes in `recent`. `skill` narrows it to one. `clear` true starts the record again. |
| `look_around` | in the world | The surroundings in words: the ground underfoot, named furniture and walls, loose items and people. `radius` is optional. |
| `line_of_sight` | in the world | Whether one point sees another, as the shard judges a shot or a spell. To: `serial`, or `x`, `y` and `z`. From: the character's eyes, or `from` (a serial), or `from_x`, `from_y` and `from_z`. `mode` picks the rules: `runuo` (also `modernuo` and `servuo`; the default of the `sight_mode` option), `pol` (a window lets sight through), or `sphere` (one point on each tile crossed; walls and pieces that stop a shot block, windows do not). `trace` true gives every point of the line with what stops it (`land`, `piece` with its flags, `off_map`, `edge_of_world`, `too_far`). Without it, `blocked_at` names the first stop. Answers `{in_sight, from, aim, mode}`. |
| `route` | in the world | Plans the walk `move_to` would take, with the same choices (`x`, `y`, `z` or `name`; `accuracy`, `avoid`, `open_doors`, `roads`). It says if the spot is `reachable`, where the route `ends_at`, the `steps`, the `route` tiles, or `why` not, and the `search`: `nodes` opened, `ms`, and `flat` when the heights had to be ignored. It takes no step. |
| `journal_search` | a session | Journal lines that match `query`. With `since`, only newer lines; the answer is `{last_seq, lines}`. |
| `can_walk` | map files or the mock grid | `{walkable}`, and `why` when it is not: off the map, a mobile stands there, the shard refused it lately, a door, a named static, a building or an item, the ground, or nothing to stand on at that height. |
| `properties` | in the world | `serial`. The tooltip `lines` of one object, and `entries`: the same lines as the shard sent them, each a `cliloc` text number with its `arguments`, so a caller can read a property by its number in any language. When the session has none, it asks the shard, so the next call has them. |

`observe` also holds what the shard has open for the character: `shop` (the
goods and prices of a buy or sell list), `context_menu` (its lines), `menu`
(an old-style menu), `book` (its pages), and `paperdoll` (the last paperdoll
the shard opened: `serial`, the `text` at its top, and a `seq` that grows
with each one; the worn items are in that mobile's `equipment`). It holds
what the character knows: `skills` (those trained or locked, with value,
base, cap and lock), `spellbooks` (the spells of each book the shard has
sent), and `doing.last_walk` (how the last walk ended).

### Act

| Tool | Needs, and notes |
| --- | --- |
| `say`, `whisper` | In the world. The persona refuses `*emotes*` and empty text, and `say` has a rate limit (see [PERSONAS.md](PERSONAS.md)). A line has at most 128 characters. `say` takes `channel`: `say` (default), `yell`, `party`, `guild` or `alliance`. `hue` sets the colour of the line. The default is the `speech_hue` option, else the client's own. |
| `reply` | A line waits in `unanswered`. `text`, and `to` to pick the speaker. See [Talking to players](#talking-to-players). |
| `emote` | `allow_emote` in the persona. |
| `move_to` | In the world. `x` and `y` (and `z`), or `name` (the nearest landmark of that name). Choices: `run` (true runs, false walks; the default goes by stamina and danger), `accuracy` (stop within that many tiles, up to 18; the goal may then be a tree or an anvil), `open_doors` (default true; the shard's rules and the `no_doors_hidden` option still apply), `avoid` (a list of `{x, y, radius}` areas and `{serial, radius}` creatures to keep away from; `r` is a short form of `radius`, and the default is 3; a creature is read where it stands at each plan), `roads` (default true: grass and forest cost a little more, so the route keeps to roads), `exact` (fail instead of walking as near as a route goes). The answer names `heading_to` and the `route` search (`nodes`, `ms`, `steps`, `flat`). When one route cannot reach the goal (a wall or a high spot, a gap with a gate or a pad, or too far), it walks to the nearest spot it can reach on the way and answers `{partial:true, goal, heading_to, reason}`. Call `move_to` again from there. When a mobile or a refused step blocks the route a few steps ahead, the walk goes round it with a short detour and keeps the rest of the route. |
| `walk` | In the world. `direction` (`n`/`ne`/`e`/`se`/`s`/`sw`/`w`/`nw`; `dir` also works), `running` (`run` also works), `hold_ms` (0 = one step). `slide` true lets a held walk go on beside a wall. `force` true sends one step the map blocks. `open_doors` true first opens a shut door on the tile ahead, as a player's client with auto open doors does. |
| `open_door` | In the world. Opens a door on one of the 8 tiles around you. The character turns to it by itself (`0x12`/`0x58`). |
| `follow`, `stop` | `follow` needs a mobile serial. `stop` ends the follow, the travel, and a hunt, walk, loot or deposit job, as `cancel_goal` does. |
| `logout` | In the world. Sends the logout request. A shard may hold it until she is somewhere it allows, such as an inn or a house. The session closes the link when the shard lets her go. `then_play` (a character of the account) logs in as that character in the same session once the shard lets this one go. Without it, the session ends. |
| `use`, `single_click`, `attack`, `war_mode` | `serial`. A call with none is refused. `use` also takes `who` `last`. `war_mode` takes `on`. |
| `lift`, `drop`, `equip`, `unequip` | An item serial. `lift` takes `amount`. `equip` takes `layer` (default: the one-handed layer). `equip` with `who` `last` wears the last weapon put away, or asks the shard for the last weapon it remembers. `drop` takes `dest`: a container, or a mobile to give to. With none it drops at your feet. `x` and `y` give an exact place in the container, or a ground tile with `z` when there is no `dest`. `unequip` needs a `layer`. It refuses layer 0, the backpack, and layers above the bank layer. An empty layer answers `layer empty`. |
| `cast`, `use_skill` | In the world. `spell` or `skill` is needed, by number or by name (`greater heal`, `hiding`). `cast` takes `target`, a serial or `self`, and answers the spell's cursor with it when it comes. It takes `book`, a spellbook serial, to cast from that book. A skill no button uses is refused. |
| `wait_target` | Nothing. |
| `target` | `serial` or `who` answers the cursor that is open. With no cursor open, the target waits up to 5 seconds for the next one. `x` and `y` (a ground target) need a cursor that is open. A call with no arguments cancels the cursor. |
| `open_container`, `loot`, `trade_offer` | `serial`. A call with none is refused. `open_container` also takes `who` `last`. `loot` sends `job_ended` `loot: done` at the end. |
| `deposit` | A bank box is open: say `bank` beside a banker first, or the call is refused. It moves the pack into the box, one item a tick. `graphic` banks only those. It sends `job_ended` `deposit: done`. |
| `vendor_sell` | A vendor near: `vendor_name` and `graphic`. It says `sell` to the vendor and sells every pack item of that graphic from the list that comes. |
| `vendor_buy` | `vendor`, `item` (the serial of a row of `observe` `shop`) and `amount` (default 1). Open the buy list first. |
| `wait_journal` | A session. `q` and `timeout_ms` (default 5000, most 7000). Waits for a new journal line that holds the words. |
| `prompt_answer`, `prompt_cancel` | A prompt or a one-field dialog waits (`prompt_opened`). `text` is the answer. A prompt takes at most 128 characters. |
| `party` | In the world. `action` `invite` (with `serial`), `accept` or `decline` the invite that waits, `leave`, `kick` (with `serial`), or `loot` (with `on`). The loot choice shows in `watch` `party_can_loot`. It goes back to false when the party ends. |
| `mobile_status` | In the world. `serial`. Asks for the mobile's status, as a click on its health bar does. Its hits come back in `find_mobiles`. `close` true tells the shard its status bar is shut. A shard sends the mana and the stamina of a party member (and some shards of a pet). They show on that mobile as `mana`, `mana_max`, `stam` and `stam_max`. |
| `gump_respond`, `gump_close` | An open gump. `gump` names the gump id to answer (the oldest open one when left out). `button` is a button id. `switches` are the choices to tick. `texts` are the typed fields as `[{id, text}]` (`observe` lists them under `entries`). Button `0` closes. A button or switch that is not on the gump is refused. |
| `context_menu`, `close_menu` | `serial` alone asks for the context menu. Its lines come in `observe` `context_menu`, with a `context_menu_opened` event. With `index`, it picks that line. With `cliloc`, one call asks and picks. `close_menu` closes it with no pick. A shard with no context menus says so. |
| `shop_checkout`, `shop_close` | A shop list is open (`observe` `shop`). `items` is `[{serial, amount}]`. It buys or sells by the kind of list. |
| `dye` | A dye tub asks for a colour (`watch` `dye`). `hue` is the colour. |
| `menu_pick`, `book_close` | An old-style menu or a book is open (`observe` `menu`, `book`). `index` counts from 1. None walks away. |
| `book_read` | A book is open. `page` from 1: its lines when the shard sent them. Else the session asks the shard for the page (`asked` true), and the next call has it. |
| `book_write` | A book is open. `title` and `author` name it. `page` (from 1) and `text` write one page; a line break parts its lines. |
| `board_read`, `board_post`, `board_remove`, `board_close` | A bulletin board is open: `use` the board first. `watch` shows it under `board` with its `posts`. `board_read` takes `message`, and the lines come in `posts[].lines`. `board_post` takes `subject`, `text` and, for an answer, `reply_to`. `board_remove` takes `message`. |
| `map_pin`, `map_close` | A map item is open: `use` the map first. `watch` shows it under `maps` with its `pins` and `may_plot`. `map_pin` takes `x` and `y` in pixels of its picture; `action` `move` with `pin` (its place in the list, from 0) and `x`, `y`; `action` `remove` with `pin`; or `action` `clear` or `edit`. `map_close` takes `serial`, or none. |
| `profile` | In the world. `serial` asks for the profile a player wrote about a character. `text` writes your own. The words come back in `watch` under `profiles`. |
| `house_edit` | The house designer is open (`watch` `designing`). `action` `add`, `remove`, `stair`, `roof` or `remove_roof` with `graphic`, `x`, `y` (and `z` to remove); `floor` with `level`; and `clear`, `revert`, `commit`, `exit`, `backup`, `restore`, `sync` (the shard sends the design again). The parts to build with are in `watch` `house_parts`. |
| `house_content` | In the world. `show` true or false: whether the shard shows what stands inside public houses. |
| `help` | In the world. Asks the shard for its help menu. It answers with a gump. |
| `chat` | In the world. `action` `open` (with `name`), `join` (with `channel` and `password`), `create` (a new channel, with `channel` and `password`), `say` (with `text`), `leave`. `watch` shows the chat under `chat`. |
| `open_spellbook` | In the world. `kind` `magery`, `necromancy`, `chivalry`, `bushido`, `ninjitsu`, `spellweaving` or `mysticism`. Its spells come in `observe` `spellbooks`. |
| `tip`, `quest_arrow` | A tip of the day or a quest arrow is shown. `tip` takes `next` (false for the tip before). `quest_arrow` clicks the arrow; `right` true clicks it with the right button. `watch` shows them as `shard_tip` and `quest_arrow`. |
| `boat_move` | You pilot a boat. `direction`, and `speed` `stop`, `slow` or `fast` (default). |
| `track_members` | In a party or a guild. `who` `party` or `guild`. The places come in `observe` `tracked_members`, with a `member_positions` event. |
| `race_change` | A race change is open (`race_change_opened`). `observe` `race_change` has the `race`, `female`, `hair_styles` and `beard_styles` with their `graphic` and `name`, `skin_hues` and `hair_hues`. `hair` and `beard` are graphics (0 for none). `skin_hue`, `hair_hue` and `beard_hue` are hues from the lists. Each one you leave out takes the first choice. `cancel` true says no. |
| `trade_accept`, `trade_cancel`, `trade_gold` | A trade is open. See [Trades](#trades). `trade_gold` takes `gold` and `platinum`. |
| `virtue`, `virtue_gump` | In the world. `virtue` invokes one of the eight virtues by `name`: humility, sacrifice, compassion, spirituality, valor, honor, justice, honesty. Honor, sacrifice and valor go as the virtue macro. The others go as a press on their picture in the virtue gump. `virtue_gump` asks for the virtue gump of `serial`, or of the character. |
| `skill_lock`, `stat_lock` | In the world. `skill` (number or name) or `stat` (`str`, `dex`, `int`), and `lock` `up`, `down` or `locked`. |
| `rename` | A pet in view: `serial` and `name`. |
| `set_ability` | In the world. `ability` `primary`, `secondary`, `stun` or `disarm`. `on` false clears it. |
| `emote_action`, `fly`, `menu_button` | In the world. `emote_action` plays a body action (`action` `bow`, `salute`). `fly` takes off a gargoyle, or lands with `on` false. `menu_button` presses the paperdoll's `quests` or `guild` button (`which`). |
| `target_resource` | A harvest tool: `tool` and `resource` (`ore`, `sand`, `wood`, `graves`, `red mushrooms`), with no cursor. |
| `use_type` | In the world. Double-clicks the first item of `graphic`, of `hue` (default: any), in `source` (`backpack` by default, `ground`, `world`, or a container serial), within `range` on the ground. |
| `use_on` | In the world. Uses `item` on the mobile `target` with no cursor, as a bandage is used. It takes the time of one act. |
| `catch_bag` | In the world. `serial` sets the container loot goes into in place of the backpack. `clear` true clears it. With neither, it says which. Saved for each character. |
| `mount`, `dismount` | `mount` rides `serial`, the remount agent's mount, or the nearest pet of the character's that can be ridden. `dismount` gets off. War mode goes off first, so the double-click is no attack. In a fight both are refused. |
| `attack_nearest` | In the world. Attacks the nearest mobile in sight (by the `sight_mode` option) that may be harmed without a crime: never an innocent, a friend, a pet or a party member, or one nobody can harm. `notoriety` (`gray`, `criminal`, `enemy`, `murderer`), `species`, `name` and `distance` narrow it. The shard's rule on closest targets is obeyed. |
| `ignore_list` | A session. `list` `gumps` or `journal`; `action` `add`, `remove`, `clear` or `show` (default); `value` a gump id or words. An ignored gump is left out of `observe` and does not wake `next_event`. An ignored line (by speaker or words) is left out of `observe`, `journal_search` and `wait_journal`, and does not count as spoken to. Saved for each character. |
| `set_goal` | In the world. `goal`: see [Goals](#goals). |
| `cancel_goal` | A session. Ends the goal, and stops a hunt, walk, loot or deposit job (`job_ended` reason `stopped`). |
| `set_persona` | A session. A persona as JSON (see [PERSONAS.md](PERSONAS.md)). `typo_rate` is clamped to `0.0..=1.0`. |
| `jobs`, `job_start`, `job_stop` | A session, in the world, a job that runs. Hunt: `job` `hunt`, with `include` or `avoid` (not both). Walk: `job` `walk`, `x` and `y` or `name`, `watch`. `replace` true stops the old job (`job_ended` `stopped`) and then starts the new one. The job hands back with `job_ended`. See [playbooks/hunt.md](playbooks/hunt.md) and [playbooks/walk.md](playbooks/walk.md). |

### Goals

`set_goal` takes `goal`. A goal is a simple plan the session runs on each
tick. Before the goal, the session always takes care of the character
first: it runs away when health is under the persona's flee share, uses
bandages and potions, and fights back anyone who hit her in the last 30
seconds. A ghost says "i am dead", unless the goal is `ress`.

| Goal | What it does |
| --- | --- |
| `idle` | Nothing more. A name UOTerm does not know also gives `idle`. |
| `gather` (or `chop`, `lumber`) | Finds the nearest tree within 12 tiles, walks beside it, uses the axe the character carries and aims at the tree, one swing at a time. A tree the shard says is empty is left for 20 minutes. With no axe, no tree near, or a tool that does not work on a mount, the goal ends with `job_failed`. |
| `mine` | The same with a pickaxe or a shovel, on rock and cave floors. |
| `hunt` | Starts the hunt job with empty lists (see [playbooks/hunt.md](playbooks/hunt.md)). |
| `travel` | Walks to `x`, `y` (and `z`). With no spot, it walks to the nearest bank. It ends with `arrived`. On ground that may have hostiles, use the walk job instead. |
| `flee` | Walks away from what threatens the character. |
| `bank` | Walks to the nearest bank within 400 tiles, from the banks of the standard towns on every map, and ends there with `arrived`. With none that near, or on a shard with its own towns, `set_goal` refuses, and the agent must find a banker. |
| `shop` | Uses a nearby innocent mobile, else walks to the bank when it is near. |
| `social` | Says `yo`. |
| `ress` (or `res`) | A ghost walks to a healer in view and takes the offer to live again. With no healer in view, it walks to the nearest healer of the marker file, else to the nearest bank, where towns keep one. It ends with `job_ended` `ress: alive` when the character lives. |

Any goal other than `hunt` stops a hunt or walk job that runs.

### Scripts, agents and hotkeys

[SCRIPTS.md](SCRIPTS.md) explains the script language and its tools.
[AGENTS.md](AGENTS.md) explains the agents, hotkeys and recording, and the
`[options]` that change how tools work (`sight_mode`, `listen_range`,
`catch_bag`, the ignore lists, `speech_hue` and the others).

| Tool | Needs | Result |
| --- | --- | --- |
| `run_script` | no script in that slot | `name` or `text`, in a slot of its own (`slot`). Up to 8 scripts run side by side. `loop`, `for` and `iterations` as in SCRIPTS.md. |
| `stop_script`, `script_status`, `list_scripts`, `script_read`, `script_save` | a session | See SCRIPTS.md. |
| `hotkeys`, `hotkey` | a session, in the world | List by `group` or `name`; press by `name`. |
| `agents`, `agent_set`, `agent_on` | a session | The settings; replace the `settings` of an `agent` (or its `list`); switch it `on`. |
| `agent_run`, `agent_stop` | in the world | Run a job once: organizer, restock, dress, undress, autoloot. |
| `damage_meter`, `target_filter` | a session, in the world | `action` start, pause, resume, stop, report; pick with the filter `name`. |
| `record_macro` | a session | `action` start (with `name`), stop (saves), cancel. |

### Characters and login

These tools belong to the runtime, not to a session. Before a login there is
no session to call, so they take no `session_id`.

| Tool | Result |
| --- | --- |
| `characters` | The character list of an account: `slots` (each `slot`, `name` and `empty`) and `start_towns` (`index`, `town`, `building`). No character is played. |
| `character_create` | Makes a character and logs in as it. Answers `session_id`. `name` (2 to 16 letters, spaces, dashes, dots or quotes), `female`, `race` (`human`; `elf` from client 4.0.11d; `gargoyle` from 6.0.14.4; only human in the T2A era), `str`, `dex`, `int` (each 10 to 60, 80 in all, or 90 from client 7.0.16), `skills` (`[{skill, value}]`: 3, or 4 from client 7.0.16, each up to 50, 100 or 120 in all), `skin_hue`, `hair`, `hair_hue`, `beard`, `beard_hue`, `shirt_hue`, `pants_hue`, `profession` (a profession names its own stats and skills), `start_city` (an index of `start_towns`). UOTerm checks the rules before it asks the shard. A refusal of the shard comes back in its words. |
| `character_delete` | Deletes a character by `name` or `slot`, and answers the `slots` after. The shard may refuse, for example for a character played lately. |
| `connect` | Logs in as `character` (or the first of the account). Answers `session_id` and `character`. |
| `disconnect` | Ends the session `session_id` at once, with no logout. |

The account comes from a saved login, `profile`, or from `account` and
`password_env`. `profile` is the name of a saved login: a file of the
`logins` folder of the config folder, else of the `profiles` folder, as
`uoterm connect --profile` finds it. `password_env` is the name of an
environment variable that holds the password. A password is never an
argument, and it is never written to a log or given back. `host`, `port`,
`shard`, `era`, `version`, `encryption` and `proxy` default to the saved
login and to `uoterm.toml`.

To play another character of the account in the same session, `logout` with
`then_play`.

### Tools for a play window

These are for the native window and the web client, not for an agent. MCP
does not list `command`, `take_control`, `release_control` or `game_view`.

| Tool | Notes |
| --- | --- |
| `watch` | `observe` with each list at full length, and what only a screen draws: each container with all its items, `journal_lines` with hue and kind, every skill, `party_members` (with the `hits`, `mana` and `stam` of each, and their most, when the shard told them), `party_can_loot`, `multis`, `gump_layouts` (each gump piece with its place, page and pictures), `maps`, `profiles`, `designed_houses` (the walls and floors a player designed, with their offsets), `placing` (a building that waits for its place; answer with `target` and a tile), `chat`, `designing` (the house, the `floor` worked on, and the `plot_width` and `plot_depth` of its plot when the client files hold the foundation), `house_parts`, `cues` (damage, animations, effects, the death screen and a status bar the shard shut, each with a `seq` that counts up), `buff_icons` (each buff with its icon, title, text and seconds left), `live_map` (the map blocks an UltimaLive shard changed near the character), `season`, `light`, `weather`, `prompt`, `text_entry`, `target_cursor`, `board`, and the items of an open `trade`. It also carries `time`, `personal_light`, `quest_arrow`, `waypoints`, `shard_url`, `shard_notice` and `shard_tip` (the number of the tip of the day shown; none for a notice). |
| `command` | One script command line (`text`) as one act of a human. Needs `human` true. |
| `take_control`, `release_control` | While a human has control, only a call with `human` true acts. Control goes back by itself after 90 seconds with no human act. |
| `game_view` | `width` and `height`: the size in pixels of the game view the window draws. The window sends it once a new size has held for half a second. The shard hears it (`0xBF` `0x05`) at each login, and at once when it changes in the world. A session with no window tells 600 by 480. |

The shard may send the newer mobile packets `0xD2`, `0xD3`, `0x2D` and
`0xDE`. They fill the same fields as `0x77`, `0x78` and the stat bars, so
nothing changes for a driver. The gear of a corpse is in its container, and
a boat carries what stands on it.

## HTTP

`uoterm connect`, `uoterm populate` and `uoterm play` serve the runtime API
on `--api-bind`. `uoterm play` starts it after the first login. `uoterm web`
serves the same API on `--bind`, with the web client routes below. The
default address is `api_bind` in `uoterm.toml`, else `127.0.0.1:7733`.

| Route | Notes |
| --- | --- |
| `GET /health` | `ok`. Needs no token. |
| `GET /v1/sessions` | `{"sessions": [ids]}` |
| `POST /v1/sessions` | Makes a session. Body: `host`, `port`, `account`, `password`, `character`, and the optional `shard`, `era`, `version`, `obey_shard_rules`, `answer_when_named`, `play_along`, `reconnect` and `proxy`. The encryption is always `none`. Answers 201 `{"id": ...}`, or 502 `{"error": ...}` when the login fails. The client files and the markers come from `uoterm.toml`. |
| `GET /v1/sessions/{id}/state` | The `observe` JSON. When `observe` fails, a plain snapshot. |
| `POST /v1/sessions/{id}/tools/{name}` | The body is the JSON arguments of the tool. 200 when the tool succeeds, 409 when it refuses, 400 for a session that does not exist. The body is the tool result: `ok`, `result`, and `error` when it failed. |
| `POST /v1/tools/{name}` | A runtime tool (`connect`, `disconnect`, `characters`, `character_create`, `character_delete`), with JSON arguments. 200 or 409, as above. 404 for any other name. |
| `POST /v1/web/token` | Gives the token cookie. See below. |
| `GET /v1/sessions/{id}/live` | The live link. See below. |
| `GET /v1/login/live` | The login link. See below. |

A session id that does not exist gets 404 `{"error": "not found"}`, except
on the tool route.

### Who the API answers

Three rules apply to every route, the page of the web client too.

**Token.** When `UOTERM_API_TOKEN` is set to a value, every route except
`/health` and `/v1/web/token` needs the token. A caller sends it as
`Authorization: Bearer <token>`, or in the `uoterm_token` cookie. The CLI
and MCP send the header when the variable is set. The token may hold only
visible ASCII marks, without `"`, `,`, `;` or `\`, because it goes in a
cookie. The server refuses to start with any other token. A bind to an
address that is not loopback is refused unless the variable is set.

**Loopback.** An API bound to loopback (`127.0.0.1`, `localhost`, `::1`)
answers only a caller that names this machine in its `Host` header
(`127.0.0.1`, `localhost`, `::1` or `[::1]`, with any port), or in the
authority of an HTTP/2 request. Any other caller gets 403. This stops a web
page that points a name it owns at this machine. `/health` follows this rule
too.

**Origin.** A request with an `Origin` header must come from a page of this
server: the `Origin` must be `http://` followed by the `Host` of the
request, and on a loopback API that host must name this machine. Any other
`Origin` gets 403. A caller that is not a web page sends no `Origin`, so the
rule does not touch it.

Loopback with no token stays open for local tests. Do not bind the API to a
public address without a token.

### Token cookie

A web page cannot put headers on a WebSocket. So the page sends the token
once to `POST /v1/web/token`, with the body `{"token": "..."}`. The right
token gets 204 and a `uoterm_token` cookie (`HttpOnly; SameSite=Strict;
Path=/`). A wrong token, or any token when the server has none, gets 401.

### Live link

`GET /v1/sessions/{id}/live` is a WebSocket. The web client keeps one open to
the session it shows. A message from the page is 1 MiB at most. The server
calls the `watch` tool every 33 ms, and sends a `frame` only when it differs
from the last one it sent on this link. The radar size of the frames is 5
until the page sends a `size`.

| The page sends | Meaning |
| --- | --- |
| `{"kind":"call","id":1,"tool":"say","args":{...}}` | One tool call. |
| `{"kind":"act","id":2,"calls":[{"tool":"...","args":{...}}]}` | Calls made in order, 650 ms apart (`ACT_STEP_GAP_MS`). It stops at the first one that fails. 4 steps at most. |
| `{"kind":"size","size":N}` | The radar size the frames use. |

| The server sends | Meaning |
| --- | --- |
| `{"kind":"frame","watch":{...}}` | The result of the `watch` tool. |
| `{"kind":"answer","id":1,"ok":true,"result":{...}}` | The answer to the call or act with that `id`. A refused one has `ok: false` and `error`. The answer to an act is the result of its last call. |
| `{"kind":"ended"}` | The session is gone, and the link closes. |

Each call runs in a task of its own, so a tool that waits does not hold back
the other calls of the page. One link runs 8 calls at one time. A call past
that gets an error.

An act runs on the server, so a page that closes between a lift and its drop
does not leave the item in the hand. The acts of one session run one at a
time, whatever link sent them. One act runs and 4 wait. One more gets an
error. An act with no call, or with more than 4, gets an error. The server
skips a message it cannot read. It closes the link when the page takes more
than 5 seconds to take one message.

### Login link

`GET /v1/login/live` is a WebSocket, 64 KiB per message. The web client logs
in through it. The first message must come within 10 seconds, and it must
be a login:

```json
{"kind":"login","host":"127.0.0.1","port":2593,"account":"a","password":"p",
 "shard":"","character":"","era":null,"version":null,"encryption":"none"}
```

`shard`, `character`, `era`, `version` and `encryption` are optional. A blank
`shard` or `character` lets the page pick. The era and the version default
to those of `uoterm.toml`, and `encryption` is `none` or `osi`. The other
session options come from `uoterm.toml`. The password is used for this login
only, and it is kept nowhere.

The server sends each question of the login, and the page answers each one:

| The server sends | The page answers |
| --- | --- |
| `{"kind":"ask","ask":{"kind":"Shard","names":[...]},"version":"..."}` | `{"kind":"reply","reply":{"kind":"Pick","index":0}}` |
| `{"kind":"ask","ask":{"kind":"Characters","names":[...],"refused":null,"choices":{"towns":[...],"features":0,"list_flags":0}},"version":"..."}` | `{"kind":"reply","reply":{"kind":"Request","request":{"Play":0}}}` |
| `{"kind":"ready","session":"s1"}` | The page opens the live link of that session. The login link closes. |
| `{"kind":"failed","words":"..."}` | The login ended with a fault. The link closes. |

A character request is one of these:

- `{"Play":slot}` plays the character in that slot.
- `{"Delete":slot}` deletes it. The shard answers with a new list or a
  refusal.
- `{"Make":{...}}` makes a new character. The wish has `name`, `female`,
  `race`, `strength`, `dexterity`, `intelligence`, `skills` (a list of
  `[skill, value]` pairs), `skin_hue`, `hair`, `hair_hue`, `beard`,
  `beard_hue`, `shirt_hue`, `pants_hue`, `profession`, `start_city` and
  `slot`.
- `"Leave"` plays no character, and the login ends at the character list.

`version` is the client version the login speaks. A new character follows
it. `refused` holds the words of the last refusal of the shard. A reply that
does not fit the open question is not taken, and the question comes again.
A page that closes before the end plays no character: an open character
question is answered with `"Leave"`, and an open shard question ends the
login. A session the page cannot learn of is stopped. The words of a fault
in a message name only its kind and place, never its text.

## Web client routes

`uoterm web` serves the browser client: a Three.js page from the `web/`
folder, built with `npm run build`, that runs the WebAssembly build of
`crates/uoterm-web`. It shares `crates/uoterm-view` with the native play
window. `--web-dir` names the folder of the built page (default `web/dist`),
and `--uopath` the client files (default `uopath` in `uoterm.toml`).

`uoterm web` serves the routes below on top of those above, behind the same
token, loopback and origin rules. `uoterm connect`, `uoterm populate` and
`uoterm play` do not serve them. Any path under `/v1` (also `/v1` and
`/v1/`) with no route gets 404 `{"error": "not found"}`, whatever the
method. Any other path gets a file of the built page, or its `index.html`.
The page itself needs no token, so a browser on another machine can load it
and ask the player for the token. The loopback and origin rules still apply
to it.

### Pictures, map and tables

These need client files. With none, they answer 503. A picture or a table
that the browser keeps has an `ETag` from the version of the client files
and of UOTerm, with `Cache-Control: public, max-age=31536000, immutable`.
The server makes 4 pictures at one time.

| Route | Notes |
| --- | --- |
| `POST /v1/art` | JSON `ArtRequest`, tagged by `kind`: `Land`, `Texture`, `Item`, `Gump`, `Cursor`, `Text` or `Figure`. A PNG, with the point of the picture that goes on the tile in the `x-uoterm-anchor` header (`x,y`). 404 for no picture, 400 for one too large. |
| `POST /v1/text/measure` | Body `{"text": ..., "look": ...}`, as a `Text` picture asks. Answers `{"lines": [...], "line_height": N}`: the lines the words break into in a UO font, and the height of one line. 400 for words a picture would refuse, 404 with no UO fonts. |
| `GET /v1/gump-mask/{id}` | `{"width", "height", "bits"}`: the drawn pixels of a gump, one bit for each pixel, row by row from the top left, the lowest bit of each byte first, in base64. 404 for no gump. |

The map routes take an optional `?session={id}`. With it, the map has the
UltimaLive changes that session sent (see `POST /v1/sessions/{id}/map/live`).
Without it, the map files alone. A map picture of a facet that a live map
changed is sent with `Cache-Control: no-store`, so the browser does not keep
it.

| Route | Notes |
| --- | --- |
| `GET /v1/map/{map}/{block_x}/{block_y}` | The 64 tiles of one block, row by row from its north west corner. Never kept (`no-store`). 404 past the edge of the map. |
| `GET /v1/map/near/{map}/{x}/{y}` | PNG: the land round a tile in radar colours, with the tile in the middle. 404 when no tile round it has a colour. |
| `GET /v1/map-picture/{map}/{tx}/{ty}` | PNG: one tile of the whole-world picture of a map. 404 past the picture. |
| `GET /v1/map-item/{facet}/{start_x}/{start_y}/{end_x}/{end_y}` | PNG: the land of a map item between its corners. 400 when the end is not past the start. |
| `POST /v1/sessions/{id}/map/live` | Body: the `live_map` value of `watch`, 8 MiB at most. It lays the changes over the map files of that session, and answers the blocks that changed as `[{"map", "bx", "by"}]`, for the page to ask for again. 404 for a session that does not exist. The changes of a session are dropped when it ends. |

| Route | Notes |
| --- | --- |
| `GET /v1/data/{table}` | `tiledata`, `animdata`, `anim-rules`, `radarcol`, `seasons`, `cliloc` and `item-layers`. With an id: `multis/{id}` (`[]` when there is none), `lights/{id}` and `hues-text/{hue}`. |
| `GET /v1/data/frames/{body}/{action}/{direction}/{mounted}` | The frame count of a body for an action (`stand`, `walk`, `run` or a group number), a direction, on a mount or not. 400 for a question the animation files cannot hold. |
| `GET /v1/data/creation?towns=...` | What the character creation reads: `{"professions", "skill_names", "town_texts", "words", "hue_colors"}`. `towns` is a comma-separated list of the text numbers of the start-town words the shard offers (it may be empty or left out). `words` holds only the text numbers of the professions and of these towns (null when the client files have no `Cliloc.enu`). `hue_colors` gives `[r, g, b]` by hue for each hue of the palettes. 400 for a `towns` value that is not a list of numbers. |

A table the client files do not hold gets 404.

### Sound

| Route | Notes |
| --- | --- |
| `GET /v1/sound/{id}` | One sound as a WAV file (16-bit mono). 503 when the client files hold no sounds, 404 for no sound. |
| `GET /v1/music/{id}?midi=true` | The file of a track: MP3, or MIDI only with `midi=true` (a page with a sound font). The `x-uoterm-repeats` header says if the track repeats. 503 when the client files hold no music, 404 for no track. |
| `GET /v1/soundfont?shard=&character=` | The MIDI sound font the profile names: the profile of that character, or the default profile with neither. 400 for one of the two alone. 404 when the profile names none, or names a file that is no sound font, or a file outside the config folder and the client files folder. |

### Files of the config folder

These are the same files the native play window reads, so the options are
the same in both clients.

| Route | Notes |
| --- | --- |
| `GET`, `PUT /v1/profiles/default` | The default profile, as JSON. PUT answers 204. |
| `GET`, `PUT /v1/profiles/{shard}/{character}` | The profile of a character. `shard` is `host:port`; 400 without a port. A character with no profile has the default one. PUT answers 204. |
| `GET`, `PUT /v1/kept/{name}` | `watch-hotbar.toml` and `watch-grab-bags.toml`, read and written as JSON. PUT answers 204, or 400 for JSON the file cannot hold. `markers` gives the marker and zone files of the map folder, and is read only (PUT is 405). 404 for any other name. |
| `POST /v1/map-markers/user` | Changes the player's own marker file, 64 KiB at most. Body: `{"add": marker}`, `{"keep": {"at", "marker", "expected"}}` or `{"remove": {"at", "expected"}}`. 200 `{"changed": true}`; 400 for a bad marker; 409 when the file no longer holds the marker the change expects; 422 when the file is full. |
| `GET /v1/fonts`, `GET /v1/fonts/{name}` | The player fonts of the `Fonts` folder. Only a listed name is read. |
| `POST /v1/screenshots` | Body: a PNG, 32 MiB at most. Saves it in the `screenshots` folder. 201 `{"file": name}`; 400 for a body that is no PNG. |
| `GET /v1/logins` | The saved logins, as the login screen of `uoterm play` lists them, and the server of `uoterm.toml`: `{"host", "port", "logins": [{"name", "host", "port", "account", "shard", "character", "encryption", "era", "version"}]}`. No password. |
| `PUT /v1/logins/{name}` | Saves a login form under `name`, and gives the list. The body has `host`, `port`, `account`, and the optional `shard`, `character` and `encryption`. A body with any other field, such as a password, is refused. So is an empty name. |

### Jev

These run in UOTerm, so the TypeSafe key (`TYPESAFE_API_KEY`) never goes to
the browser. `GET /v1/jev` tells the page whether Jev can answer. The other
routes answer 503 when there is no key, 404 for an unknown session, and 409
when Jev gives no answer, each with `{"error": words}`.

| Route | Body | Answer |
| --- | --- | --- |
| `GET /v1/jev` | None | `{"on": bool}`: true when UOTerm has the key. |
| `POST /v1/sessions/{id}/jev/order` | `{"words": "...", "frame": {...}}` (`frame` is a `watch` result) | `{"act": {"calls": [{"tool", "args"}], "words": "..."}}`. The calls go on the live link as an act, each one marked `human`. `words` tells the player what was done. |
| `POST /v1/sessions/{id}/jev/pick` | `{"question": ..., "names": [...], "wish": "..."}`. `question` is `shard`, `profile`, `house_part`, `wear`, `channel` or `landmark`. | `{"index": N}`, or null when Jev is not sure. |
| `POST /v1/sessions/{id}/jev/lines` | `{"wish": "..."}` | `{"lines": [...]}`: the script lines of the hotkey the wish names. |

## MCP

```
uoterm mcp
```

Start `connect`, `populate`, `play` or `web` first. `uoterm mcp` speaks
JSON-RPC 2.0 on stdio (`protocolVersion` `2024-11-05`) and sends each call
to the HTTP API of `--api`. It reads newline JSON and `Content-Length`
framing. It skips a blank line, and answers bad JSON with `-32700`. A
`Content-Length` body larger than 1 MiB gets a `-32700` answer. The server
skips that body and goes on with the next message.

Methods: `initialize`, `ping`, `tools/list`, `tools/call`, `resources/list`,
`resources/read`. It takes the notifications `initialized`,
`notifications/initialized` and `notifications/cancelled` without an answer.

`tools/list` gives every tool an agent may call. Each tool has its own
arguments, their types, and the ones it cannot do without (`required`). A
tool that needs one of several says so in its description. The runtime's
tools come first and take no `session_id` (see
[Characters and login](#characters-and-login)). For every other tool,
`session_id` names the session a call is for. With none, the first session
answers. The tools of a play window are not listed.

A tool result comes as text, and as `structuredContent`. A failure has
`isError` true.

Resource URIs:

- `uo://session/{id}/state`: the observe JSON of that session.
- `uo://playbook/{name}`: a Markdown playbook (`text/markdown`), from
  `docs/playbooks/`.

| Playbook | Use |
| --- | --- |
| `driver` | The `next_event` loop and the order to act in |
| `login` | One `connect` owns the socket |
| `hunt` | The melee job: kill, loot own kills, run away |
| `walk` | A guarded walk that stops on a hostile |
| `navigation` | `move_to` or the walk job, doors, pads |
| `loot` | Corpses the hunt job did not make |
| `bank` | Walk to a banker, open the box, deposit |
| `death` | Come back as a ghost, then restock |
| `moongate` | Step onto the gate tile, then the gump |
| `dungeon` | Pads, stairs, z jumps |
| `mounts` | War mode off, then `use`; dismount is `use` on yourself |
| `runebook` | Recall from the book gump |
| `buy`, `sell` | The vendor keywords |
| `containers` | Open, lift, bags in bags |
| `talk` | `reply` and `unanswered` |
| `inspect` | Names, `look_around`, `find_*` |
| `equip` | Wear and take off |

Give the model `observe`, `say`, `move_to`, `job_start` and `next_event` to
start. Have it read the `driver` playbook first, then `hunt` or `walk`. Do
not ask it to walk tile by tile.

### The `screenshot` tool

The MCP server adds one tool that is not a session tool: `screenshot`. It
opens the native watch window for one picture, and gives it back as a JPEG
image, at most 1024 px on its long side. A vision model then sees what a
human sees: the real map, the mobiles with their names, and the panels. Use
it when the text radar is not enough, for example in a crowd or in a
dungeon. It needs a desktop, and it takes some seconds (30 at most). It is
not on the HTTP API.
