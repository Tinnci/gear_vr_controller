using System;
using System.Threading.Tasks;
using Windows.Storage;
using GearVRController.Services.Interfaces;
using System.Collections.Generic;
using System.Linq;
using GearVRController.Models;
using GearVRController.Enums;
using GearVRController.ViewModels;

namespace GearVRController.Services
{
    /// <summary>
    /// LocalSettingsService 实现了 ISettingsService 接口，用于管理应用程序的本地设置。
    /// 它负责加载、保存和提供对各种应用程序配置的访问，包括鼠标、键盘、手势设置和校准数据。
    /// 所有设置都持久化到 Windows 应用程序的本地设置存储中。
    /// </summary>
    public class LocalSettingsService : ISettingsService
    {
        /// <summary>
        /// 鼠标灵敏度设置的键名。
        /// </summary>
        private const string MOUSE_SENSITIVITY_KEY = "MouseSensitivity";
        /// <summary>
        /// 鼠标输入启用状态设置的键名。
        /// </summary>
        private const string IS_MOUSE_ENABLED_KEY = "IsMouseEnabled";
        /// <summary>
        /// 键盘输入启用状态设置的键名。
        /// </summary>
        private const string IS_KEYBOARD_ENABLED_KEY = "IsKeyboardEnabled";
        /// <summary>
        /// 整体控制启用状态设置的键名。
        /// </summary>
        private const string IS_CONTROL_ENABLED_KEY = "IsControlEnabled";
        /// <summary>
        /// 自然滚动启用状态设置的键名。
        /// </summary>
        private const string USE_NATURAL_SCROLLING_KEY = "UseNaturalScrolling";
        /// <summary>
        /// Y轴反转设置的键名。
        /// </summary>
        private const string INVERT_Y_AXIS_KEY = "InvertYAxis";
        /// <summary>
        /// 鼠标平滑处理启用状态设置的键名。
        /// </summary>
        private const string ENABLE_SMOOTHING_KEY = "EnableSmoothing";
        /// <summary>
        /// 平滑等级设置的键名。
        /// </summary>
        private const string SMOOTHING_LEVEL_KEY = "SmoothingLevel";
        /// <summary>
        /// 非线性曲线启用状态设置的键名。
        /// </summary>
        private const string ENABLE_NON_LINEAR_CURVE_KEY = "EnableNonLinearCurve";
        /// <summary>
        /// 非线性曲线幂次设置的键名。
        /// </summary>
        private const string NON_LINEAR_CURVE_POWER_KEY = "NonLinearCurvePower";
        /// <summary>
        /// 死区设置的键名。
        /// </summary>
        private const string DEAD_ZONE_KEY = "DeadZone";

        /// <summary>
        /// 手势模式启用状态设置的键名。
        /// </summary>
        private const string IS_GESTURE_MODE_KEY = "IsGestureMode";
        /// <summary>
        /// 手势灵敏度设置的键名。
        /// </summary>
        private const string GESTURE_SENSITIVITY_KEY = "GestureSensitivity";
        /// <summary>
        /// 显示手势提示设置的键名。
        /// </summary>
        private const string SHOW_GESTURE_HINTS_KEY = "ShowGestureHints";
        /// <summary>
        /// 向上滑动动作设置的键名。
        /// </summary>
        private const string SWIPE_UP_ACTION_KEY = "SwipeUpAction";
        /// <summary>
        /// 向下滑动动作设置的键名。
        /// </summary>
        private const string SWIPE_DOWN_ACTION_KEY = "SwipeDownAction";
        /// <summary>
        /// 向左滑动动作设置的键名。
        /// </summary>
        private const string SWIPE_LEFT_ACTION_KEY = "SwipeLeftAction";
        /// <summary>
        /// 向右滑动动作设置的键名。
        /// </summary>
        private const string SWIPE_RIGHT_ACTION_KEY = "SwipeRightAction";

        /// <summary>
        /// 校准数据中MinX的键名。
        /// </summary>
        private const string CALIBRATION_MIN_X_KEY = "CalibrationMinX";
        /// <summary>
        /// 校准数据中MaxX的键名。
        /// </summary>
        private const string CALIBRATION_MAX_X_KEY = "CalibrationMaxX";
        /// <summary>
        /// 校准数据中MinY的键名。
        /// </summary>
        private const string CALIBRATION_MIN_Y_KEY = "CalibrationMinY";
        /// <summary>
        /// 校准数据中MaxY的键名。
        /// </summary>
        private const string CALIBRATION_MAX_Y_KEY = "CalibrationMaxY";
        /// <summary>
        /// 校准数据中CenterX的键名。
        /// </summary>
        private const string CALIBRATION_CENTER_X_KEY = "CalibrationCenterX";
        /// <summary>
        /// 校准数据中CenterY的键名。
        /// </summary>
        private const string CALIBRATION_CENTER_Y_KEY = "CalibrationCenterY";

