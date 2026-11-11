# Slurm integration

`src/slurm/` owns the external Slurm boundary. UI code validates input and sends
owned `SlurmCommand` requests through `SlurmHandle`. It never waits for a Slurm
process. Completed work arrives through `AppMessage::Slurm(SlurmEvent)`.

## Responsibilities

- `client.rs` is the only Slurm process execution boundary. It captures output
  and errors and accepts explicit executable paths for isolated tests.
- `commands.rs` defines typed write operations and translates them to argv.
  Input is passed as process arguments, never shell source.
- `jobs.rs` plans controller/accounting queries and translates their filter
  dialects. Its parser normalizes both formats into the same `Job` model.
  The client caches the installed `sacct --helpformat` schema on the history
  worker's first query. Core job fields are required; optional output paths,
  reason, working directory, and command are requested only when supported.
  Missing metadata is normalized to empty fields without shifting columns.
- `resources.rs` fetches and parses resource information. The client's cached
  primary group avoids running `id -gn` on every resource refresh.
- `runtime.rs` owns independent periodic workers for active jobs, history, and
  resources, plus a serial command worker. A slow accounting query cannot hold
  up active-job updates or user commands.
- `snapshots.rs` reconciles data independently of I/O, timers, and rendering.
  The UI preserves selection by job identity when applying a merged snapshot.

There is deliberately no async runtime, generic task framework, or worker per
keypress. The existing crossbeam channels and a fixed set of threads suffice.

## Queries and cadence

Active jobs use `squeue --array` with explicit long-format fields and no time
window, so allocations that started more than two days ago remain visible.
History uses `sacct --allocations` over the previous two days and requests
terminal states only. State filters select the relevant sources; e.g.
`--states=RUNNING` skips accounting entirely. Other scope filters are preserved.
`--me` remains restricted to the current user on both sources.
`SPECIAL_EXIT`/`SE` represents a requeued job held in the controller, not a
terminal accounting state; it stays on the live source and is never sent to
`sacct` as a state filter.

Standard Slurm 20.11 does not advertise the `Stdout`, `Stderr`, or `Command`
accounting fields present in some vendor builds. History queries omit these
fields instead of failing the whole refresh. When paths are unavailable, the
normalizer uses the standard Slurm output filename relative to `WorkDir` when
available; custom historical output paths cannot be recovered from absent data.

The default account view requests other users' live allocations but restricts
the accounting query to the current user on the server. It does not repeatedly
download everyone else's history and discard it afterwards. If `USER` is
unavailable in that view, history is disabled rather than widening its scope.

Live jobs/resources default to two seconds and history to 60 seconds, configured
by `--slurm-refresh` and `--history-refresh`. Each worker queries immediately on
startup, then waits its interval after completion. Intervals are clamped to at
least one second to prevent a zero-interval busy loop. There is no synchronous
startup resource fetch.

## Reconciliation

Snapshots carry their source and query start time. Older responses from the
same source are ignored. Live allocations win over history with the same job
identity, including requeued jobs. Array elements have distinct identities.

When a live allocation disappears, retain its last observed state until a
history query started after its disappearance confirms a terminal state.
This prevents temporary disappearance during accounting lag and avoids treating
an older terminal record as proof that a requeued allocation has just finished.
Other users' jobs in the default account view are removed immediately, since
their history is outside the query scope. Explicit state, name, partition, or
node filters also remove missing allocations immediately: a job may have moved
outside the selected scope without completing.
Each successful history snapshot replaces the previous history window.
Query failures preserve the last successful data.

## Operations and shutdown

The command worker executes accepted operations in order. Submitting an
operation closes its confirmation/edit dialog immediately; command errors are
delivered later. Errors queue until the current dialog closes, preserving any
input the user entered while the command was running. Commands are not retried
automatically.

Dropping the handle disconnects the stop channel. Idle pollers wake and exit;
workers stop after an in-flight process returns. The UI never joins a worker
that may be blocked in an external command. Unstarted queued commands are
discarded on shutdown.

## Scope

This phase uses only the installed Slurm CLI. Native adapters, helper processes,
ABI dispatch, and REST integration are not part of the implementation. No Slurm
headers, native libraries, compiler, or additional service are required at runtime.
The existing prebuilt-binary distribution remains unchanged.

Timeouts, command timing, failure backoff, and query-status presentation are
deferred. In this phase a hung process can still stall its own worker, and a
missing job can retain its last known state while accounting remains unavailable.
The client and runtime boundaries are the extension points for the next phase.
