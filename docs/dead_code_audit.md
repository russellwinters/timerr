# Dead Code Audit

This document records all uses of `#[allow(dead_code)]` in the codebase, explains the context
of each suppressed warning, and evaluates whether the code has a reasonable future use or should
be removed.

All occurrences are in `src/models.rs`.

---

## 1. `Project::time_sum` — line 8

```rust
#[allow(dead_code)]
pub time_sum: i64, // Total time in seconds
```

**Context**

`time_sum` is a field on the `Project` struct that stores the project's cumulative tracked time in
seconds. The database column is kept up-to-date: `db::stop_timer` adds each completed session's
duration to it with `UPDATE projects SET time_sum = time_sum + ?1`. The field is also read back
from the database and stored into the struct in every query that returns a `Project`
(`upsert_project`, `get_all_projects`, `get_project_by_name`, `get_active_running_projects`).

Despite being populated on every `Project` value, the field is **never read by any Rust code
path**. The `list` command calculates per-project totals on the fly by calling
`get_project_time_in_range` (which sums from the `instances` table), so the cached `time_sum`
value is bypassed entirely.

**Opportunity for use**

`time_sum` was clearly designed as a fast-access cache for the project's all-time total. It could
replace the `get_project_time_in_range(conn, project.id, DateTime::UNIX_EPOCH, now)` call in the
`list` command for the "Total" column, reducing one database query per project. Alternatively, if
the cache-vs-live-recalculation trade-off is intentional, `time_sum` could be removed from the
struct and the SQL `SELECT` to keep the model clean.

---

## 2. `Project::status` — line 10

```rust
#[allow(dead_code)]
// Status is stored and used in SQL filtering; field reserved for future use
pub status: String, // 'active' or 'inactive'
```

**Context**

`status` is a `TEXT` column on the `projects` table with valid values `'active'` and
`'inactive'`. It is used heavily at the SQL layer: every query that retrieves projects filters on
`WHERE status = 'active'`, and `db::delete_project` sets a project to `'inactive'` to perform a
soft delete. The field is returned from the database and stored in the struct, but **no Rust
application code ever reads `project.status`** after the struct is populated.

**Opportunity for use**

Because soft-deleted projects are filtered out in SQL, there is currently no Rust-level scenario
where the application needs to inspect `status`. Potential future uses include:

- Displaying status in a `timerr list --all` command that shows inactive projects.
- A `timerr restore <project_name>` command that reactivates an inactive project (noted as future
  work in `docs/delete_command_init.md`).
- Defensive assertions or logging that verify a returned project is indeed active.

Until such features are added, the field is needed in the struct to satisfy the row-mapping code
(`row.get(3)?`) but is otherwise unused in logic.

---

## 3. `Instance` struct — line 17

```rust
#[allow(dead_code)] // Fields are used for future functionality
pub struct Instance {
    pub id: i64,
    pub project_id: i64,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
}
```

**Context**

`Instance` represents a single time-tracking session (a row in the `instances` table).
`db::create_instance` constructs and returns an `Instance`, but its caller in
`commands/start.rs` discards the return value:

```rust
create_instance(conn, project.id, start_time)?;  // return value is dropped
```

None of the other DB functions (`stop_timer`, `get_active_instance_start_time`,
`get_active_running_projects`, `get_project_time_in_range`) return `Instance` values; they work
with raw column data instead. This means **no `Instance` value is ever used after construction**.

**Opportunity for use**

The struct is the natural return type for any future function that needs to expose full instance
data to callers. Immediate candidates:

- Returning an `Instance` from `start` and displaying the formatted start time to the user.
- A `timerr history <project_name>` command that lists all sessions for a project.
- The two `impl Instance` methods described below.

---

## 4. `Instance::duration()` — line 28

```rust
#[allow(dead_code)] // Method reserved for future use
pub fn duration(&self) -> Option<i64> {
    self.stop_time
        .map(|stop| (stop - self.start_time).num_seconds())
}
```

**Context**

`duration` computes how long a completed instance ran, in seconds, returning `None` if the
instance has no `stop_time` (i.e., it is still running). The method is never called; duration
calculations in the current codebase are done ad-hoc at the call site (e.g., in `db::stop_timer`
with `(stop_time - start_time).num_seconds()`).

**Opportunity for use**

This method provides a clean, reusable abstraction over the inline arithmetic already present in
`db::stop_timer`. It would become directly useful for:

- Any reporting or history command that needs per-session durations.
- Replacing the inline calculation in `db::stop_timer` to reduce duplication.

---

## 5. `Instance::is_running()` — line 35

```rust
#[allow(dead_code)] // Method reserved for future use
pub fn is_running(&self) -> bool {
    self.stop_time.is_none()
}
```

**Context**

`is_running` returns `true` when `stop_time` is `None`, indicating the instance has not been
stopped. The method is never called; all running-state checks in the codebase are performed
directly in SQL (`WHERE stop_time IS NULL`) rather than on `Instance` values in Rust.

**Opportunity for use**

The method would become useful alongside any feature that retrieves and works with `Instance`
values in Rust rather than delegating the check to SQL (e.g., filtering a `Vec<Instance>` in
application logic, or guarding against stopping an already-stopped instance at the model level).

---

## Summary

| Location | Attribute target | Currently used in Rust? | Reasonable future use? |
|---|---|---|---|
| `Project::time_sum` | Field | No (populated but never read) | Yes — fast-access total cache for `list` |
| `Project::status` | Field | No (populated but never read) | Yes — needed for restore/list-all features |
| `Instance` struct | Struct | No (constructed but return value discarded) | Yes — foundation for history/reporting commands |
| `Instance::duration()` | Method | No | Yes — reusable per-session duration helper |
| `Instance::is_running()` | Method | No | Yes — convenience predicate for future instance-level logic |

All five suppressions are in `src/models.rs`. None of the dead code appears to be vestigial;
each item was written in anticipation of features that are not yet implemented. The suppressions
are reasonable for now, but should be revisited if the planned features are deprioritised or
removed from scope.
