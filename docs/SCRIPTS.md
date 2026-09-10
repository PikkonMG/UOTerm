# UOTerm scripts

A script is a list of things for your character to do, one on each line.
UOTerm does the lines in order, at the pace a person plays. The commands
follow the command style that UO assistant scripts have long used, so a
script you already have will most likely run.

## A first script

```text
// Heal when hurt, cure when poisoned.
while not dead
  if poisoned
    cast 'Cure' 'self'
  elseif hits < maxhits
    bandageself
    pause 10000
  endif
  pause 500
endwhile
```

## The rules

1. **One line, one thing.** A line holds one command, or one check.
2. **A person's pace.** A command that acts waits until the character may act
   again. A script never acts faster than a person can.
3. **A wait always ends.** Every `waitfor...` command has a time. When the
   time runs out, the script goes on to the next line.
4. **A bad line stops the script.** An unknown command, a missing argument or
   a spell name that does not exist stops the script. `script_status` tells
   you the line number and the reason.
5. **Safety comes first.** The character still heals and fights back while a
   script runs. The agents act before the script on each tick.
6. **One script at a time.** Stop the running script before you start
   another.
7. **Death stops a script.** So does a lost connection.

## How to write a line

- Put text in quotes: `msg 'Hello'`. Use double quotes for text with an
  apostrophe: `cast "Nature's Fury"`.
- Numbers are decimal or hex: `useobject 0x40001234`, `pause 500`.
- Case does not matter: `CAST 'heal'` is the same as `cast 'Heal'`.
- `//` starts a note. UOTerm skips the rest of the line.
- `;` puts two commands on one line: `msg 'hi'; pause 500`.
- `@` in front of a command keeps it quiet when it finds nothing:
  `@findtype 0x0E21`.
- `!` after a command gives its stronger form. For `target!`, the target goes
  only to a cursor that is open now, and is not held for the next one. For
  `pushlist!`, the value goes in only when the list does not hold it.

## Names for objects (aliases)

A command that needs an object takes a serial (`0x40001234`) or a name.

| Name | What it is |
| --- | --- |
| `self` | Your character. |
| `backpack` | Your backpack. |
| `bank` | Your bank box. Say "bank" beside a banker first. |
| `last`, `lasttarget` | The last object you targeted. |
| `lastobject` | The last object you used. |
| `lefthand`, `righthand` | What you hold in that hand. |
| `found` | What the last `findtype`, `findobject` or `findlayer` found. |
| `enemy`, `friend` | What the last `getenemy` or `getfriend` picked. |
| `mount` | Your mount, when you set it: `setalias 'mount' 0x00001234`. |

Make your own: `setalias 'pet' 0x00001234`, then `useobject 'pet'`.
Names you make stay for every script of the character, until its session ends.

## Checks and loops

```text
if hits < 50
  drinkpotion 'heal'
elseif poisoned
  drinkpotion 'cure'
else
  msg 'fine'
endif
```

- `if`, `elseif` (or `else if`), `else`, `endif`.
- `while (check)` ... `endwhile` repeats while the check is true.
- `for 5` ... `endfor` repeats 5 times.
- `for 1 to 10` ... `endfor` counts from 1 to 10, both ends included.
- `for 0 to 'fruit'` ... `endfor` walks the list `fruit`. In the body,
  `fruit[]` is the item it is on. `fruit[2]` is the item at place 2.
- `for 0 to 2 in 'fruit'` walks places 0 to 2 of the list.
- `break` leaves the loop. `continue` goes on with the next pass.
- `stop` ends the script. `replay` starts it again from the top.
- `not`, `and`, `or`: `if not dead and hits < 50`. They are read left to
  right, with no brackets.
- Compare signs: `==`, `!=`, `<`, `>`, `<=`, `>=`. The right side can be a
  number, text, or another word: `if hits < maxhits`.

## Commands

### Spells and healing

| Line | What it does |
| --- | --- |
| `cast 'Greater Heal'` | Casts a spell by name or number. |
| `cast 'Greater Heal' 'self'` | Casts it and answers its target cursor with that object. |
| `cast 'Lightning' 'enemy'` | The same, on the enemy. |
| `cast 'last'` | Casts the last spell again. |
| `miniheal ['friend']` | Heal, or Cure when the target is poisoned. Yourself when no target. |
| `bigheal ['friend']` | Greater Heal, or Arch Cure when poisoned. |
| `chivalryheal ['friend']` | Close Wounds, or Cleanse by Fire when poisoned. |
| `bandageself` | Uses a bandage on yourself. |
| `bandagetarget 'friend'` | Uses a bandage on someone else. |
| `drinkpotion 'heal'` | Drinks a potion by name: heal, cure, refresh, agility, strength, explosion, night sight, and more. |
| `interrupt` | Breaks the spell you are casting. |

