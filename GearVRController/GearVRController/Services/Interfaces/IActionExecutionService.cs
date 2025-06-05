using GearVRController.Enums;

namespace GearVRController.Services.Interfaces
{
    /// <summary>
    /// 定义一个服务，用于执行与 Gear VR 控制器手势相关的各种操作。
    /// </summary>
    public interface IActionExecutionService
    {
        /// <summary>
        /// 执行指定的手势动作。
        /// </summary>
        /// <param name="action">要执行的 GestureAction 枚举值。</param>
        void ExecuteAction(GestureAction action);
    }
}