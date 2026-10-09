# Dungeon playbook

A dungeon mouth, a ladder or a pad is often a teleporter of the server. A
route cannot plan through it. The landmark tile may be the hole itself,
which you cannot walk on.

0. `find_entrances` lists the stairs, ladders, learned pads and dungeon
   landmarks round you, nearest first.
1. Walk to the landmark, or to the last tile you can walk on beside it
   (`move_to` with `accuracy: 1`).
2. If a warning gump opens, answer it first. A walk while it is open is
   dropped.
3. Step onto the stair, ladder or pad. Check `can_walk` on the tiles around
   you. The way in often has a large change in z.
4. Wait for `map_changed` or a jump in x and y. The client learns pads from
   that jump.
5. To leave, stand on the landing and step the way that jumps z back.

Do not `use` a pad you already stand on. If it did not fire, step off and
back on.
