# Gear VR Controller for Windows (Rust Edition)

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)
![Status](https://img.shields.io/badge/status-Active-brightgreen.svg)

Empower your Samsung Gear VR Controller (SM-R323 / SM-R324 / SM-R325) on Windows with this modern, high-performance driver written in Rust. Experience low-latency input, customizable gestures, and versatile control modes including Air Mouse functionality.

## ✨ Key Features

- **🚀 High Performance**: Built with Rust for minimal latency and resource usage.
- **🔌 Seamless Connectivity**: Automatic Bluetooth LE discovery and reconnection.
- **🖱️ Versatile Control Modes**:
  - **✈️ Air Mouse**: Wave your controller to move the cursor (using Gyroscope/IMU).
  - **💻 Touchpad**: Use the controller trackpad like a laptop trackpad.
  - **📽️ Presenter**: Optimized for PowerPoint presentations and media control.
- **🎨 Radial Menu**: Quick-access overlay menu to switch modes on the fly (Long press `Back` button).
- **👆 Gestures**: Configurable touchpad gestures for scrolling and navigation.
- **⚙️ Customization**: Fine-tune sensitivity, dead zones, and acceleration.
- **🛡️ Admin Tools**: Built-in tools to manage Bluetooth ghost devices and driver issues.

## 🛠️ Installation

1.  **Download**: Get the latest release from the [Releases](https://github.com/Tinnci/gear_vr_controller/releases) page.
2.  **Run**: Launch `gear_vr_controller_rust.exe`.
3.  **Connect**: Press and hold the **Home** button on your controller to enter pairing mode. The app will automatically detect and connect.

## 🎮 Controls & Modes

Switch modes by **holding the Back button** for 0.5s to open the Radial Menu.

| Mode | Trigger | Touchpad | Back | Home | Vol +/- |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Air Mouse** (Default) | Left Click | Scroll (Wheel) | Right Click | Win Key | Volume |
| **Touchpad** | Left Click | Move Cursor | Right Click | Win + D | Scroll |
| **Presenter** | Next Slide | Play/Pause | Prev Slide | - | Volume |

> **Note**: In Air Mouse mode, hold the controller naturally like a pointer.

## 🔧 Building from Source

Requirements:
- [Rust Toolchain](https://rustup.rs/) (Stable)
- Windows 10/11 SDK

```powershell
# Clone the repository
git clone https://github.com/Tinnci/gear_vr_controller.git
cd gear_vr_controller

# Build
cargo build --release

# Run
cargo run --release
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgements

- Based on reverse engineering of the Gear VR Controller BLE protocol.
- Built with [egui](https://github.com/emilk/egui) for the UI.
- Uses [windows-rs](https://github.com/microsoft/windows-rs) for OS integration.
