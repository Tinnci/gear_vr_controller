using System.Threading.Tasks;
using GearVRController.Models;
using System;

namespace GearVRController.Services.Interfaces
{
    /// <summary>
    /// 定义一个服务，用于处理 Gear VR 控制器的数据和命令。
    /// </summary>
    public interface IControllerService
    {
        /// <summary>
        /// 当控制器数据被处理时触发的事件。
        /// </summary>
        event EventHandler<ControllerData>? ControllerDataProcessed;
        /// <summary>
        /// 异步发送命令到控制器。
        /// </summary>
        /// <param name="command">要发送的命令字节数组。</param>
        /// <param name="repeat">发送命令的重复次数（默认为 1）。</param>
        /// <returns>表示异步操作的任务。</returns>
        Task SendCommandAsync(byte[] command, int repeat = 1);
        /// <summary>
        /// 处理从控制器接收到的原始数据。
        /// </summary>
        /// <param name="data">包含控制器数据的 ControllerData 对象。</param>
        void ProcessControllerData(ControllerData data);
    }
}