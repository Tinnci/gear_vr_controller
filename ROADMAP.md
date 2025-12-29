# Gear VR Controller Rust Port - Development Roadmap

This roadmap outlines the path to full parity with the original C# implementation, focusing on stability, input quality, and user experience.

## Phase 1: Connection & Protocol Stability (Current Priority)
**Goal:** Ensure robust device connection and full protocol compliance.

- [ ] **UUID-Based Device Discovery (Auto-Connect)**
    - *Current State:* Requires manual Bluetooth Address input.
    - *Plan:* Implement `BluetoothLEAdvertisementWatcher` to scan for devices broadcasting the specific service UUID:
      > `4f63756c-7573-2054-6872-65656d6f7465`
    - User simplifies connection by just clicking "Scan & Connect" instead of typing addresses.
- [ ] **Complete Initialization Sequence**
    - *Current State:* Sends only partial start command.
    - *Plan:* Port the full 4-step initialization handshake + connection parameter optimization command from C# to ensure compatibility with all controller firmware versions.
- [ ] **Auto-Reconnection Mechanism**
    - *Current State:* No auto-reconnect on disconnect.
    - *Plan:* Implement a background monitor that attempts to reconnect when the link is lost, with configurable retry intervals.

## Phase 2: Input Experience Polish
**Goal:** Match the smooth, responsive feel of the original driver.

- [ ] **Touchpad Smoothing (Moving Average)**
    - *Plan:* Implement a ring buffer to smooth out raw sensor noise from the touchpad X/Y coordinates.
- [ ] **Dead Zone Implementation**
    - *Plan:* Add configurable dead zones (center and edges) to prevent cursor drift when the finger is stationary but slightly trembly.
- [ ] **Non-Linear Sensitivity Curve**
    - *Plan:* Implement "Mouse Acceleration" logic (Power Curve) so slow movements are precise, but fast swipes cover large distances.
- [ ] **Input Debouncing**
    - *Plan:* Add software debouncing (approx. 50ms) for physical buttons to prevent double-clicks.

## Phase 3: Advanced Interaction & Gestures
**Goal:** Restore advanced control capabilities.

- [ ] **Gesture Recognition Engine**
    - *Plan:* Port `GestureRecognizer.cs` logic.
    - Support: Swipe Up, Swipe Down, Swipe Left, Swipe Right.
- [ ] **Key Mapping Customization**
    - *Plan:* Allow remapping:
        - Volume Keys (currently hardcoded to Scroll) -> System Volume / Custom.
        - Back Button (currently ESC) -> Browser Back / Custom.
        - Gestures -> Custom Actions.

## Phase 4: UI & System Integration
- [ ] **Settings Persistence**: Ensure all new calibration and sensitivity settings are saved/loaded correctly.
- [ ] **System Tray Icon**: Allow the app to run in the background without an open window.