        /// <summary>
        /// 最大重连尝试次数设置的键名。
        /// </summary>
        private const string MAX_RECONNECT_ATTEMPTS_KEY = "MaxReconnectAttempts";
        /// <summary>
        /// 重连延迟时间（毫秒）设置的键名。
        /// </summary>
        private const string RECONNECT_DELAY_MS_KEY = "ReconnectDelayMs";

        /// <summary>
        /// 鼠标灵敏度缩放因子设置的键名。
        /// </summary>
        private const string MOUSE_SENSITIVITY_SCALING_FACTOR_KEY = "MouseSensitivityScalingFactor";
        /// <summary>
        /// 移动阈值设置的键名。
        /// </summary>
        private const string MOVE_THRESHOLD_KEY = "MoveThreshold";
        /// <summary>
        /// 触摸阈值设置的键名。
        /// </summary>
        private const string TOUCH_THRESHOLD_KEY = "TouchThreshold";

        /// <summary>
        /// 已知蓝牙地址列表设置的键名。
        /// </summary>
        private const string KNOWN_BLUETOOTH_ADDRESSES_KEY = "KnownBluetoothAddresses";

        /// <summary>
        /// 用于访问应用程序本地设置的容器。
        /// </summary>
        private readonly ApplicationDataContainer _localSettings;

        /// <summary>
        /// LocalSettingsService 的构造函数。
        /// 初始化对本地设置的访问，并加载默认设置。
        /// </summary>
        public LocalSettingsService()
        {
            _localSettings = ApplicationData.Current.LocalSettings;
            LoadDefaultSettings();
        }

        /// <summary>
        /// 获取或设置鼠标灵敏度。
        /// 默认值为 1.0。
        /// </summary>
        public double MouseSensitivity
        {
            get => GetSetting(MOUSE_SENSITIVITY_KEY, 1.0);
            set => SaveSetting(MOUSE_SENSITIVITY_KEY, value);
        }

        /// <summary>
        /// 获取或设置鼠标输入是否启用。
        /// 默认值为 true。
        /// </summary>
        public bool IsMouseEnabled
        {
            get => GetSetting(IS_MOUSE_ENABLED_KEY, true);
            set => SaveSetting(IS_MOUSE_ENABLED_KEY, value);
        }

        /// <summary>
        /// 获取或设置键盘输入是否启用。
        /// 默认值为 true。
        /// </summary>
        public bool IsKeyboardEnabled
        {
            get => GetSetting(IS_KEYBOARD_ENABLED_KEY, true);
            set => SaveSetting(IS_KEYBOARD_ENABLED_KEY, value);
        }

        /// <summary>
        /// 获取或设置整体控制是否启用。
        /// 默认值为 true。
        /// </summary>
        public bool IsControlEnabled
        {
            get => GetSetting(IS_CONTROL_ENABLED_KEY, true);
            set => SaveSetting(IS_CONTROL_ENABLED_KEY, value);
        }

        /// <summary>
        /// 获取或设置是否使用自然滚动。
        /// 默认值为 false。
        /// </summary>
        public bool UseNaturalScrolling
        {
            get => GetSetting(USE_NATURAL_SCROLLING_KEY, false);
            set => SaveSetting(USE_NATURAL_SCROLLING_KEY, value);
        }

        /// <summary>
        /// 获取或设置是否反转Y轴。
        /// 默认值为 true。
        /// </summary>
        public bool InvertYAxis
        {
            get => GetSetting(INVERT_Y_AXIS_KEY, true);
            set => SaveSetting(INVERT_Y_AXIS_KEY, value);
        }

        /// <summary>
        /// 获取或设置是否启用平滑处理。
        /// 默认值为 true。
        /// </summary>
        public bool EnableSmoothing
        {
            get => GetSetting(ENABLE_SMOOTHING_KEY, true);
            set => SaveSetting(ENABLE_SMOOTHING_KEY, value);
        }

        /// <summary>
        /// 获取或设置平滑等级。
        /// 默认值为 3，范围限制在 1 到 10 之间。
        /// </summary>
        public int SmoothingLevel
        {
            get => GetSetting(SMOOTHING_LEVEL_KEY, 3);
            set => SaveSetting(SMOOTHING_LEVEL_KEY, Math.Max(1, Math.Min(value, 10)));
        }

