# UOTerm agents and hotkeys

Agents are helpers that work for your character on their own: they loot,
pick up, bandage, remount, and more. Hotkeys are named actions you press by
name. Both go at the pace a person plays, and both run after the
character's own self-care on each tick.

## Settings

Each character has one settings file:
`~/.config/uoterm/agents/<character>.toml` on Linux. UOTerm writes it when
you change settings with the `agent_set` or `agent_on` tool. You can also
write it by hand. A part you leave out keeps its default.

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
All are off until you set them. Set them in the file, or with `agent_set`
and `"agent": "options"`.

| Option | What it does |
| --- | --- |
| `smart_last_target` | "Last target" at a harmful cursor is the last harmful target; at a helpful cursor, the last helpful one. |
| `last_target_range` | A last target farther than this many tiles is not targeted. |
| `block_heal_poisoned` | The cursor of Heal, Greater Heal or Close Wounds is not answered with a poisoned target. |
| `unequip_before_cast` | Puts the weapon and shield in the pack before a Magery cast. Spellbooks stay. |
| `free_hand_for_potions` | Puts the shield away to drink a potion, and takes it out again after. |
| `stack_at_feet` | Drops ore, logs and fish from the pack at your feet. |
| `no_run_hidden` | Walks, never runs, while hidden. |
| `no_doors_hidden` | Opens no door on the way while hidden. |
| `block_dismount_in_war` | Refuses a double-click on yourself while mounted in war mode. |

`observe` also shows `stealth_steps`: the steps taken since you hid.

## Item rules

The loot, scavenge, organizer, restock, buy and sell agents take lists of
item rules. A field you leave out matches anything.

| Field | What it does |
| --- | --- |
| `name` | A label for you. It takes no part in the match. |
| `graphic`, `color` | The item graphic and colour. |
| `amount` | How many: the most to move, the level to restock to, or the most to buy or sell. |
| `bag` | A bag for this item only. |
| `properties` | Properties the item must have, each with `min` and `max`. |
| `disabled` | Keeps the rule but matches nothing. |

## The agents

| Agent | What it does |
| --- | --- |
| `autoloot` | Opens corpses in range and moves the items its list wants into the bag. It stops while the pack has less than 5 stones free. |
| `scavenger` | Picks up the ground items its list wants. |
| `organizer` | A job: moves listed items from one bag to another, then stops. |
| `restock` | A job: tops listed items up to their amount, from the bank box or a bag. |
| `dress` | A job: puts on a dress list, taking off what is in the way. Also `undress`. |
| `buy` | Answers a vendor's buy list with its list. With `complete_amount`, it buys only what you are short of. |
| `sell` | Answers a vendor's sell list with its list, up to each amount. |
| `bandage` | Bandages you, your most hurt friend, or one target, when their health is under `hp_pct`. |
| `friends` | Your friends list. It can keep you from attacking a friend and accept a friend's party invite. |
| `remount` | Mounts again after you are knocked off. Set `mount` to the pet or ethereal. |
| `bone_cutter` | Uses `blade` on bone piles next to you. |
| `carver` | Uses `blade` on each fresh corpse next to you. |
| `open_corpses` | Opens each new corpse in range. |
| `targets` | Named target filters, for the `target_filter` tool and the `targetfilter` script command. |

## Tools

| Tool | What it does |
| --- | --- |
| `agents` | Every agent's settings, which agents are on, and the job running. |
| `agent_set` | Replaces an agent's settings: `{"agent": "bandage", "settings": {...}}`. For one list: `{"agent": "autoloot", "list": "gems", "settings": [...]}`. A serial or graphic may be `0x...` text. |
| `agent_on` | `{"agent": "autoloot", "on": true, "list": "gems"}`. |
| `agent_run` | Runs a job once: `{"agent": "organizer", "list": "reagents"}`; also restock, dress, undress, autoloot. |
| `agent_stop` | Stops the job. |
| `damage_meter` | `{"action": "start"}`; also pause, resume, stop, report. A stopped meter keeps its totals until the next start. |
| `target_filter` | `{"name": "greys"}`: picks a mobile and makes it the last target. |

## Hotkeys

The `hotkeys` tool lists every hotkey by group. The `hotkey` tool presses
one by name, in any case: `{"name": "Bandage Self"}`.

| Group | Examples |
| --- | --- |
| general | Resync, Ping Server, Accept Party, Decline Party, Where Am I, Damage Meter Start |
| actions | Fly On/Off, Use Last Item, Use Left Hand, Show Names Mobiles, Mount / Dismount |
| pets | All Come, All Follow Me, All Guard Me, All Kill, All Stay, All Stop |
| agents | Autoloot On/Off, Autoloot Once, Dress, Undress, Save Dress, Buy On, Sell Off, and one hotkey for each organizer, restock and dress list |
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
again, the hotkey says so; press it again a moment later.

## Recording

`record_macro` with `{"action": "start", "name": "fish"}` starts a
recording. Each tool call that acts, and each hotkey you press, becomes a
script line. When the shard opens a target cursor, a gump or a prompt, a
wait for it goes in by itself. `{"action": "stop"}` saves the script. Run it
with `run_script`, and give `"loop": true` to run it again each time it
ends.
