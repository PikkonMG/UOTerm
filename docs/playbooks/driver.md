# Driver playbook

This is the loop that runs the character. Other playbooks are the branches.

1. Confirm the character is in the world (`login`).
2. Call `next_event` and wait.
3. Act in this order:
   0. `control_taken`: a human has the character. Do not act. Each acting tool is refused until `control_released`. Keep the `next_event` loop going, and look only. On `control_released`, call `observe` first: the human may have moved her, fought, or changed the pack.
   1. Danger: `died`, `low_health`, poison. See `death`.
   2. Waiters: target cursor, gump, prompt (`prompt_answer`), trade, party invite (`party`), a shop list, a context menu or an old-style menu (`observe` holds each).
   3. Chat: `spoken_to` / `unanswered`. See `talk`.
   4. A running job: read `doing.job`. Do not `move_to` or `attack` over it.
   5. `job_ended`: read the reason, then hunt, walk, bank, or rest.
   6. Your own task.
   7. `disconnected`: the link dropped and the session logs in again by itself. Calls are refused until `logged_in` comes; then `observe` again.

Start one job at a time. Hunt and walk refuse a second start unless `replace` is true. Replace emits `job_ended` with reason `stopped` for the old job, then starts the new one.

Watch the character with `uoterm watch`, `connect --view`, or `connect --text-view`. `--view` opens the window in the same process; closing the window ends the program. The window draws the real map from the client files. A human can press "Take control" in it and then play by hand: walk, drag items, shop, trade, answer gumps. See step 0 above. Do not call `watch` or `command`: they are for the window. `observe` holds all an agent needs of what the window shows.

To see what the human sees, call the MCP tool `screenshot`. It gives one picture of the watch window. Use it when the text radar is not enough, for example in a crowd or in a dungeon. It takes some seconds, so do not call it in a fight.
