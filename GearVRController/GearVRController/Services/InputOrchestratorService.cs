using GearVRController.Events;
using GearVRController.Services.Interfaces;
using GearVRController.Models;
using GearVRController.Enums;

namespace GearVRController.Services
{
    public class InputOrchestratorService : IInputOrchestratorService
    {
        private readonly ILogger _logger;
        private readonly IEventAggregator _eventAggregator;
        private readonly ISettingsService _settingsService;
        private readonly IInputHandlerService _inputHandlerService;
        private readonly GestureRecognizer _gestureRecognizer;
        private readonly IActionExecutionService _actionExecutionService;

        public InputOrchestratorService(
            ILogger logger,
            IEventAggregator eventAggregator,
            ISettingsService settingsService,
            IInputHandlerService inputHandlerService,
            GestureRecognizer gestureRecognizer,
            IActionExecutionService actionExecutionService)
        {
            _logger = logger;
            _eventAggregator = eventAggregator;
            _settingsService = settingsService;
            _inputHandlerService = inputHandlerService;
            _gestureRecognizer = gestureRecognizer;
            _actionExecutionService = actionExecutionService;

            _eventAggregator.Subscribe<ControllerDataReceivedEvent>(OnControllerDataReceived);
            _gestureRecognizer.GestureDetected += OnGestureDetected;
            _logger.LogInfo("InputOrchestratorService initialized.", nameof(InputOrchestratorService));
        }

        private void OnControllerDataReceived(ControllerDataReceivedEvent e)
        {
            ProcessControllerData(e.Data, false, _settingsService.IsControlEnabled);
        }

        private void OnGestureDetected(object? sender, GestureDirection direction)
        {
            if (!_settingsService.IsControlEnabled) return;

            if (_settingsService.IsGestureMode)
            {
                GestureAction action = GestureAction.None;
                switch (direction)
                {
                    case GestureDirection.Up:
                        action = _settingsService.SwipeUpAction;
                        break;
                    case GestureDirection.Down:
                        action = _settingsService.SwipeDownAction;
                        break;
                    case GestureDirection.Left:
                        action = _settingsService.SwipeLeftAction;
                        break;
                    case GestureDirection.Right:
                        action = _settingsService.SwipeRightAction;
                        break;
                }
                _actionExecutionService.ExecuteAction(action);
                _eventAggregator.Publish(new GestureExecutedEvent(direction, action));
                _logger.LogInfo($"Gesture Mode: Discrete Swipe gesture detected ({direction}), executing action.", nameof(InputOrchestratorService));
            }
            else
            {
                _logger.LogInfo($"Relative Mode: Gesture detected ({direction}), no action executed here as continuous movement is handled by InputHandlerService.", nameof(InputOrchestratorService));
            }
        }

        public void ProcessControllerData(ControllerData data, bool isCalibrating, bool isControlEnabled)
        {
            if (isControlEnabled && !isCalibrating)
            {
                if (_settingsService.IsGestureMode)
                {
                    _gestureRecognizer.ProcessTouchpadPoint(new TouchpadPoint
                    {
                        X = data.TouchpadX,
                        Y = data.TouchpadY,
                        IsTouched = data.TouchpadTouched
                    });
                }
                else // Relative mode (mouse simulation and button input)
                {
                    _inputHandlerService.ProcessInput(data);
                }
            }
        }
    }
}