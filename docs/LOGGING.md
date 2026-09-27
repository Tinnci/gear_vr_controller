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

## Connection failure diagnostics

Connection failures retain a typed category and the operation's native GATT
status in the error chain. `connection.failed` includes the attempt ID, category
and full cause; the UI details retain that attempt ID. Opening a Windows device
handle is `ble.device.opened`, not evidence of an established connection.
Initialization writes are awaited and checked before `ble.init.finished`.
Transient notification retries remain warnings in the file, without announcing
a terminal UI failure while recovery is pending. Informational progress does
not replace the last error or warning in diagnostic details. A successful
connection clears its failure notice, while retaining the cause for review.

For example, the 2026-09-27 13:49 UTC attempt failed at service discovery with
GATT `Unreachable (1)`, after Windows reported the device as unpaired. This
identifies the failed stage; it does not prove a stale pairing or a wrong device.
Wake the controller, check its battery and Bluetooth state, then rescan before
retrying. Do not remove a Windows pairing automatically based on this status.

## Bluetooth DEBUG observations

Close any running controller app, then use `./scripts/start-diagnostics.ps1` to
launch the packaged app. An explicit `-Executable` accepts another build path.
The script passes `info,gear_vr_controller_rust::infrastructure::bluetooth=debug`
to the child and restores the caller's `RUST_LOG`; saved preferences stay intact.
Reproduce the connection failure and close the app to drain the logger. File
logging must be enabled in settings. The default folder is
`%LOCALAPPDATA%/GearVRController/logs`; custom folders remain supported.

| Event | Diagnostic evidence |
| --- | --- |
| `ble.adapter.snapshot` | Native architecture, Classic/LE and central/peripheral support |
| `ble.radio.snapshot` | Radio state name and native code |
| `ble.windows_service.snapshot` | Local `bthserv` and `BthLEEnum` state, exit codes, pending-state checkpoint and wait hint |
| `ble.connection.scan_context`, `ble.connection.target` | Visible matches, address type, availability, known/service match, smoothed RSSI, presence of a name |
| `ble.device.snapshot` | Connection status, address type, enabled/paired/can-pair, pairing protection and current access |
| `ble.session.snapshot`, `ble.session.unavailable` | Session status, maintain-connection flag, maximum PDU size, or unavailable-session cause |
| `ble.operation.started`, `ble.operation.finished` | Operation, elapsed milliseconds, `api_success`, hexadecimal HRESULT on API failure |
| `ble.service.result`, `ble.characteristics.result` | Native communication status and optional ATT protocol error |
| `ble.characteristic.snapshot` | Protocol UUID, attribute handle, property flags and protection level |
| `ble.notifications.attempt`, `ble.init.command.result` | Retry number or initialization step and returned status |
| `ble.diagnostic.property.failed`, `ble.diagnostic.timeout` | Unavailable observation with HRESULT, or observation deadline |

Properties use native JSON booleans/numbers. An absent field means unavailable,
not false or zero; nullable ATT protocol errors are normal when none exists.
`api_success` means the WinRT call returned a result; inspect the separate GATT
status for communication success. A successful API call may report an unreachable
device. `connection.finished.success` describes the overall connection outcome.
Native codes follow Windows enums: connection 0=disconnected/1=connected;
address type 0=public/1=random/2=unspecified; device access
0=unspecified/1=allowed/2=denied by user/3=denied by system.
Scan visibility is the current UI-filtered snapshot, not the entire Windows
device cache. A service match may come from the 60-second discovery cache.
Neither a scan match nor an open handle proves a working GATT connection.

Connection records carry `attempt_id` and a pseudonymous `device_key`, also
included in the connection span. Keys are stable within one process and use a
new randomized hash seed in each process. Full MAC addresses, device names,
Windows device IDs and raw input packets are not added by these observations.

DEBUG-disabled observation helpers make no additional Windows diagnostic calls.
DEBUG-enabled adapter/radio queries share a two-second future deadline, once
per connection attempt, inside the existing 30-second attempt budget. Optional
property failures do not fail the connection or alter radio/pairing state.
Dropping a timed-out WinRT future does not guarantee cancellation in Windows.
The two synchronous local service queries request read-only SCM rights, release
handles with guards and are outside that future deadline. A stopped service or
missing driver alone does not prove the cause of a device connection failure.
There is no continuous device enumeration or per-advertisement logging.
Scan-context lookup is bounded by the 128-entry catalog; other work is per
attempt, protocol characteristic, initialization write or configured retry.
Existing queue, record-size, rotation and retention limits still apply.

For a read-only hardware probe, set the same filter and run
`cargo run --locked --example scan_diagnostics`. It observes the adapter/radio
and scans for 15 seconds without connecting or injecting desktop input.

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
- [Windows GATT communication status](https://learn.microsoft.com/en-us/uwp/api/windows.devices.bluetooth.genericattributeprofile.gattcommunicationstatus): `Unreachable (1)` means communication is currently unavailable; it does not identify the underlying cause.
