# Bluetooth diagnostic CLI

`gearvr-debug.exe` is a console program. It reuses the app's BLE scanner,
GATT discovery, controller initialization and packet decoder. It does not start
WinUI or send mouse or keyboard input. Each invocation runs one bounded command
and releases its subscriptions, session and device handles on completion.

Close the control panel before probing the controller. Keep the controller awake
and near the computer. The Windows radio must be on.

## Commands

From an extracted package:

```powershell
./gearvr-debug.exe --help
./gearvr-debug.exe status
./gearvr-debug.exe scan --seconds 15
./gearvr-debug.exe scan --seconds 15 --all
```

The scan returns a final stable list. A name may be empty. Identify the controller
using `matches_service`, its known address and its advertised address type.
`--all` includes unrelated nearby BLE advertisements for discovery diagnosis.
Do not select a device by signal strength alone.
Some paired controllers advertise their name without the Gear VR service UUID.
They may only appear with `--all` in this CLI, which does not load saved device
history. A missing advertised service UUID does not prove GATT incompatibility.

Use an explicit 12-digit hexadecimal address from the scan or a verified Windows
device record. The following address is a placeholder:

```powershell
$address = 'AABBCCDDEEFF'
./gearvr-debug.exe status --address $address --address-type public
./gearvr-debug.exe pair --address $address --address-type public
./gearvr-debug.exe connect --address $address --address-type public --seconds 10 --timeout 40
```

`--address-type` accepts `public`, `random` or `auto` (default). Prefer the type
returned by scanning. `auto` uses Windows' address lookup. A current advertisement
or existing Windows device record is needed for lookup.

`pair` reads Windows pairing state. An already-paired device returns
`already_paired`; this does not prove it is reachable. An unpaired device uses
Windows' basic `PairAsync` operation. Complete any Windows pairing confirmation.
No PIN or confirmation is silently supplied. Successful pairing and successful
GATT connection are separate results. Direct GATT access can also work without
traditional pairing, so `connect` does not require `pair` first.

`connect` uses the built-in Gear VR service and characteristic UUIDs and default
retry settings. It opens the device, reads services without using the GATT cache,
subscribes, writes initialization commands and counts decoded packets. It does
not load or change saved GUI settings or connection history. It fails if the
connection drops or no valid controller packets arrive during capture.

`--seconds` is the scan/capture duration (1–120; default 15). `--timeout` is the
total command deadline (3–180; default 45), including connection setup and capture.
It must exceed `--seconds`. Allow time for Windows connection queues and retries.
Dropping an awaited operation cannot guarantee that Windows immediately cancels
its underlying Bluetooth work. Wait for it to settle before another attempt.

From source, replace the executable with:

```powershell
cargo run --locked --bin gearvr-debug -- scan --seconds 15
```

## Output and logs

Stdout contains UTF-8 JSON lines only. Each record has `schema` (currently 1),
`event`, `pid`, `elapsed_ms` since command start, and `data`. Exit code 0 means
the command completed successfully; other codes mean failure. The last event
is `completed` or `failed`. `scan` succeeding with an empty list is a valid scan
result, not evidence that a controller is reachable.

- `started`: command, deadline, log directory and effective logging filter.
- `adapter_status`: radio name/state, LE and central support.
- `device_status`: name/address/Windows device ID, pairing, access and connection state.
- `scan_result`: device list, RSSI, service match and advertisement availability.
- `pair_result`: Windows pairing status and numeric code.
- `progress`: transport-stage messages during connection setup.
- `connected`: GATT initialization completed; packet reception is checked next.
- `capture_progress` / `capture_result`: valid/invalid packet counts, average
  packet rate and last packet age (`null` until a valid packet arrives).
- `logging_health`: bounded logger's dropped events and writer errors.
- `failed`: contextual detail, typed connection failure kind or HRESULT if present.

Windows codes are preserved: radio 1=On, 2=Off, 3=Disabled; connection
0=Disconnected, 1=Connected; address type 0=Public, 1=Random;
access 1=Allowed, 2=DeniedByUser, 3=DeniedBySystem (0=Unspecified).
An `enabled` flag from `DeviceInformation` describes that enumerated record;
it alone does not establish a physical adapter fault or controller reachability.

The CLI deliberately prints local identifiers so a subsequent command can target
the device. Review JSON output before sharing it. Regular log files retain the
app's address/name privacy policy.

Logs use a separate `%LOCALAPPDATA%/GearVRController/logs/cli` directory and prefix.
The existing bounded writer, rotation and retention policy applies. DEBUG is the
default, but inherited `RUST_LOG` overrides it. For consistent observations:

```powershell
$env:RUST_LOG = 'info,gear_vr_controller_rust::infrastructure::bluetooth=debug'
./gearvr-debug.exe connect --address $address --seconds 10 --timeout 40 |
    Tee-Object -FilePath ./connection.jsonl
$LASTEXITCODE
```

Use `pid` to correlate output with JSON log records. Commands run in separate
processes; the privacy-preserving `device_key` changes between processes.
No persistent pipe listener is needed: a tool can launch a command, read stdout
incrementally and inspect its exit code. The CLI uses bounded scan storage and
aggregate packet counters; it does not retain every packet or log each one.

## Diagnosing a stalled stack

Capture `status`, a fresh scan, and one `connect` attempt before recovery.
Compare adapter/radio/service snapshots, pairing state, advertised identity,
operation timings, native HRESULTs and GATT status. `api_success=true` means a
Windows API returned a result; GATT status may still be `Unreachable`.

No broadcast plus `Unreachable` can mean a sleeping controller, range, interference
or stack trouble. It is not a sufficient reason to delete the pairing record.
If the controller is awake and advertising but repeated connections fail, collect
Windows PnP/events or a bounded Bluetooth ETW trace before restarting anything.
Compare the same sequence after the specific recovery step to isolate its effect.
This tool does not unpair, restart services, disable the adapter or reboot Windows.

Microsoft references: [device pairing](https://learn.microsoft.com/en-us/windows/uwp/devices-sensors/pair-devices),
[GATT client connection behavior](https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/gatt-client),
[Bluetooth cache modes](https://learn.microsoft.com/en-us/uwp/api/windows.devices.bluetooth.bluetoothcachemode).

## Verification

Unit tests cover explicit targets, address types, option conflicts and time limits.
Package smoke checks JSON framing, help and rejection of targetless connections;
it does not access Bluetooth. Real pairing dialogs, controller packet reception
and failure recovery require hardware checks.
