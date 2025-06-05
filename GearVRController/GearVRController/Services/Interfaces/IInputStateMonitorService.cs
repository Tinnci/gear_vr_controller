using System.Threading.Tasks;

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
        void StartMonitor();
        /// <summary>
        /// 强制释放所有已按下的按钮。
        /// </summary>
        void ForceReleaseAllButtons();
        /// <summary>
        /// 停止监控输入状态。
        /// </summary>
        void StopMonitor();
        /// <summary>
        /// 通知服务发生了输入活动，用于重置不活动计时器。
        /// </summary>
        void NotifyInputActivity();
        // Additional methods or properties if needed by other services/viewmodels
    }
}