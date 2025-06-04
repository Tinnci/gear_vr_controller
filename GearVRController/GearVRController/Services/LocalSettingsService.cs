using System;
using System.Threading.Tasks;
using Windows.Storage;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class LocalSettingsService : ISettingsService
    {
        private const string MOUSE_SENSITIVITY_KEY = "MouseSensitivity";
        private const string IS_MOUSE_ENABLED_KEY = "IsMouseEnabled";
        private const string IS_KEYBOARD_ENABLED_KEY = "IsKeyboardEnabled";
        private const string IS_CONTROL_ENABLED_KEY = "IsControlEnabled";
        private const string USE_NATURAL_SCROLLING_KEY = "UseNaturalScrolling";
        private const string INVERT_Y_AXIS_KEY = "InvertYAxis";
        private const string ENABLE_SMOOTHING_KEY = "EnableSmoothing";
        private const string SMOOTHING_LEVEL_KEY = "SmoothingLevel";
        private const string ENABLE_NON_LINEAR_CURVE_KEY = "EnableNonLinearCurve";
        private const string NON_LINEAR_CURVE_POWER_KEY = "NonLinearCurvePower";
        private const string DEAD_ZONE_KEY = "DeadZone";

        // Add keys for gesture settings
        private const string IS_GESTURE_MODE_KEY = "IsGestureMode";
        private const string GESTURE_SENSITIVITY_KEY = "GestureSensitivity";
        private const string SHOW_GESTURE_HINTS_KEY = "ShowGestureHints";
        private const string SWIPE_UP_ACTION_KEY = "SwipeUpAction";
        private const string SWIPE_DOWN_ACTION_KEY = "SwipeDownAction";
        private const string SWIPE_LEFT_ACTION_KEY = "SwipeLeftAction";
        private const string SWIPE_RIGHT_ACTION_KEY = "SwipeRightAction";

        // Add keys for calibration data
        private const string CALIBRATION_MIN_X_KEY = "CalibrationMinX";
        private const string CALIBRATION_MAX_X_KEY = "CalibrationMaxX";
        private const string CALIBRATION_MIN_Y_KEY = "CalibrationMinY";
        private const string CALIBRATION_MAX_Y_KEY = "CalibrationMaxY";
        private const string CALIBRATION_CENTER_X_KEY = "CalibrationCenterX";
        private const string CALIBRATION_CENTER_Y_KEY = "CalibrationCenterY";
        // Directional calibration keys could be added here if needed, but for simplicity,
        // we might only save min/max/center for basic processing or save the whole object serialized.
        // Let's just save min/max/center for now as per the interface return type (TouchpadCalibrationData).

        private readonly ApplicationDataContainer _localSettings;

        public LocalSettingsService()
        {
            _localSettings = ApplicationData.Current.LocalSettings;
            LoadDefaultSettings();
        }

        public double MouseSensitivity
        {
            get => GetSetting(MOUSE_SENSITIVITY_KEY, 1.0);
            set => SaveSetting(MOUSE_SENSITIVITY_KEY, value);
        }

        public bool IsMouseEnabled
        {
            get => GetSetting(IS_MOUSE_ENABLED_KEY, true);
            set => SaveSetting(IS_MOUSE_ENABLED_KEY, value);
        }

        public bool IsKeyboardEnabled
        {
            get => GetSetting(IS_KEYBOARD_ENABLED_KEY, true);
            set => SaveSetting(IS_KEYBOARD_ENABLED_KEY, value);
        }

        public bool IsControlEnabled
        {
            get => GetSetting(IS_CONTROL_ENABLED_KEY, true);
            set => SaveSetting(IS_CONTROL_ENABLED_KEY, value);
        }

        public bool UseNaturalScrolling
        {
            get => GetSetting(USE_NATURAL_SCROLLING_KEY, false);
            set => SaveSetting(USE_NATURAL_SCROLLING_KEY, value);
        }

        public bool InvertYAxis
        {
            get => GetSetting(INVERT_Y_AXIS_KEY, false);
            set => SaveSetting(INVERT_Y_AXIS_KEY, value);
        }

        public bool EnableSmoothing
        {
            get => GetSetting(ENABLE_SMOOTHING_KEY, true);
            set => SaveSetting(ENABLE_SMOOTHING_KEY, value);
        }

        public int SmoothingLevel
        {
            get => GetSetting(SMOOTHING_LEVEL_KEY, 3);
            set => SaveSetting(SMOOTHING_LEVEL_KEY, Math.Max(1, Math.Min(value, 10)));
        }

        public bool EnableNonLinearCurve
        {
            get => GetSetting(ENABLE_NON_LINEAR_CURVE_KEY, true);
            set => SaveSetting(ENABLE_NON_LINEAR_CURVE_KEY, value);
        }

        public double NonLinearCurvePower
        {
            get => GetSetting(NON_LINEAR_CURVE_POWER_KEY, 1.5);
            set => SaveSetting(NON_LINEAR_CURVE_POWER_KEY, Math.Max(1.0, Math.Min(value, 3.0)));
        }

        public double DeadZone
        {
            get => GetSetting(DEAD_ZONE_KEY, 8.0);
            set => SaveSetting(DEAD_ZONE_KEY, Math.Max(0.0, Math.Min(value, 20.0)));
        }

        // Implement gesture settings properties
        public bool IsGestureMode
        {
            get => GetSetting(IS_GESTURE_MODE_KEY, false);
            set => SaveSetting(IS_GESTURE_MODE_KEY, value);
        }

        public bool IsRelativeMode
        {   // IsRelativeMode is the inverse of IsGestureMode
            get => !IsGestureMode;
            set => IsGestureMode = !value;
        }

        public float GestureSensitivity
        {
            get => GetSetting(GESTURE_SENSITIVITY_KEY, 0.3f);
            set => SaveSetting(GESTURE_SENSITIVITY_KEY, Math.Clamp(value, 0.1f, 1.0f));
        }

        public bool ShowGestureHints
        {
            get => GetSetting(SHOW_GESTURE_HINTS_KEY, true);
            set => SaveSetting(SHOW_GESTURE_HINTS_KEY, value);
        }

        public GearVRController.Enums.GestureAction SwipeUpAction
        {
            get => (GearVRController.Enums.GestureAction)GetSetting(SWIPE_UP_ACTION_KEY, (int)GearVRController.Enums.GestureAction.PageUp);
            set => SaveSetting(SWIPE_UP_ACTION_KEY, (int)value);
        }

        public GearVRController.Enums.GestureAction SwipeDownAction
        {
            get => (GearVRController.Enums.GestureAction)GetSetting(SWIPE_DOWN_ACTION_KEY, (int)GearVRController.Enums.GestureAction.PageDown);
            set => SaveSetting(SWIPE_DOWN_ACTION_KEY, (int)value);
        }

        public GearVRController.Enums.GestureAction SwipeLeftAction
        {
            get => (GearVRController.Enums.GestureAction)GetSetting(SWIPE_LEFT_ACTION_KEY, (int)GearVRController.Enums.GestureAction.BrowserBack);
            set => SaveSetting(SWIPE_LEFT_ACTION_KEY, (int)value);
        }

        public GearVRController.Enums.GestureAction SwipeRightAction
        {
            get => (GearVRController.Enums.GestureAction)GetSetting(SWIPE_RIGHT_ACTION_KEY, (int)GearVRController.Enums.GestureAction.BrowserForward);
            set => SaveSetting(SWIPE_RIGHT_ACTION_KEY, (int)value);
        }

        public Task LoadSettingsAsync()
        {
            // 设置已经在属性访问器中加载
            return Task.CompletedTask;
        }

        public Task SaveSettingsAsync()
        {
            // 设置已经在属性访问器中保存
            return Task.CompletedTask;
        }

        public void ResetToDefaults()
        {
            MouseSensitivity = 1.0;
            IsMouseEnabled = true;
            IsKeyboardEnabled = true;
            IsControlEnabled = true;
            UseNaturalScrolling = false;
            InvertYAxis = false;
            EnableSmoothing = true;
            SmoothingLevel = 3;
            EnableNonLinearCurve = true;
            NonLinearCurvePower = 1.5;
            DeadZone = 8.0;

            // Reset gesture settings
            IsGestureMode = false;
            GestureSensitivity = 0.3f;
            ShowGestureHints = true;
            SwipeUpAction = GearVRController.Enums.GestureAction.PageUp;
            SwipeDownAction = GearVRController.Enums.GestureAction.PageDown;
            SwipeLeftAction = GearVRController.Enums.GestureAction.BrowserBack;
            SwipeRightAction = GearVRController.Enums.GestureAction.BrowserForward;

            // Reset calibration data keys by removing them
            _localSettings.Values.Remove(CALIBRATION_MIN_X_KEY);
            _localSettings.Values.Remove(CALIBRATION_MAX_X_KEY);
            _localSettings.Values.Remove(CALIBRATION_MIN_Y_KEY);
            _localSettings.Values.Remove(CALIBRATION_MAX_Y_KEY);
            _localSettings.Values.Remove(CALIBRATION_CENTER_X_KEY);
            _localSettings.Values.Remove(CALIBRATION_CENTER_Y_KEY);
        }

        private void LoadDefaultSettings()
        {
            if (!_localSettings.Values.ContainsKey(MOUSE_SENSITIVITY_KEY))
            {
                ResetToDefaults();
            }
        }

        private T GetSetting<T>(string key, T defaultValue)
        {
            if (_localSettings.Values.TryGetValue(key, out object? value))
            {
                if (value is T typedValue)
                {
                    return typedValue;
                }
            }
            return defaultValue;
        }

        private void SaveSetting<T>(string key, T value)
        {
            _localSettings.Values[key] = value;
        }

        // Implement LoadCalibrationData method
        public ViewModels.TouchpadCalibrationData? LoadCalibrationData()
        {
            if (_localSettings.Values.ContainsKey(CALIBRATION_MIN_X_KEY) &&
                _localSettings.Values.ContainsKey(CALIBRATION_MAX_X_KEY) &&
                _localSettings.Values.ContainsKey(CALIBRATION_MIN_Y_KEY) &&
                _localSettings.Values.ContainsKey(CALIBRATION_MAX_Y_KEY) &&
                _localSettings.Values.ContainsKey(CALIBRATION_CENTER_X_KEY) &&
                _localSettings.Values.ContainsKey(CALIBRATION_CENTER_Y_KEY))
            {
                try
                {
                    var calibrationData = new ViewModels.TouchpadCalibrationData
                    {
                        MinX = GetSetting(CALIBRATION_MIN_X_KEY, 0), // Default 0, but should always be present if keys exist
                        MaxX = GetSetting(CALIBRATION_MAX_X_KEY, 0),
                        MinY = GetSetting(CALIBRATION_MIN_Y_KEY, 0),
                        MaxY = GetSetting(CALIBRATION_MAX_Y_KEY, 0),
                        CenterX = GetSetting(CALIBRATION_CENTER_X_KEY, 0),
                        CenterY = GetSetting(CALIBRATION_CENTER_Y_KEY, 0)
                        // Directional data is not saved/loaded this way currently.
                    };
                    // Validate loaded data might be needed
                    if (calibrationData.MaxX > calibrationData.MinX && calibrationData.MaxY > calibrationData.MinY) // Basic validation
                    {
                        return calibrationData;
                    }
                }
                catch (Exception ex)
                {
                    System.Diagnostics.Debug.WriteLine($"加载校准数据失败: {ex.Message}");
                }
            }
            return null; // Return null if not all keys exist or loading fails
        }

        // Implement SaveCalibrationData method
        public void SaveCalibrationData(GearVRController.ViewModels.TouchpadCalibrationData calibrationData)
        {
            if (calibrationData != null)
            {
                SaveSetting(CALIBRATION_MIN_X_KEY, calibrationData.MinX);
                SaveSetting(CALIBRATION_MAX_X_KEY, calibrationData.MaxX);
                SaveSetting(CALIBRATION_MIN_Y_KEY, calibrationData.MinY);
                SaveSetting(CALIBRATION_MAX_Y_KEY, calibrationData.MaxY);
                SaveSetting(CALIBRATION_CENTER_X_KEY, calibrationData.CenterX);
                SaveSetting(CALIBRATION_CENTER_Y_KEY, calibrationData.CenterY);
                // Note: Directional calibration data is not saved here currently.
                System.Diagnostics.Debug.WriteLine("触摸板校准数据已保存");
            }
        }
    }
}