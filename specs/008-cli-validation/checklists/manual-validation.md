# Manual Validation Checklist: CLI Command Coverage

**Purpose**: Systematic validation of all 250+ RAPS CLI commands — help text, argument parsing, defaults, output formats, error handling. Cross-referenced against automated test coverage to identify gaps.
**Created**: 2026-03-13
**Feature**: [specs/008-cli-validation/spec.md](../spec.md)

**Legend**:
- `[x]` = Validated (passes)
- `[!]` = Validated (FAILS — bug found)
- `[ ]` = Not yet validated
- `[~]` = Skipped (requires auth/infra not available)
- `[T]` = Has automated test
- `[-]` = No automated test (GAP)

---

## Domain 1: Authentication (`auth`)

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK001 | `auth test` | [T] | [T] | [ ] | [ ] | help+args |
| CHK002 | `auth login` | [T] | [T] | [ ] | [ ] | help+args |
| CHK003 | `auth logout` | [T] | [T] | [ ] | [ ] | help+logout_no_token |
| CHK004 | `auth status` | [T] | [T] | [T] | [ ] | help+output_flag |
| CHK005 | `auth whoami` | [T] | [ ] | [ ] | [ ] | help only |
| CHK006 | `auth inspect` | [T] | [ ] | [ ] | [ ] | help only |

**Gaps**: whoami/inspect missing args validation tests.

---

## Domain 2: Object Storage (`bucket`, `object`)

### Bucket Commands

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK010 | `bucket create` | [T] | [T] | [T] | [T] | full |
| CHK011 | `bucket list` | [T] | [T] | [ ] | [ ] | help+args |
| CHK012 | `bucket info` | [T] | [T] | [ ] | [ ] | help+args |
| CHK013 | `bucket delete` | [T] | [T] | [ ] | [ ] | help+args |

### Object Commands

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK020 | `object upload` | [T] | [T] | [ ] | [ ] | help+file_validation |
| CHK021 | `object upload` (stdin `-`) | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK022 | `object upload-batch` | [T] | [ ] | [ ] | [ ] | help only |
| CHK023 | `object download` | [T] | [ ] | [ ] | [ ] | help only |
| CHK024 | `object download-bulk` | [T] | [T] | [ ] | [ ] | help+args+paths |
| CHK025 | `object list` | [T] | [T] | [T] | [ ] | help+output_format |
| CHK026 | `object delete` | [T] | [ ] | [ ] | [ ] | help only |
| CHK027 | `object signed-url` | [T] | [T] | [ ] | [ ] | help+args |
| CHK028 | `object info` | [T] | [ ] | [ ] | [ ] | help only |
| CHK029 | `object diff` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK030 | `object copy` | [T] | [ ] | [ ] | [ ] | scenario only |
| CHK031 | `object rename` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK032 | `object batch-copy` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK033 | `object batch-rename` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK034 | `object upload-status` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK035 | `object upload-abort` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK036 | `object upload-cleanup` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK037 | `object inspect` | [T] | [T] | [ ] | [ ] | help+args+detection |
| CHK038 | `object audit` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK039 | `object tag set` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK040 | `object tag get` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK041 | `object tag delete` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK042 | `object tag search` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

**Gaps**: stdin upload (CHK021), diff, rename, batch-*, upload-mgmt, audit, all tag subcommands — no tests.

---

## Domain 3: Model Derivative (`translate`)

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK050 | `translate start` | [T] | [ ] | [ ] | [ ] | help only |
| CHK051 | `translate status` | [T] | [ ] | [ ] | [ ] | help only |
| CHK052 | `translate manifest` | [T] | [ ] | [ ] | [ ] | help only |
| CHK053 | `translate derivatives` | [T] | [ ] | [ ] | [ ] | help only |
| CHK054 | `translate download` | [T] | [ ] | [ ] | [ ] | help only |
| CHK055 | `translate metadata` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK056 | `translate tree` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK057 | `translate properties` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK058 | `translate query-properties` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK059 | `translate preset list` | [T] | [ ] | [ ] | [ ] | help only |
| CHK060 | `translate preset show` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK061 | `translate preset create` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK062 | `translate preset delete` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK063 | `translate preset use` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK064 | `translate timeline` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

