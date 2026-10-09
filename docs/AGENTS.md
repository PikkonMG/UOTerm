# UOTerm agents and hotkeys

Agents are helpers that work for your character on their own. They loot,
pick up items, bandage, mount again, and more. Hotkeys are named actions
that you press by name. Both go at the pace a person plays.

On each session tick, the character's own reflexes act first, then the
agents, then the scripts ([SCRIPTS.md](SCRIPTS.md)).

Hunt and walk are session jobs, not agents. See
[AGENT_API.md](AGENT_API.md) and the hunt and walk playbooks in
[playbooks/](playbooks/). The bandage agent still heals while a job runs.

A shard can send a list of assistant features it forbids. The agents obey
it: for example autoloot, auto-bandage, auto-remount, the bone cutter,
restock, buy, sell, closest or random targets, the unequip before a cast,
and the free hand for potions. Set `obey_shard_rules = false` in
`uoterm.toml` to ignore the list.

## Settings

Each character has one settings file. On Linux it is
`~/.config/uoterm/agents/<character>.toml`. UOTerm writes it when you change
settings with the `agent_set` or `agent_on` tool. You can also write it by
hand. A part you leave out keeps its default.

Take care when you edit it by hand. If the file does not read as TOML, every
agent starts with its defaults, and the log shows a warning. The next
`agent_set` or `agent_on` then writes over your file.

```toml
[autoloot]
enabled = true
range = 2            # tiles
delay_ms = 600       # between two moves
bag = 0x40001234     # where loot goes; the backpack when left out
active = "default"   # the list in use

[[autoloot.lists.default]]
name = "gold"
graphic = 0x0EED

[[autoloot.lists.default]]
name = "good rings"
graphic = 0x108A
properties = [{ name = "Faster Casting", min = 1 }]

[bandage]
enabled = true
whom = "friend_or_self"   # self_only, friend, friend_or_self, or { target = 0x00001234 }
hp_pct = 80

[friends]
friends = [0x00001234]
include_party = true
prevent_attack = true
accept_party = true

[organizer.reagents]
destination = 0x40005555
items = [{ graphic = 0x0F7A }, { graphic = 0x0F7B }]

[restock.reagents]
items = [{ graphic = 0x0F7A, amount = 50 }]   # from the bank box to the pack

[dress.pvp]
items = [{ layer = 1, serial = 0x40006666 }]

[targets.greys]
notorieties = ["gray", "criminal"]
range_max = 12
selector = "nearest"
```

## Options

The `[options]` part holds switches that change how the character plays.
Each one is off until you set it. Set them in the file, or with `agent_set`
and `"agent": "options"`. `agent_set` replaces the whole `[options]` part,
so send every option you want to keep, not only the one you change.

| Option | What it does |
| --- | --- |
| `smart_last_target` | "Last target" at a harmful cursor is the last harmful target. At a helpful cursor, it is the last helpful one. |
| `last_target_range` | A last target farther than this many tiles is not targeted. |
| `block_heal_poisoned` | The cursor of Heal, Greater Heal or Close Wounds is not answered with a poisoned target. |
| `unequip_before_cast` | Puts the weapon and shield in the pack before a Magery cast. Spellbooks stay. |
| `free_hand_for_potions` | Puts the shield away to drink a potion, and takes it out again after. |
| `stack_at_feet` | Drops ore, logs and fish from the pack at your feet. |
| `no_run_hidden` | Walks, never runs, while hidden. |
| `no_doors_hidden` | Opens no door on the way while hidden. |
| `block_dismount_in_war` | Refuses a double-click on yourself while mounted in war mode. |
| `sight_mode` | The rules for line of sight: `runuo` (the default; also right for ModernUO and ServUO), `pol` or `sphere`. `line_of_sight`, `find_mobiles` `in_sight` and `attack_nearest` use it. |
| `listen_range` | A line said aloud this many tiles away or nearer counts as said to the character, with or without its name. None is off. |
| `catch_bag` | The container `loot` fills in place of the backpack. The `catch_bag` tool sets it. |
| `ignore_gumps` | Gump ids an agent does not hear of. The `ignore_list` tool sets them. |
| `ignore_journal` | Speakers or words of journal lines an agent does not hear of. The `ignore_list` tool sets them. |
| `speech_hue` | The colour the character speaks in. None is the client's own. |

