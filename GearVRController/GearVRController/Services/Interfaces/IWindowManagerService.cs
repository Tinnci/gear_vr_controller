/// <summary>
/// 定义一个用于管理应用程序中窗口打开和关闭的服务接口。
/// </summary>
namespace GearVRController.Services.Interfaces
{
    public interface IWindowManagerService
    {
        /// <summary>
        /// 打开触控板校准窗口。
        /// </summary>
        void OpenTouchpadCalibrationWindow();
        /// <summary>
        /// 关闭触控板校准窗口。
        /// </summary>
        void CloseTouchpadCalibrationWindow();
    }
}