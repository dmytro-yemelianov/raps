---
layout: default
title: APS Feature Coverage
---

# APS Feature Coverage

This page provides a comprehensive overview of how RAPS CLI covers the available Autodesk Platform Services (APS) features. 

| Service | Feature | RAPS Command | Coverage Level |
|---------|---------|--------------|----------------|
| **Authentication** | 2-legged OAuth | `auth test` | ✅ Full |
| | 3-legged OAuth | `auth login` | ✅ Full |
| | Token Inspection | `auth inspect-token` | ✅ Full |
| **Object Storage (OSS)** | Bucket Management | `bucket` | ✅ Full |
| | Object Upload/Delete | `object upload/delete` | ✅ Full |
| | Resumable Uploads | `object upload --resume` | ✅ Full |
| | Batch Operations | `object upload --batch` | ✅ Full |
| | Signed URLs | `object signed-url` | ✅ Full |
| **Model Derivative** | Job Management | `translate start` | ✅ Full |
| | Status Polling | `translate status --wait` | ✅ Full |
| | Manifest/Metadata | `translate manifest` | ✅ Full |
| | Derivative Download | `translate download` | ✅ Full |
| | SVF/SVF2 Support | `--format svf/svf2` | ✅ Full |
| **Data Management** | Hubs & Projects | `hub list`, `project list` | ✅ Full |
| | Folders & Items | `folder`, `item` | ✅ Full |
| | ACC/BIM 360 Support | Included in DM | ✅ Full |
| | Object Binding | `item bind` | ✅ Full |
| **ACC Issues** | Issue CRUD | `issue list/create/edit` | ✅ Full |
| | Comments/Attachments | `issue comment/attachment` | ✅ Full |
| | State Transitions | `issue transition` | ✅ Full |
| **ACC RFIs** | RFI CRUD | `rfi list/create/update`| ✅ Full |
| **ACC Assets** | Asset CRUD | `acc asset list/...` | ✅ Full |
| **ACC Submittals** | Submittal CRUD | `acc submittal list/...` | ✅ Full |
| **ACC Checklists** | Checklist CRUD | `acc checklist list/...` | ✅ Full |
| | Templates | `acc checklist templates`| ✅ Full |
| **Design Automation** | Engines/Activities | `da engines/activities` | ✅ Partial |
| | Work Items | `da workitem run/get` | ✅ Partial |
| | App Bundles | `da appbundles` | ✅ Partial |
| **Reality Capture** | Photoscene CRUD | `reality create/get/del`| ✅ Full |
| | Photo Management | `reality upload` | ✅ Full |
| | Result Download | `reality result` | ✅ Full |
| **Webhooks** | Subscription CRUD | `webhook create/del/list`| ✅ Full |
| | Event Discovery | `webhook events` | ✅ Full |
| | Endpoint Testing | `webhook test` | ✅ New |

## Implementation Status Legend

- ✅ **Full**: All major endpoints and operations are implemented.
- ✅ **Partial**: Basic operations are supported, but some advanced settings or secondary endpoints may be missing.
- ✅ **New**: Feature recently added in the current major release.
- ⬜ **Planned**: Not yet implemented, but on the roadmap.

> [!NOTE]
> RAPS CLI prioritized user-centric workflows. If you find an APS API feature that is missing and critical for your automation, please [open an issue](https://github.com/dmytro-yemelianov/raps/issues).
