# Gear VR Controller for Windows

A Rust desktop application that connects a Samsung Gear VR Controller over
Bluetooth LE and maps its motion, touchpad and buttons to Windows input.
The interface uses native WinUI 3 through Windows Reactor.

## Requirements and installation

- Windows 10 version 1809 or later, or Windows 11, on x64.
- A Bluetooth LE adapter and a compatible Gear VR Controller.
- Download the Windows ZIP from [Releases](https://github.com/Tinnci/gear_vr_controller/releases).
  Verify its SHA-256 against the accompanying `.sha256` file, extract the complete
  folder, and run `gear_vr_controller_rust.exe`.
- Keep all bundled DLLs, resource files and language directories beside the EXE.
  The EXE alone is not a portable distribution.

Put the controller into pairing mode, scan in the dashboard, select its address,
and connect. Windows may prompt for pairing. Input is sent at the normal user's
integrity level; Windows can reject injection into elevated applications.

## Controls

| Mode | Trigger / touchpad press | Touchpad | Short Back | Home | Volume |
| --- | --- | --- | --- | --- | --- |
| Air Mouse | Hold left mouse button | Vertical scroll | Right click on release | Windows key | System volume |
| Touchpad | Hold left mouse button | Move cursor | Right click on release | Show desktop | Scroll |
| Presenter | Trigger: next slide; touchpad press: play/pause | Swipe left/up: previous; right/down: next | Previous slide | Unassigned | System volume |

Hold Back for at least 600 ms, then release to cycle Air Mouse → Touchpad →
Presenter. To choose directly, keep touching the upper region for Air Mouse,
the left region for Touchpad, or the right region for Presenter while releasing
Back. The dashboard reflects the selected mode. Cursor motion pauses while Back
is held. No graphical radial overlay is implemented.

Calibration is explicit: keep the controller still and start gyroscope
calibration to collect 50 samples. For touch calibration, start capture, move
around the full edge of the pad, then save; at least 20 samples and sufficient
travel on both axes are required. Touch calibration persists; gyroscope offsets
apply to the current process session.

Automatic profiles recognize PowerPoint, WPS (`wps.exe`), Acrobat, VLC,
PotPlayer and Bilibili process names. When enabled, Presenter mode prevents
display/system sleep while connected. With the tray option enabled, minimizing
hides the window; click its tray icon to restore it or use its menu to exit.
Closing the window exits normally.

The diagnostics view can restart the Bluetooth service using a one-shot UAC
helper. This interrupts Bluetooth service availability. Cancelling UAC is shown
as an error; normal controller use does not require administrator privileges.
The helper does not accept arbitrary device-removal commands.

## Settings and logs

Settings are stored at `%APPDATA%\GearVRController\settings.json` (falling back
to `%LOCALAPPDATA%`). Saves use an atomic replacement and preserve the previous
file as `settings.json.bak`. Missing fields in older files receive defaults.
Invalid, unreadable or future-version files remain untouched; the app displays
an error and uses temporary in-memory defaults until the file is repaired or
restored from its backup.

Logs default to `%LOCALAPPDATA%\GearVRController\logs`, with daily rotation and
14-day retention. Level, location and retention can be configured in JSON;
logging changes take effect after restart. `RUST_LOG` accepts a single level
(`trace`, `debug`, `info`, `warn`, `error`), not module filter expressions.
Detailed raw-packet trace logging is available in Debug builds.
Set `debug_raw_data_logging` to `true` as well as the `trace` level to enable it.

Input processing runs separately from UI updates. Telemetry is limited to about
30 updates per second; the UI drains batches approximately every 100 ms. A lost
input stream or queue overflow disconnects and releases held mouse buttons.
After a disconnect or mode change, release all controller buttons before using
them again; a carried-over hold cannot trigger an unintended action in the new mode.

## Building and checking

Install Rust through rustup, MSVC build tools and the Windows SDK. The repository
pins Rust 1.95.0 with rustfmt and Clippy. `cargo-deny` is required for the shared
quality gate:

```powershell
cargo install cargo-deny --version 0.19.8 --locked
./scripts/quality.ps1
./scripts/package.ps1
```

The package script verifies the pinned NuGet runtime payloads, builds for
`x86_64-pc-windows-msvc`, stages the complete distribution, smoke-tests startup
and graceful shutdown, then creates a ZIP and SHA-256 file in `dist/`. Runtime
downloads require network access on first use; Windows `curl.exe` and `tar.exe`
must be available. Updating Windows Reactor requires updating the runtime lock
and its allow-list together.

For development:

```powershell
./scripts/prepare-runtime.ps1
cargo run --locked
```

`quality.ps1 -Full` also packages the release and optionally runs coverage,
unused-dependency and source-structure reports. Formatting, check, Clippy,
tests and dependency policy are mandatory. CI runs the same gate and package
smoke test. Release tags must exactly match `v` plus the Cargo package version;
the release job publishes only the already-checked ZIP and checksum. PDB symbols
are retained as a separate CI artifact for diagnosis.

## Verification limits

Unit tests exercise packet decoding, input edges and releases, settings,
calibration and IPC boundaries. The smoke test checks native initialization and
shutdown without a controller. Real Bluetooth hardware, pointer tuning, pairing
dialogs, UAC recovery and clean-machine compatibility still need manual checks;
the smoke test alone does not validate those behaviors. Releases are currently
unsigned. See [ROADMAP.md](ROADMAP.md) for remaining product work.

## Source layout and license

- `src/domain`: settings, packet-independent input mapping, touchpad, IMU and gestures.
- `src/application`: cancellable worker, bounded events and foreground profiles.
- `src/infrastructure`: Bluetooth, Windows input, power, foreground inspection and logs.
- `src/presentation`: WinUI views and UI-thread tray integration.
- `src/admin_*`: restricted, authenticated, one-shot elevated recovery.
- `scripts`: shared checks, runtime verification and packaging.

Project source is MIT-licensed; see [LICENSE](LICENSE). Bundled Microsoft runtime
components have their own terms; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
