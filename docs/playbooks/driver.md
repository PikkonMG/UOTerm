# Driver playbook

This is the loop that runs the character. The other playbooks are the
branches.

1. Make sure the character is in the world (`login`).
2. Call `next_event` and wait.
3. Act in this order:
   0. `control_taken`: a human has the character. Do not act. Each acting
      tool is refused until `control_released`. Keep the `next_event` loop
      going, and only look. Control comes back by itself after 90 seconds
      with no human act. On `control_released`, call `observe` first: the
      human may have moved her, fought, or changed the pack.
   1. Danger: `died`, `low_health`, poison. See `death`.
   2. Things that wait for an answer: a target cursor, a gump, a prompt
      (`prompt_answer`), a trade, a party invite (`party`), a shop list, a
      context menu or an old-style menu (`observe` holds each one).
   3. Chat: the `spoken_to` event, and the `unanswered` list on every tool
      result. See `talk`.
   4. A running job: read `doing.job`. Do not `move_to` or `attack` over it.
   5. `job_ended`: read the reason, then hunt, walk, bank, or rest.
   6. Your own task.
   7. `disconnected`: the link dropped. The session logs in again by itself,
      unless `reconnect = false` is set in `uoterm.toml`. Until it is back,
      each call is refused with words that say so. Then call `observe`
      again.

Start one job at a time. Hunt and walk refuse a second start unless
`replace` is true. With `replace`, the old job ends with `job_ended` reason
`stopped`, then the new one starts.

A human can watch the character with `uoterm watch`, `connect --view`, or
`connect --text-view`. `--view` opens the window in the same process.
Closing that window ends the program. The window draws the real map from
the client files. A human can press "Take control" in it and play by hand:
walk, drag items, shop, trade, answer gumps. See step 0 above. Do not call
`watch` or `command`: they are for the window. `observe` holds all an agent
needs of what the window shows.

To see what the human sees, call the MCP tool `screenshot`. It gives one
picture of the watch window. Use it when the text radar is not enough, for
example in a crowd or in a dungeon. It takes some seconds, so do not call it
in a fight.
