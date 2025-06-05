using System.Threading.Tasks;
using GearVRController.ViewModels;
using GearVRController.Models;
using System.Collections.Generic;

namespace GearVRController.Services.Interfaces
{
    /// <summary>
    /// 定义一个用于管理应用程序设置的服务接口。
    /// </summary>
    public interface ISettingsService
    {
        /// <summary>
        /// 获取或设置鼠标灵敏度。
        /// </summary>
        double MouseSensitivity { get; set; }
        /// <summary>
        /// 获取或设置鼠标是否启用。
        /// </summary>
        bool IsMouseEnabled { get; set; }
        /// <summary>
        /// 获取或设置键盘是否启用。
        /// </summary>
        bool IsKeyboardEnabled { get; set; }
        /// <summary>
        /// 获取或设置控制器控制是否启用。
        /// </summary>
        bool IsControlEnabled { get; set; }
        /// <summary>
        /// 获取或设置是否使用自然滚动。
        /// </summary>
        bool UseNaturalScrolling { get; set; }
        /// <summary>
        /// 获取或设置是否反转 Y 轴。
        /// </summary>
        bool InvertYAxis { get; set; }
        /// <summary>
        /// 获取或设置是否启用平滑处理。
        /// </summary>
        bool EnableSmoothing { get; set; }
        /// <summary>
        /// 获取或设置平滑处理级别。
        /// </summary>
        int SmoothingLevel { get; set; }
        /// <summary>
        /// 获取或设置是否启用非线性曲线。
        /// </summary>
        bool EnableNonLinearCurve { get; set; }
        /// <summary>
        /// 获取或设置非线性曲线的幂值。
        /// </summary>
        double NonLinearCurvePower { get; set; }
        /// <summary>
        /// 获取或设置死区值。
        /// </summary>
        double DeadZone { get; set; }
        /// <summary>
        /// 获取或设置是否处于手势模式。
        /// </summary>
        bool IsGestureMode { get; set; }
        /// <summary>
        /// 获取或设置是否处于相对模式。
        /// </summary>
        bool IsRelativeMode { get; set; }
        /// <summary>
        /// 获取或设置手势灵敏度。
        /// </summary>
        float GestureSensitivity { get; set; }
        /// <summary>
        /// 获取或设置是否显示手势提示。
        /// </summary>
        bool ShowGestureHints { get; set; }
        /// <summary>
        /// 获取或设置向上滑动时执行的动作。
        /// </summary>
        GearVRController.Enums.GestureAction SwipeUpAction { get; set; }
        /// <summary>
        /// 获取或设置向下滑动时执行的动作。
        /// </summary>
        GearVRController.Enums.GestureAction SwipeDownAction { get; set; }
        /// <summary>
        /// 获取或设置向左滑动时执行的动作。
        /// </summary>
        GearVRController.Enums.GestureAction SwipeLeftAction { get; set; }
        /// <summary>
        /// 获取或设置向右滑动时执行的动作。
        /// </summary>
        GearVRController.Enums.GestureAction SwipeRightAction { get; set; }
        /// <summary>
        /// 获取手势配置。
        /// </summary>
        GearVRController.Models.GestureConfig GestureConfig { get; }
        /// <summary>
        /// 获取最大重连尝试次数。
        /// </summary>
        int MaxReconnectAttempts { get; }
        /// <summary>
        /// 获取重连延迟（毫秒）。
        /// </summary>
        int ReconnectDelayMs { get; }

        /// <summary>
        /// 获取鼠标灵敏度缩放因子。
        /// </summary>
        double MouseSensitivityScalingFactor { get; }
        /// <summary>
        /// 获取移动阈值。
        /// </summary>
        double MoveThreshold { get; }
        /// <summary>
        /// 获取触摸阈值。
        /// </summary>
        int TouchThreshold { get; }

        /// <summary>
        /// 获取或设置已知的蓝牙地址列表。
        /// </summary>
        List<ulong> KnownBluetoothAddresses { get; set; }

        /// <summary>
        /// 将所有设置重置为默认值。
        /// </summary>
        void ResetToDefaults();
        /// <summary>
        /// 初始化应用程序设置，确保默认值已加载。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        Task InitializeSettings();
        /// <summary>
        /// 加载触控板校准数据。
        /// </summary>
        /// <returns>触控板校准数据，如果不存在则为 null。</returns>
        GearVRController.ViewModels.TouchpadCalibrationData? LoadCalibrationData();
        /// <summary>
        /// 保存触控板校准数据。
        /// </summary>
        /// <param name="calibrationData">要保存的触控板校准数据。</param>
        void SaveCalibrationData(GearVRController.ViewModels.TouchpadCalibrationData calibrationData);
    }
}