`observe` shows `self_state.stealth_steps`: the steps taken since you hid.

## Item rules

The loot, scavenge, organizer, restock, buy and sell agents take lists of
item rules. A field you leave out matches anything.

| Field | What it does |
| --- | --- |
| `name` | A label for you. It takes no part in the match. |
| `graphic`, `color` | The item graphic and colour. |
| `amount` | How many. For a move or a sale, the most to move or sell (none means all). For restock, the level to fill up to (a restock rule with no amount does nothing). For buy, the most to buy (none means 1). |
| `bag` | A bag for this item only. |
| `properties` | Properties the item must have, each with `min` and `max`. |
| `disabled` | Keeps the rule but matches nothing. |

## The agents

| Agent | What it does |
| --- | --- |
| `autoloot` | Opens corpses in range and moves the items its list wants into the bag. It stops while the pack has less than 5 stones free. |
| `scavenger` | Picks up the ground items its list wants. It also stops while the pack has less than 5 stones free. |
| `organizer` | A job: moves listed items from one bag to another, then stops. It needs a `destination`. |
| `restock` | A job: tops listed items up to their amount, from the bank box or a bag. |
| `dress` | A job: puts on a dress list, and takes off what is in the way. Also `undress`. |
| `buy` | Answers a vendor's buy list with its list. With `complete_amount`, it buys only what you are short of. |
| `sell` | Answers a vendor's sell list with its list, up to each amount. |
| `bandage` | Bandages you, your most hurt friend, or one target, when their health is under `hp_pct`. |
| `self_heal` | Casts `heal_spell` (Greater Heal) on you when your health is under `hp_pct`. Casts `cure_spell` (Cure) when you are poisoned and `cure_poison` is on. It waits `delay_ms` after each cast, and skips a spell you lack the mana for. |
| `friends` | Your friends list. It can keep you from attacking a friend, and accept a friend's party invite. |
| `remount` | Mounts again after you are knocked off. Set `mount` to the pet or the ethereal. |
| `bone_cutter` | Uses `blade` on bone piles next to you. |
| `carver` | Uses `blade` on each fresh corpse next to you. |
| `open_corpses` | Opens each new corpse in range. It never runs while you are hidden. |
| `targets` | Named target filters, for the `target_filter` tool and the `targetfilter` script command. |

### More settings

Each agent has a few more keys. The defaults are in brackets.

| Agent | Keys |
| --- | --- |
| `autoloot` | `no_open_corpse` (false): loot only corpses whose contents are known, with no open. `while_hidden` (false). |
| `scavenger` | `enabled`, `range` (2), `delay_ms` (600), `bag`, `while_hidden` (false), and the lists. |
| `organizer`, `restock` lists | `source`, `destination`, `delay_ms` (600). Restock takes from the bank box and puts into the pack when these are not set. The organizer takes from the pack. |
| `dress` lists | `delay_ms` (600), `undress_bag` (the pack), `replace_worn` (true): take off what is on a layer first. |
| `buy` | `complete_amount` (false). |
| `sell` | `bag` (the pack): sell only from this bag and the bags in it. |
| `bandage` | `range` (2), `delay_ms` (none: the time follows your dexterity), `skip_poisoned` (false), `skip_when_hidden` (true), `bandage_graphic`, `bandage_color` (for a shard with its own bandage). |
| `self_heal` | `hp_pct` (80), `heal_spell` (29), `cure_spell` (11), `cure_poison` (true), `delay_ms` (2000), `skip_when_hidden` (true). |
| `remount` | `delay_ms` (600). |
| `open_corpses` | `range` (2). |
| `targets` filters | `notorieties`, `bodies`, `colors`, `name` (part of the name), `range_min`, `range_max`, and the flags `poisoned`, `human`, `ghost`, `war`, `friend`, `paralyzed`. `selector`: `nearest` (default), `farthest`, `weakest`, `strongest`, `random`, `next` or `previous`. |