        /// <summary>
        /// 获取或设置是否启用非线性曲线。
        /// 默认值为 true。
        /// </summary>
        public bool EnableNonLinearCurve
        {
            get => GetSetting(ENABLE_NON_LINEAR_CURVE_KEY, true);
            set => SaveSetting(ENABLE_NON_LINEAR_CURVE_KEY, value);
        }

        /// <summary>
        /// 获取或设置非线性曲线的幂次。
        /// 默认值为 1.5，范围限制在 1.0 到 3.0 之间。
        /// </summary>
        public double NonLinearCurvePower
        {
            get => GetSetting(NON_LINEAR_CURVE_POWER_KEY, 1.5);
            set => SaveSetting(NON_LINEAR_CURVE_POWER_KEY, Math.Max(1.0, Math.Min(value, 3.0)));
        }

        /// <summary>
        /// 获取或设置死区大小。
        /// 默认值为 8.0，范围限制在 0.0 到 20.0 之间。
        /// </summary>
        public double DeadZone
        {
            get => GetSetting(DEAD_ZONE_KEY, 8.0);
            set => SaveSetting(DEAD_ZONE_KEY, Math.Max(0.0, Math.Min(value, 20.0)));
        }

        /// <summary>
        /// 获取或设置是否启用手势模式（与相对模式相反）。
        /// 默认值为 false。
        /// </summary>
        public bool IsGestureMode
        {
            get => GetSetting(IS_GESTURE_MODE_KEY, false);
            set => SaveSetting(IS_GESTURE_MODE_KEY, value);
        }

        /// <summary>
        /// 获取或设置是否启用相对模式（与手势模式相反）。
        /// 这是一个计算属性，直接反映 `IsGestureMode` 的反向。
        /// </summary>
        public bool IsRelativeMode
        {   // IsRelativeMode is the inverse of IsGestureMode
            get => !IsGestureMode;
            set => IsGestureMode = !value;
        }

        /// <summary>
        /// 获取或设置手势识别的灵敏度。
        /// 默认值为 0.3f，范围限制在 0.1f 到 1.0f 之间。
        /// </summary>
        public float GestureSensitivity
        {
            get => GetSetting(GESTURE_SENSITIVITY_KEY, 0.3f);
            set => SaveSetting(GESTURE_SENSITIVITY_KEY, Math.Clamp(value, 0.1f, 1.0f));
        }

        /// <summary>
        /// 获取或设置是否在 UI 上显示手势提示。
        /// 默认值为 true。
        /// </summary>
        public bool ShowGestureHints
        {
            get => GetSetting(SHOW_GESTURE_HINTS_KEY, true);
            set => SaveSetting(SHOW_GESTURE_HINTS_KEY, value);
        }

        /// <summary>
        /// 获取或设置向上滑动手势对应的动作。
        /// 默认值为 `GestureAction.PageUp`。
        /// </summary>
        public GestureAction SwipeUpAction
        {
            get => (GestureAction)GetSetting(SWIPE_UP_ACTION_KEY, (int)GestureAction.PageUp);
            set => SaveSetting(SWIPE_UP_ACTION_KEY, (int)value);
        }

        /// <summary>
        /// 获取或设置向下滑动手势对应的动作。
        /// 默认值为 `GestureAction.PageDown`。
        /// </summary>
        public GestureAction SwipeDownAction
        {
            get => (GestureAction)GetSetting(SWIPE_DOWN_ACTION_KEY, (int)GestureAction.PageDown);
            set => SaveSetting(SWIPE_DOWN_ACTION_KEY, (int)value);
        }

        /// <summary>
        /// 获取或设置向左滑动手势对应的动作。
        /// 默认值为 `GestureAction.BrowserBack`。
        /// </summary>
        public GestureAction SwipeLeftAction
        {
            get => (GestureAction)GetSetting(SWIPE_LEFT_ACTION_KEY, (int)GestureAction.BrowserBack);
            set => SaveSetting(SWIPE_LEFT_ACTION_KEY, (int)value);
        }

        /// <summary>
        /// 获取或设置向右滑动手势对应的动作。
        /// 默认值为 `GestureAction.BrowserForward`。
        /// </summary>
        public GestureAction SwipeRightAction
        {
            get => (GestureAction)GetSetting(SWIPE_RIGHT_ACTION_KEY, (int)GestureAction.BrowserForward);
            set => SaveSetting(SWIPE_RIGHT_ACTION_KEY, (int)value);
        }

