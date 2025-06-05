using System;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services.Interfaces;
using System.Collections.Generic;
using System.Linq;

namespace GearVRController.Services
{
    /// <summary>
    /// ControllerService 负责处理来自 Gear VR 控制器的原始数据，并根据应用程序设置将其转换为可模拟的输入事件。
    /// 它集成了触摸板处理、手势识别和输入模拟功能。
    /// </summary>
    public class ControllerService : IControllerService
    {
        private readonly IBluetoothService _bluetoothService;
        private readonly ISettingsService _settingsService;
        private readonly TouchpadProcessor _touchpadProcessor;
        private readonly IInputSimulator _inputSimulator;
        private readonly GestureRecognizer _gestureRecognizer;
        private readonly ILogger _logger;

        private bool _isTouchpadCurrentlyTouched = false;
        private double _lastTouchpadProcessedX = 0;
        private double _lastTouchpadProcessedY = 0;

        /// <summary>
        /// 用于平滑处理鼠标移动的X轴缓冲区。
        /// </summary>
        private double _smoothedDeltaX = 0;
        /// <summary>
        /// 用于平滑处理鼠标移动的Y轴缓冲区。
        /// </summary>
        private double _smoothedDeltaY = 0;
        /// <summary>
        /// 存储X轴位移历史，用于平滑计算。
        /// </summary>
        private List<double> _smoothingBufferX;
        /// <summary>
        /// 存储Y轴位移历史，用于平滑计算。
        /// </summary>
        private List<double> _smoothingBufferY;

        /// <summary>
        /// 当控制器数据被处理完成后触发的事件。
        /// </summary>
        public event EventHandler<ControllerData>? ControllerDataProcessed;

        /// <summary>
        /// ControllerService 的构造函数。
        /// 通过依赖注入接收所有必要的服务。
        /// </summary>
        /// <param name="bluetoothService">蓝牙服务，用于发送命令到控制器。</param>
        /// <param name="settingsService">设置服务，用于获取应用程序的各项配置。</param>
        /// <param name="touchpadProcessor">触摸板处理器，用于处理原始触摸板坐标。</param>
        /// <param name="inputSimulator">输入模拟器，用于模拟鼠标和键盘输入。</param>
        /// <param name="gestureRecognizer">手势识别器，用于识别触摸板手势。</param>
        /// <param name="logger">日志服务，用于记录日志。</param>
        public ControllerService(IBluetoothService bluetoothService, ISettingsService settingsService, TouchpadProcessor touchpadProcessor, IInputSimulator inputSimulator, GestureRecognizer gestureRecognizer, ILogger logger)
        {
            _bluetoothService = bluetoothService;
            _settingsService = settingsService;
            _touchpadProcessor = touchpadProcessor;
            _inputSimulator = inputSimulator;
            _gestureRecognizer = gestureRecognizer;
            _logger = logger;

            _smoothingBufferX = new List<double>();
            _smoothingBufferY = new List<double>();
        }

        // public Task InitializeAsync()
        // {
        //     // 初始化控制器
        //     return Task.CompletedTask;
        // }

        /// <summary>
        /// 异步发送命令到控制器。
        /// </summary>
        /// <param name="command">要发送的命令字节数组。</param>
        /// <param name="repeat">重复发送命令的次数（默认为1）。</param>
        /// <returns>表示异步操作的任务。</returns>
        public Task SendCommandAsync(byte[] command, int repeat = 1)
        {
            // 发送命令到控制器
            return _bluetoothService.SendDataAsync(command, repeat);
        }

