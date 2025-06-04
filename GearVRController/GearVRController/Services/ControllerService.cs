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
        private readonly TouchpadProcessor _touchpadProcessor;
        private readonly IInputSimulator _inputSimulator;
        private readonly GestureRecognizer _gestureRecognizer;

        private bool _isTouchpadCurrentlyTouched = false;
        private double _lastTouchpadProcessedX = 0;
        private double _lastTouchpadProcessedY = 0;

        public event EventHandler<ControllerData>? ControllerDataProcessed;

        public ControllerService(IBluetoothService bluetoothService, ISettingsService settingsService, TouchpadProcessor touchpadProcessor, IInputSimulator inputSimulator, GestureRecognizer gestureRecognizer)
        {
            _bluetoothService = bluetoothService;
            _settingsService = settingsService;
            _touchpadProcessor = touchpadProcessor;
            _inputSimulator = inputSimulator;
            _gestureRecognizer = gestureRecognizer;
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

            // 处理触摸板数据，区分手势模式和非手势模式 (相对模式)
            if (_settingsService.IsGestureMode)
            {
                // 手势模式：只将处理后的触摸板数据传递给手势识别器
                // 不进行鼠标移动，手势识别结果由 MainViewModel 处理离散动作
                var (processedX, processedY) = _touchpadProcessor.ProcessRawData(data.AxisX, data.AxisY);
                _gestureRecognizer.ProcessTouchpadPoint(new Models.TouchpadPoint { X = (float)processedX, Y = (float)processedY, IsTouched = data.TouchpadTouched });

                if (!data.TouchpadTouched && _isTouchpadCurrentlyTouched) // 触摸结束
                {
                    _isTouchpadCurrentlyTouched = false;
                    System.Diagnostics.Debug.WriteLine("[ControllerService] 触摸结束 (手势模式).");
                }
                else if (data.TouchpadTouched && !_isTouchpadCurrentlyTouched) // 触摸开始
                {
                    _isTouchpadCurrentlyTouched = true;
                    System.Diagnostics.Debug.WriteLine("[ControllerService] 触摸开始 (手势模式).");
                }
            }
            else // 非手势模式 (相对模式)：处理连续鼠标移动
            {
                ProcessRelativeModeTouchpad(data); // 调用重命名后的方法
            }

            // 触发数据处理完成事件 (无论模式如何，数据都传递出去)
            OnControllerDataProcessed(data);
        }

        private void ProcessRelativeModeTouchpad(ControllerData data)
        {
            // 使用 TouchpadProcessor 处理原始触摸板数据到归一化坐标 (-1 to 1)
            var (processedX, processedY) = _touchpadProcessor.ProcessRawData(data.AxisX, data.AxisY);

            // 将处理后的点传递给手势识别器（无论触摸状态如何，让它跟踪点）
            // 注意：GestureRecognizer 需要 TouchpadPoint 对象，这里我们需要根据 ControllerData 创建
            _gestureRecognizer.ProcessTouchpadPoint(new Models.TouchpadPoint { X = (float)processedX, Y = (float)processedY, IsTouched = data.TouchpadTouched });

            if (data.TouchpadTouched)
            {
                if (!_isTouchpadCurrentlyTouched) // 触摸开始
                {
                    // 记录起始点，不立即移动鼠标
                    _lastTouchpadProcessedX = processedX;
                    _lastTouchpadProcessedY = processedY;
                    _isTouchpadCurrentlyTouched = true;
                    System.Diagnostics.Debug.WriteLine($"[ControllerService] 触摸开始: ({{processedX:F2}}, {{processedY:F2}})");
                }
                else // 触摸持续
                {
                    // 计算与上一个点的增量
                    double deltaX = processedX - _lastTouchpadProcessedX;
                    double deltaY = processedY - _lastTouchpadProcessedY;

                    // 更新上一个点
                    _lastTouchpadProcessedX = processedX;
                    _lastTouchpadProcessedY = processedY;

                    // 根据增量、灵敏度和缩放因子计算鼠标移动量
                    // 缩放因子需要调整以获得合适的移动速度，可能需要根据实际测试来确定。
                    // 一个小的缩放因子可能更合适，因为deltaX/Y是-2到2之间的变化。
                    double mouseDeltaX = deltaX * _settingsService.MouseSensitivity * 100.0; // 示例缩放因子 100.0
                    double mouseDeltaY = deltaY * _settingsService.MouseSensitivity * 100.0; // 示例缩放因子 100.0

                    // 模拟鼠标移动
                    // 增加一个小的阈值，避免微小移动导致的抖动
                    const double MOVE_THRESHOLD = 0.005; // 示例阈值
                    if (Math.Abs(mouseDeltaX) > MOVE_THRESHOLD || Math.Abs(mouseDeltaY) > MOVE_THRESHOLD)
                    {
                        _inputSimulator.SimulateMouseMovement(mouseDeltaX, mouseDeltaY);
                        System.Diagnostics.Debug.WriteLine($"[ControllerService] 模拟鼠标移动: DeltaX={{mouseDeltaX:F2}}, DeltaY={{mouseDeltaY:F2}}");
                    }
                }
            }
            else // 触摸结束
            {
                if (_isTouchpadCurrentlyTouched) // 刚抬起
                {
                    _isTouchpadCurrentlyTouched = false;
                    // 触摸结束时，GestureRecognizer 会根据接收到的点序列尝试识别离散手势
                    System.Diagnostics.Debug.WriteLine("[ControllerService] 触摸结束 (相对模式).");
                }
                // 如果本来就没有触摸，则不做任何事
            }
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