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
                HandleTouchpadInput(data);

                // 处理按钮输入
                HandleButtonInput(data);
            });
        }

        private (double deltaX, double deltaY) CalculateTouchpadDelta(ControllerData data)
        {
            const int deadZone = 10;
            int centerX = _calibrationData?.CenterX ?? 512;
            int centerY = _calibrationData?.CenterY ?? 512;

            double deltaX = (data.AxisX - centerX);
            double deltaY = (data.AxisY - centerY);

            // 应用死区
            if (Math.Abs(deltaX) < deadZone) deltaX = 0;
            if (Math.Abs(deltaY) < deadZone) deltaY = 0;

            // 应用校准数据
            if (_calibrationData != null)
            {
                double rangeX = _calibrationData.MaxX - _calibrationData.MinX;
                double rangeY = _calibrationData.MaxY - _calibrationData.MinY;

                deltaX = deltaX / (rangeX / 2);
                deltaY = deltaY / (rangeY / 2);

                // 应用方向校准
                if (Math.Abs(deltaX) > 0 || Math.Abs(deltaY) > 0)
                {
                    double magnitude = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);
                    double normalizedX = deltaX / magnitude;
                    double normalizedY = deltaY / magnitude;

                    // 应用校准方向
                    ApplyDirectionalCalibration(ref normalizedX, ref normalizedY);

                    deltaX = normalizedX * magnitude;
                    deltaY = normalizedY * magnitude;
                }
            }

            // 应用平滑和灵敏度
            if (deltaX != 0 || deltaY != 0)
            {
                deltaX = Math.Sign(deltaX) * Math.Pow(Math.Abs(deltaX), 1.5) * 20;
                deltaY = Math.Sign(deltaY) * Math.Pow(Math.Abs(deltaY), 1.5) * 20;
            }

            return (deltaX, deltaY);
        }

        private void ApplyDirectionalCalibration(ref double normalizedX, ref double normalizedY)
        {
            if (_calibrationData == null) return;

            // 计算与各个方向向量的点积
            double upDot = normalizedX * _calibrationData.UpDirection.AverageX +
                         normalizedY * _calibrationData.UpDirection.AverageY;
            double downDot = normalizedX * _calibrationData.DownDirection.AverageX +
                           normalizedY * _calibrationData.DownDirection.AverageY;
            double leftDot = normalizedX * _calibrationData.LeftDirection.AverageX +
                           normalizedY * _calibrationData.LeftDirection.AverageY;
            double rightDot = normalizedX * _calibrationData.RightDirection.AverageX +
                            normalizedY * _calibrationData.RightDirection.AverageY;

            const double directionThreshold = 0.7;

            // 应用方向校准
            if (Math.Abs(leftDot) > Math.Abs(rightDot) && Math.Abs(leftDot) > directionThreshold)
            {
                normalizedX = -1;
                normalizedY = 0;
            }
            else if (Math.Abs(rightDot) > Math.Abs(leftDot) && Math.Abs(rightDot) > directionThreshold)
            {
                normalizedX = 1;
                normalizedY = 0;
            }

            if (Math.Abs(upDot) > Math.Abs(downDot) && Math.Abs(upDot) > directionThreshold)
            {
                normalizedY = -1;
                if (Math.Abs(leftDot) <= directionThreshold && Math.Abs(rightDot) <= directionThreshold)
                {
                    normalizedX = 0;
                }
            }
            else if (Math.Abs(downDot) > Math.Abs(upDot) && Math.Abs(downDot) > directionThreshold)
            {
                normalizedY = 1;
                if (Math.Abs(leftDot) <= directionThreshold && Math.Abs(rightDot) <= directionThreshold)
                {
                    normalizedX = 0;
                }
            }
        }

        private void HandleTouchpadInput(ControllerData data)
        {
            if (!IsControlEnabled || _isCalibrating) return;

            if (IsMouseEnabled && data.TouchpadTouched)
            {
                var (deltaX, deltaY) = CalculateTouchpadDelta(data);
                _inputSimulator.SimulateMouseMovement(deltaX * MouseSensitivity, deltaY * MouseSensitivity);
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