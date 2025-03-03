using System;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class ControllerService : IControllerService
    {
        private readonly IBluetoothService _bluetoothService;
        public event EventHandler<ControllerData>? ControllerDataProcessed;

        public ControllerService(IBluetoothService bluetoothService)
        {
            _bluetoothService = bluetoothService;
        }

        public Task InitializeAsync()
        {
            // 初始化控制器
            return Task.CompletedTask;
        }

        public Task SendCommandAsync(byte[] command, int repeat = 1)
        {
            // 发送命令到控制器
            return _bluetoothService.SendDataAsync(command, repeat);
        }

        public void ProcessControllerData(ControllerData data)
        {
            if (data == null)
                return;

            // 验证数据有效性
            if (!ValidateControllerData(data))
                return;

            // 预处理数据
            PreprocessControllerData(data);

            // 触发数据处理完成事件
            OnControllerDataProcessed(data);
        }

        private bool ValidateControllerData(ControllerData data)
        {
            // 临时关闭范围检查，以便观察实际的触摸板数据范围
            // 检查触摸板数据范围（使用原始值范围0-1023）
            if (data.AxisX < 0 || data.AxisX > 1023 || data.AxisY < 0 || data.AxisY > 1023)
            {
                System.Diagnostics.Debug.WriteLine($"[警告] 触摸板数据超出预期范围: X={data.AxisX}, Y={data.AxisY}");
                // 不返回false，继续处理
            }

            // 检查加速度计数据
            if (float.IsNaN(data.AccelX) || float.IsNaN(data.AccelY) || float.IsNaN(data.AccelZ))
                return false;

            // 检查陀螺仪数据
            if (float.IsNaN(data.GyroX) || float.IsNaN(data.GyroY) || float.IsNaN(data.GyroZ))
                return false;

            return true;
        }

        private void PreprocessControllerData(ControllerData data)
        {
            // 更新时间戳
            data.Timestamp = DateTime.Now;

            // 处理触摸状态
            data.TouchpadTouched = data.AxisX != 0 || data.AxisY != 0;

            // 处理按钮状态
            data.NoButton = !data.TriggerButton && !data.HomeButton &&
                           !data.BackButton && !data.TouchpadButton &&
                           !data.VolumeUpButton && !data.VolumeDownButton;
        }

        protected virtual void OnControllerDataProcessed(ControllerData data)
        {
            ControllerDataProcessed?.Invoke(this, data);
        }
    }
}