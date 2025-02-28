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
            private set
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
            if (_calibrationData == null)
                return (rawX, rawY);

            // 计算相对于中心点的偏移
            double deltaX = rawX - _calibrationData.CenterX;
            double deltaY = rawY - _calibrationData.CenterY;

            // 应用死区
            const double DEAD_ZONE = 5.0; // 可以根据需要调整
            if (Math.Abs(deltaX) < DEAD_ZONE)
                deltaX = 0;
            if (Math.Abs(deltaY) < DEAD_ZONE)
                deltaY = 0;

            // 如果在死区内，直接返回
            if (deltaX == 0 && deltaY == 0)
                return (0, 0);

            // 计算归一化系数
            double xScale = deltaX > 0 ?
                (_calibrationData.MaxX - _calibrationData.CenterX) :
                (_calibrationData.CenterX - _calibrationData.MinX);

            double yScale = deltaY > 0 ?
                (_calibrationData.MaxY - _calibrationData.CenterY) :
                (_calibrationData.CenterY - _calibrationData.MinY);

            // 归一化坐标
            double normalizedX = deltaX / xScale;
            double normalizedY = deltaY / yScale;

            // 应用灵敏度
            normalizedX *= _mouseSensitivity;
            normalizedY *= _mouseSensitivity;

            return (normalizedX, normalizedY);
        }

        private void ProcessTouchpadMovement(ControllerData data)
        {
            if (!_isMouseEnabled || !_isControlEnabled || _isCalibrating)
                return;

            // 应用校准
            var (calibratedX, calibratedY) = ApplyCalibration(data.AxisX, data.AxisY);

            // 应用平滑处理
            var (smoothX, smoothY) = SmoothMovement(calibratedX, calibratedY);

            // 计算最终的鼠标移动
            const double MOVEMENT_SCALE = 5.0; // 可以根据需要调整
            int finalDeltaX = (int)(smoothX * MOVEMENT_SCALE);
            int finalDeltaY = (int)(smoothY * MOVEMENT_SCALE);

            // 检查是否需要移动鼠标
            if (finalDeltaX != 0 || finalDeltaY != 0)
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
            if (magnitude < 1) // 如果移动太小，不计入
            {
                return false;
            }

            // 归一化方向向量
            double normalizedX = deltaX / magnitude;
            double normalizedY = deltaY / magnitude;

            // 检查是否向左上方移动（大约-135度方向）
            // 理想的左上方向量是 (-0.707, -0.707)
            if (normalizedX < -0.707 - DIRECTION_TOLERANCE || normalizedX > -0.707 + DIRECTION_TOLERANCE ||
                normalizedY < -0.707 - DIRECTION_TOLERANCE || normalizedY > -0.707 + DIRECTION_TOLERANCE)
            {
                // 如果不是向左上方移动，重置计数
                _abnormalMovementCount = 0;
                return false;
            }

            // 更新计数和时间戳
            _abnormalMovementCount++;
            _lastAbnormalMovementTime = DateTime.Now;

            // 如果连续向左上角移动次数超过阈值，返回true
            return _abnormalMovementCount >= ABNORMAL_MOVEMENT_THRESHOLD;
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
    }
}