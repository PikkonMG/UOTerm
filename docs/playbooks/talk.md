# Talk playbook

The client tells you when someone spoke to this character: the `spoken_to`
event, and the `unanswered` list on each tool result. You write the words.

1. On `spoken_to`, read `observe`: `doing`, the people near, the last
   journal lines.
2. `reply` answers the newest unanswered line. Use `say` or `whisper` when
   you start the talk.
3. Match what the character already does. If she follows someone, do not say
   she will stay.
4. With `play_along` off, say no to plans, except with friends. With it on,
   say yes to party, follow and fight as the persona lists them, for a set
   time.
5. Near bankers and vendors, do not say `bank`, `buy`, `sell`, `guards` or
   the vendor's name unless you mean the command.

Do not answer your own speech. Party and guild lines count as spoken to
even when the speaker is out of sight.