### Targets

| Line | What it does |
| --- | --- |
| `waitfortarget 3000` | Waits up to 3 seconds for a target cursor. |
| `target 'self'` | Answers the cursor. With no cursor open, the target waits 5 seconds for one. |
| `target! 'enemy'` | Answers only a cursor that is open now. |
| `targettype 0x0F7A` | Targets the nearest object of that graphic: pack, then ground, then mobiles. |
| `targetground 0x0F7A 'any' 8` | The same, on the ground only, within 8 tiles. |
| `targettile 1440 1695 0` | Targets a map tile. Also `targettile 'current'` and `targettile 'last'`. |
| `targettileoffset 1 0 0` | Targets the tile one step east of you. |
| `targettilerelative 'self' 2` | Targets the tile 2 steps in front of you. Add `'true'` for behind. |
| `targetresource 0x40001234 'ore'` | Uses a tool on ore, sand, wood, graves or red mushrooms, with no cursor. |
| `canceltarget` | Closes the open cursor. |
| `autotargetobject 'enemy'` | The next cursor goes to that object. Also `autotargetself`, `autotargetlast`, `autotargettype`, `autotargetground`, `autotargettile`, `autotargettileoffset`, `autotargettilerelative`, `autotargetghost`. |
| `cancelautotarget`, `cleartargetqueue` | Drops a target that waits for a cursor. |
| `clearlasttarget` | Forgets the last target. |
| `getenemy 'gray' 'criminal' 'closest'` | Sets `enemy` to a mobile. Words: any, innocent, friend, gray, criminal, enemy, murderer; humanoid, transformation; closest (the nearest), nearest (takes turns between the two nearest). With neither, each call moves to the next match. |
| `getfriend 'innocent' 'friend'` | Sets `friend` the same way. |
| `targetfilter 'greys'` | Uses a named target filter (see the agents guide) and sets `enemy`. |

### Items

| Line | What it does |
| --- | --- |
| `useobject 0x40001234` | Uses (double-clicks) an object. |
| `usetype 0x0E21 ['any'] ['backpack'] [range]` | Uses the first item of a graphic. Places: `backpack`, `ground`, `world`, or a container. |
| `useonce 0x0F0C` | Uses an item of that graphic that `useonce` has not used yet. `clearuseonce` forgets them. |
| `moveitem 'found' 'backpack' [amount]` | Moves an item into a container, or onto a mobile to give it. |
| `moveitem 'found' 'ground' 1440 1695 0` | Drops an item on a tile. |
| `moveitemoffset 'found' 'ground' 1 0 0` | Drops it on the tile east of you. |
| `movetype 0x0F7A 'backpack' 0x40005555` | Moves the first item of a graphic from one place to another. |
| `equipitem 0x40001234 1` | Wears an item on a layer (1 right hand, 2 left hand). |
| `clearhands 'both'` | Puts what you hold into the pack. Also `'left'` and `'right'`. |
| `togglehands 'right'` | Puts the weapon away, or takes it out again. |
| `equipwand 'Heal' 5` | Wears a wand of that spell with at least 5 charges. |
| `feed 'pet' 'Fruits and Vegetables'` | Gives food to a mobile: a food name, a group (Fish, Fruits and Vegetables, Meat), `any`, or a graphic. |
| `clickobject 'found'` | Single-clicks, so the name shows in the journal. |
| `shownames 'mobiles'` | Shows the names of the mobiles near you. Also `'corpses'`. |
| `rename 'pet' 'Snorlax'` | Renames your pet. |
| `waitforcontents 'found' 2000` | Opens a container and waits for what is in it. |
| `waitforproperties 'found' 2000` | Asks for an object's properties and waits for them. |
| `ignoreobject 'found'` | The find commands pass over it. `clearignorelist` forgets them all. |

### Moving and fighting

| Line | What it does |
| --- | --- |
| `walk 'north'` | Takes one step. A list works too: `walk "north, east, east"`. |
| `run 'south'` | Runs. |
| `turn 'east'` | Turns without a step. |
| `pathfindto 1440 1695` | Walks to a tile and waits until you get there. |
| `opendoor` | Opens the door beside you. |
| `attack 'enemy'` | Attacks. |
| `warmode 'on'` | Goes into war mode. `togglewar` switches it. |
| `setability 'primary' 'on'` | Arms your weapon's primary move. Also `'secondary'`, `'stun'`, `'disarm'`, and `'off'`. |
| `fly`, `land`, `togglefly` | A gargoyle takes off or lands. |
| `togglemounted` | Mounts or dismounts. Set the `mount` alias first. |
| `useskill 'Hiding'` | Uses a skill. `useskill 'last'` uses the last one again. |
| `setskill 'Magery' 'locked'` | Sets a skill lock: up, down or locked. |
| `virtue 'honor'` | Invokes a virtue: honor, sacrifice or valor. |
| `setstatlock 'str' 'locked'` | Sets a stat lock: str, dex or int; up, down or locked. |
| `emoteaction 'bow'` | Plays an emote animation, such as bow or salute. |

