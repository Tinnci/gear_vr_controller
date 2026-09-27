# Logging policy

The controller is a latency-sensitive desktop app. Logging must explain failures
without putting disk latency on the input or UI thread. It is diagnostic output,
not a durable audit trail. Events can be lost during overload or abnormal exit.

## Layers and severity

Use one chronological stream. The Rust module target identifies the application,
domain, transport, Windows platform or presentation layer. Separate files for
each severity would duplicate events and make sequences harder to reconstruct.
Console and file output receive the same level/module filter.

| Level | Use |
| --- | --- |
| ERROR | An operation failed; input is paused or user action is needed. |
| WARN | A recoverable attempt failed, a retry is deferred, or configuration falls back. |
| INFO | Application lifecycle, connection, mode, output and calibration results. |
| DEBUG | Low-volume algorithm decisions and internal state changes. |
| TRACE | Input values, individual packets, calibration progress and per-input actions. |

Write stable English messages. Use a lowercase dotted `event` field, such as
`connection.finished`, plus typed fields. Avoid interpolating values into the
message. Prefer `attempt_id`, `status`, `reason`, `elapsed_ms`, `delay_ms`,
`expected_bytes`, `actual_bytes`, `previous`, `mode` and `source` where relevant.
Name units explicitly. Keep translated instructions in the UI, not in log IDs.

File output is JSON lines. Defaults include UTC timestamp, level, module target,
source file and line, structured fields and active tracing spans. The connection
span carries a monotonically increasing attempt ID. Connection start/finish
events also carry that ID; completion records duration and success. File names
identify the process and startup session, so IDs are scoped to one process run.
The start event identifies app version, PID and storage policy. Configuration
can disable source/target fields; leave them enabled for code diagnosis.

```rust
tracing::warn!(
    event = "ble.notifications.failed",
    attempt,
    max_attempts,
    error = %error,
    "Notification subscription attempt failed"
);
```

Errors reported by the worker use their actual severity. Operational failures
in the UI are logged before being shown. Connection errors preserve their
`anyhow` cause chain. Rust panics are sent to the logger and the original panic
hook; this does not capture native crashes or guarantee delivery after a panic.
Repeated power failures are reported at most once every 30 seconds while failure
continues; a successful call clears that suppression window.

## Rotation and retention

Default `log_settings` values (existing files inherit missing fields):

```json
{
  "level": "info",
  "rotation": "daily",
  "retention_days": 14,
  "max_file_size_bytes": 10485760,
  "max_files": 20,
  "file_logging_enabled": true,
  "console_logging_enabled": false
}
```

Settings validate segment size from 64 KiB to 100 MiB and file count from 2 to
100. Restart the app after editing logging settings. The default folder is
`%LOCALAPPDATA%\GearVRController\logs`.
Relative configured folders resolve under `%LOCALAPPDATA%\GearVRController`;
opening the log folder uses the same resolution. Absolute folders are preserved.

Time policies are `minutely`, `hourly`, `daily` and `never`, using UTC epoch
buckets. Size rotation remains active with every time policy. Open a new segment
before a record would exceed its cap. Records are not split across segments.
New sessions use new files; an existing segment is never truncated or appended.
Names have this form:

`<prefix>-<day|hour|minute|never>-<bucket>-<startup_ns>-<pid>-<sequence>.log`

Retention runs at startup, each rotation and, while writing, at least once an
hour. It removes expired files first, then the oldest files to satisfy count
and space limits, reserving one segment for the active writer. Cleanup recognizes
exact generated names and legacy `<prefix>.log` or time-bucket names. It skips
directories, symlinks and unrelated files. Changing the prefix or folder leaves
the previous location outside the new policy.

For one writer and successful filesystem operations, default managed log space
is at most **200 MiB**, with at most 20 files. This is a capacity limit, not a
promise of 14 days of history. Creation can briefly add an empty extra file
before pruning. Other processes, deletion failures, old prefixes, unrelated
files and diagnostic exports are outside that bound. Concurrent processes use
different segment names, but folder-wide retention is best effort; this is not
a cross-process quota service. I/O and cleanup errors remain observable counters.

## Runtime cost and loss policy