## Tools

| Tool | What it does |
| --- | --- |
| `agents` | Every agent's settings, which agents are on, and the job that runs. |
| `agent_set` | Replaces an agent's settings: `{"agent": "bandage", "settings": {...}}`. For one list: `{"agent": "autoloot", "list": "gems", "settings": [...]}`. The organizer, restock, dress and targets need `list`. A serial or graphic may be `0x...` text. |
| `agent_on` | `{"agent": "autoloot", "on": true, "list": "gems"}`. |
| `agent_run` | Runs a job once: `{"agent": "organizer", "list": "reagents"}`. Also restock, dress, undress and autoloot. It ends with `job_ended` (`agent: organizer done`) or `job_failed` (`agent: <reason>`). |
| `agent_stop` | Stops the job. |
| `damage_meter` | `{"action": "start"}`. Also pause, resume, stop and report (the default). A stopped meter keeps its totals until the next start. |
| `target_filter` | `{"name": "greys"}`: picks a mobile and makes it the last target. |

`dress` with no list, and the Save Dress hotkey, use a list named `temp`
that lives in memory only. `undress` with no list takes off all you wear
except the pack, the bank box, hair, beard and the mount.

## Hotkeys

The `hotkeys` tool lists every hotkey by group. Give `group` for one group,
or `name` for one hotkey and the script lines it runs. The `hotkey` tool
presses one by name: `{"name": "Bandage Self"}`. A name matches in any case,
and spaces and marks do not count.

| Group | Examples |
| --- | --- |
| general | Resync, Ping Server, Accept Party, Decline Party, Where Am I, Damage Meter Start |
| actions | Fly On/Off, Use Last Item, Use Left Hand, Show Names Mobiles, Mount / Dismount |
| pets | All Come, All Follow Me, All Guard Me, All Kill, All Stay, All Stop |
| agents | Autoloot On/Off, Autoloot Once, Dress, Undress, Save Dress, Buy On, Sell Off. One hotkey for each organizer and restock list, and two (`Dress <list>`, `Undress <list>`) for each dress list |
| combat | Primary Ability, Secondary Ability, Attack Nearest Enemy, War Mode On/Off, Bandage Self, Bandage Last, Toggle Right Hand |
| potions | Potion Heal, Potion Cure, Potion Refresh, and the rest |
| items | Enchanted Apple, Orange Petals, Smoke Bomb, Healing Stone |
| wands | Wand Heal, Wand Lightning, and the rest |
| skills | Use Hiding, Use Meditation, and each skill with a use button; Last Skill |
| spells | Cast Greater Heal, and every other spell; Mini Heal, Big Heal, Interrupt, Last Spell |
| virtues | Honor, Sacrifice, Valor |
| targets | Target Self, Target Last, Cancel Target, and one hotkey for each target filter |
| scripts | Script heal (starts or stops it), Stop All Scripts |

A hotkey acts once, at once. When the character must wait before it may act
again, the hotkey says so. Press it again a moment later.

## Recording

`record_macro` with `{"action": "start", "name": "fish"}` starts a
recording. A name has letters, digits, `-` and `_` only. Each tool call that
acts and succeeds, and each hotkey you press, becomes a script line, for the
tools that have a script command. When the shard opens a target cursor, a
gump or a prompt, a wait for it goes in by itself. `{"action": "stop"}`
saves the script in the `scripts` folder. `{"action": "cancel"}` drops it.
Run it with `run_script`. Give `"loop": true` to run it again each time it
ends.