        /// <summary>
        /// 处理从蓝牙服务接收到的原始控制器数据。
        /// 根据应用程序设置区分手势模式和相对模式，并进行相应的处理。
        /// </summary>
        /// <param name="data">接收到的控制器数据。</param>
        public void ProcessControllerData(ControllerData data)
        {
            if (data == null)
                return;

            // 验证数据有效性
            if (!ValidateControllerData(data))
                return;

            // 预处理数据（例如Y轴翻转，计算触摸状态）
            PreprocessControllerData(data);

            // 处理触摸板数据，区分手势模式和非手势模式 (相对模式)
            if (_settingsService.IsGestureMode)
            {
                // 手势模式：只将处理后的触摸板数据传递给手势识别器
                // 不进行鼠标移动，手势识别结果由 MainViewModel 处理离散动作
                var (processedX, processedY) = _touchpadProcessor.ProcessRawData(data.AxisX, data.AxisY);
                data.ProcessedTouchpadX = processedX;
                data.ProcessedTouchpadY = processedY;
                _gestureRecognizer.ProcessTouchpadPoint(new Models.TouchpadPoint { X = (float)processedX, Y = (float)processedY, IsTouched = data.TouchpadTouched });

                if (!data.TouchpadTouched && _isTouchpadCurrentlyTouched) // 触摸结束
                {
                    _isTouchpadCurrentlyTouched = false;
                    _logger.LogInfo("触摸结束 (手势模式).", nameof(ControllerService));
                }
                else if (data.TouchpadTouched && !_isTouchpadCurrentlyTouched) // 触摸开始
                {
                    _isTouchpadCurrentlyTouched = true;
                    _logger.LogInfo("触摸开始 (手势模式).", nameof(ControllerService));
                }
            }
            else // 非手势模式 (相对模式)：处理连续鼠标移动
            {
                ProcessRelativeModeTouchpad(data); // 调用重命名后的方法
            }

            // 触发数据处理完成事件 (无论模式如何，数据都传递出去)
            OnControllerDataProcessed(data);
        }

        /// <summary>
        /// 在相对模式下处理触摸板数据，模拟连续鼠标移动。
        /// 此方法包括死区、平滑和非线性曲线等鼠标移动增强功能。
        /// </summary>
        /// <param name="data">接收到的控制器数据，包含原始触摸板坐标。</param>
        private void ProcessRelativeModeTouchpad(ControllerData data)
        {
            // 使用 TouchpadProcessor 处理原始触摸板数据到归一化坐标 (-1 to 1)
            var (processedX, processedY) = _touchpadProcessor.ProcessRawData(data.AxisX, data.AxisY);
            data.ProcessedTouchpadX = processedX;
            data.ProcessedTouchpadY = processedY;

            // 将处理后的点传递给手势识别器（无论触摸状态如何，让它跟踪点），以便在触摸结束时能够识别离散手势
            _gestureRecognizer.ProcessTouchpadPoint(new Models.TouchpadPoint { X = (float)processedX, Y = (float)processedY, IsTouched = data.TouchpadTouched });

            if (data.TouchpadTouched)
            {
                if (!_isTouchpadCurrentlyTouched) // 触摸开始
                {
                    // 记录起始点，不立即移动鼠标，等待后续增量计算
                    _lastTouchpadProcessedX = processedX;
                    _lastTouchpadProcessedY = processedY;
                    _isTouchpadCurrentlyTouched = true;
                    _logger.LogInfo($"触摸开始: ({processedX:F2}, {processedY:F2}), _lastTouchpadProcessedX={_lastTouchpadProcessedX:F2}, _lastTouchpadProcessedY={_lastTouchpadProcessedY:F2}", nameof(ControllerService));
                }
                else // 触摸持续
                {
                    // 计算与上一个点的增量
                    double deltaX = processedX - _lastTouchpadProcessedX;
                    double deltaY = processedY - _lastTouchpadProcessedY;

                    // 应用死区：忽略小于死区阈值的微小移动
                    (deltaX, deltaY) = ApplyDeadZone(deltaX, deltaY);

                    // 应用平滑：减少鼠标抖动，使移动更流畅
                    if (_settingsService.EnableSmoothing)
                    {
                        _smoothedDeltaX = ApplySmoothing(deltaX, _smoothingBufferX, _settingsService.SmoothingLevel);
                        _smoothedDeltaY = ApplySmoothing(deltaY, _smoothingBufferY, _settingsService.SmoothingLevel);
                    }
                    else
                    {
                        _smoothedDeltaX = deltaX;
                        _smoothedDeltaY = deltaY;
                        _smoothingBufferX.Clear(); // 清空缓冲区以立即响应，不进行平滑
                        _smoothingBufferY.Clear();
                    }

                    // 更新上一个点，用于下一次增量计算
                    _lastTouchpadProcessedX = processedX;
                    _lastTouchpadProcessedY = processedY;

                    // 应用非线性曲线：调整鼠标加速行为，使小移动更精确，大移动更迅速
                    double finalDeltaX = ApplyNonLinearCurve(_smoothedDeltaX, _settingsService.NonLinearCurvePower, _settingsService.EnableNonLinearCurve);
                    double finalDeltaY = ApplyNonLinearCurve(_smoothedDeltaY, _settingsService.NonLinearCurvePower, _settingsService.EnableNonLinearCurve);

                    // 根据增量、灵敏度和缩放因子计算鼠标移动量
                    double mouseDeltaX = finalDeltaX * _settingsService.MouseSensitivity * _settingsService.MouseSensitivityScalingFactor;
                    double mouseDeltaY = finalDeltaY * _settingsService.MouseSensitivity * _settingsService.MouseSensitivityScalingFactor;

                    // System.Diagnostics.Debug.WriteLine($"[ControllerService] 触摸持续: rawX={{data.AxisX}}, rawY={{data.AxisY}}, processedX={{processedX:F2}}, processedY={{processedY:F2}}, deltaX={{deltaX:F2}}, deltaY={{deltaY:F2}}, smoothedX={{_smoothedDeltaX:F2}}, smoothedY={{_smoothedDeltaY:F2}}, finalX={{finalDeltaX:F2}}, finalY={{finalDeltaY:F2}}, mouseDeltaX={{mouseDeltaX:F2}}, mouseDeltaY={{mouseDeltaY:F2}}");

                    // 模拟鼠标移动，增加一个小的阈值，避免微小移动导致的抖动
                    if (Math.Abs(mouseDeltaX) > _settingsService.MoveThreshold || Math.Abs(mouseDeltaY) > _settingsService.MoveThreshold)
                    {
                        _inputSimulator.SimulateMouseMovement(mouseDeltaX, mouseDeltaY);
                        // System.Diagnostics.Debug.WriteLine($"[ControllerService] 模拟鼠标移动: DeltaX={{mouseDeltaX:F2}}, DeltaY={{mouseDeltaY:F2}}");
                    }
                }
            }
            else // 触摸结束
            {
                if (_isTouchpadCurrentlyTouched) // 刚抬起
                {
                    _isTouchpadCurrentlyTouched = false;
                    // 触摸结束时，GestureRecognizer 会根据接收到的点序列尝试识别离散手势
                    _logger.LogInfo("触摸结束 (相对模式).", nameof(ControllerService));
                }
                // 如果本来就没有触摸，则不做任何事
            }
        }

