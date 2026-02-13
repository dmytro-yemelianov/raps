# RAPS CLI Feature Gap Analysis

Generated: 2026-02-13

## BLANK ZONES (Complete Gaps)

| # | Gap | Impact | Category |
|---|-----|--------|----------|
| B1 | No Delete for Issues, RFIs, Submittals | Medium | CRUD completeness |
| B2 | No Get for Webhooks | Low | CRUD completeness |
| B3 | No Update for Webhooks | Low | CRUD completeness |
| B4 | No List for DA Workitems | Medium | CRUD completeness |
| B5 | No List for Reality Scenes | Medium | CRUD completeness |
| B6 | Zero MCP coverage for Webhooks (5 CLI commands) | High | Surface parity |
| B7 | Zero MCP coverage for Design Automation (9 CLI commands) | High | Surface parity |
| B8 | Zero MCP coverage for Reality Capture (7 CLI commands) | Medium | Surface parity |
| B9 | No cross-project reports for Submittals/Checklists/Assets | High | Portfolio visibility |
| B10 | No Update for Objects (rename, copy CLI-side) | Medium | CRUD completeness |
| B11 | No Read for Folder Rights (can set, can't query) | High | Security audit |
| B12 | No CSV import for Issues, RFIs, Submittals, Checklists | Medium | Bulk operations |
| B13 | No hub info in MCP | Low | Surface parity |
| B14 | No Folder Update/Delete/Rename anywhere | Medium | CRUD completeness |
| B15 | No admin user import --from-csv (new users) | High | Bulk operations |

## GREY ZONES (Partial Coverage)

| # | Grey Zone | What Exists | What's Missing |
|---|----------|-------------|----------------|
| G1 | Asset Get/Delete surface split | MCP: delete, CLI: get | CLI: delete, MCP: get |
| G2 | Item Delete/Rename MCP-only | MCP has both | No CLI surface |
| G3 | Object ops CLI/MCP split | CLI: 6 ops, MCP: 9 ops | CLI missing: info,copy,urn,delete-batch |
| G4 | Issue Comments MCP gap | CLI has full CRUD | MCP: zero comment support |
| G5 | ACC Project CLI gap | MCP: create/update/archive | CLI: only admin project list |
| G6 | Checklist Templates unexposed | API method exists | No CLI or MCP |
| G7 | Translation output formats | Only Table+JSON | Missing YAML, CSV |
| G8 | Auth output formats | Only Table+JSON | Cosmetic inconsistency |
| G9 | DA/Reality output formats | Only Table+JSON | Limits automation |
| G10 | Bulk ops scope | Users + Folder Rights only | Issues,RFIs,Assets,Submittals single-shot |
| G11 | Company update no read-back | Can set company | Can't verify current |
| G12 | Project Users zero CLI | 5 MCP tools | 0 CLI commands |
| G13 | Template convert MCP-only | MCP tool exists | No CLI command |
| G14 | --wait pattern inconsistent | translate, reality | DA workitem, bulk ops |
| G15 | --since date filter inconsistent | report commands | issue list, rfi list |
| G16 | Webhook MCP gap | CLI: 5 commands | MCP: 0 tools |

## STRUCTURAL PATTERN INCONSISTENCIES

| Pattern | Applied To | Missing From |
|---------|-----------|--------------|
| --wait polling | translate, reality | DA workitem, bulk admin ops |
| --dry-run | admin user/folder ops | issue create, rfi create |
| --from-csv | admin user update | issue create, rfi create, submittal create |
| Progress bars | admin bulk, upload-batch | report iteration, translate download |
| --filter expressions | admin project list | issue list, rfi list |
| Operation resumability | admin bulk ops | DA workitems, translation jobs |
| --since date filter | report commands | issue list, rfi list, submittal list |
| Interactive prompts | translate, bucket, webhook | issue create (partial), rfi create |

## COVERAGE SCORES (lowest = highest priority)

| Entity | Score | Notes |
|--------|-------|-------|
| Design Automation | 33% | Zero MCP, limited output |
| Project Users CLI | 33% | Zero CLI surface |
| Webhooks | 39% | Zero MCP coverage |
| Reality Capture | 39% | Zero MCP, limited output |
| Translation | 47% | Limited output, no manifest MCP |
| Folder Rights | 61% | Can't read/audit |
| Folder | 61% | No update/delete |
| Item | 61% | CLI gaps for delete/rename |
| Asset | 63% | Surface split |
| Submittal | 63% | No cross-project, no delete |
| Checklist | 63% | No cross-project, templates hidden |
| Report | 65% | Only RFI+Issues, no submittals/checklists/assets |
