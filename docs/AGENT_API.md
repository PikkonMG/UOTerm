# Agent API

Tools return immediately with `action_id`. Completion is an event: `arrived` or `path_failed` for a walk, `job_ended` for a hunt, a walk, a loot, a bank deposit, a script or an agent job (with `<job>: done`, `stopped` or the reason), `job_failed` when one gives up, and `target_requested`, `gump_opened` and the others for what the shard sends.

## Banks

`observe` shows `nearest_bank`: the town, the `location` where its banker stands, and the `dist`, for the nearest bank on this map within 400 tiles. The banks are those of the standard towns on every map; a shard with its own towns has others, which you find by their bankers (`find_mobiles` with `name` `banker`). The `bank` goal walks to `nearest_bank` and ends there.

## Landmarks

`find_landmarks` reads named places from the marker file the runner set in the config (`markers`): gates, banks, towns. Filter by `name` (part of the place name), `kind` (the marker word, such as `bank` or `moongate`; for a file with no marker words, part of the name), `map` (the one underfoot by default), and `distance`. `closest` (a kind) gives the nearest place of that kind and no other. Each place has its `map`, `location`, `dist` (only on the current map) and `kind`. When the map underfoot has no match and the call names no map, the answer is `{places, note, elsewhere}`: the matches on the other maps, so you know to travel there first. `landmarks_info` says what marker data is loaded: `loaded`, `count`, and the count on each map and of each kind. `move_to` and `route` take `name` to walk to the nearest landmark of that name.

A town moongate is not in the map files: the shard drops it in as a live item. So walk to the landmark, then `find_items` with the gate graphic (blue `0x0F6C`, red `0x0DDA`) to lock the exact gate that stands there, step onto it, and answer the moongate gump. With no marker file loaded, `find_landmarks` fails with a message that says so.

A moongate is triggered by stepping onto its tile, not by `use`. A second `use` while already standing on the gate does nothing on most shards, so if the gump did not open, step off the tile and back onto it rather than calling `use` again.

Dungeon teleporter pads are not in the map files either. The character learns a pad the first time she uses one: stepping onto a tile and being teleported at once on the same map records that tile and where she landed. After that, a `move_to` whose goal is reachable only across a learned pad routes to the pad on its own (`{partial:true, via:"teleporter"}`); step on it and call `move_to` again from the far side. Moongates are told apart from pads and are not learned as pads.

## Gumps

A gump is a window the shard opens: a moongate, a bank question, a vendor menu. `observe` `gumps` and the `next_event` state show each open gump in words, with the text numbers read from the client files:

- `texts`: the words that are not a label.
- `buttons`: each `id` with the `label` beside it. A button with `to_page` only shows another page.
- `choices`: each round or square button with its `switch`, its `label`, and the `section`, the label of the page it is on.

To take the moongate to Moonglow on Trammel, tick the choice with that label and section and press OKAY: `gump_respond` with `button` 1 and `switches` [0].

## Trades

When another player opens a secure trade, you get a `trade_opened` event, and `observe` shows `trade`: the player, what `theirs` and `mine` hold, and `i_accept` and `they_accept`. Read what they offer before you agree. `trade_accept` ticks your accept box (`accept: false` unticks it); `trade_cancel` closes the trade; `trade_gold` sets the gold and platinum you offer. A change to either side clears both accept boxes, so accept again after it.

Several trades can be open at once. `observe` `trades` lists each, the newest last, with `serial` (the other player) and `box_serial` (your box of that trade); `trade` is the newest. `trade_accept`, `trade_cancel` and `trade_gold` take `trade` (the other player, or a box of the trade) to pick one; without it they act on the newest.

## The agent loop

An agent that drives a character must not miss what happens between its calls. Run one loop:

