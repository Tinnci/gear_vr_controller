# Gear VR Controller Rust Port - Development Roadmap

This roadmap outlines the path to full parity with the original C# implementation, focusing on stability, input quality, and user experience.

## Phase 1: Connection & Protocol Stability (Completed)
**Goal:** Ensure robust device connection and full protocol compliance.

- [x] **UUID-Based Device Discovery (Auto-Connect)**
    - Implemented `BluetoothLEAdvertisementWatcher` to scan for specific service UUIDs.
    - Added "Known Devices" listing and auto-save of successful connections.
- [x] **Complete Initialization Sequence**
    - Ported full 4-step initialization handshake + connection parameter optimization.
- [x] **Auto-Reconnection Mechanism**
    - Background task monitors connection status and attempts reconnection.
- [x] **Configurable BLE Settings**
    - Exposed UUIDs (Service, Data, Command) in UI.

## Phase 2: Input Experience Polish (Completed)
**Goal:** Match the smooth, responsive feel of the original driver.

- [x] **Touchpad Smoothing (Moving Average)**
    - Configurable sample count ring buffer.
- [x] **Dead Zone Implementation**
    - Configurable center/edge dead zones.
- [x] **Non-Linear Sensitivity Curve**
    - Implemented Mouse Acceleration (Power Curve).
- [x] **Input Configuration UI**
    - Dedicated settings group with tooltips and progressive disclosure.
- [x] **Input Debouncing**
    - Added software debouncing for trigger, touchpad, back, and volume buttons.
- [ ] **Invert Y-Axis & Natural Scrolling**
    - *Plan:* Add options to invert touchpad Y-axis and scroll direction to match user preference.

## Phase 3: Advanced Interaction & Gestures (In Progress)
**Goal:** Restore advanced control capabilities and customizability.

- [x] **Gesture Recognition Engine**
    - Implemented `GestureRecognizer` for Swipes (Up, Down, Left, Right).
    - Integrated logic into main loop with configurable thresholds.
- [ ] **Key Mapping System (Next Priority)**
    - *Goal:* Allow remapping Buttons and Gestures to specific Keyboard/Mouse actions.
    - *Plan:* Create a `InputMapper` struct and a UI to bind events (e.g., "Trigger" -> "Mouse Left", "Swipe Up" -> "Scroll Up").
- [ ] **Media Control Support**
    - *Goal:* Support Volume Up/Down, Play/Pause, Next/Prev Track actions.
- [ ] **Macro Support**
    - *Goal:* Record and playback sequence of actions.

## Phase 4: UI & System Integration (Partial)
- [x] **Settings Persistence**
    - JSON-based saving/loading of all configurations.
- [x] **Advanced Logging System**
    - Configurable log levels, file rotation, and format (Thread IDs, Source Lines).
- [ ] **System Tray Support**
    - *Plan:* Allow the app to run in the background (hidden window) with a tray icon for status/exit.
- [ ] **Battery Level Monitoring**
    - *Plan:* Parse battery level from BLE characteristics (Standard Battery Service or proprietary).
- [ ] **Dark/Light Mode Theme**
    - *Plan:* Polish the UI aesthetics further.

## Phase 5: Visuals & Extras (Planned)
- [ ] **3D Controller Visualization**
    - *Plan:* Render a real-time 3D model of the controller using IMU (Gyro/Accel) data to verify sensor fusion.
- [ ] **Haptic Feedback**
    - *Plan:* Investigate and implement vibration commands.
- [ ] **Firmware Information**
    - display Device Information (Model, Firmware Version, Serial).

## Architecture and Quality Track (In Progress)
**Goal:** Keep the Rust port maintainable without adding unnecessary build-time dependencies.

- [x] **Dependency Reduction**
    - Replaced helper crates for IPC, logging, UAC launch, and Bluetooth service recovery with local Windows API integrations.
- [x] **Quality Pipeline**
    - Added project Clippy thresholds, dependency policy, and a local PowerShell quality runner.
    - Current optional reports cover coverage, dependency policy, unused dependencies, line counts, and duplication.
- [x] **BLE Connection Decomposition**
    - Split connection orchestration from pairing, GATT discovery, initialization, notification retry logic, and shared types.
- [ ] **Application State Decomposition**
    - *Plan:* Move event handling and input mapping out of `GearVRApp` after the key mapping model is introduced.
- [ ] **Workspace Crate Split**
    - *Plan:* Split into workspace crates only after domain APIs stabilize. Candidate boundaries are `domain`, `application`, `windows-adapters`, and `desktop-ui`.
- [ ] **Coverage Gate**
    - *Plan:* Add meaningful domain and protocol tests first, then enable a line coverage threshold.