### Speech and party

| Line | What it does |
| --- | --- |
| `msg 'bank'` | Says the words. Town and pet commands work: `msg 'all kill'`. |
| `yellmsg`, `whispermsg`, `emotemsg`, `guildmsg`, `allymsg` | Other ways to speak. |
| `partymsg 'heal me'` | Says it to your party. Add a serial for one member only. |
| `partyaccept`, `partydecline` | Answers a party invite. |
| `partyinvite 'friend'` | Asks someone into your party. With no one named, the shard gives a target cursor. |
| `partyremove 'friend'` | Removes someone from your party. |
| `partyloot 'on'` | Lets your party loot your corpses, or not. |
| `promptmsg 'my rune'` | Answers a text prompt. 128 characters at most. |
| `textentrymsg 'Rowan'` | Answers an open one-field text dialog. |
| `canceltextentry` | Cancels that dialog. |
| `waitfortextentry 5000` | Waits for a one-field text dialog. |
| `waitforprompt 5000` | Waits for a text prompt. |
| `cancelprompt` | Cancels the open prompt. |
| `sysmsg 'done'` | Writes a line in the script's output. So does `headmsg`. |
| `timermsg 5000 'time to eat'` | Writes a line after 5 seconds, and goes on now. |

### Gumps and menus

| Line | What it does |
| --- | --- |
| `waitforgump 0x1EC8C837 5000` | Waits for a gump. `'any'` for any gump. |
| `replygump 0x1EC8C837 1 [switches...]` | Presses a gump button. |
| `closegump 'container' 'found'` | Forgets an open container. |
| `contextmenu 'bank' 'Open Bankbox'` | Picks a context menu entry by its words, or by its number. |
| `waitforcontext 'vendor' 'Buy' 3000` | Asks for the menu and waits until the entry is picked. |
| `autocolorpick 35` | The next dye tub gets colour 35. |

### Journal

| Line | What it does |
| --- | --- |
| `waitforjournal 'You finish applying' 10000` | Waits for words in the journal. Add a name, or `'system'`, for who says them. |
| `clearjournal` | Old journal lines no longer count for `injournal` and `waitforjournal`. |

### Lists and timers

| Line | What it does |
| --- | --- |
| `createlist 'fruit'` | Makes a list. |
| `pushlist 'fruit' 'apple' ['front']` | Adds to the back, or the front. |
| `poplist 'fruit' 'apple'` | Removes a value, or `'front'`, or `'back'`. `!` removes every copy. |
| `clearlist 'fruit'`, `removelist 'fruit'` | Empties or deletes a list. |
| `createtimer 'band'` | Makes a timer that counts up from 0, in milliseconds. |
| `settimer 'band' 0` | Sets a timer. |
| `removetimer 'band'` | Deletes a timer. |
| `pause 1500` | Waits 1.5 seconds. |

### Agents

See the agents guide for the lists these use.

| Line | What it does |
| --- | --- |
| `organizer 'reagents' [source] [destination] [delay]` | Runs an organizer list. |
| `restock 'reagents' [source] [destination] [delay]` | Runs a restock list. |
| `dress ['pvp']`, `undress ['pvp']` | Puts on or takes off a dress list. With no name, `undress` takes off everything but the pack. |
| `dressconfig` | Saves what you wear now as the dress list `temp`. |
| `autoloot` | Loots the corpses in range once with the autoloot list. |
| `toggleautoloot`, `togglescavenger` | Switches the agent on or off. |
| `buy ['reagents']`, `sell ['loot']` | Switches the vendor agent on, with that list. |
| `clearbuy`, `clearsell` | Switches it off. |

### Scripts

| Line | What it does |
| --- | --- |
| `playmacro 'heal'` | Stops this script and runs another in its place. |
| `script 'run' 'heal'` | The same. |
| `script 'stop'` | Stops this script. |
| `script 'isrunning' 'heal' 'on'` | Sets the alias `on` to 1 when that script runs, else 0. |
| `where` | Writes your tile in the script's output. `location 'friend'` writes someone else's. |
| `resync`, `ping` | Asks the shard to send your place again; pings the shard. |
| `paperdoll ['friend']` | Opens a paperdoll. |
| `guildbutton`, `questsbutton`, `logoutbutton` | The paperdoll buttons. |

