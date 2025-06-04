using System;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class ControllerService : IControllerService
    {
        private readonly IBluetoothService _bluetoothService;
        private readonly ISettingsService _settingsService;
        public event EventHandler<ControllerData>? ControllerDataProcessed;

        public ControllerService(IBluetoothService bluetoothService, ISettingsService settingsService)
        {
            _bluetoothService = bluetoothService;
            _settingsService = settingsService;
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

            // 应用Y轴翻转（如果启用）
            if (_settingsService.InvertYAxis)
            {
                // 在0-1023范围内翻转Y轴
                data.AxisY = 1023 - data.AxisY;
            }

            // 处理触摸状态 - 更新判断逻辑，避免误判
            // 注意：当AxisX和AxisY都很小（接近零）且TouchpadButton为false时，认为没有触摸
            const int TOUCH_THRESHOLD = 10; // 小于这个阈值且没有按下TouchpadButton时，认为是误差，没有触摸
            System.Diagnostics.Debug.WriteLine($"[ControllerService] ProcessControllerData: Raw AxisX={{data.AxisX}}, AxisY={{data.AxisY}}, TouchpadButton={{data.TouchpadButton}}");
            data.TouchpadTouched = data.TouchpadButton ||
                                  (Math.Abs(data.AxisX) > TOUCH_THRESHOLD || Math.Abs(data.AxisY) > TOUCH_THRESHOLD);
            System.Diagnostics.Debug.WriteLine($"[ControllerService] ProcessControllerData: Calculated TouchpadTouched={{data.TouchpadTouched}}");

            // 处理按钮状态
            data.NoButton = !data.TriggerButton && !data.HomeButton &&
                           !data.BackButton && !data.TouchpadButton &&
                           !data.VolumeUpButton && !data.VolumeDownButton;
        }

        // 处理滚轮移动，应用自然滚动设置
        public int ProcessWheelMovement(int delta)
        {
            // 如果启用了自然滚动，则反转滚动方向
            if (_settingsService.UseNaturalScrolling)
            {
                return -delta;
            }
            return delta;
        }

        protected virtual void OnControllerDataProcessed(ControllerData data)
        {
            ControllerDataProcessed?.Invoke(this, data);
        }
    }
}