        /// <summary>
        /// 验证控制器数据的有效性。
        /// </summary>
        /// <param name="data">要验证的控制器数据。</param>
        /// <returns>如果数据有效则返回 true，否则返回 false。</returns>
        private bool ValidateControllerData(ControllerData data)
        {
            // 检查触摸板数据范围（使用原始值范围0-1023）
            if (data.AxisX < 0 || data.AxisX > 1023 || data.AxisY < 0 || data.AxisY > 1023)
            {
                _logger.LogWarning($"触摸板数据超出预期范围: X={data.AxisX}, Y={data.AxisY}", nameof(ControllerService));
                // 不返回false，继续处理，因为有时数据可能暂时超出范围，但仍需处理
                // 如果需要严格的过滤，这里可以返回false
            }

            // 检查加速度计数据 (目前未用于核心逻辑)
            // if (float.IsNaN(data.AccelX) || float.IsNaN(data.AccelY) || float.IsNaN(data.AccelZ))
            //     return false;

            // 检查陀螺仪数据 (目前未用于核心逻辑)
            // if (float.IsNaN(data.GyroX) || float.IsNaN(data.GyroY) || float.IsNaN(data.GyroZ))
            //     return false;

            return true;
        }

        /// <summary>
        /// 预处理控制器数据，包括更新时间戳、应用Y轴翻转和计算触摸状态。
        /// </summary>
        /// <param name="data">要预处理的控制器数据。</param>
        private void PreprocessControllerData(ControllerData data)
        {
            data.Timestamp = DateTimeOffset.Now.ToUnixTimeMilliseconds();

            // 应用Y轴翻转（如果启用）
            if (_settingsService.InvertYAxis)
            {
                // 在0-1023范围内翻转Y轴，使其行为符合用户的偏好
                data.AxisY = 1023 - data.AxisY;
            }

            // 获取处理后的触摸板坐标，用于更精确的触摸检测
            var (processedX, processedY) = _touchpadProcessor.ProcessRawData(data.AxisX, data.AxisY);
            data.ProcessedTouchpadX = processedX;
            data.ProcessedTouchpadY = processedY;

            // 处理触摸状态 - 更新判断逻辑，避免误判
            // 认为触摸板被触摸的条件：触摸板按钮被按下，或者处理后的触摸板X/Y坐标的绝对值超过了预设的归一化触摸阈值
            System.Diagnostics.Debug.WriteLine($"[ControllerService] PreprocessControllerData: Raw AxisX={{data.AxisX}}, AxisY={{data.AxisY}}, TouchpadButton={{data.TouchpadButton}}");
            data.TouchpadTouched = data.TouchpadButton ||
                                  (Math.Abs(processedX) > _settingsService.ProcessedTouchThreshold ||
                                   Math.Abs(processedY) > _settingsService.ProcessedTouchThreshold);
            System.Diagnostics.Debug.WriteLine($"[ControllerService] PreprocessControllerData: Processed TouchpadX={{processedX:F2}}, Processed TouchpadY={{processedY:F2}}, Calculated TouchpadTouched={{data.TouchpadTouched}}");

            // 处理按钮状态，判断是否有任何按钮被按下
            data.NoButton = !data.TriggerButton && !data.HomeButton &&
                           !data.BackButton && !data.TouchpadButton &&
                           !data.VolumeUpButton && !data.VolumeDownButton;
        }