## Condition words

Use these after `if`, `elseif` and `while`. Words that name an object read
yourself when you give none.

| Word | What it gives |
| --- | --- |
| `hits`, `maxhits`, `diffhits` `[serial]` | Health, the most, and the difference. |
| `stam`, `maxstam`, `mana`, `maxmana`, `str`, `dex`, `int` | Your stats. |
| `weight`, `maxweight`, `diffweight`, `gold`, `luck`, `followers`, `maxfollowers`, `tithingpoints` | More of your status. |
| `physical`, `fire`, `cold`, `poison`, `energy` | Your resists. |
| `x`, `y`, `z` `[serial]` | Where a thing is. |
| `dead`, `poisoned`, `paralyzed`, `hidden`, `flying`, `mounted`, `war`, `yellowhits` `[serial]` | True or false. |
| `innocent`, `friend`, `gray`, `criminal`, `enemy`, `murderer`, `invulnerable` `[serial]` | The notoriety of a mobile. |
| `serial`, `graphic`, `color`, `amount`, `name`, `direction`, `directionname` `[serial]` | Details of an object. |
| `skill 'Magery'`, `skillbase 'Magery'` | A skill value and base value. |
| `skillstate 'Magery' == 'locked'` | A skill lock: up, down or locked. |
| `findtype 0x0F7A ['any'] ['backpack'] [amount] [range]` | True when found. Sets `found`. |
| `findobject 'pet' ['any'] ['ground'] [amount] [range]` | True when the object is there. Sets `found`. |
| `findlayer 'self' 2` | True when a mobile wears something on the layer. Sets `found`. |
| `findwand 'Heal' ['backpack'] [charges]` | True when you have such a wand. Sets `found`. |
| `counttype 0x0F7A 'any' 'backpack' > 10` | How many of a graphic. With `!`, how many stacks. |
| `counttypeground 0x0F7A 'any' 8 > 0` | How many on the ground within 8 tiles. |
| `counter 'bp' < 20` | How many of a supply: band, bp, bm, gl, gs, mr, ns, sa, ss, bw, db, gd, nc, pi. |
| `bandage` | How many bandages you carry. |
| `contents 'backpack' > 100` | How many items are in a container. |
| `distance 'enemy' <= 2` | How far away, in tiles. |
| `inrange 'enemy' 10` | True when within the range. |
| `buffexists 'Bless'` | True when you have the buff. |
| `property 'Faster Casting' 'found' >= 2` | A property of an item. |
| `durability 'righthand' < 20` | An item's durability. |
| `injournal 'too far away' ['system']` | True when the words are in the journal. |
| `ingump 'any' 'Home'`, `gumpexists 'any'` | Gump checks. |
| `targetexists ['harmful']` | True when a target cursor is open: any, harmful, beneficial or neutral. |
| `waitingfortarget` | True when a target waits for a cursor. |
| `inparty 'friend'`, `infriendlist 'friend'` | Party and friends list. |
| `findalias 'pet'`, `listexists 'fruit'`, `list 'fruit' > 2`, `inlist 'fruit' 'apple'` | Aliases and lists. `findalias` also knows the game's own names, such as `'bank'` once the bank box is open. |
| `timer 'band' > 10000`, `timerexists 'band'` | Timers. |
| `organizing`, `restocking`, `dressing` | True while that agent job runs. |
| `usetype`, `useobject`, `useonce`, `moveitem`, `movetype`, `clearhands` | These commands also work as checks: true when they found what they look for and did it. |

## Running scripts

Save a script as a text file in a `scripts` folder: in the folder you run
UOTerm from, or in your config folder (on Linux `~/.config/uoterm/scripts`).
The file name without its extension is the script name.

| Tool | What it does |
| --- | --- |
| `run_script` | `name` runs a saved script; `text` runs the text you give. `loop: true` runs it again each time it ends. |
| `stop_script` | Stops it. |
| `script_status` | The status (running, done, stopped or failed), the line, the reason for a failure, and the script's output. |
| `list_scripts` | The saved scripts. |
| `record_macro` | Records what you do as a script: `action: start` with a `name`, then `stop` to save it. |

## What does nothing here

UOTerm has no game window and no mouse.

- `playsound`, `snapshot`, `hotkeys`, `messagebox`, `mapuo`, `clickscreen`
  and `info` only draw on a game window. They write a note and go on.
- `promptalias`, `setalias` with no serial, `addfriend` with no serial, and
  `helpbutton` need a person with a mouse. They stop the script and say so.
  Give the serial instead: `setalias 'pet' 0x00001234`, `addfriend 'enemy'`.
- `inregion` needs the shard's map of regions. A shard does not send it, so
  `inregion` stops the script and says so.
