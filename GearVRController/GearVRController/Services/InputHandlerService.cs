using GearVRController.Enums;
using GearVRController.Models;
using GearVRController.Services.Interfaces;
using GearVRController.Events;
using System;
using System.Collections.Generic;
using System.Linq;
// using System.Diagnostics; // Added for Debug.WriteLine, will be replaced by logger

namespace GearVRController.Services
{
    public class InputHandlerService : IInputHandlerService, IDisposable
    {
        private readonly IInputSimulator _inputSimulator;
        private readonly IInputStateMonitorService _inputStateMonitorService;
        private readonly ISettingsService _settingsService;
        private readonly ILogger _logger;
        private readonly IEventAggregator _eventAggregator;

        // Debounce and state variables
        private bool _isTriggerButtonPressed = false;
        private DateTime _lastTriggerActionTime = DateTime.MinValue;

        private bool _isVolumeUpHeld = false;
        private DateTime _lastVolumeUpActionTime = DateTime.MinValue;

        private bool _isVolumeDownHeld = false;
        private DateTime _lastVolumeDownActionTime = DateTime.MinValue;

        private bool _isBackButtonPressed = false;
        private DateTime _lastBackActionTime = DateTime.MinValue;

        private bool _isTouchpadButtonPressed = false;
        private DateTime _lastTouchpadButtonActionTime = DateTime.MinValue;

        private readonly TimeSpan _debounceTime = TimeSpan.FromMilliseconds(50);

        // Touchpad movement simulation fields
        private bool _isTouchpadCurrentlyTouched = false;
        private double _lastTouchpadProcessedX = 0;
        private double _lastTouchpadProcessedY = 0;
        private double _smoothedDeltaX = 0;
        private double _smoothedDeltaY = 0;
        private List<double> _smoothingBufferX;
        private List<double> _smoothingBufferY;

        public InputHandlerService(IInputSimulator inputSimulator, IInputStateMonitorService inputStateMonitorService, ISettingsService settingsService, ILogger logger, IEventAggregator eventAggregator)
        {
            _inputSimulator = inputSimulator ?? throw new ArgumentNullException(nameof(inputSimulator));
            _inputStateMonitorService = inputStateMonitorService ?? throw new ArgumentNullException(nameof(inputStateMonitorService));
            _settingsService = settingsService ?? throw new ArgumentNullException(nameof(settingsService));
            _logger = logger ?? throw new ArgumentNullException(nameof(logger));
            _eventAggregator = eventAggregator ?? throw new ArgumentNullException(nameof(eventAggregator));

            // Initialize smoothing buffers
            _smoothingBufferX = new List<double>();
            _smoothingBufferY = new List<double>();

            // Removed direct subscription, MainViewModel will now call ProcessInput
            // _dataSubscription = _eventAggregator.Subscribe<ControllerDataReceivedEvent>(e => ProcessInput(e.Data));
        }

