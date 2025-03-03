using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services;
using GearVRController.Services.Interfaces;
using GearVRController.Helpers;
using Microsoft.UI.Xaml;
using Microsoft.UI.Dispatching;
using Windows.Devices.Bluetooth;
using System.Collections.Generic;
using System.Linq;
using EnumsNS = GearVRController.Enums; // 添加命名空间别名

namespace GearVRController.ViewModels
{
    public class MainViewModel : INotifyPropertyChanged
    {
        private readonly IBluetoothService _bluetoothService;
        private readonly IControllerService _controllerService;
        private readonly IInputSimulator _inputSimulator;
        private readonly ISettingsService _settingsService;
        private readonly DispatcherQueue _dispatcherQueue;
        private bool _isConnected;
        private string _statusMessage = string.Empty;
        private ControllerData _lastControllerData = new ControllerData();
        private const int NumberOfWheelPositions = 64;

        // 添加触摸板校准数据
        private TouchpadCalibrationData? _calibrationData;

        // 添加新的属性
        private bool _isControlEnabled = true;
        private double _mouseSensitivity = 1.0;
        private bool _isMouseEnabled = true;
        private bool _isKeyboardEnabled = true;

        // 添加异常移动检测相关字段
        private const int ABNORMAL_MOVEMENT_THRESHOLD = 10; // 连续向左上角移动的次数阈值
        private const double DIRECTION_TOLERANCE = 0.3; // 判定为左上角移动的方向容差
        private int _abnormalMovementCount = 0;
        private bool _isAutoCalibrating = false;
        private DateTime _lastAbnormalMovementTime = DateTime.MinValue;
        private const int RESET_INTERVAL_SECONDS = 5; // 重置异常计数的时间间隔

        private bool _isCalibrating = false;
        private bool _isConnecting = false;

        private const int MOVEMENT_BUFFER_SIZE = 3;
        private Queue<(double X, double Y)> _movementBuffer = new Queue<(double X, double Y)>();
        private DateTime _lastMovementTime = DateTime.MinValue;
        private const double MOVEMENT_TIMEOUT_MS = 100; // 100ms timeout for movement buffer

        // 在类的字段部分添加触摸板可视化相关的字段
        private EnumsNS.TouchpadGesture _lastGesture = EnumsNS.TouchpadGesture.None; // 使用命名空间别名
        private readonly List<TouchpadPoint> _touchpadHistory = new List<TouchpadPoint>();
        private const int MAX_TOUCHPAD_HISTORY = 50;

        public event PropertyChangedEventHandler? PropertyChanged;
        public event EventHandler<ControllerData>? ControllerDataReceived;
        public event EventHandler? AutoCalibrationRequired;

        public bool IsControlEnabled
        {
            get => _isControlEnabled;
            set
            {
                if (_isControlEnabled != value)
                {
                    _isControlEnabled = value;
                    OnPropertyChanged();
                }
            }
        }

        public double MouseSensitivity
        {
            get => _mouseSensitivity;
            set
            {
                if (_mouseSensitivity != value)
                {
                    _mouseSensitivity = Math.Max(0.1, Math.Min(value, 2.0)); // 限制灵敏度范围在0.1到2.0之间
                    OnPropertyChanged();
                }
            }
        }

        public bool IsMouseEnabled
        {
            get => _isMouseEnabled;
            set
            {
                if (_isMouseEnabled != value)
                {
                    _isMouseEnabled = value;
                    OnPropertyChanged();
                }
            }
        }

        public bool IsKeyboardEnabled
        {
            get => _isKeyboardEnabled;
            set
            {
                if (_isKeyboardEnabled != value)
                {
                    _isKeyboardEnabled = value;
                    OnPropertyChanged();
                }
            }
        }

        public bool IsConnected
        {
            get => _isConnected;
            private set
            {
                if (_isConnected != value)
                {
                    _isConnected = value;
                    OnPropertyChanged();
                }
            }
        }

        public string StatusMessage
        {
            get => _statusMessage;
            set
            {
                if (_statusMessage != value)
                {
                    _statusMessage = value;
                    OnPropertyChanged();
                }
            }
        }

        public ControllerData LastControllerData
        {
            get => _lastControllerData;
            private set
            {
                if (_lastControllerData != value)
                {
                    _lastControllerData = value;
                    OnPropertyChanged();
                }
            }
        }

