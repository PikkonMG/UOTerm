# Login playbook

One `connect` process owns the game socket. Do not start a second one on
the same account.

## Start

```bash
export UO_PASS=...
uoterm connect --host HOST --port PORT --account NAME --character NAME --era modern
```

With no `--era`, the era comes from `uoterm.toml`, and else it is `modern`.
The mock demo shard takes both eras.

`--view` opens the play window in the same process. Closing it ends the
program. `--text-view` prints the radar in that terminal. The HTTP API runs
in both cases.

Wait until `observe` shows `self_state` in the world, at a real location.
Then you may walk, hunt or talk.

The password comes from the `UO_PASS` variable, unless `--password-env` or
the saved login names another one. Never put it in a file.

If `connect` is already up, use `uoterm session list` and talk to that API,
or attach `uoterm mcp`. Do not open another game socket. The HTTP default is
`http://127.0.0.1:7733`.

## Over MCP, with no session yet

`characters` lists the slots and start towns of an account.
`character_create` makes a character and logs in as it. `character_delete`
deletes one. `connect` logs in as one. Give `profile` (a saved login), or
`account` and `password_env` (the name of the variable that holds the
password). Never put a password in a call. `logout` with `then_play`
switches to another character in the same session.
