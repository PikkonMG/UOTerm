# Login playbook

One `connect` process owns the game socket. Do not start a second one on the same account.

## Start

```bash
export UO_PASS=...
uoterm connect --host HOST --port PORT --account NAME --character NAME --era modern
```

The CLI default for `--era` is `modern`. Use `--era t2a` only for the mock demo shard.

Optional `--view` opens the watch window in the same process. Closing it ends the program. Optional `--text-view` prints the radar in that terminal. The HTTP API still runs.

Wait until `observe` shows `self_state` in the world (`logged_in` / a real location). Then you may walk, hunt, or talk.

Password is `UO_PASS`. Never put it in a file.

If connect is already up, use `uoterm session list` and talk to that API, or attach `uoterm mcp`. Do not open another game socket. HTTP default is `http://127.0.0.1:7733`.