Let `L` be a formatted event's byte length, `D` directory entries and `N` matching
unexpired archive files.

| Operation | Time | Additional memory |
| --- | --- | --- |
| Enabled event formatting and enqueue | O(L), filter-dependent | O(L) temporary buffer |
| Background record write | O(L) transfer; normal rotation check O(1) | O(1) writer state |
| Cleanup | O(D + N log N), plus filesystem calls | O(N) metadata |
| Diagnostic counter snapshot | O(1), plus filter-string copy | Small fixed metadata |

Disk work runs on one `tracing-appender` thread, without a shared file mutex on
producers. Formatting still happens on the caller; asynchronous output does not
make TRACE free. Console output, if explicitly enabled, remains synchronous.
Normal managed archives are bounded by the configured file count; the first
cleanup of a large legacy folder can take longer and happens during startup.

The file queue accepts **4096 records**. A complete formatted record larger than
**16 KiB** is rejected and counted rather than truncated into invalid JSON.
Queued payload is therefore at most **64 MiB**, plus queue/allocation overhead;
this is not a bound on the whole process or a temporarily formatted oversized
event. Queue saturation drops new events rather than blocking input. Even ERROR
records can be lost under saturation. `dropped_events` and `oversized_events`
make this tradeoff visible; there is no priority queue or durable error channel.

Keep the logging guard alive until UI and worker producers stop. Normal shutdown
requests the appender's bounded drain/flush through its worker guard. Forced
termination, disk failure or the library's shutdown timeout can still lose tail
records. File flush does not mean a power-loss-safe disk sync.

## Diagnosis and data handling

Use Help → Diagnostic details → Open log folder to inspect recent `.log` files.
Parse each line as JSON and filter by `fields.event`, `level`, `target` and the
connection attempt ID. The default source location maps an event back to code.
Start from `connection.started`, follow the GATT/pairing/subscription stages,
then inspect `connection.finished` and adjacent ERROR records.

For targeted detail set, for example:

`RUST_LOG=info,gear_vr_controller_rust::infrastructure::bluetooth=debug`

Field-filter regex matching is disabled. Invalid `RUST_LOG` falls back to the
configured level and records a warning. Raw packet logging requires a Debug
build, `debug_raw_data_logging=true` and TRACE. Pointer/key traces also expose
user activity, so enable them only for a short reproduction. Routine connection
logs omit device names/MAC addresses and foreground process names. Error text
can still contain OS paths or other context; review files before sharing.

Export diagnostic summary records app/runtime state, the effective logging
configuration and current counters: queue drops, oversized records, disk I/O
errors and retention errors. It does not attach logs, export raw sensor/input
values, expose device IDs or force a synchronous file flush. Counts are a
point-in-time snapshot; a queued write can fail after the export.
Discovery records `ble.scan.started` and `ble.scan.finished`, correlated by
`scan_id`. Completion includes duration, advertisement count, rendered device
count, service matches, unnamed devices, read errors and dropped snapshot updates.
Read failures warn once per scan and are then counted. Per-device identifiers
and names are intentionally absent from these events.

## Prior implementation assessment

Before this change, producers took a shared mutex and synchronously wrote each
event. Time rotation was O(1), but disk latency entered the input path. Startup
cleanup scanned files in O(D), with no ongoing cleanup. It missed the `never`
filename. There was no segment-size, file-count or total-byte bound: storage
could grow indefinitely, and even 14-day daily retention did not cap a day's
volume. Worker errors were logged as INFO; message styles mixed positional
strings and fields, with no connection correlation or logger-health snapshot.

## References

The package smoke check temporarily sets INFO for its child process, parses its
actual JSON records and requires startup, shutdown and zero failure counters.
It restores the caller's filter. Unit tests also cover size/time boundaries,
legacy retention, whole-record rejection, saturation and background draining.

- [tracing-appender non-blocking builder](https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/struct.NonBlockingBuilder.html): bounded queues and the lossy/backpressure tradeoff.
- [OWASP logging guidance](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html): consistent event attributes, sanitization and data minimization. This desktop app uses the relevant diagnostic guidance; it does not claim security-audit compliance.
