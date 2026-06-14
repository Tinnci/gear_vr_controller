# Gear VR Controller for Windows

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)
![Status](https://img.shields.io/badge/status-Active-brightgreen.svg)

A Windows desktop application for using the Samsung Gear VR Controller
(SM-R323, SM-R324, and SM-R325) as a mouse, touchpad, and presentation
remote. The application is written in Rust and uses Bluetooth LE for
controller communication, egui for the user interface, and windows-rs for
Windows input and Bluetooth integration.

## Features

- Bluetooth LE discovery and connection for Gear VR Controller devices.
- Air Mouse mode using IMU data for cursor movement.
- Touchpad mode for laptop-style cursor control.
- Presenter mode for slide navigation and media control.
- Radial menu for switching control modes from the controller.
- Touchpad gesture support for scrolling and navigation.
- Adjustable sensitivity, dead zones, acceleration, and protocol settings.
- Diagnostic views for Bluetooth state, controller telemetry, and IMU data.
- Optional elevated helper for Bluetooth service recovery tasks.

## Requirements

- Windows 10 or Windows 11.
- A Samsung Gear VR Controller model SM-R323, SM-R324, or SM-R325.
- Bluetooth LE support on the Windows machine.
- Rust stable toolchain when building from source.
- Windows SDK and MSVC build tools when building from source.

## Installation

1. Download the latest release from the
   [Releases](https://github.com/Tinnci/gear_vr_controller/releases) page.
2. Run `gear_vr_controller_rust.exe`.
3. Put the controller into pairing mode by holding the Home button.
4. Use the app to scan for the controller and connect.

## Control Modes

Hold the Back button to open the radial menu and switch modes.

| Mode | Trigger | Touchpad | Back | Home | Volume |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Air Mouse | Left click | Scroll wheel | Right click | Windows key | System volume |
| Touchpad | Left click | Move cursor | Right click | Show desktop | Scroll |
| Presenter | Next slide | Play or pause | Previous slide | Unassigned | System volume |

In Air Mouse mode, hold the controller like a pointer for the most predictable
cursor movement.

## Build From Source

Install Rust from [rustup.rs](https://rustup.rs/) and make sure the stable MSVC
toolchain is available.

```powershell
git clone https://github.com/Tinnci/gear_vr_controller.git
cd gear_vr_controller
cargo build --release --locked
```

Run the development build:

```powershell
cargo run
```

Run the release build:

```powershell
cargo run --release --locked
```

## Development Checks

The repository includes `rust-toolchain.toml` to install the expected Rust
channel and components. Before submitting changes, run the local quality gate:

```powershell
.\scripts\quality.ps1
```

For a fuller local report that also tries coverage, dependency policy, unused
dependency, line-count, and duplication checks when those optional tools are
installed:

```powershell
.\scripts\quality.ps1 -Full
```

The core checks are:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --release --locked
```

Clippy is configured to warn on long functions, complex types, excessive
argument lists, panic-style placeholders, and unchecked `unwrap` or `expect`
usage. Optional tools such as `cargo-llvm-cov`, `cargo-deny`, `cargo-machete`,
`cargo-modules`, `tokei`, and `jscpd` are treated as local reports unless
explicitly installed.

The GitHub Actions CI workflow runs the format, check, clippy, and test steps
on Windows.

## Troubleshooting

If the controller is detected but cannot connect, remove it from Windows
Bluetooth settings and pair it again. If GATT service discovery fails, restart
the Bluetooth service from the app diagnostics or from Windows Services.

For persistent connection issues:

- Confirm the controller is not connected to another device.
- Replace or recharge the controller battery.
- Remove old Gear VR Controller entries from Windows Bluetooth settings.
- Reboot the computer after removing stale Bluetooth devices.
- Run the app again and scan for the controller.

## Project Layout

- `src/domain`: controller models, settings, gestures, touchpad, and IMU logic.
- `src/infrastructure`: Bluetooth, input simulation, logging, and OS services.
- `src/presentation`: egui application state, tabs, components, and theming.
- `src/admin_client.rs` and `src/admin_worker.rs`: elevated helper process.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for
details.

## Acknowledgements

- Based on reverse engineering of the Samsung Gear VR Controller BLE protocol.
- Built with [egui](https://github.com/emilk/egui).
- Uses [windows-rs](https://github.com/microsoft/windows-rs) for Windows APIs.
