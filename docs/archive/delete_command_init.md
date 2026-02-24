# Delete Command — Implementation Plan

## Summary

Add a `timerr delete <project_name>` command that soft-deletes a project by setting its `status` to `'inactive'`. Deleted projects no longer appear in listings. Deletion is blocked when the project has associated time-tracking instances.

---

## Acceptance Criteria

- `timerr delete <project_name>` marks the named project as `'inactive'` in the database.
- Deleted projects are **not** shown in `timerr list` output.
- Attempting to delete a project that has instances in the `instances` table returns an error and leaves the project unchanged.
- Attempting to delete a project that does not exist (or is already inactive) returns a clear error message.
- `timerr start <project_name>` and `timerr stop <project_name>` continue to work only for active projects.
- The `projects` table has a new `status` column (`TEXT NOT NULL DEFAULT 'active'`) added via a migration that is safe to run on an existing database.

---

## Database Changes

### Migration: Add `status` column to `projects`

```sql
ALTER TABLE projects ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
```

- Applied at application start-up inside `db::init_database()`.
- Safe to run repeatedly: the migration is skipped if the column already exists (detected via `PRAGMA table_info`).
- Valid values: `'active'` (default) | `'inactive'`.

### Foreign Key Constraint

- The `instances` table already has `FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE`.
- Because deletion is a **soft delete** (status change, not a row removal), the FK constraint is not triggered at the database level.
- The application-level `delete` command checks for the presence of any instances for the project and **rejects** the deletion if any are found.

---

## Implementation Plan

1. **`src/models.rs`** — Add `status: String` field to the `Project` struct.

2. **`src/db.rs`**
   - `init_database`: apply the `status` column migration after table creation.
   - `upsert_project`: include `status` in the `SELECT` query and returned `Project`.
   - `get_all_projects`: filter with `WHERE status = 'active'`.
   - `get_project_by_name`: filter with `WHERE status = 'active'`.
   - New function `has_instances(conn, project_id) -> Result<bool>`: returns `true` if any rows exist in `instances` for the given `project_id`.
   - New function `delete_project(conn, project_id) -> Result<()>`: sets `status = 'inactive'` for the given project.

3. **`src/commands/delete.rs`** — New file implementing the delete command:
   - Validate that `project_name` is non-empty.
   - Look up the active project by name; error if not found.
   - Check `has_instances`; error with a friendly message if instances exist.
   - Call `delete_project`; print success message.

4. **`src/commands/mod.rs`** — Expose `pub mod delete;`.

5. **`src/main.rs`** — Add `Delete { project_name: String }` variant to `Commands` enum and handle it in `main`.

6. **`docs/delete_command_init.md`** — This file.

---

## Files Changed

| File | Change |
|---|---|
| `src/models.rs` | Add `status: String` to `Project` |
| `src/db.rs` | Migration, updated queries, new `has_instances` / `delete_project` functions |
| `src/commands/delete.rs` | New delete command implementation |
| `src/commands/mod.rs` | Register `delete` module |
| `src/main.rs` | Add `Delete` subcommand |
| `docs/delete_command_init.md` | This planning document |

---

## Risks & Open Questions

- **Existing databases**: The `ALTER TABLE ADD COLUMN` migration must be idempotent. We guard against re-running it by checking `PRAGMA table_info('projects')` before executing.
- **Active timer on deleted project**: A project with a running (unstopped) timer will also have a row in `instances`, so `has_instances` will correctly block the deletion. The user must stop the timer before deleting.
- **Re-creating a deleted project**: If a user calls `timerr start <name>` after deletion, `upsert_project` (which uses `INSERT OR IGNORE`) will find no active project with that name and insert a fresh one. The old inactive row remains for audit purposes.
- **Future work**: Hard-delete or purge functionality, bulk delete, restore (`timerr restore <project_name>`).