        public bool IsCalibrating
        {
            get => _isCalibrating;
            private set
            {
                if (_isCalibrating != value)
                {
                    _isCalibrating = value;
                    OnPropertyChanged();
                    // 当校准状态改变时，更新控制状态
                    UpdateControlState();
                }
            }
        }

        public bool IsConnecting
        {
            get => _isConnecting;
            private set
            {
                if (_isConnecting != value)
                {
                    _isConnecting = value;
                    OnPropertyChanged();
                    // 当连接状态改变时，更新控制状态
                    UpdateControlState();
                }
            }
        }

        public EnumsNS.TouchpadGesture LastGesture // 使用命名空间别名
        {
            get => _lastGesture;
            private set
            {
                if (_lastGesture != value)
                {
                    _lastGesture = value;
                    OnPropertyChanged();
                }
            }
        }

        public IReadOnlyList<TouchpadPoint> TouchpadHistory => _touchpadHistory.AsReadOnly();

        // 提供一个公共属性来访问校准数据
        public TouchpadCalibrationData? CalibrationData => _calibrationData;

        public MainViewModel(
            IBluetoothService bluetoothService,
            IControllerService controllerService,
            IInputSimulator inputSimulator,
            ISettingsService settingsService,
            DispatcherQueue dispatcherQueue)
        {
            _bluetoothService = bluetoothService;
            _controllerService = controllerService;
            _inputSimulator = inputSimulator;
            _settingsService = settingsService;
            _dispatcherQueue = dispatcherQueue;

            // 订阅事件
            _bluetoothService.DataReceived += BluetoothService_DataReceived;
            _bluetoothService.ConnectionStatusChanged += BluetoothService_ConnectionStatusChanged;

            // 加载设置
            LoadSettings();
        }

        private async void LoadSettings()
        {
            await _settingsService.LoadSettingsAsync();
            MouseSensitivity = _settingsService.MouseSensitivity;
            IsMouseEnabled = _settingsService.IsMouseEnabled;
            IsKeyboardEnabled = _settingsService.IsKeyboardEnabled;
            IsControlEnabled = _settingsService.IsControlEnabled;
        }

        private void RegisterHotKeys()
        {
            // 这里可以添加热键注册逻辑
            // 由于WinUI 3的限制，我们可能需要使用原生Windows API来实现
            // 暂时可以通过UI按钮来控制
        }

        public async Task ConnectAsync(ulong deviceAddress)
        {
            if (_bluetoothService.IsConnected) return;

            IsConnecting = true;
            StatusMessage = "正在连接...";

            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
                await _controllerService.InitializeAsync();
            }
            catch (Exception ex)
            {
                StatusMessage = $"连接失败: {ex.Message}";
            }
            finally
            {
                IsConnecting = false;
            }
        }

        public void Disconnect()
        {
            _bluetoothService.Disconnect();
            StatusMessage = "已断开连接";
        }

