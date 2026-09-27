# Development Roadmap

## Implemented baseline

- Native WinUI 3 interface with four language dictionaries.
- BLE discovery, pairing, GATT initialization and notifications.
- Air mouse, touchpad and presenter input mapping, edge-triggered buttons,
  hold-to-switch mode selection and presenter swipe gestures.
- Explicit gyroscope and touchpad calibration.
- Optional foreground profiles, connected presenter sleep inhibition and
  minimize-to-tray with restore/exit controls.
- Validated versioned settings, atomic saves with backup, startup diagnostics,
  rotating logs with retention, bounded event channels and input fault recovery.
- Owned GATT sessions and callback revocation, cancellable worker shutdown.
- Authenticated one-shot Bluetooth service recovery; arbitrary device removal
  is intentionally not exposed.
- Pinned toolchain, locked dependencies, shared CI/release checks, verified
  NuGet runtime payloads, complete ZIP, checksums and startup/shutdown smoke test.

## Manual acceptance

Before a public release, test the package on a clean Windows 10/11 x64 machine
without the matching Windows App SDK framework installed. Check BLE pairing,
connect/disconnect/reconnect, all input modes, drags released on disconnect,
calibration, tray restore/exit, foreground profiles and UAC acceptance/cancellation.
Verify mouse tuning and controller model compatibility against physical devices.
Automated smoke tests do not claim this hardware acceptance.

## Future product work

- Configurable button/gesture bindings and a visible radial selector.
- Input preference controls in the UI (JSON currently exposes sensitivity,
  smoothing, acceleration, enable flags and BLE configuration).
- Persisted gyroscope profiles, timestamp-based motion integration and tuning
  against captured physical-device data.
- Verified battery reporting and user-controlled automatic reconnect.
- Code signing, installer/update support and ARM64 validation.
- More localized diagnostic error messages and accessibility review.
- Coverage reporting thresholds after the meaningful behavioral suite grows.

Keep changes in the current module structure until interfaces justify a
workspace split. Performance profile changes require measurements; release
symbols are retained separately rather than forcing panic-abort or maximal LTO.