**Gaps**: metadata, tree, properties, query-properties, preset CRUD, timeline — no tests.

---

## Domain 4: Data Management (`hub`, `project`, `folder`, `item`)

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK070 | `hub list` | [T] | [T] | [ ] | [ ] | help+creds |
| CHK071 | `hub info` | [T] | [T] | [ ] | [ ] | help+args |
| CHK072 | `project list` | [T] | [T] | [ ] | [T] | help+non_interactive |
| CHK073 | `project info` | [T] | [T] | [ ] | [T] | help+non_interactive |
| CHK074 | `folder list` | [T] | [T] | [ ] | [ ] | help+creds |
| CHK075 | `folder create` | [T] | [T] | [ ] | [T] | help+non_interactive |
| CHK076 | `folder rename` | [ ] | [ ] | [ ] | [T] | non_interactive only |
| CHK077 | `folder delete` | [T] | [ ] | [ ] | [ ] | help only |
| CHK078 | `folder permissions` | [T] | [ ] | [ ] | [ ] | help only |
| CHK079 | `item info` | [T] | [T] | [ ] | [ ] | help+args |
| CHK080 | `item versions` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK081 | `item create-from-oss` | [T] | [T] | [ ] | [ ] | help+args |
| CHK082 | `item delete` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK083 | `item rename` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK084 | `init` | [T] | [ ] | [ ] | [T] | help+non_interactive |
| CHK085 | `status` | [T] | [ ] | [ ] | [ ] | scenario only |

**Gaps**: folder rename (help), item versions/delete/rename — no tests.

---

## Domain 5: ACC Modules (`issue`, `rfi`, `acc`, `report`)

### Issues

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK090 | `issue list` | [T] | [T] | [ ] | [ ] | help+args |
| CHK091 | `issue create` | [T] | [T] | [ ] | [ ] | help+args |
| CHK092 | `issue create --issue-subtype-id` | [-] | [-] | [-] | [-] | **NO TEST** (new flag) |
| CHK093 | `issue update` | [T] | [ ] | [ ] | [ ] | help only |
| CHK094 | `issue types` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK095 | `issue comment list` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK096 | `issue comment add` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK097 | `issue comment delete` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK098 | `issue attachments` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK099 | `issue transition` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK100 | `issue delete` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

### RFIs

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK105 | `rfi list` | [T] | [ ] | [ ] | [ ] | help only |
| CHK106 | `rfi get` | [T] | [ ] | [ ] | [ ] | help only |
| CHK107 | `rfi create` | [T] | [ ] | [ ] | [ ] | help only |
| CHK108 | `rfi create --priority` default | [-] | [-] | [-] | [-] | **NO TEST** (default "Normal") |
| CHK109 | `rfi update` | [T] | [ ] | [ ] | [ ] | help only |
| CHK110 | `rfi delete` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

### ACC Resources

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK115 | `acc asset list` | [T] | [T] | [ ] | [ ] | help+args |
| CHK116 | `acc asset get/create/update/delete` | [T] | [ ] | [ ] | [ ] | help only |
| CHK117 | `acc submittal list/get/create/update/delete` | [T] | [ ] | [ ] | [ ] | help only |
| CHK118 | `acc checklist list/get/create/update/delete` | [T] | [ ] | [ ] | [ ] | help only |
| CHK119 | `acc checklist templates` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK120 | `acc export` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

### Reports

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK125 | `report rfi-summary` | [T] | [ ] | [ ] | [ ] | help only |
| CHK126 | `report issues-summary` | [T] | [ ] | [ ] | [ ] | help only |
| CHK127 | `report submittals-summary` | [T] | [ ] | [ ] | [ ] | help only |
| CHK128 | `report checklists-summary` | [T] | [ ] | [ ] | [ ] | help only |
| CHK129 | `report assets-summary` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

---