        public void ProcessInput(ControllerData data)
        {
            // This method will contain the logic moved from MainViewModel's HandleButtonInput
            // For now, it's a placeholder.
            _logger.LogInfo("InputHandlerService: Processing input...");

            // Handle Trigger Button
            bool isCurrentlyTriggerPressed = data.TriggerButton;
            if (isCurrentlyTriggerPressed != _isTriggerButtonPressed)
            {
                if ((DateTime.UtcNow - _lastTriggerActionTime) > _debounceTime)
                {
                    _isTriggerButtonPressed = isCurrentlyTriggerPressed;
                    _lastTriggerActionTime = DateTime.UtcNow;

                    if (_isTriggerButtonPressed)
                    {
                        _inputSimulator.SimulateMouseButtonEx(true, (int)MouseButtons.Left);
                        _logger.LogInfo("Left mouse button pressed");
                    }
                    else
                    {
                        _inputSimulator.SimulateMouseButtonEx(false, (int)MouseButtons.Left);
                        _logger.LogInfo("Left mouse button released");
                    }
                }
            }

            // Handle Volume Up Button
            bool isCurrentlyVolumeUpPressed = data.VolumeUpButton;
            if (isCurrentlyVolumeUpPressed != _isVolumeUpHeld)
            {
                if ((DateTime.UtcNow - _lastVolumeUpActionTime) > _debounceTime)
                {
                    _isVolumeUpHeld = isCurrentlyVolumeUpPressed;
                    _lastVolumeUpActionTime = DateTime.UtcNow;

                    if (_isVolumeUpHeld)
                    {
                        _inputSimulator.SimulateKeyDown((int)VirtualKeyCode.VOLUME_UP);
                        _logger.LogInfo("Volume Up pressed");
                    }
                    else
                    {
                        _inputSimulator.SimulateKeyUp((int)VirtualKeyCode.VOLUME_UP);
                        _logger.LogInfo("Volume Up released");
                    }
                }
            }

            // Handle Volume Down Button
            bool isCurrentlyVolumeDownPressed = data.VolumeDownButton;
            if (isCurrentlyVolumeDownPressed != _isVolumeDownHeld)
            {
                if ((DateTime.UtcNow - _lastVolumeDownActionTime) > _debounceTime)
                {
                    _isVolumeDownHeld = isCurrentlyVolumeDownPressed;
                    _lastVolumeDownActionTime = DateTime.UtcNow;

                    if (_isVolumeDownHeld)
                    {
                        _inputSimulator.SimulateKeyDown((int)VirtualKeyCode.VOLUME_DOWN);
                        _logger.LogInfo("Volume Down pressed");
                    }
                    else
                    {
                        _inputSimulator.SimulateKeyUp((int)VirtualKeyCode.VOLUME_DOWN);
                        _logger.LogInfo("Volume Down released");
                    }
                }
            }

            // Handle Back Button
            bool isCurrentlyBackPressed = data.BackButton;
            if (isCurrentlyBackPressed != _isBackButtonPressed)
            {
                if ((DateTime.UtcNow - _lastBackActionTime) > _debounceTime)
                {
                    _isBackButtonPressed = isCurrentlyBackPressed;
                    _lastBackActionTime = DateTime.UtcNow;

                    if (_isBackButtonPressed)
                    {
                        _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_BACK);
                        _logger.LogInfo("Browser Back pressed");
                    }
                    // No 'else' for KeyPress, as it's a single press, not a hold
                }
            }

            // Handle Touchpad Press (mouse right click)
            bool isCurrentlyTouchpadPressed = data.TouchpadButton;
            if (isCurrentlyTouchpadPressed != _isTouchpadButtonPressed)
            {
                if ((DateTime.UtcNow - _lastTouchpadButtonActionTime) > _debounceTime)
                {
                    _isTouchpadButtonPressed = isCurrentlyTouchpadPressed;
                    _lastTouchpadButtonActionTime = DateTime.UtcNow;

                    if (_isTouchpadButtonPressed)
                    {
                        _inputSimulator.SimulateMouseButtonEx(true, (int)MouseButtons.Right);
                        _logger.LogInfo("Right mouse button pressed");
                    }
                    else
                    {
                        _inputSimulator.SimulateMouseButtonEx(false, (int)MouseButtons.Right);
                        _logger.LogInfo("Right mouse button released");
                    }
                }
            }