        private void BluetoothService_ConnectionStatusChanged(object? sender, BluetoothConnectionStatus status)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                IsConnected = status == BluetoothConnectionStatus.Connected;
                StatusMessage = IsConnected ? "已连接" : "已断开连接";
                UpdateControlState();
            });
        }

        private void BluetoothService_DataReceived(object? sender, ControllerData data)
        {
            if (data == null) return;

            _dispatcherQueue.TryEnqueue(() =>
            {
                // 触发控制器数据接收事件
                ControllerDataReceived?.Invoke(this, data);

                // 如果正在校准或控制被禁用，直接返回
                if (IsCalibrating || !IsControlEnabled) return;

                LastControllerData = data;

                // 处理触摸板输入
                ProcessTouchpadMovement(data);

                // 处理按钮输入
                HandleButtonInput(data);
            });
        }

        private (double X, double Y) SmoothMovement(double x, double y)
        {
            var now = DateTime.Now;

            // 如果距离上次移动时间太长，清空缓冲区
            if ((now - _lastMovementTime).TotalMilliseconds > MOVEMENT_TIMEOUT_MS)
            {
                _movementBuffer.Clear();
            }

            _movementBuffer.Enqueue((x, y));
            _lastMovementTime = now;

            if (_movementBuffer.Count > MOVEMENT_BUFFER_SIZE)
            {
                _movementBuffer.Dequeue();
            }

            // 计算平均移动
            double avgX = _movementBuffer.Average(m => m.X);
            double avgY = _movementBuffer.Average(m => m.Y);

            return (avgX, avgY);
        }

        private (double X, double Y) ApplyCalibration(int rawX, int rawY)
        {
            // 记录原始数据以便观察实际范围
            System.Diagnostics.Debug.WriteLine($"[校准前] 触摸板原始值: X={rawX}, Y={rawY}");
            
            // 如果原始值都很小，直接返回零移动
            const int NOISE_THRESHOLD = 5; // 认为是噪音的阈值
            if (Math.Abs(rawX) <= NOISE_THRESHOLD && Math.Abs(rawY) <= NOISE_THRESHOLD)
            {
                System.Diagnostics.Debug.WriteLine("[校准] 值太小，被认为是噪声，返回零移动");
                return (0, 0);
            }
            
            // 定义触摸板中心点坐标
            const double CENTER_X = 157.5; // 触摸板中心X坐标 (315/2)
            const double CENTER_Y = 157.5; // 触摸板中心Y坐标 (315/2)
            const double MAX_RADIUS = 157.5; // 从中心到边缘的最大半径
            
            if (_calibrationData == null)
            {
                // 如果没有校准数据，基于中心点(157.5, 157.5)计算归一化值(-1到1)
                double deltaX = rawX - CENTER_X;
                double deltaY = rawY - CENTER_Y;
                
                // 归一化到[-1,1]范围，同时反转Y轴使之符合标准坐标系（向上为正）
                double rawNormalizedX = Math.Max(-1.0, Math.Min(1.0, deltaX / MAX_RADIUS));
                double rawNormalizedY = Math.Max(-1.0, Math.Min(1.0, -deltaY / MAX_RADIUS)); // Y轴翻转
                
                System.Diagnostics.Debug.WriteLine($"[未校准归一化] 中心点偏移: ({deltaX:F2}, {deltaY:F2}) => 归一化: ({rawNormalizedX:F2}, {rawNormalizedY:F2})");
                return (rawNormalizedX, rawNormalizedY);
            }

            // 使用校准中心点
            // 计算相对于中心点的偏移
            double calibratedDeltaX = rawX - _calibrationData.CenterX;
            double calibratedDeltaY = rawY - _calibrationData.CenterY;

            // 应用死区
            const double DEAD_ZONE = 8.0; // 增加死区范围，减少抖动
            if (Math.Abs(calibratedDeltaX) < DEAD_ZONE)
                calibratedDeltaX = 0;
            if (Math.Abs(calibratedDeltaY) < DEAD_ZONE)
                calibratedDeltaY = 0;

            // 如果在死区内，直接返回
            if (calibratedDeltaX == 0 && calibratedDeltaY == 0)
                return (0, 0);

            // 计算归一化系数，确保不会除以零
            double xScale = calibratedDeltaX > 0 ?
                Math.Max(10, _calibrationData.MaxX - _calibrationData.CenterX) :
                Math.Max(10, _calibrationData.CenterX - _calibrationData.MinX);

            double yScale = calibratedDeltaY > 0 ?
                Math.Max(10, _calibrationData.MaxY - _calibrationData.CenterY) :
                Math.Max(10, _calibrationData.CenterY - _calibrationData.MinY);

            // 归一化坐标，限制在[-1, 1]范围内
            double normalizedX = Math.Max(-1.0, Math.Min(1.0, calibratedDeltaX / xScale));
            double normalizedY = Math.Max(-1.0, Math.Min(1.0, -calibratedDeltaY / yScale)); // 注意Y轴反转

            // 应用非线性曲线，使小幅度移动更精确
            normalizedX = Math.Sign(normalizedX) * Math.Pow(Math.Abs(normalizedX), 1.5);
            normalizedY = Math.Sign(normalizedY) * Math.Pow(Math.Abs(normalizedY), 1.5);

            // 应用灵敏度
            normalizedX *= _mouseSensitivity;
            normalizedY *= _mouseSensitivity;
            
            System.Diagnostics.Debug.WriteLine($"[校准后] X={normalizedX:F2}, Y={normalizedY:F2}");

            return (normalizedX, normalizedY);
        }

        private void ProcessTouchpadMovement(ControllerData data)
        {
            if (!_isMouseEnabled || !_isControlEnabled || _isCalibrating)
                return;
                
            // 如果没有触摸触摸板，不处理鼠标移动
            if (!data.TouchpadTouched)
            {
                // 记录一个零移动的历史，但标记为未触摸
                RecordTouchpadHistory(0, 0, false);
                return;
            }

            // 应用校准
            var (calibratedX, calibratedY) = ApplyCalibration(data.AxisX, data.AxisY);

            // 应用平滑处理
            var (smoothX, smoothY) = SmoothMovement(calibratedX, calibratedY);

            // 计算最终的鼠标移动
            const double MOVEMENT_SCALE = 4.0; // 降低移动比例，使控制更精确
            
            // 应用自适应缩放 - 小幅度移动更精确，大幅度移动更快
            double adaptiveScale = MOVEMENT_SCALE * (0.5 + Math.Min(1.0, Math.Sqrt(smoothX * smoothX + smoothY * smoothY)));
            
            int finalDeltaX = (int)(smoothX * adaptiveScale);
            int finalDeltaY = (int)(smoothY * adaptiveScale);

            // 记录触摸板历史数据
            RecordTouchpadHistory(smoothX, smoothY, data.TouchpadButton);

            // 检查异常移动
            if (CheckAbnormalMovement(finalDeltaX, finalDeltaY))
            {
                TriggerAutoCalibration();
                return;
            }
            
            // 再次确认移动不是因为小数据误差
            if (Math.Abs(finalDeltaX) > 0 || Math.Abs(finalDeltaY) > 0)
            {
                _inputSimulator.SimulateMouseMovement(finalDeltaX, finalDeltaY);
            }
        }

        private void HandleButtonInput(ControllerData data)
        {
            if (!IsControlEnabled || _isCalibrating) return;

            var inputSimulator = (WindowsInputSimulator)_inputSimulator;

            if (IsMouseEnabled)
            {
                if (data.TriggerButton)
                {
                    // 扳机键作为右键
                    inputSimulator.SimulateMouseButtonEx(true, WindowsInputSimulator.MouseButtons.Right);
                }
                else
                {
                    inputSimulator.SimulateMouseButtonEx(false, WindowsInputSimulator.MouseButtons.Right);
                }

                // 触摸板点击作为左键
                inputSimulator.SimulateMouseButton(data.TouchpadButton);
            }

            if (IsKeyboardEnabled)
            {
                if (data.HomeButton)
                {
                    inputSimulator.SimulateKeyPress(WindowsInputSimulator.VirtualKeys.VK_HOME);
                }

                if (data.BackButton)
                {
                    inputSimulator.SimulateKeyPress(WindowsInputSimulator.VirtualKeys.VK_BACK);
                }

                if (data.VolumeUpButton)
                {
                    inputSimulator.SimulateKeyPress(WindowsInputSimulator.VirtualKeys.VK_VOLUME_UP);
                }

                if (data.VolumeDownButton)
                {
                    inputSimulator.SimulateKeyPress(WindowsInputSimulator.VirtualKeys.VK_VOLUME_DOWN);
                }
            }
        }

        private int CalculateWheelPosition(int x, int y)
        {
            if (x == 0 && y == 0) return -1;
            double angle = Math.Atan2(y, x);
            return (int)Math.Floor((angle + Math.PI) / (2 * Math.PI) * NumberOfWheelPositions);
        }

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

        public void ToggleControl()
        {
            _isControlEnabled = !_isControlEnabled;
            UpdateControlState();
            StatusMessage = _isControlEnabled ? "控制已启用" : "控制已禁用";
        }

        public void ResetSettings()
        {
            _settingsService.ResetToDefaults();
            MouseSensitivity = _settingsService.MouseSensitivity;
            IsMouseEnabled = _settingsService.IsMouseEnabled;
            IsKeyboardEnabled = _settingsService.IsKeyboardEnabled;
            IsControlEnabled = _settingsService.IsControlEnabled;
        }

        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            StatusMessage = "已应用触摸板校准数据";
        }

        private bool CheckAbnormalMovement(double deltaX, double deltaY)
        {
            // 检查是否需要重置计数器
            if ((DateTime.Now - _lastAbnormalMovementTime).TotalSeconds > RESET_INTERVAL_SECONDS)
            {
                _abnormalMovementCount = 0;
            }

            // 计算移动方向
            double magnitude = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);
            if (magnitude < 3) // 增加阈值，忽略小幅度移动
            {
                return false;
            }

            // 归一化方向向量
            double normalizedX = deltaX / magnitude;
            double normalizedY = deltaY / magnitude;

            // 检查是否向左上方移动（大约-135度方向）
            // 理想的左上方向量是 (-0.707, -0.707)
            // 增加容差范围，减少误判
            double tolerance = DIRECTION_TOLERANCE * 1.5; // 增加50%的容差
            if (normalizedX < -0.707 - tolerance || normalizedX > -0.707 + tolerance ||
                normalizedY < -0.707 - tolerance || normalizedY > -0.707 + tolerance)
            {
                // 如果不是向左上方移动，重置计数
                _abnormalMovementCount = 0;
                return false;
            }

            // 更新计数和时间戳
            _abnormalMovementCount++;
            _lastAbnormalMovementTime = DateTime.Now;

            // 增加阈值，需要更多次数的异常移动才触发校准
            return _abnormalMovementCount >= (ABNORMAL_MOVEMENT_THRESHOLD + 5);
        }

        private void TriggerAutoCalibration()
        {
            if (_isAutoCalibrating)
            {
                return;
            }

            _isAutoCalibrating = true;
            StatusMessage = "检测到异常移动，正在启动自动校准...";
            AutoCalibrationRequired?.Invoke(this, EventArgs.Empty);
        }

        public void StartAutoCalibration()
        {
            IsCalibrating = true;
            _isAutoCalibrating = true;
            StatusMessage = "自动校准已启动，请在触摸板边缘划圈...";
        }

        public void StartManualCalibration()
        {
            IsCalibrating = true;
            StatusMessage = "手动校准已启动，请在触摸板边缘划圈...";
        }

        public void EndCalibration()
        {
            IsCalibrating = false;
            _isAutoCalibrating = false;
            _abnormalMovementCount = 0;
            StatusMessage = "校准完成";
        }

        private void UpdateControlState()
        {
            // 在以下情况下禁用控制：
            // 1. 正在校准
            // 2. 正在连接
            // 3. 未连接
            // 4. 用户手动禁用
            bool shouldBeEnabled = !_isCalibrating &&
                                 !_isConnecting &&
                                 _isConnected &&
                                 _isControlEnabled;

            // 更新鼠标和键盘控制状态
            IsMouseEnabled = shouldBeEnabled && _isMouseEnabled;
            IsKeyboardEnabled = shouldBeEnabled && _isKeyboardEnabled;
        }

        // 添加记录触摸板历史的方法
        private void RecordTouchpadHistory(double x, double y, bool isPressed)
        {
            if (x == 0 && y == 0 && !isPressed)
                return; // 忽略无意义的数据
            
            var point = new TouchpadPoint(x, y, isPressed);
            _touchpadHistory.Add(point);
            
            // 维持最大历史点数
            while (_touchpadHistory.Count > MAX_TOUCHPAD_HISTORY)
            {
                _touchpadHistory.RemoveAt(0);
            }
            
            // 检测手势
            DetectGesture();
        }

        // 添加手势检测方法
        private void DetectGesture()
        {
            if (_touchpadHistory.Count < 10)
            {
                LastGesture = EnumsNS.TouchpadGesture.None; // 使用命名空间别名
                return;
            }
            
            // 获取最近的10个点
            var recentPoints = _touchpadHistory.Skip(_touchpadHistory.Count - 10).ToList();
            
            // 计算起点和终点
            var startPoint = recentPoints.First();
            var endPoint = recentPoints.Last();
            
            // 计算移动距离
            double deltaX = endPoint.X - startPoint.X;
            double deltaY = endPoint.Y - startPoint.Y;
            double distance = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);
            
            // 如果移动太小，不认为是手势
            if (distance < 0.3)
            {
                LastGesture = EnumsNS.TouchpadGesture.None; // 使用命名空间别名
                return;
            }
            
            // 根据移动方向判断手势
            double angle = Math.Atan2(deltaY, deltaX);
            
            // 角度转换为度数
            double degrees = angle * 180 / Math.PI;
            
            // 调整为0-360度
            if (degrees < 0)
                degrees += 360;
            
            // 根据角度判断方向
            if (degrees >= 315 || degrees < 45)
            {
                LastGesture = EnumsNS.TouchpadGesture.SwipeRight; // 使用命名空间别名
            }
            else if (degrees >= 45 && degrees < 135)
            {
                LastGesture = EnumsNS.TouchpadGesture.SwipeDown; // 使用命名空间别名
            }
            else if (degrees >= 135 && degrees < 225)
            {
                LastGesture = EnumsNS.TouchpadGesture.SwipeLeft; // 使用命名空间别名
            }
            else // degrees >= 225 && degrees < 315
            {
                LastGesture = EnumsNS.TouchpadGesture.SwipeUp; // 使用命名空间别名
            }
        }

        // 添加清除触摸板历史的方法
        public void ClearTouchpadHistory()
        {
            _touchpadHistory.Clear();
            LastGesture = EnumsNS.TouchpadGesture.None; // 使用命名空间别名
        }
    }
}