## Domain 6: Admin (`admin`)

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK130 | `admin user list` | [T] | [ ] | [ ] | [ ] | help only |
| CHK131 | `admin user create` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK132 | `admin user get` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK133 | `admin user update-account` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK134 | `admin user add` | [T] | [T] | [ ] | [ ] | help+args |
| CHK135 | `admin user remove` | [T] | [T] | [ ] | [ ] | help+args |
| CHK136 | `admin user update` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK137 | `admin user add-to-project` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK138 | `admin user remove-from-project` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK139 | `admin user update-in-project` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK140 | `admin user import` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK141 | `admin user export-permissions` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK142 | `admin user clone-permissions` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK143 | `admin user add-to-all-projects` | [T] | [T] | [ ] | [ ] | help+args+scenario |
| CHK144 | `admin folder set-permissions` | [T] | [ ] | [ ] | [ ] | help only |
| CHK145 | `admin project list/get/create/update/archive` | [T] | [ ] | [ ] | [ ] | help only |
| CHK146 | `admin project create-batch` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK147 | `admin operation status/resume/cancel/list` | [T] | [ ] | [ ] | [ ] | help only |
| CHK148 | `admin company list/get/search/create/update` | [T] | [ ] | [ ] | [ ] | help only |
| CHK149 | `admin role list` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

---

## Domain 7: Design Automation (`da`)

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK150 | `da engines` | [T] | [ ] | [T] | [ ] | help+output |
| CHK151 | `da appbundles` | [T] | [ ] | [ ] | [ ] | help |
| CHK152 | `da appbundle-create` | [T] | [ ] | [ ] | [ ] | help |
| CHK153 | `da appbundle-delete` | [T] | [T] | [ ] | [ ] | help+args |
| CHK154 | `da activities` | [T] | [ ] | [ ] | [ ] | help |
| CHK155 | `da activity-create` | [T] | [ ] | [ ] | [ ] | help |
| CHK156 | `da activity-delete` | [T] | [T] | [ ] | [ ] | help+args |
| CHK157 | `da run` | [T] | [T] | [ ] | [ ] | help+args |
| CHK158 | `da workitems` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK159 | `da status` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

---

## Domain 8: Webhooks (`webhook`)

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK160 | `webhook list` | [T] | [ ] | [ ] | [ ] | help |
| CHK161 | `webhook create` | [T] | [ ] | [ ] | [T] | help+non_interactive |
| CHK162 | `webhook get` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK163 | `webhook update` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK164 | `webhook delete` | [T] | [ ] | [ ] | [ ] | help |
| CHK165 | `webhook events` | [T] | [ ] | [ ] | [ ] | help |
| CHK166 | `webhook test` | [T] | [ ] | [ ] | [ ] | help |
| CHK167 | `webhook verify-signature` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK168 | `webhook serve` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK169 | `webhook status` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK170 | `webhook drain` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

---

## Domain 9: Reality Capture (`reality`)

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK175 | `reality list` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK176 | `reality create` | [T] | [ ] | [ ] | [ ] | help only |
| CHK177 | `reality upload` | [T] | [ ] | [ ] | [ ] | help only |
| CHK178 | `reality process` | [T] | [ ] | [ ] | [ ] | help only |
| CHK179 | `reality status` | [T] | [ ] | [ ] | [ ] | help only |
| CHK180 | `reality result` | [T] | [ ] | [ ] | [ ] | help only |
| CHK181 | `reality formats` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK182 | `reality delete` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

---

## Domain 10: Infrastructure & Tooling

### Config

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK185 | `config profile create/list/use/delete` | [T] | [T] | [ ] | [ ] | help+args+ops |
| CHK186 | `config profile export/import/diff` | [T] | [T] | [ ] | [ ] | help+args |
| CHK187 | `config profile current` | [T] | [ ] | [ ] | [ ] | works test |
| CHK188 | `config get/set` | [T] | [T] | [ ] | [ ] | help+args |
| CHK189 | `config context` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK190 | `config migrate-tokens` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK191 | `config wizard` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

### Cache, Doctor, Logs

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK195 | `cache stats/clear/dir/prune` | [T] | [ ] | [ ] | [ ] | help+ops |
| CHK196 | `doctor` | [T] | [T] | [T] | [ ] | help+json+yaml |
| CHK197 | `logs show/path/clear/follow` | [T] | [ ] | [ ] | [ ] | help+ops |

