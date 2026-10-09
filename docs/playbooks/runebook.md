# Runebook playbook

A runebook recalls to a stored rune. Its charges come from recall scrolls
dropped onto the book.

1. `use` the book and read the gump. The slot buttons recall. Do not guess
   numbers: read the line beside each button.
2. Stand still for the cast. Wait for a jump in location, or `map_changed`
   when the rune is on another map.
3. To charge it: `drop` recall scrolls onto the book.
4. To fill it: mark a rune where you stand (`use` a mark scroll, target the
   rune), then `drop` the rune onto the book.
5. Close other gumps before you press a slot, so the press goes to this
   book.

A recall fails when you carry too much, in a house you do not own, or when
the shard forbids the facet.
