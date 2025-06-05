using System.Threading.Tasks;
using GearVRController.ViewModels;
using GearVRController.Models;

namespace GearVRController.Services.Interfaces
{
    public interface ISettingsService
    {
        double MouseSensitivity { get; set; }
        bool IsMouseEnabled { get; set; }
        bool IsKeyboardEnabled { get; set; }
        bool IsControlEnabled { get; set; }
        bool UseNaturalScrolling { get; set; }
        bool InvertYAxis { get; set; }
        bool EnableSmoothing { get; set; }
        int SmoothingLevel { get; set; }
        bool EnableNonLinearCurve { get; set; }
        double NonLinearCurvePower { get; set; }
        double DeadZone { get; set; }
        bool IsGestureMode { get; set; }
        bool IsRelativeMode { get; set; }
        float GestureSensitivity { get; set; }
        bool ShowGestureHints { get; set; }
        GearVRController.Enums.GestureAction SwipeUpAction { get; set; }
        GearVRController.Enums.GestureAction SwipeDownAction { get; set; }
        GearVRController.Enums.GestureAction SwipeLeftAction { get; set; }
        GearVRController.Enums.GestureAction SwipeRightAction { get; set; }
        GearVRController.Models.GestureConfig GestureConfig { get; }
        int MaxReconnectAttempts { get; }
        int ReconnectDelayMs { get; }

        // New configurable parameters for ControllerService
        double MouseSensitivityScalingFactor { get; }
        double MoveThreshold { get; }
        int TouchThreshold { get; }

        void ResetToDefaults();
        Task SaveSettingsAsync();
        Task LoadSettingsAsync();
        GearVRController.ViewModels.TouchpadCalibrationData? LoadCalibrationData();
        void SaveCalibrationData(GearVRController.ViewModels.TouchpadCalibrationData calibrationData);
    }
}