### Commands with ZERO Tests

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK200 | `lint` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK201 | `snapshot create/diff/list` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK202 | `marketplace` (all subcommands) | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK203 | `history` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK204 | `replay` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK205 | `watch` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK206 | `man` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK207 | `workflow` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK208 | `sync` | [T] | [ ] | [ ] | [ ] | help+flags |
| CHK209 | `swarm` (all subcommands) | [T] | [ ] | [ ] | [ ] | help only |
| CHK210 | `schema list/generate/all` | [T] | [ ] | [ ] | [ ] | help+ops |

### Other Commands

| # | Command | Help | Args | Output | Non-Interactive | Auto Test |
|---|---------|------|------|--------|-----------------|-----------|
| CHK215 | `api get/post/put/patch/delete` | [T] | [T] | [ ] | [ ] | help+args |
| CHK216 | `template list/info/create/update/archive` | [T] | [ ] | [ ] | [ ] | help+creds |
| CHK217 | `plugin list/enable/disable/info` | [T] | [ ] | [ ] | [ ] | help |
| CHK218 | `plugin trust/verify` | [-] | [-] | [-] | [-] | **NO TEST** |
| CHK219 | `plugin alias list/add/remove` | [T] | [ ] | [ ] | [ ] | help |
| CHK220 | `skill list/install/uninstall/info/path` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |
| CHK221 | `generate files` | [T] | [T] | [ ] | [ ] | help+ops |
| CHK222 | `demo` (all subcommands) | [T] | [ ] | [ ] | [ ] | help |
| CHK223 | `pipeline` | [T] | [T] | [ ] | [ ] | help+args+dry_run |
| CHK224 | `job status/list/cancel` | [T] | [T] | [T] | [ ] | help+args+ops |
| CHK225 | `inspect zip` | [T] | [T] | [ ] | [ ] | help+args |
| CHK226 | `docs mcp` | [T] | [ ] | [ ] | [ ] | help+ops |
| CHK227 | `mcp` | [ ] | [ ] | [ ] | [ ] | indirect only |
| CHK228 | `completions` | [T] | [ ] | [ ] | [ ] | help only |
| CHK229 | `shell` | [T] | [ ] | [ ] | [ ] | help only |
| CHK230 | `stats` | [ ] | [ ] | [ ] | [ ] | **NO TEST** |

---

## Bug Regression Checks (from v5.7.0 testing)

| # | Bug | Test Exists | Description |
|---|-----|-------------|-------------|
| CHK300 | Issue create `--issue-subtype-id` | [-] **NO** | Flag must be accepted and passed to API |
| CHK301 | Issue types shows subtype IDs | [-] **NO** | Output must include `(subtype-id)` |
| CHK302 | RFI create priority default | [-] **NO** | Default must be `"Normal"` not `"normal"` |
| CHK303 | Stdin upload requires bucket | [-] **NO** | `upload - -k name` without bucket must error |
| CHK304 | Object copy temp file sandbox | [-] **NO** | Temp file must be in cwd, not `/tmp` |
| CHK305 | Lint L020 false positive | [-] **NO** | `get_token()` lines must NOT trigger L020 |

---

## Coverage Summary

| Category | Total Commands | Have Help Test | Have Args Test | Have Output Test | Have Scenario | Zero Tests |
|----------|--------------|----------------|----------------|------------------|---------------|------------|
| Auth | 6 | 6 | 4 | 1 | 1 | 0 |
| Bucket | 4 | 4 | 4 | 1 | 0 | 0 |
| Object | 23 | 10 | 5 | 1 | 1 | 13 |
| Translate | 15 | 6 | 0 | 0 | 1 | 9 |
| Data Mgmt | 16 | 13 | 8 | 0 | 2 | 3 |
| ACC/Issue/RFI | 25 | 14 | 3 | 0 | 1 | 11 |
| Admin | 20 | 8 | 4 | 0 | 3 | 12 |
| DA | 10 | 8 | 4 | 1 | 3 | 2 |
| Webhooks | 11 | 5 | 0 | 0 | 1 | 6 |
| Reality | 8 | 5 | 0 | 0 | 0 | 3 |
| Config | 7 | 5 | 4 | 0 | 1 | 3 |
| Infrastructure | 31 | 14 | 3 | 2 | 0 | 12 |
| **TOTAL** | **176** | **98** | **39** | **6** | **14** | **74** |

**Bug regressions**: 0 of 6 have automated tests.
