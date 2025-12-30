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
- [ ] **Input Debouncing**
    - *Plan:* Add software debouncing (approx. 50ms) for physical buttons to prevent double-clicks.
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