        /// <summary>
        /// 获取当前的 `GestureConfig` 对象，其中包含手势灵敏度等配置。
        /// </summary>
        public GestureConfig GestureConfig
        {
            get
            {
                return new GestureConfig
                {
                    Sensitivity = GestureSensitivity
                };
            }
        }

        /// <summary>
        /// 获取最大重连尝试次数。
        /// 默认值为 3。
        /// </summary>
        public int MaxReconnectAttempts
        {
            get => GetSetting(MAX_RECONNECT_ATTEMPTS_KEY, 3);
        }

        /// <summary>
        /// 获取重连延迟时间（毫秒）。
        /// 默认值为 2000 毫秒。
        /// </summary>
        public int ReconnectDelayMs
        {
            get => GetSetting(RECONNECT_DELAY_MS_KEY, 2000);
        }

        /// <summary>
        /// 获取鼠标灵敏度缩放因子。
        /// 默认值为 100.0。
        /// </summary>
        public double MouseSensitivityScalingFactor
        {
            get => GetSetting(MOUSE_SENSITIVITY_SCALING_FACTOR_KEY, 100.0);
        }

        /// <summary>
        /// 获取移动阈值。小于此阈值的鼠标移动将被忽略。
        /// 默认值为 0.005。
        /// </summary>
        public double MoveThreshold
        {
            get => GetSetting(MOVE_THRESHOLD_KEY, 0.005);
        }

        /// <summary>
        /// 获取触摸阈值。用于判断触摸板是否被触摸。
        /// 默认值为 10。
        /// </summary>
        public int TouchThreshold
        {
            get => GetSetting(TOUCH_THRESHOLD_KEY, 10);
        }

        /// <summary>
        /// 获取或设置已知蓝牙设备的地址列表。
        /// 默认包含一些预设的地址。
        /// </summary>
        public List<ulong> KnownBluetoothAddresses
        {
            get
            {
                if (_localSettings.Values.TryGetValue(KNOWN_BLUETOOTH_ADDRESSES_KEY, out object? value) && value is string addressesString)
                {
                    return addressesString.Split(',')
                                        .Where(s => ulong.TryParse(s.Trim(), out ulong address))
                                        .Select(s => ulong.Parse(s.Trim()))
                                        .ToList();
                }
                // Default known addresses if not found or parse failed
                return new List<ulong>
                {
                    49180499202480, // 2C:BA:BA:25:6A:A1
                    49180499202481, // 2C:BA:BA:25:6A:A2 (可能的变体)
                    49180499202482  // 2C:BA:BA:25:6A:A3 (可能的变体)
                };
            }
            set
            {
                SaveSetting(KNOWN_BLUETOOTH_ADDRESSES_KEY, string.Join(",", value));
            }
        }

        /// <summary>
        /// 异步加载所有应用程序设置。
        /// 注意：由于设置通过属性访问器按需加载，此方法目前仅作为占位符。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        public Task LoadSettingsAsync()
        {
            // 设置已经在属性访问器中加载
            return Task.CompletedTask;
        }

        /// <summary>
        /// 异步保存所有应用程序设置。
        /// 注意：由于设置通过属性访问器实时保存，此方法目前仅作为占位符。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        public Task SaveSettingsAsync()
        {
            // 设置已经在属性访问器中保存
            return Task.CompletedTask;
        }

