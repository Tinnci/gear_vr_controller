using System;
using Microsoft.UI.Dispatching;
using GearVRController.Services.Interfaces;
using System.Diagnostics;
using WindowsInput.Native; // Assuming this is needed for VirtualKeys
using Microsoft.UI.Xaml; // Add this for DispatcherTimer
using WindowsInput; // Add this for WindowsInputSimulator

namespace GearVRController.Services
{
    public class InputStateMonitorService : IInputStateMonitorService
    {
        private readonly IInputSimulator _inputSimulator;
        private readonly DispatcherQueue _dispatcherQueue; // Used for DispatcherTimer

        private bool _leftButtonPressed = false;
        private bool _rightButtonPressed = false;
        private DateTime _lastInputTime = DateTime.MinValue;
        private const int INPUT_TIMEOUT_MS = 5000; // 5秒超时
        private DispatcherTimer? _stateCheckTimer;

        public InputStateMonitorService(IInputSimulator inputSimulator, DispatcherQueue dispatcherQueue)
        {
            _inputSimulator = inputSimulator;
            _dispatcherQueue = dispatcherQueue;
        }

        public void Initialize()
        {
            InitializeStateCheck();
        }

        private void InitializeStateCheck()
        {
            _stateCheckTimer = new DispatcherTimer();
            _stateCheckTimer.Interval = TimeSpan.FromSeconds(1);
            _stateCheckTimer.Tick += (s, e) =>
            {
                MonitorInputState();
            };
            _stateCheckTimer.Start();
            Debug.WriteLine("[InputStateMonitorService] State check timer initialized and started.");
        }

        private void MonitorInputState()
        {
            try
            {
                var now = DateTime.Now;
                if (_leftButtonPressed || _rightButtonPressed)
                {
                    if ((now - _lastInputTime).TotalMilliseconds > INPUT_TIMEOUT_MS)
                    {
                        System.Diagnostics.Debug.WriteLine("检测到按键可能卡住，强制释放");
                        ForceReleaseAllButtons();
                        _leftButtonPressed = false;
                        _rightButtonPressed = false;
                    }
                }
                // Update _lastInputTime only if buttons were pressed or are currently pressed to prevent false timeouts.
                // If no buttons are pressed, _lastInputTime remains at the last actual input.
                if (_leftButtonPressed || _rightButtonPressed)
                {
                    _lastInputTime = now;
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"监控输入状态异常: {ex}");
            }
        }

        public void ForceReleaseAllButtons()
        {
            try
            {
                // Check if _inputSimulator is of type WindowsInputSimulator before casting
                if (_inputSimulator is WindowsInputSimulator inputSimulator)
                {
                    inputSimulator.SimulateMouseButtonEx(false, WindowsInputSimulator.MouseButtons.Left);
                    inputSimulator.SimulateMouseButtonEx(false, WindowsInputSimulator.MouseButtons.Right);
                    inputSimulator.SimulateMouseButtonEx(false, WindowsInputSimulator.MouseButtons.Middle);
                    System.Diagnostics.Debug.WriteLine("已强制释放所有按键");
                }
                else
                {
                    System.Diagnostics.Debug.WriteLine("InputSimulator is not a WindowsInputSimulator instance, cannot force release specific mouse buttons.");
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"强制释放按键异常: {ex}");
            }
        }

        // Method to stop the state check timer, useful during disconnect
        public void StopMonitor()
        {
            _stateCheckTimer?.Stop();
            Debug.WriteLine("[InputStateMonitorService] State check timer stopped.");
        }

        public void UpdateInputState(bool triggerButton, bool touchpadButton, bool isControlEnabled, bool isCalibrating)
        {
            if (!isControlEnabled || isCalibrating) return; // Only process if control is enabled and not calibrating

            // Cast _inputSimulator to WindowsInputSimulator
            if (_inputSimulator is not WindowsInputSimulator inputSimulator)
            {
                Debug.WriteLine("InputSimulator is not a WindowsInputSimulator instance. Cannot simulate mouse events.");
                return;
            }

            // Update right button state
            if (_rightButtonPressed != triggerButton)
            {
                _rightButtonPressed = triggerButton;
                inputSimulator.SimulateMouseButtonEx(triggerButton, WindowsInputSimulator.MouseButtons.Right);
                _lastInputTime = DateTime.Now;
                Debug.WriteLine($"[InputStateMonitorService] Right button state changed to: {triggerButton}");
            }

            // Update left button state
            if (_leftButtonPressed != touchpadButton)
            {
                _leftButtonPressed = touchpadButton;
                inputSimulator.SimulateMouseButtonEx(touchpadButton, WindowsInputSimulator.MouseButtons.Left);
                _lastInputTime = DateTime.Now;
                Debug.WriteLine($"[InputStateMonitorService] Left button state changed to: {touchpadButton}");
            }
        }
    }
}