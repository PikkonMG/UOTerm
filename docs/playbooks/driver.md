# Driver playbook

This is the loop that runs the character. Other playbooks are the branches.

1. Confirm the character is in the world (`login`).
2. Call `next_event` and wait.
3. Act in this order:
   1. Danger: `died`, `low_health`, poison. See `death`.
   2. Waiters: target cursor, gump, prompt, trade, party invite.
   3. Chat: `spoken_to` / `unanswered`. See `talk`.
   4. A running job: read `doing.job`. Do not `move_to` or `attack` over it.
   5. `job_ended`: read the reason, then hunt, walk, bank, or rest.
   6. Your own task.

Start one job at a time. Hunt and walk refuse a second start unless `replace` is true. Replace emits `job_ended` with reason `stopped` for the old job, then starts the new one.

Watch the character with `uoterm watch`, `connect --view`, or `connect --text-view`. `--view` starts watch as a child; closing the window does not drop the socket. The picture is a 2D radar, not an isometric game window.
