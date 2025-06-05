using GearVRController.Enums;
using GearVRController.Models;
using GearVRController.Services.Interfaces;
using System;
// using System.Diagnostics; // Added for Debug.WriteLine, will be replaced by logger

namespace GearVRController.Services
{
    public class InputHandlerService : IInputHandlerService
    {
        private readonly IInputSimulator _inputSimulator;
        private readonly IInputStateMonitorService _inputStateMonitorService;
        private readonly ISettingsService _settingsService;
        private readonly ILogger _logger;

        // Debounce and state variables from MainViewModel
        private int _triggerDebounceCounter = 0;
        private bool _isTriggerButtonPressed = false;
        private bool _isTriggerButtonReleased = false;

        private int _volumeUpDebounceCounter = 0;
        private bool _isVolumeUpHeld = false;

        private int _volumeDownDebounceCounter = 0;
        private bool _isVolumeDownHeld = false;

        private int _backDebounceCounter = 0;
        private bool _isBackButtonPressed = false;

        private int _touchpadPressDebounceCounter = 0;
        private bool _isTouchpadPressActive = false;

        private bool _isTouchpadButtonPressed = false;
        private int _touchpadButtonPressCounter = 0;
        private int _touchpadButtonReleaseCounter = 0;
        private int _triggerButtonPressCounter = 0;
        private int _triggerButtonReleaseCounter = 0;

        public InputHandlerService(IInputSimulator inputSimulator, IInputStateMonitorService inputStateMonitorService, ISettingsService settingsService, ILogger logger)
        {
            _inputSimulator = inputSimulator;
            _inputStateMonitorService = inputStateMonitorService;
            _settingsService = settingsService;
            _logger = logger;
        }

        public void ProcessInput(ControllerData data)
        {
            // This method will contain the logic moved from MainViewModel's HandleButtonInput
            // For now, it's a placeholder.
            _logger.LogInfo("InputHandlerService: Processing input...");

            // Handle Trigger Button
            if (data.TriggerButton)
            {
                _triggerButtonPressCounter++;
                _triggerButtonReleaseCounter = 0;
                if (_triggerButtonPressCounter >= _settingsService.ButtonDebounceThreshold && !_isTriggerButtonPressed)
                {
                    _isTriggerButtonPressed = true;
                    _isTriggerButtonReleased = false;
                    _inputSimulator.SimulateMouseButtonEx(true, (int)MouseButtons.Left);
                    _logger.LogInfo("Left mouse button pressed");
                }
            }
            else
            {
                _triggerButtonReleaseCounter++;
                _triggerButtonPressCounter = 0;
                if (_triggerButtonReleaseCounter >= _settingsService.ButtonDebounceThreshold && _isTriggerButtonPressed)
                {
                    _isTriggerButtonPressed = false;
                    _isTriggerButtonReleased = true;
                    _inputSimulator.SimulateMouseButtonEx(false, (int)MouseButtons.Left);
                    _logger.LogInfo("Left mouse button released");
                }
            }

            // Handle Volume Up Button
            if (data.VolumeUpButton)
            {
                _volumeUpDebounceCounter++;
                if (_volumeUpDebounceCounter >= _settingsService.ButtonDebounceThreshold && !_isVolumeUpHeld)
                {
                    _isVolumeUpHeld = true;
                    _inputSimulator.SimulateKeyDown((int)VirtualKeyCode.VOLUME_UP);
                    _logger.LogInfo("Volume Up pressed");
                }
            }
            else
            {
                if (_isVolumeUpHeld)
                {
                    _inputSimulator.SimulateKeyUp((int)VirtualKeyCode.VOLUME_UP);
                    _isVolumeUpHeld = false;
                    _logger.LogInfo("Volume Up released");
                }
                _volumeUpDebounceCounter = 0;
            }

            // Handle Volume Down Button
            if (data.VolumeDownButton)
            {
                _volumeDownDebounceCounter++;
                if (_volumeDownDebounceCounter >= _settingsService.ButtonDebounceThreshold && !_isVolumeDownHeld)
                {
                    _isVolumeDownHeld = true;
                    _inputSimulator.SimulateKeyDown((int)VirtualKeyCode.VOLUME_DOWN);
                    _logger.LogInfo("Volume Down pressed");
                }
            }
            else
            {
                if (_isVolumeDownHeld)
                {
                    _inputSimulator.SimulateKeyUp((int)VirtualKeyCode.VOLUME_DOWN);
                    _isVolumeDownHeld = false;
                    _logger.LogInfo("Volume Down released");
                }
                _volumeDownDebounceCounter = 0;
            }

            // Handle Back Button
            if (data.BackButton)
            {
                _backDebounceCounter++;
                if (_backDebounceCounter >= _settingsService.ButtonDebounceThreshold && !_isBackButtonPressed)
                {
                    _isBackButtonPressed = true;
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_BACK);
                    _logger.LogInfo("Browser Back pressed");
                }
            }
            else
            {
                _backDebounceCounter = 0;
                _isBackButtonPressed = false;
            }

            // Handle Touchpad Press (mouse right click)
            if (data.TouchpadButton)
            {
                _touchpadButtonPressCounter++;
                _touchpadButtonReleaseCounter = 0;

                if (_touchpadButtonPressCounter >= _settingsService.ButtonDebounceThreshold && !_isTouchpadButtonPressed)
                {
                    _isTouchpadButtonPressed = true;
                    _inputSimulator.SimulateMouseButtonEx(true, (int)MouseButtons.Right);
                    _logger.LogInfo("Right mouse button pressed");
                }
            }
            else
            {
                _touchpadButtonReleaseCounter++;
                _touchpadButtonPressCounter = 0;

                if (_touchpadButtonReleaseCounter >= _settingsService.ButtonDebounceThreshold && _isTouchpadButtonPressed)
                {
                    _isTouchpadButtonPressed = false;
                    _inputSimulator.SimulateMouseButtonEx(false, (int)MouseButtons.Right);
                    _logger.LogInfo("Right mouse button released");
                }
            }

            // Inform InputStateMonitorService about activity only if control is enabled
            if (_settingsService.IsControlEnabled)
            {
                _inputStateMonitorService.NotifyInputActivity();
            }
        }
    }
}