            // Handle touchpad movement for mouse simulation (relative mode)
            if (data.TouchpadTouched) // If touchpad is touched
            {
                if (!_isTouchpadCurrentlyTouched) // Touch started
                {
                    _lastTouchpadProcessedX = data.ProcessedTouchpadX;
                    _lastTouchpadProcessedY = data.ProcessedTouchpadY;
                    _isTouchpadCurrentlyTouched = true;
                    _logger.LogInfo($"触摸开始 (InputHandlerService): ({data.ProcessedTouchpadX:F2}, {data.ProcessedTouchpadY:F2})");
                }
                else // Touch continued
                {
                    double deltaX = data.ProcessedTouchpadX - _lastTouchpadProcessedX;
                    double deltaY = data.ProcessedTouchpadY - _lastTouchpadProcessedY;

                    // Apply dead zone
                    (deltaX, deltaY) = ApplyDeadZone(deltaX, deltaY);

                    // Apply smoothing
                    if (_settingsService.EnableSmoothing)
                    {
                        _smoothedDeltaX = ApplySmoothing(deltaX, _smoothingBufferX, _settingsService.SmoothingLevel);
                        _smoothedDeltaY = ApplySmoothing(deltaY, _smoothingBufferY, _settingsService.SmoothingLevel);
                    }
                    else
                    {
                        _smoothedDeltaX = deltaX;
                        _smoothedDeltaY = deltaY;
                        _smoothingBufferX.Clear();
                        _smoothingBufferY.Clear();
                    }

                    // Update last processed point
                    _lastTouchpadProcessedX = data.ProcessedTouchpadX;
                    _lastTouchpadProcessedY = data.ProcessedTouchpadY;

                    // Apply non-linear curve
                    double finalDeltaX = ApplyNonLinearCurve(_smoothedDeltaX, _settingsService.NonLinearCurvePower, _settingsService.EnableNonLinearCurve);
                    double finalDeltaY = ApplyNonLinearCurve(_smoothedDeltaY, _settingsService.NonLinearCurvePower, _settingsService.EnableNonLinearCurve);

                    // Calculate mouse movement
                    double mouseDeltaX = finalDeltaX * _settingsService.MouseSensitivity * _settingsService.MouseSensitivityScalingFactor;
                    double mouseDeltaY = finalDeltaY * _settingsService.MouseSensitivity * _settingsService.MouseSensitivityScalingFactor;

                    // Simulate mouse movement
                    if (Math.Abs(mouseDeltaX) > _settingsService.MoveThreshold || Math.Abs(mouseDeltaY) > _settingsService.MoveThreshold)
                    {
                        _inputSimulator.SimulateMouseMovement(mouseDeltaX, mouseDeltaY);
                    }
                }
            }
            else // Touch ended
            {
                if (_isTouchpadCurrentlyTouched) // Just lifted
                {
                    _isTouchpadCurrentlyTouched = false;
                    _logger.LogInfo("触摸结束 (InputHandlerService).", nameof(InputHandlerService));
                }
            }
        }

        /// <summary>
        /// 对鼠标移动增量应用死区。小于死区阈值的移动将被忽略。
        /// </summary>
        private (double, double) ApplyDeadZone(double deltaX, double deltaY)
        {
            double deadZoneThreshold = _settingsService.DeadZone / 100.0; // 将百分比转换为 [-1, 1] 范围的比例
            double magnitude = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);

            if (magnitude <= deadZoneThreshold)
            {
                return (0.0, 0.0);
            }
            return (deltaX, deltaY);
        }

        /// <summary>
        /// 对鼠标移动增量应用平滑处理，以减少抖动并使移动更流畅。
        /// </summary>
        private double ApplySmoothing(double newValue, List<double> buffer, int smoothingLevel)
        {
            buffer.Add(newValue);
            while (buffer.Count > smoothingLevel)
            {
                buffer.RemoveAt(0);
            }
            return buffer.Average();
        }

        /// <summary>
        /// 对鼠标移动增量应用非线性曲线，以调整鼠标加速行为。
        /// </summary>
        private double ApplyNonLinearCurve(double value, double power, bool enable)
        {
            if (!enable || power <= 0) return value;

            // Preserve sign
            int sign = Math.Sign(value);
            double absValue = Math.Abs(value);

            // Apply power curve
            double result = Math.Pow(absValue, power);

            // Scale back to original range if needed, or simply apply sign
            return result * sign;
        }

        /// <summary>
        /// 处理滚轮移动，并根据"自然滚动"设置调整方向。
        /// </summary>
        public int ProcessWheelMovement(int delta)
        {
            // 如果启用了自然滚动，则反转滚动方向
            if (_settingsService.UseNaturalScrolling)
            {
                return -delta;
            }
            return delta;
        }

        public void Dispose()
        {
            // Removed direct subscription
            // _dataSubscription?.Dispose();
            _inputStateMonitorService.ForceReleaseAllButtons();
            _inputStateMonitorService.StopMonitoring();
        }
    }
}