        /// <summary>
        /// 加载所有设置的默认值到本地设置中，如果它们尚未存在。
        /// 这确保了应用程序首次启动时或重置设置时，所有配置都具有合理的初始值。
        /// </summary>
        private void LoadDefaultSettings()
        {
            SaveSetting(MOUSE_SENSITIVITY_KEY, 1.0);
            SaveSetting(IS_MOUSE_ENABLED_KEY, true);
            SaveSetting(IS_KEYBOARD_ENABLED_KEY, true);
            SaveSetting(IS_CONTROL_ENABLED_KEY, true);
            SaveSetting(USE_NATURAL_SCROLLING_KEY, false);
            SaveSetting(INVERT_Y_AXIS_KEY, true);
            SaveSetting(ENABLE_SMOOTHING_KEY, true);
            SaveSetting(SMOOTHING_LEVEL_KEY, 3);
            SaveSetting(ENABLE_NON_LINEAR_CURVE_KEY, true);
            SaveSetting(NON_LINEAR_CURVE_POWER_KEY, 1.5);
            SaveSetting(DEAD_ZONE_KEY, 8.0);

            SaveSetting(IS_GESTURE_MODE_KEY, false);
            SaveSetting(GESTURE_SENSITIVITY_KEY, 0.3f);
            SaveSetting(SHOW_GESTURE_HINTS_KEY, true);
            SaveSetting(SWIPE_UP_ACTION_KEY, (int)GestureAction.PageUp);
            SaveSetting(SWIPE_DOWN_ACTION_KEY, (int)GestureAction.PageDown);
            SaveSetting(SWIPE_LEFT_ACTION_KEY, (int)GestureAction.BrowserBack);
            SaveSetting(SWIPE_RIGHT_ACTION_KEY, (int)GestureAction.BrowserForward);

            // Reconnection settings
            SaveSetting(MAX_RECONNECT_ATTEMPTS_KEY, 3);
            SaveSetting(RECONNECT_DELAY_MS_KEY, 2000);

            // ControllerService parameters
            SaveSetting(MOUSE_SENSITIVITY_SCALING_FACTOR_KEY, 100.0);
            SaveSetting(MOVE_THRESHOLD_KEY, 0.005);
            SaveSetting(TOUCH_THRESHOLD_KEY, 10);

            // Known Bluetooth Addresses - only set default if not already present
            if (!_localSettings.Values.ContainsKey(KNOWN_BLUETOOTH_ADDRESSES_KEY))
            {
                SaveSetting(KNOWN_BLUETOOTH_ADDRESSES_KEY, string.Join(",", new List<ulong>
                {
                    49180499202480, // 2C:BA:BA:25:6A:A1
                    49180499202481, // 2C:BA:BA:25:6A:A2 (可能的变体)
                    49180499202482  // 2C:BA:BA:25:6A:A3 (可能的变体)
                }));
            }
        }

        /// <summary>
        /// 从本地设置中获取指定键的值。
        /// 如果键不存在，则返回提供的默认值。
        /// </summary>
        /// <typeparam name="T">值的类型。</typeparam>
        /// <param name="key">设置的键名。</param>
        /// <param name="defaultValue">如果键不存在时使用的默认值。</param>
        /// <returns>从设置中读取的值，或默认值。</returns>
        private T GetSetting<T>(string key, T defaultValue)
        {
            if (_localSettings.Values.TryGetValue(key, out object? value))
            {
                return (T)value; // Cast directly, assuming type compatibility
            }
            return defaultValue;
        }

        /// <summary>
        /// 将指定的值保存到本地设置中。
        /// </summary>
        /// <typeparam name="T">值的类型。</typeparam>
        /// <param name="key">设置的键名。</param>
        /// <param name="value">要保存的值。</param>
        private void SaveSetting<T>(string key, T value)
        {
            _localSettings.Values[key] = value;
        }

        /// <summary>
        /// 从本地设置加载触摸板校准数据。
        /// 如果不存在，则返回 null。
        /// </summary>
        /// <returns>加载的 TouchpadCalibrationData 对象，如果不存在则为 null。</returns>
        public TouchpadCalibrationData? LoadCalibrationData()
        {
            if (_localSettings.Values.ContainsKey(CALIBRATION_MIN_X_KEY))
            {
                return new TouchpadCalibrationData
                {
                    MinX = GetSetting(CALIBRATION_MIN_X_KEY, 0),
                    MaxX = GetSetting(CALIBRATION_MAX_X_KEY, 0),
                    MinY = GetSetting(CALIBRATION_MIN_Y_KEY, 0),
                    MaxY = GetSetting(CALIBRATION_MAX_Y_KEY, 0),
                    CenterX = GetSetting(CALIBRATION_CENTER_X_KEY, 0),
                    CenterY = GetSetting(CALIBRATION_CENTER_Y_KEY, 0)
                };
            }
            return null;
        }

        /// <summary>
        /// 将触摸板校准数据保存到本地设置。
        /// </summary>
        /// <param name="calibrationData">要保存的 TouchpadCalibrationData 对象。</param>
        public void SaveCalibrationData(TouchpadCalibrationData calibrationData)
        {
            SaveSetting(CALIBRATION_MIN_X_KEY, calibrationData.MinX);
            SaveSetting(CALIBRATION_MAX_X_KEY, calibrationData.MaxX);
            SaveSetting(CALIBRATION_MIN_Y_KEY, calibrationData.MinY);
            SaveSetting(CALIBRATION_MAX_Y_KEY, calibrationData.MaxY);
            SaveSetting(CALIBRATION_CENTER_X_KEY, calibrationData.CenterX);
            SaveSetting(CALIBRATION_CENTER_Y_KEY, calibrationData.CenterY);
        }

        /// <summary>
        /// 将所有设置重置为它们的默认值。
        /// </summary>
        public void ResetToDefaults()
        {
            _localSettings.Values.Clear(); // Clear all existing settings
            LoadDefaultSettings(); // Load default values again
        }
    }
}