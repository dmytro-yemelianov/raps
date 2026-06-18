# RAPS Executive Demo

The "wow beat": what normally takes **6 manual Autodesk APS API steps** (OAuth
token exchange, base64 URN encoding, chunked multipart upload, status polling,
retries, and finding a Viewer link) collapses into **4 clean `raps` commands**
ending with a live Autodesk Viewer URL.

```
1. raps auth test                              # 2-legged OAuth, no token juggling
2. raps bucket create --key <bucket>           # idempotent
3. raps object upload <bucket> <model>         # auto-chunking + base64 URN printed
4. raps translate start <urn> --format svf2 --watch   # live progress → Viewer URL
```

A scripted version of exactly this flow lives in `scripts/demo/`.

---

## 1. Prerequisites

- `raps` on your `PATH`. Build a release binary for the smoothest demo:
  ```bash
  cargo build --release -p raps-cli
  export PATH="$PWD/target/release:$PATH"   # so `raps` resolves to the fresh build
  ```
- `jq` and `base64` on `PATH` (used by the demo scripts).
- An APS application with **OSS** + **Model Derivative** access from
  <https://aps.autodesk.com/myapps>.

## 2. Credentials / environment

Set the two required env vars (or use a `.env` in the working directory — see
`.env.example`):

```bash
export APS_CLIENT_ID=your_client_id
export APS_CLIENT_SECRET=your_client_secret
```

Alternatively store them in a profile:

```bash
raps config set client_id "your_client_id"
raps config set client_secret "your_client_secret"
```

Verify before recording:

```bash
raps auth test
```

> The 4-command flow uses **2-legged** OAuth (client credentials) only — no
> browser login needed. `raps auth login` (3-legged) is only required for the
> Data Management / ACC demos, not for this one.

## 3. Run the demo

### Option A — scripted (recommended for takes)

```bash
scripts/demo/run.sh
```

This runs all four commands against a tiny committed sample model
(`scripts/demo/sample-cube.obj`), prints the base64 URN, shows a live spinner
during translation, and finishes by printing the **Autodesk Viewer URL** for the
translated SVF2 model. Use your own model with `--file`:

```bash
scripts/demo/run.sh --file /path/to/model.rvt
```

The bucket name is derived from the hostname (`raps-demo-<host>`) so it is stable
across takes and `reset.sh` can find it.

### Option B — single built-in command

`raps demo model-pipeline` runs the same upload → translate → watch → summary
flow end to end (it generates a synthetic cube if no `--file` is given) and now
prints the Viewer URL in its summary:

```bash
raps demo model-pipeline --keep-bucket
```

### Option C — type the four commands live

For a hand-typed take, run them one at a time. Copy the URN that `object upload`
prints (it also prints the exact `translate` command to run next):

```bash
raps auth test
raps bucket create --key raps-demo
raps object upload raps-demo scripts/demo/sample-cube.obj
raps translate start <paste-urn-here> --format svf2 --watch
```

## 4. Reset / teardown between takes

```bash
scripts/demo/reset.sh
```

Deletes the demo bucket (and its objects) and clears the local state file. Safe
to run repeatedly. Override the target with `--bucket <name>`. If you used
Option C with a custom bucket, pass it explicitly:

```bash
scripts/demo/reset.sh --bucket raps-demo
```

## 5. What the audience sees (output polish)

- `object upload` prints the base64 URN under a bold yellow label plus the exact
  `translate ... --watch` command to run next.
- `translate start --watch` shows a single live spinner (`status=… progress=…`),
  then a green check and a **`Viewer:` URL** (underlined). In non-color/quiet
  mode it prints just the URL on its own line — easy to pipe or click.
- The Viewer URL is `https://aps.autodesk.com/viewer?urn=<urn>` (the official APS
  Viewer reference app). Opening it loads the translated model; on first use the
  reference viewer prompts for APS login (`viewables:read`).

---

## TO BUILD (not yet implemented — documented per the demo brief)

- **One-shot Viewer auto-open.** No `--open` flag exists to launch the Viewer URL
  in a browser at the end of `translate --watch`. (A browser-open helper already
  exists for 3-legged auth in `raps-kernel/src/auth/three_leg.rs` and could be
  reused.) For now, click/paste the printed URL.
- **Self-hosted viewer fallback.** `aps.autodesk.com/viewer` requires an
  interactive APS login for OSS-hosted URNs. A zero-login experience would need a
  small bundled viewer page that injects a `viewables:read` token. Out of scope
  for a minimal additive change.
- **`raps demo run` subcommand.** The scripted flow lives in `scripts/demo/` to
  stay additive. Promoting it to a first-class `raps demo wow`/`run` subcommand
  (mirroring `model-pipeline`) would remove the shell-script dependency.
