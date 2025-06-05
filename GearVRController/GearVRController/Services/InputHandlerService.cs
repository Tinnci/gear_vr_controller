using GearVRController.Enums;
using GearVRController.Models;
using GearVRController.Services.Interfaces;
using GearVRController.Events;
using System;
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
        private IDisposable _dataSubscription;

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

        public InputHandlerService(IInputSimulator inputSimulator, IInputStateMonitorService inputStateMonitorService, ISettingsService settingsService, ILogger logger, IEventAggregator eventAggregator)
        {
            _inputSimulator = inputSimulator;
            _inputStateMonitorService = inputStateMonitorService;
            _settingsService = settingsService;
            _logger = logger;
            _eventAggregator = eventAggregator;

            // Subscribe to the event
            _dataSubscription = _eventAggregator.Subscribe<ControllerDataReceivedEvent>(e => ProcessInput(e.Data));
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
        }

        public void Dispose()
        {
            _dataSubscription?.Dispose();
        }
    }
}