1. Call `next_event`. It returns the moment something important happens, or after `timeout_ms` (default 5000, max 7000) with no events.
2. Read `events` and `state`. Act on them in this order:
   0. `control_taken`: a human took the character through the watch window. Your goal, job and script are stopped. Each acting tool is refused with `a human has control of the character`. Look only (`observe`, `look_around`, `find_*`, `next_event`), and wait. `control_released` gives the character back: read `observe` again, because the human may have moved her. `state.human_control` and `observe.human_control` say which it is now.
   1. Danger: `died`, `low_health`, `damaged`, `enemy_near`, `combatant_changed`, `pk_flag`.
   2. Something waits for an answer: `target_requested`, `gump_opened`, `prompt_opened` (answer with `prompt_answer`), `trade_opened`, `party_invite` (answer with `party`), `shop_opened`, `context_menu_opened` and `menu_opened` (`observe` holds the goods, the lines or the entries), `race_change_opened` (answer with `race_change`).
   3. Chat: `spoken_to`, and `state.unanswered`. Answer with `reply`.
   4. A running job: read `doing.job`. Do not `move_to` or `attack` over a hunt or walk job.
   5. `job_ended`: a hunt or walk job handed back (`hunt: <reason>` or `walk: <reason>`). See [playbooks/hunt.md](playbooks/hunt.md) and [playbooks/walk.md](playbooks/walk.md). Then hunt, walk, bank, or rest.
   6. Your own task: `item_added`, `arrived`, `path_failed`, `lift_rejected`, `play_along_ended`, `map_changed` (a moongate or recall took her to another map; the old map's mobiles and items are gone, and a walk or follow there is dropped), `gump_closed`, `buff_changed` (a buff came on or went off, by icon), `system_message` (the shard's own words, such as "that is too far away"), `skill_changed` (`skill 27 70.3 (+0.1)`: the skill number, its value and the change), `stat_changed` (`strength 51 (+1)`), `ability_changed` (the armed weapon move is spent, or a stance came on or went off), `quest_arrow` (`shown at x,y` or `removed`), and `map_opened` (a map item opened; `watch` shows it under `maps`).
   8. Busy kinds, only when you ask for them with `ambient`: `sound`, `effect`, `animation` (each swing, cast and bow of every mobile in view), `item_deleted` and `member_positions`. Give `ambient: ["sound"]`, a list of kinds, or `["all"]`. A plain call skips them, and a full log drops them first.
   7. `disconnected`: the link to the shard dropped. The session logs in again by itself after 5 s, then after longer waits up to 60 s, and each call meanwhile is refused with the words that say so. `logged_in` comes when the character is back. The `reconnect` setting (`uoterm.toml`, and the session create body) turns this off. A `logout` ends the session for good.
3. Go back to 1.

Events wait in the session for you. When you are slow, the next call gives you all of them, in order, 50 at most per call. `missed` counts events that were dropped before you asked; the session keeps the last 256, and drops the busy kinds first.

A death shows the death screen: you get `died`, `state.dead` is true, and the session leaves war mode.

`state` holds `hits`, `mana` and `stam` with their maximums, `war`, `dead`, `location`, `combatant`, `enemies_near` (the 5 nearest mobiles you may fight, within 10 tiles, not party members or friends), `unanswered`, `chat_mode`, `target_cursor`, `pack` (`items`, `weight`, `weight_max`), and `doing`: the `goal`, the tile she is `walking_to`, whom she is `following` or `playing_along_with`, the corpse she is `looting`, whether she is `banking`, the first `script` running and the slots of all of them in `scripts`, and `job` (hunt or walk, with phase). Health under half is `low_health`. A loot or deposit job that gives up sends `job_failed` with the reason. A hunt or walk job that ends sends `job_ended` with `hunt: <reason>` or `walk: <reason>`. `job_start` with `replace` true stops the old job first (`job_ended` reason `stopped`), then starts the new one.

Read `doing` before you answer a player. Her words must match what she does: when she already follows the player, say "right behind you", not "I will stay here"; when she is already at the cows, do not say "lead the way". Never say she will do something unless you start it in the same step.

## Spoken to by name

When another character says your character's name, the session sends a `spoken_to` event. Each tool result then carries these lines in `unanswered`, whatever the tool, so an agent that is busy still sees them. A line leaves the list when it is answered (see `reply` below) or after three minutes. `observe` lists the lines of the last three minutes in `spoken_to`, answered or not. Each line has the speaker's `serial` and `name`, the `text`, the `channel` it came in (`say`, `whisper`, `yell`, `party`, `party_private`, `guild` or `alliance`), and `asks_if_bot`. Party, guild and alliance lines count even when the speaker is out of sight. To start a line yourself, use `say` with `channel`: `say` (default), `party`, `guild` or `alliance`. Answer with `reply`: it sends your text back in the channel the line came in (a yell is answered in a normal voice, a private party line only to that member). Give `to`, a name or serial, to pick the speaker when more than one waits; without it, the newest line is answered. A reply answers that speaker's lines only. A `say`, `whisper` or `emote` answers every waiting line said nearby, and a party, guild or alliance line answers the lines of that channel. A result with `unanswered` lines also has `chat_mode`, which `observe` shows too: `basic` or `play_along`. The name counts only as a whole word: "Tamara" does not name "Mara". System lines, spell words and your own lines never count. The `answer_when_named` setting turns this off.

The `listen_range` option (`agent_set` with agent `options`) makes a line said aloud by anyone that many tiles away or nearer count as said to the character, name or no name. It is off by default. The `ignore_list` tool (list `journal`) hides lines by speaker or words: they do not count as spoken to, and `observe`, `journal_search` and `wait_journal` leave them out.

Answer with `reply` the way a player would. What you may agree to depends on `chat_mode`:

- `basic` (the `play_along` setting is off): answer in a few friendly words and say no to every plan: hunting, following, a party, a trade, "come here". For example "not right now, busy" or "maybe later". Keep doing your own task. The client refuses `follow` and `partyaccept` for a player who asked in chat, unless that player is on the friends list.
- `play_along`: you may say yes to the plans the persona lists (see `docs/PERSONAS.md`): join the player's party (the client accepts the invite of a player who spoke to you), `follow` them, and `attack` what they fight when `fight` is listed. The client refuses a plan that is not listed. `observe` shows `playing_along` (the player and the minutes left). When the persona's time is up, or the character is hurt past its play-along risk, the character stops following and you get a `play_along_ended` event: say a short goodbye and go back to your own task. Talk the way `reply_style` says, when it is given.