        /// <summary>
        /// 处理滚轮移动，并根据"自然滚动"设置调整方向。
        /// </summary>
        /// <param name="delta">原始滚轮滚动量。</param>
        /// <returns>调整后的滚轮滚动量。</returns>
        public int ProcessWheelMovement(int delta)
        {
            // 如果启用了自然滚动，则反转滚动方向
            if (_settingsService.UseNaturalScrolling)
            {
                return -delta;
            }
            return delta;
        }

        /// <summary>
        /// 对鼠标移动增量应用死区。小于死区阈值的移动将被忽略。
        /// </summary>
        /// <param name="deltaX">X轴方向的原始位移增量。</param>
        /// <param name="deltaY">Y轴方向的原始位移增量。</param>
        /// <returns>应用死区后的位移增量。</returns>
        private (double, double) ApplyDeadZone(double deltaX, double deltaY)
        {
            double deadZoneThreshold = _settingsService.DeadZone / 100.0; // 将百分比转换为 [-1, 1] 范围的比例
            double magnitude = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);

            if (magnitude <= deadZoneThreshold)
            {
                return (0.0, 0.0);
            }
            return (deltaX, deltaY);
        }

        /// <summary>
        /// 对鼠标移动增量应用平滑处理，以减少抖动并使移动更流畅。
        /// </summary>
        /// <param name="newValue">当前的位移增量。</param>
        /// <param name="buffer">用于平滑计算的缓冲区。</param>
        /// <param name="smoothingLevel">平滑等级，决定缓冲区的长度。</param>
        /// <returns>平滑处理后的位移增量。</returns>
        private double ApplySmoothing(double newValue, List<double> buffer, int smoothingLevel)
        {
            buffer.Add(newValue);
            while (buffer.Count > smoothingLevel)
            {
                buffer.RemoveAt(0);
            }
            return buffer.Average();
        }

        /// <summary>
        /// 对鼠标移动增量应用非线性曲线，以实现加速或减速效果。
        /// </summary>
        /// <param name="value">原始位移增量。</param>
        /// <param name="power">非线性曲线的幂次。</param>
        /// <param name="enableNonLinearCurve">是否启用非线性曲线。</param>
        /// <returns>应用非线性曲线后的位移增量。</returns>
        private double ApplyNonLinearCurve(double value, double power, bool enableNonLinearCurve)
        {
            if (!enableNonLinearCurve || power == 1.0)
            {
                return value;
            }

            // 应用非线性曲线： sign(value) * abs(value)^power
            return Math.Sign(value) * Math.Pow(Math.Abs(value), power);
        }

        /// <summary>
        /// 触发 ControllerDataProcessed 事件。
        /// </summary>
        /// <param name="data">已处理的控制器数据。</param>
        protected virtual void OnControllerDataProcessed(ControllerData data)
        {
            ControllerDataProcessed?.Invoke(this, data);
        }
    }
}