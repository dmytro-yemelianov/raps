---
layout: default
title: Admin Commands
---

# Admin Commands

Bulk account administration for ACC/BIM 360 accounts.

> **Note:** Admin commands require 3-legged OAuth with account admin privileges. Run `raps auth login` first.

## User Commands

### raps admin user add

Add a user to multiple projects with a specified role.

```
raps admin user add [OPTIONS] <EMAIL>
```

Options: `-a/--account`, `-r/--role`, `-f/--filter`, `--project-ids`, `--concurrency`, `--dry-run`

### raps admin user remove

Remove a user from multiple projects.

```
raps admin user remove [OPTIONS] <EMAIL>
```

Options: `-a/--account`, `-f/--filter`, `--project-ids`, `--concurrency`, `--dry-run`

### raps admin user update

Update a user's role and/or company across multiple projects.

```
raps admin user update [OPTIONS] <EMAIL>
```

Options: `-a/--account`, `-r/--role`, `--company`, `--from-role`, `-f/--filter`, `--project-ids`, `--from-csv`, `--concurrency`, `--dry-run`

### raps admin user add-to-all-projects

Add a user to all active projects in an account with an optional role.

```
raps admin user add-to-all-projects [OPTIONS] <EMAIL>
```

Options: `-a/--account`, `--role`, `--concurrency`, `--dry-run`

### raps admin user list

List users in an account or specific project.

```
raps admin user list [OPTIONS]
```

Options: `-a/--account`, `-p/--project`, `--role`, `--status`, `--search`

### raps admin user import

Import users to a project from a CSV file.

```
raps admin user import --project <PROJECT_ID> --from-csv <FILE>
```

---

## Project Commands

### raps admin project list

```
raps admin project list [OPTIONS]
```

Options: `-a/--account`, `-f/--filter`, `--status`, `--platform`, `--limit`

### raps admin project create

```
raps admin project create [OPTIONS] --name <NAME>
```

Options: `-a/--account`, `-n/--name`, `-t/--type`, `--classification`, `--start-date`, `--end-date`, `--timezone`

### raps admin project update

```
raps admin project update [OPTIONS] --project <PROJECT>
```

Options: `-a/--account`, `-p/--project`, `-n/--name`, `--status`, `--start-date`, `--end-date`

### raps admin project archive

```
raps admin project archive --account <ACCOUNT_ID> --project <PROJECT_ID>
```

---

## Folder Commands

### raps admin folder rights

Update folder permissions for a user across multiple projects.

```
raps admin folder rights [OPTIONS] <EMAIL>
```

Options: `-a/--account`, `--permission`, `--folder`, `-f/--filter`, `--dry-run`

---

## Operation Commands

### raps admin operation status / resume / cancel / list

```
raps admin operation status [--id <UUID>]
raps admin operation resume [--id <UUID>]
raps admin operation cancel [--id <UUID>]
raps admin operation list [--status <STATUS>] [--limit <N>]
```

---

## Company Commands

### raps admin company-list

```
raps admin company-list [-a/--account <ACCOUNT>]
```

---

See also: [Admin Commands on raps-website](https://raps-website/docs/admin)
