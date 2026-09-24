# Dungeon playbook

A dungeon mouth, ladder, or pad is often a server teleporter. Pathfind cannot plan through it. The landmark tile may be the hole, which does not walk.

0. `find_entrances` lists the stairs, ladders, learned pads and dungeon landmarks round you, nearest first.
1. Walk to the landmark or the last walkable tile beside it (`move_to` with `accuracy: 1`).
2. If a warning gump opens, answer it first. A walk while it is open is dropped.
3. Step onto the stair, ladder, or pad. Check `can_walk` on the tiles around you. The way in is often a large z change.
4. Wait for `map_changed` or a jump in x,y. The client learns pads from that jump.
5. To leave, stand on the landing and step the direction that jumps z the other way.

Do not `use` a pad you already stand on. Step off and back if it did not fire.
