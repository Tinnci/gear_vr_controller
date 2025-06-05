using System;
using GearVRController.Events;
using GearVRController.Enums;

namespace GearVRController.Services.Interfaces
{
    /// <summary>
    /// 定义一个服务，用于监控输入状态并处理按钮释放。
    /// </summary>
    public interface IInputStateMonitorService
    {
        /// <summary>
        /// 开始监控输入状态。
        /// </summary>
        void StartMonitoring();
        /// <summary>
        /// 强制释放所有已按下的按钮。
        /// </summary>
        void ForceReleaseAllButtons();
        /// <summary>
        /// 停止监控输入状态。
        /// </summary>
        void StopMonitoring();
        /// <summary>
        /// 通知服务发生了输入活动，用于重置不活动计时器。
        /// </summary>
        void NotifyInputActivity();

        /// <summary>
        /// 跟踪当前按下的键，以便在超时时释放。
        /// </summary>
        void AddPressedKey(VirtualKeyCode keyCode);

        /// <summary>
        /// 从跟踪列表中移除已释放的键。
        /// </summary>
        void RemovePressedKey(VirtualKeyCode keyCode);

        /// <summary>
        /// 当检测到输入超时时触发的事件，表示某些键可能卡滞。
        /// </summary>
        event EventHandler<InputTimeoutDetectedEvent> InputTimeoutDetected;

        /// <summary>
        /// 注册一个全局热键。
        /// </summary>
        /// <param name="keyCode">要注册的虚拟键码。</param>
        /// <param name="action">热键触发时执行的动作。</param>
        void RegisterHotKey(VirtualKeyCode keyCode, Action action);
        // Additional methods or properties if needed by other services/viewmodels
    }
}