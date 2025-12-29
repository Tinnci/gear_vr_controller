# Gear VR Controller - Rust Implementation

A modern Rust implementation of the Gear VR Controller Windows application using egui for the GUI and direct Windows API calls for input simulation.

## Features

- **Direct Bluetooth LE Connection**: Connect to Gear VR Controller via Bluetooth Low Energy
- **Modern GUI**: Built with egui for a responsive, immediate-mode user interface
- **Touchpad Control**: Map touchpad movements to mouse movements
- **Button Mapping**: Configurable button actions
- **Calibration**: Touchpad calibration wizard
- **Settings Persistence**: JSON-based settings storage

## Architecture

### Core Modules

1. **bluetooth.rs**: Handles Bluetooth LE communication using Windows Runtime APIs
2. **input_simulator.rs**: Direct Win32 API calls for mouse/keyboard input simulation
3. **controller.rs**: Touchpad data processing and coordinate normalization
4. **models.rs**: Data structures for controller state and configuration
5. **settings.rs**: Settings management with JSON serialization
6. **ui.rs**: egui-based user interface

## Technology Stack

- **Rust 2021 Edition**: Modern, safe systems programming
- **egui/eframe**: Immediate-mode GUI framework
- **windows-rs**: Official Microsoft Windows API bindings for Rust
- **tokio**: Async runtime for Bluetooth operations
- **serde**: Serialization framework for settings

## How It Works

### Windows API Integration

Unlike the C# version that uses WinUI 3, this Rust implementation calls Windows APIs directly:

```rust
// Mouse movement using Win32 API
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, MOUSEINPUT};

pub fn move_mouse(&self, dx: i32, dy: i32) -> anyhow::Result<()> {
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    dwFlags: MOUSEEVENTF_MOVE,
                    ...
                },
            },
        };
        SendInput(&[input], ...);
    }
    Ok(())
}
```

### Bluetooth Communication

Uses Windows Runtime APIs (WinRT) through the `windows` crate:

```rust
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristic;

// Connect to device
let device = BluetoothLEDevice::FromBluetoothAddressAsync(address)?.await?;

// Subscribe to notifications
data_char.ValueChanged(&handler)?;
```

## Building

### Prerequisites

- Rust 1.70+ (2021 edition)
- Windows 10/11
- Bluetooth 4.0+ adapter

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run
cargo run --release
```

## Usage

1. **Connect Controller**:
   - Enter the Bluetooth address (e.g., `A4:D5:78:1E:2F:3C`)
   - Click "Connect"

2. **Calibrate Touchpad**:
   - Go to "Calibration" tab
   - Click "Start Calibration"
   - Move finger around entire touchpad surface
   - Click "Finish Calibration"

3. **Adjust Settings**:
   - Go to "Settings" tab
   - Adjust mouse sensitivity
   - Enable/disable features
   - Click "Save Settings"

## Advantages Over C# Version

1. **No Runtime Dependencies**: Single executable, no .NET or WinUI runtime needed
2. **Smaller Binary**: ~5-10MB vs ~50MB+ for .NET apps
3. **Lower Memory Usage**: Rust's zero-cost abstractions
4. **Direct API Access**: No P/Invoke overhead
5. **Type Safety**: Rust's ownership system prevents common bugs
6. **Cross-Platform Potential**: Core logic can be adapted for other platforms

## Configuration

Settings are stored in JSON format at:
```
%APPDATA%\GearVRController\settings.json
```

Example:
```json
{
  "mouse_sensitivity": 2.0,
  "touchpad_calibration": {
    "min_x": 0,
    "max_x": 315,
    "min_y": 0,
    "max_y": 315,
    "center_x": 157,
    "center_y": 157
  },
  "known_bluetooth_addresses": [],
  "enable_touchpad": true,
  "enable_buttons": true
}
```

## Protocol Details

The Gear VR Controller uses a 60-byte BLE data packet:

- Bytes 0-3: Timestamp (i32, little-endian)
- Bytes 4-15: Accelerometer data (3x f32)
- Bytes 16-27: Gyroscope data (3x f32)
- Bytes 54-57: Touchpad coordinates (2x u16)
- Byte 58: Button states (bit flags)
- Byte 59: Touch state

## Contributing

This is a migration from the C# WinUI 3 version. Core functionality has been preserved while leveraging Rust's advantages.

## License

Same as the original C# project.
