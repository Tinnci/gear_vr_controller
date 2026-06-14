# Development Roadmap

This file tracks maintainer decisions and planned work. User-facing setup,
features, build steps, and troubleshooting stay in `README.md`.

## Current Status

- The Rust port is the active implementation.
- Core BLE discovery, connection, initialization, reconnection, protocol
  parsing, touchpad input, IMU cursor movement, gestures, debouncing, settings,
  logging, diagnostics, and the elevated recovery helper are implemented.
- Direct dependencies are intentionally small. Windows IPC, UAC launch, logging
  rotation, Bluetooth service recovery, and input injection use local code and
  Windows APIs instead of extra helper crates.
- BLE connection code is split by responsibility: pairing, GATT discovery,
  initialization, notification setup, and shared types.

## Next Product Work

1. **Key Mapping System**
   - Add an input action model for buttons and gestures.
   - Move hard-coded button behavior out of `GearVRApp`.
   - Add settings UI for binding controller events to mouse, keyboard, media,
     and mode-switch actions.
2. **Input Preferences**
   - Add invert Y-axis and natural scrolling options.
   - Persist the options through `SettingsService`.
3. **Battery Level**
   - Read the standard Battery Service if present.
   - Fall back to proprietary data only if verified from real packets.
4. **System Tray**
   - Keep the app available in the background with explicit status and exit
     controls.

## Deferred Architecture Work

- **Application State Decomposition**
  - Start after the key mapping model exists.
  - Target split: event ingestion, input mapping, UI state, and Bluetooth command
    orchestration.
- **Workspace Crate Split**
  - Defer until domain APIs stabilize.
  - Candidate crates: `domain`, `application`, `windows-adapters`, and
    `desktop-ui`.
- **Coverage Gate**
  - Defer hard thresholds until meaningful unit tests exist for domain,
    settings, protocol parsing, and input mapping.
  - Keep coverage as a report in `.\scripts\quality.ps1 -Full` for now.

## Quality Baseline

- Default gate: `.\scripts\quality.ps1`
- Full local report: `.\scripts\quality.ps1 -Full`
- Current policy: format, check, Clippy, and tests are hard gates.
- Current reports: release build, coverage, dependency policy, unused
  dependencies, source metrics, BLE module structure, and duplication scan.
- Known tool note: `cargo-modules` may print a tracing static-level warning.
  The structure report is still usable when the command exits successfully.