In both modes:

- Keep it short and plain. One line is enough; no long talk.
- Stay in character as a player. Never bring up AI, bots, programs or "a game" on your own.
- When `asks_if_bot` is true, laugh it off ("lol, you're funny"), dodge ("too busy chopping to chat"), or just keep working. Never flatly say "I am a real person" or "I am not a bot".
- Never say the same line twice. Change the words each time. The session refuses a line said a short time before.

Start `uoterm connect` or `uoterm populate` first. Then drive the session with CLI, HTTP, or `uoterm mcp`.

## Perception

| Tool | Precondition | Result |
| --- | --- | --- |
| `observe` | session exists | self, radar, journal, mobiles, items, target, gumps, doors, buffs, party, prompt, forbidden (assistant features the shard forbids), `abilities` (the armed weapon move and the spells and stances on), `tracked_members` (party and guild members out of sight, after `track_members`) and `latency_ms` (the last round trip to the shard). Optional `size` (5-41, default 21) sets the radar width. `GET /state` stays at 21. |
| `find_mobiles` | in world | filter `name` / `graphic` / `distance`, `notoriety` (`innocent`, `friend`, `gray`, `criminal`, `enemy`, `murderer`, `invulnerable` or `any`), `species` (read from the body, so a named orc is an orc), `in_sight`, and `z_min` / `z_max`; `name` also matches the title, so `banker` finds "Kate the banker". Each has its title, `species`, `notoriety`, `hits_percent` when known, `war`, `hidden`, `poisoned`, `dead`, `in_sight` (by the `sight_mode` option), `npc` (a guess: no human body, or nobody can harm it), `multi` (the house or boat it stands on) and what it wears (`worn`) |
| `find_items` | in world | filter `graphic`, `graphics` (any of a list), `hue`, `container`, `name`, `distance`, `x` with `y` (the items on one tile), and `z_min` / `z_max`; each has its location, dist, hue, `movable` (a player may lift it), `is_container`, and `multi` (the house or boat it is, or stands on) |
| `find_tiles` | map files, or the mock grid for the map underfoot | map tiles of a kind in an area of any map, nearest first, a page at a time. What: `group` (`water`, `trees`, `ore`, `forge`, `anvil`, `loom`, `oven`, `mill`), `graphics` (land ids or static graphics), `flags` (tiledata flag names, all needed: `wet`, `impassable`, `surface`, `wall`, `door`, `roof`, `foliage` and the rest), `name` (a word of the tiledata name). Where: `x`, `y` and `radius` (default the character and 12, most 64), or the rectangle `x1`, `y1`, `x2`, `y2` (at most 129 tiles on a side); `map`; `layer` `land`, `statics` or `both`; `z_min` / `z_max`. `page` and `page_size` (default 50, most 200). Each tile has `x`, `y`, `z`, `layer`, `graphic`, `name`, `flags` and `dist`, with `total` and `pages` |
| `map_tile` | map or mock grid | everything on the tile `x`, `y` of any `map`: `land` (id, name, z, flags), each static (graphic, name, z, height, hue, flags), and on the map underfoot the `items` and the `multi_parts` on it; with `walkable`, `door` and the standing `z` from the height `z` (default the character's) |
| `multi_parts` | a house or boat in view | `serial` for every part of one house or boat, or `x` and `y` for the parts of any of them on that tile. A house a player designed gives the parts the shard sent. Each part has `multi`, `graphic`, `name`, `x`, `y`, `z`, `height`, `flags` and `designed`; a page at a time |
| `find_entrances` | map files | the ways into a dungeon round a spot: each run of stair or ladder statics once (with its `tiles`), the teleporter pads the character learned, and `dungeon` or `cave` landmarks. `x`, `y`, `radius` (default the character and 32), `map`; nearest first |
| `landmarks_info` | session exists | the marker data loaded: `loaded`, `count`, `maps` and `kinds` with their counts |
| `skill_gains` | session exists | the skills gained this session: for each, `gained` (points), `changes`, `value` and `per_hour`, and the newest changes in `recent`. `skill` narrows it to one; `clear` true starts the record again |
| `look_around` | in world | the surroundings in words: the surface underfoot, named furniture and walls, loose items and people. Optional `radius` |
| `line_of_sight` | in world | whether one point sees another, as the shard judges a shot or a spell. To: `serial`, or `x`, `y` and `z`. From: the character's eyes, or `from` (a serial), or `from_x`, `from_y` and `from_z`. `mode` picks the rules: `runuo` (also `modernuo`, `servuo`; the default of the `sight_mode` option), `pol` (a window lets sight through) or `sphere` (one point on each tile crossed; walls and pieces that stop a shot block, windows do not). `trace` true gives every point of the line with what stops it (`land`, `piece` with its flags, `off_map`, `edge_of_world`, `too_far`); without it, `blocked_at` names the first stop. `{in_sight, from, aim, mode}` |
| `route` | in world | plans the walk `move_to` would take, with the same choices (`x`, `y`, `z` or `name`; `accuracy`, `avoid`, `open_doors`, `roads`), and says `reachable`, where the route `ends_at`, the `steps`, the `route` tiles, or `why` not, and the `search`: `nodes` opened, `ms`, and `flat` when the heights had to be ignored. No step is taken |
| `find_landmarks` | in world | named places from the marker file (gates, banks, towns); filter name / map / distance; each has its map, location, dist (on the current map) and kind |
| `journal_search` | session exists | matching lines |
| `can_walk` | map or mock grid | `{walkable}`, and `why` when it is not: off the map, a mobile stands there, the shard refused it lately, a door, a named static, a building or an item, the ground, or nothing to stand on at that height |

## Action

| Tool | Precondition |
| --- | --- |
| `say` / `whisper` | in world; persona rejects `*emotes*` and empty text; `say` is rate-limited. `say` takes `channel`: `say` (default), `yell`, `party`, `guild` or `alliance`. `hue` sets the colour of the line; the default is the `speech_hue` option, else the client's own |
| `emote` | `persona.allow_emote` |
| `move_to` | in world; `x` and `y`, or `name` (the nearest landmark of that name). Choices: `run` (true runs, false walks; default by stamina and danger), `accuracy` (stop within that many tiles, up to 18; the goal may then be a tree or an anvil), `open_doors` (default true; the shard's rules and the `no_doors_hidden` option still apply), `avoid` (a list of `{x, y, radius}` areas and `{serial, radius}` creatures to keep away from; a creature is read where it stands at each plan), `roads` (default true: grass and forest cost a little more, so the route keeps to roads), `exact` (fail instead of walking as near as a route goes). The answer names `heading_to` and the `route` search (`nodes`, `ms`, `steps`, `flat`). When the goal cannot be reached in one route (a wall or up-high spot, a gate/teleporter gap, or too far), it walks to the nearest reachable spot on the way and returns `{partial:true, goal, heading_to, reason}` instead of a bare failure; call `move_to` again from there. When a mobile or a refused crossing blocks the route a few steps ahead, the walk mends that part with a short detour and keeps the rest of the route |
| `walk` | in world. `direction` (`n`/`ne`/`e`/`se`/`s`/`sw`/`w`/`nw`), `running`, `hold_ms` (0 = one step). `slide` true lets a held walk go on beside a wall; `force` true sends one step the map blocks; `open_doors` true opens a shut door on the tile ahead first, as a player's client with auto open doors does |
| `open_door` | in world; stand next to the door and face it (`0x12`/`0x58`) |
| `follow` / `stop` | `follow` needs a mobile serial |
| `logout` | in world. Sends the logout request; a shard may hold it until she is somewhere it allows, such as an inn or a house. The session closes the link when the shard grants it. `then_play` (a character of the account) logs in as that character in the same session once the shard lets this one go; without it the session ends |
| `use` / `single_click` / `attack` / `war_mode` | `serial` (a call with none is refused); `use` also takes `who` `last`. `war_mode` takes `on` |
| `lift` / `drop` / `equip` / `unequip` | item serial. `equip` with `who` `last` wears the last weapon put away, or asks the shard for the last weapon it remembers. `drop` takes `dest`: a container, or a mobile to give to; none drops at your feet. `x` and `y` give an exact place in the container, or a ground tile with `z` when there is no `dest`. `unequip` needs a valid `layer`. Empty layer returns `layer empty`. It does not unequip the backpack |
| `cast` / `use_skill` | in world; `spell` or `skill` is required, by number or by name (`greater heal`, `hiding`). `cast` takes `target`, a serial or `self`, and answers the spell's cursor with it when it comes, and `book`, a spellbook serial, to cast from that book. A skill no button uses is refused |
| `wait_target` | none |
| `target` | a target cursor must be pending |
| `open_container` / `loot` / `trade_offer` | `serial` (a call with none is refused). `loot` sends `job_ended` `loot: done` at the end |
| `deposit` | a bank box is open (say `bank` beside a banker). Moves the pack into it, one item a tick; optional `graphic` banks only those. Sends `job_ended` `deposit: done` |
| `vendor_sell` | a vendor near: `vendor_name` and `graphic`. Says `sell` to the vendor and sells every pack item of that graphic from the list that comes |
| `vendor_buy` | a buy list is open (`observe` `shop`): `vendor`, `item` (the shop item serial) and `amount` (default 1) |
| `wait_journal` | session exists; `q` and `timeout_ms` (default 5000, max 7000): waits for a new journal line that holds the words |
| `prompt_answer` / `prompt_cancel` | a prompt or a one-field dialog waits (`prompt_opened`): `text` is the answer. A prompt takes at most 128 characters |
| `party` | in world; `action` `invite` (with `serial`), `accept` or `decline` the invite that waits, `leave`, `kick` (with `serial`), or `loot` (with `on`). The loot choice shows in `watch` `party_can_loot`, and it goes back to false when the party ends |
| `mobile_status` | in world; `serial`. Asks for the mobile's status, as a click on its health bar: its hits come back into `find_mobiles`. `close` true tells the shard its status bar is shut. A shard sends the mana and the stamina of a party member (and some shards of a pet); they show on that mobile as `mana`, `mana_max`, `stam` and `stam_max` |
| `gump_respond` / `gump_close` | open gump. `gump` names the gump id to answer (the oldest open one when omitted). `button` is a button id, `switches` the choices to tick, `texts` the typed fields as `[{id, text}]` (`observe` lists them under `entries`); button `0` closes. A button or switch that is not on the gump is refused, because a shard drops or disconnects on it |
| `set_goal` | in world; `idle` `travel` `hunt` `gather` (or `chop`) `mine` `bank` `shop` `social` `flee` `ress`. `hunt` starts the hunt job with empty lists. `gather` and `mine` walk to the nearest tree or rock of the map, use the axe or the pickaxe carried, and aim at the spot; a spot the shard says is empty is left for 20 minutes, and the goal ends with `job_failed` when there is no tool or nothing near. `flee` runs away from what threatens the character. `ress` walks a ghost to a healer in view, or to the nearest healer of the marker file, or else to the nearest bank, and takes the healer's offer; it ends with `job_ended` `ress: alive` |
| `set_persona` | session exists; JSON persona body. `typo_rate` is clamped to `0.0..=1.0` |
| `cancel_goal` | session exists; also stops a hunt or walk job (`job_ended` reason `stopped`) |
| `watch` | For a window, not for an agent. `observe` with each list at full length, and what only a screen draws: each container with all its items, `journal_lines` with hue and kind, every skill, `party_members` (with the `hits`, `mana` and `stam` of each, and their most, when the shard told them), `party_can_loot`, `multis`, `gump_layouts` (each gump piece with its place, page and pictures), `maps`, `profiles`, `designed_houses` (the walls and floors a player designed, with their offsets), `placing` (a building that waits for its place; answer with `target` and a tile), `chat`, `designing` (the house, the `floor` worked on, and the `plot_width` and `plot_depth` of its plot when the client files hold the foundation), `house_parts`, `cues` (damage, animations, effects, the death screen and a status bar the shard shut, each with a `seq` that counts up), `buff_icons` (each buff with its icon, title, text and seconds left), `live_map` (the map blocks an UltimaLive shard changed near the character), `season`, `light`, `weather`, `prompt`, `text_entry`, `target_cursor`, `board`, and the items of an open `trade` |
| (`observe` panels) | `observe` also holds what the shard has open for the character: `shop` (the goods and prices of a buy or sell list), `context_menu` (its lines), `menu` (an old-style menu), `book` (its pages), `paperdoll` (the last paperdoll the shard opened: `serial`, the `text` at its top, and a `seq` that grows with each one; the worn items are in that mobile's `equipment`), and what he knows: `skills` (those trained or locked, with value, base, cap and lock), `spellbooks` (the spells of each book the shard has sent), and `doing.last_walk` (how the last walk ended) |
| `properties` | in world; `serial`. The tooltip `lines` of one object, and `entries`: the same lines as the shard sent them, each a `cliloc` text number with its `arguments`, so a caller reads a property by its number in any language. It asks the shard when the session has none, so the next call has them |
| `context_menu` with `serial` only | Asks for the context menu; its lines come in `observe` `context_menu` with a `context_menu_opened` event. With `index`, picks that line. `close_menu` closes it with no pick. With `cliloc`, one call asks and picks. A shard with no context menus says so |
| `shop_checkout` / `shop_close` | a shop list is open (`observe` `shop`). `items` is `[{serial, amount}]`. It buys or sells by the kind of list |
| `dye` | a dye tub asks for a colour (`watch` `dye`): `hue` is the colour |
| `menu_pick` / `book_close` | an old-style menu or a book is open (`observe` `menu`, `book`). `index` counts from 1; none walks away |
| (protocol) | The shard may send the newer mobile packets `0xD2`, `0xD3`, `0x2D` and `0xDE`. They fill the same fields as `0x77`, `0x78` and the stat bars, so nothing changes for a driver. `watch` also carries `time`, `personal_light`, `quest_arrow`, `waypoints`, `shard_url`, `shard_notice` and `shard_tip` (the number of the tip of the day shown, none for a notice); the gear of a corpse is in its container, and a boat carries what stands on it |
| `book_write` | a book is open. `title` and `author` name it; `page` (from 1) and `text` write one page, its lines parted by a line break |
| `board_read` / `board_post` / `board_remove` / `board_close` | a bulletin board is open: `use` the board first, and `watch` shows it under `board` with its `posts`. `board_read` takes `message` and the lines come in `posts[].lines`. `board_post` takes `subject`, `text` and, for an answer, `reply_to`. `board_remove` takes `message` |
| `script_read` / `script_save` | `script_read` gives the text of one script: `name`. `script_save` takes `name` and `text`; a script that does not parse is refused with the line of the fault. `hotkeys` with `name` gives one hotkey with the script lines it runs |
| `map_pin` / `map_close` | a map item is open: `use` the map first, and `watch` shows it under `maps` with its `pins` and `may_plot`. `map_pin` takes `x` and `y` in pixels of its picture, `action` `move` with `pin` (its place in the list, from 0) and `x`, `y`, `action` `remove` with `pin`, or `action` `clear` or `edit`. `map_close` takes `serial` or none |
| `profile` | in world; `serial` asks for the profile a player wrote about a character, and `text` writes your own. The words come back in `watch` under `profiles` |
| `house_edit` | the house designer is open (`watch` `designing`). `action` `add`, `remove`, `stair`, `roof`, `remove_roof` with `graphic`, `x`, `y` (and `z` to remove); `floor` with `level`; and `clear`, `revert`, `commit`, `exit`, `backup`, `restore`, `sync` (the shard sends the design again). The parts to build with are in `watch` `house_parts` |
| `help` | in world. Asks the shard for its help menu. It answers with a gump |
| `chat` | in world. `action` `open` (with `name`), `join` (with `channel` and `password`), `create` (a new channel, with `channel` and `password`), `say` (with `text`), `leave`. `watch` shows the chat under `chat` |
| `book_read` | a book is open. `page` from 1: its lines when the shard sent them; otherwise the shard is asked for the page (`asked` true) and the next call has it |
| `open_spellbook` | in world. `kind` `magery`, `necromancy`, `chivalry`, `bushido`, `ninjitsu`, `spellweaving` or `mysticism`. Its spells come in `observe` `spellbooks` |
| `tip` / `quest_arrow` | a tip of the day or a quest arrow is shown. `tip` takes `next` (false for the tip before). `quest_arrow` clicks the arrow; `right` true clicks it with the right button |
| `boat_move` | piloting a boat. `direction` and `speed` `stop`, `slow` or `fast` |
| `track_members` | in a party or a guild. `who` `party` or `guild`: the places come in `observe` `tracked_members` with a `member_positions` event |
| `house_content` | in world. `show` true or false: whether the shard shows what stands inside public houses |
| `race_change` | a race change is open (`race_change_opened`; `observe` `race_change` has the `race`, `female`, `hair_styles` and `beard_styles` with their `graphic` and `name`, `skin_hues` and `hair_hues`). `hair` and `beard` are graphics (0 for none), `skin_hue`, `hair_hue` and `beard_hue` hues from the lists; each one left out takes the first choice. `cancel` true says no |
| `trade_gold` | a trade is open; `gold`, `platinum`, and `trade` to pick one of several |
| `virtue` / `virtue_gump` | in world. `virtue` invokes one of the eight virtues by `name`: humility, sacrifice, compassion, spirituality, valor, honor, justice, honesty. Honor, sacrifice and valor go as the virtue macro; the others as a press on their picture in the virtue gump. The shard answers a virtue it has no power for. `virtue_gump` asks for the virtue gump of `serial`, or of the character |
| `skill_lock` / `stat_lock` | in world. `skill` (number or name) or `stat` (`str`, `dex`, `int`), and `lock` `up`, `down` or `locked` |
| `rename` | a pet in view: `serial` and `name` |
| `set_ability` | in world. `ability` `primary`, `secondary`, `stun` or `disarm`; `on` false clears it |
| `emote_action` / `fly` / `menu_button` | in world. `emote_action` plays a body action (`action` `bow`, `salute`). `fly` takes off a gargoyle, or lands with `on` false. `menu_button` presses the paperdoll's `quests` or `guild` button (`which`) |
| `target_resource` | a harvest tool: `tool` and `resource` (`ore`, `sand`, `wood`, `graves`, `red mushrooms`), with no cursor |
| `use_type` | in world. Double-clicks the first item of `graphic`, of `hue` (default any), in `source` (`backpack` by default, `ground`, `world`, or a container serial), within `range` on the ground |
| `use_on` | in world. Uses `item` on the mobile `target` with no cursor, as a bandage is. It takes an action's time |
| `catch_bag` | in world. `serial` sets the container loot goes into in place of the backpack; `clear` true clears it; with neither it says which. Saved per character |
| `mount` / `dismount` | `mount` rides `serial`, the remount agent's mount, or the nearest pet of the character's that can be ridden; `dismount` gets off. War mode goes off first, so the double-click is no attack; in a fight both are refused |
| `attack_nearest` | in world. Attacks the nearest mobile in sight (by the `sight_mode` option) that may be harmed without a crime: never an innocent, a friend, a pet or a party member, or one nobody can harm. `notoriety` (`gray`, `criminal`, `enemy`, `murderer`), `species`, `name` and `distance` narrow it. The shard's rule on closest targets is obeyed |
| `ignore_list` | session exists. `list` `gumps` or `journal`, `action` `add`, `remove`, `clear` or `show` (default), `value` a gump id or words. An ignored gump is left out of `observe` and does not wake `next_event`; an ignored line (by speaker or words) is left out of `observe`, `journal_search` and `wait_journal` and does not count as spoken to. Saved per character |
| `command` | For the watch window. One script command line (`text`) as one act of a human; needs `human` true |
| `take_control` / `release_control` | For the watch window, not for an agent. While a human has control, only a call with `human` true acts. Control goes back by itself after 90 s with no human act |
| `game_view` | For the watch window, not for an agent. `width` and `height`: the size in pixels of the game view the window draws. The shard hears it (`0xBF` `0x05`) at each login, and at once when it changes in the world. A session with no window tells 600 by 480 |
| `jobs` / `job_start` / `job_stop` | session exists / in world / a job is running. Hunt: `job` `hunt`, optional `include` and `avoid`. Walk: `job` `walk`, `x` and `y` or `name`, `watch`. `replace` true stops the old job (`job_ended` `stopped`) then starts the new one. Hands back with `job_ended`. See [playbooks/hunt.md](playbooks/hunt.md) and [playbooks/walk.md](playbooks/walk.md) |

## Scripts, agents, hotkeys and macros

See [SCRIPTS.md](SCRIPTS.md) for the script language and
[AGENTS.md](AGENTS.md) for agents, hotkeys and recording.

| Tool | Precondition | Result |
| --- | --- | --- |
| `run_script` | no script in that slot | `name` or `text`, in a slot of its own (`slot`; default the script name, or `text`). Up to 8 scripts run side by side, a healer beside a task, and share the character's pace: each tick they take turns, and once one sends something the others wait for the gap after it. `loop` runs it again each time it ends; `for` (seconds) and `iterations` (runs from the top) end it by themselves |
| `stop_script` / `script_status` / `list_scripts` | session exists | `stop_script` stops one `slot`, or every script; `script_status` gives one `slot` (status `running`, `suspended`, `done`, `stopped`, `failed`, with `line`, `iterations`, `ended_by` and its `output`), or with no slot the first running one with `slots`, `ended` and `shown`; saved names |
| `hotkeys` / `hotkey` | session exists / in world | list by `group`; press by `name` |
| `agents` / `agent_set` / `agent_on` | session exists | settings; replace `settings` of an `agent` (or its `list`); switch `on` |
| `agent_run` / `agent_stop` | in world | run a job once: organizer, restock, dress, undress, autoloot |
| `damage_meter` / `target_filter` | session exists / in world | `action` start, pause, resume, stop, report; pick with filter `name` |
| `record_macro` | session exists | `action` start (with `name`), stop (saves), cancel |

## Characters and login

These tools belong to the runtime, not to a session: before a login there is no session to call. They take no `session_id`.

| Tool | Result |
| --- | --- |
| `characters` | the character list of an account: `slots` (each `slot`, `name` and `empty`) and `start_towns` (`index`, `town`, `building`). No character is played |
| `character_create` | makes a character and logs in as it; answers `session_id`. `name` (2 to 16 letters, spaces, dashes, dots or quotes), `female`, `race` (`human`, `elf` from client 4.0.11d, `gargoyle` from 6.0.14.4; human only in the T2A era), `str`, `dex`, `int` (each 10 to 60, 80 in all, or 90 from client 7.0.16), `skills` (`[{skill, value}]`: 3, or 4 from client 7.0.16, each up to 50, 100 or 120 in all), `skin_hue`, `hair`, `hair_hue`, `beard`, `beard_hue`, `shirt_hue`, `pants_hue`, `profession` (a profession names its own stats and skills), `start_city` (an index of `start_towns`). The rules are checked before the shard is asked; the shard's refusal comes back in its words |
| `character_delete` | deletes a character by `name` or `slot`, and answers the `slots` after. The shard may refuse, for example a character played lately |
| `connect` | logs in as `character` (or the first of the account) and answers `session_id` and `character` |
| `disconnect` | ends the session `session_id` at once, with no logout |

The account comes from a saved login, `profile` (a file of the `profiles` folder, as `uoterm connect --profile` reads), or from `account` and `password_env`: the name of an environment variable that holds the password. A password is never an argument, and it is never written to a log or given back. `host`, `port`, `shard`, `era`, `version`, `encryption` and `proxy` default to the saved login and to the config file (`uoterm.toml`).

To play another character of the account in the same session, `logout` with `then_play`.

A session reaches the shard through a proxy when the config file (`proxy`), the session create body (`proxy`) or the call names one: `socks5://host:port` or `http://host:port` (HTTP CONNECT), with `user:password@` before the host when the proxy asks for a login. The log shows the proxy with its password left out.

## Options that change how tools work

`agent_set` with agent `options` sets these per character, beside the agent options of [AGENTS.md](AGENTS.md):

| Option | Effect |
| --- | --- |
| `sight_mode` | `runuo` (default), `pol` or `sphere`: the rules `line_of_sight`, `find_mobiles` `in_sight` and `attack_nearest` judge sight by |
| `listen_range` | a line said aloud this many tiles away or nearer counts as said to the character; none is off |
| `catch_bag` | the container `loot` fills in place of the backpack (the `catch_bag` tool sets it) |
| `ignore_gumps` / `ignore_journal` | the ignore lists (the `ignore_list` tool sets them) |
| `speech_hue` | the colour the character speaks in; none is the client's own |

## HTTP

Default bind: `http://127.0.0.1:7733`.

| Route | Notes |
| --- | --- |
| `GET /health` | No token |
| `GET /v1/sessions` | Session ids |
| `GET /v1/sessions/{id}/state` | Observe JSON |
| `POST /v1/sessions/{id}/tools/{name}` | Tool body is JSON args |
| `POST /v1/tools/{name}` | A tool of the runtime (`connect`, `disconnect`, `characters`, `character_create`, `character_delete`); JSON args. 404 for any other name |
| `POST /v1/sessions` | Create a session. Password is in the body. `proxy` is optional |

When `UOTERM_API_TOKEN` is set, every route except `/health` requires `Authorization: Bearer <token>`. A non-loopback `--api-bind` is refused unless that variable is set. CLI and MCP send the same header when the variable is set.

An API bound to this machine answers only a caller that names this machine in its `Host` header (`127.0.0.1`, `localhost` or `::1`, with any port). A web page that points a name it owns at this machine names itself, and is refused with 403.

## Names on a shard with no property lists

A shard says at login whether it sends property lists, the tooltips that name every object. When it does not, the session reads names the way a player does, one every half second: it asks for the name of each nameless mobile, as the client asks for the name it shows over a head, and clicks each nameless item once and takes the name the shard shows over it. What a click tells of an item comes back in `properties`, in the form of the shard's era: on the oldest shards, the label lines, which say it all in words (for example `a vanquishing katana crafted by Bob`, or a bag's name and then what it holds); on later ones, the click info (its name, its maker, its quality and magic, whether the magic is known, its charges). `properties` asks again by a new click when the last answer is more than 10 s old, and never more than once a second for one object.

## MCP

```
uoterm mcp
```

JSON-RPC 2.0 on stdio (`protocolVersion` `2024-11-05`). Newline JSON and `Content-Length` framing. A blank line is skipped. Bad JSON returns `-32700`. Bodies larger than 1 MiB are rejected.

Methods: `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`.

`tools/list` gives every tool an agent may call, each with its own arguments, their types, and the ones it cannot do without (`required`); a tool that needs one of several says so in its description. The runtime's tools come first and take no `session_id` (see [Characters and login](#characters-and-login)). For every other tool, `session_id` names the session a call is for; with none, the first session answers. The window's own tools (`command`, `take_control`, `release_control`, `game_view`) are not listed.

Resource URIs:

- `uo://session/{id}/state` — observe JSON for that session
- `uo://playbook/{name}` — markdown playbook (`text/markdown`)

Playbooks:

| Name | Use |
| --- | --- |
| `driver` | `next_event` loop and act order |
| `login` | One `connect` owns the socket |
| `hunt` | Melee job: kill, loot own kills, flee |
| `walk` | Guarded walk; stops on a hostile |
| `navigation` | `move_to` vs walk job, doors, pads |
| `loot` | Corpses the hunt job did not make |
| `bank` | Walk to a banker, open the box, deposit |
| `death` | Ghost recover, then restock |
| `moongate` | Step onto the gate tile, then the gump |
| `dungeon` | Pads, stairs, z jumps |
| `mounts` | War off, then `use`; dismount is `use` self |
| `runebook` | Recall from the book gump |
| `buy` / `sell` | Vendor speech keywords |
| `containers` | Open, lift, nested bags |
| `talk` | `reply` and `unanswered` |
| `inspect` | Names, `look_around`, `find_*` |
| `equip` | Wear and take off |

### The `screenshot` tool

The MCP server adds one tool that is not a session tool: `screenshot`. It opens the watch window for one picture and gives it back as a JPEG image (at most 1024 px on its long side). A vision model then sees what a human sees: the real map, the mobiles with their names, and the panels. Use it when the text radar is not enough, for example in a crowd or in a dungeon. It needs a desktop, and it takes some seconds. It is not on the HTTP API.

## CLI against a running process

```
uoterm --json session list
uoterm session attach s1
uoterm say "vendor buy"
uoterm move --to 1425,1680,0
uoterm walk --dir south --run --hold-ms 2000
uoterm open-door
uoterm look
uoterm state --json
uoterm agent run --persona personas/lumberjack.toml
uoterm agent stop
uoterm harvest log --since 1h --jsonl
```

`--api`, `--session`, `UOTERM_API`, `UOTERM_SESSION`, and `UOTERM_API_TOKEN` apply to these commands.

Exit codes: 0 ok, 2 usage, 3 network, 4 protocol, 5 world/precondition.
