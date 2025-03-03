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
        private bool _useNaturalScrolling = false;
        private bool _invertYAxis = false;
        private bool _enableAutoCalibration = true;
        private bool _enableSmoothing = true;
        private int _smoothingLevel = 3;
        private bool _enableNonLinearCurve = true;
        private double _nonLinearCurvePower = 1.5;
        private double _deadZone = 8.0;

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

        // 添加蓝牙连接相关字段
        private const int CONNECTION_CHECK_INTERVAL_MS = 5000; // 每5秒检查一次连接状态
        private const int MAX_RECONNECT_ATTEMPTS = 3; // 最大重连次数
        private const int RECONNECT_DELAY_MS = 2000; // 重连延迟时间
        private DispatcherTimer? _connectionCheckTimer;
        private ulong _lastConnectedDeviceAddress;
        private int _reconnectAttempts;
        private bool _isReconnecting;
        private DateTime _lastConnectionAttempt = DateTime.MinValue;

        // 添加按键状态监控字段
        private bool _leftButtonPressed = false;
        private bool _rightButtonPressed = false;
        private DateTime _lastInputTime = DateTime.MinValue;
        private const int INPUT_TIMEOUT_MS = 5000; // 5秒超时
        private DispatcherTimer? _stateCheckTimer;

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

        public bool UseNaturalScrolling
        {
            get => _useNaturalScrolling;
            set
            {
                if (_useNaturalScrolling != value)
                {
                    _useNaturalScrolling = value;
                    OnPropertyChanged();
                }
            }
        }

        public bool InvertYAxis
        {
            get => _invertYAxis;
            set
            {
                if (_invertYAxis != value)
                {
                    _invertYAxis = value;
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

        public bool EnableAutoCalibration
        {
            get => _enableAutoCalibration;
            set
            {
                if (_enableAutoCalibration != value)
                {
                    _enableAutoCalibration = value;
                    OnPropertyChanged();
                    
                    // 如果禁用自动校准，重置相关计数器
                    if (!value)
                    {
                        _abnormalMovementCount = 0;
                        _isAutoCalibrating = false;
                    }
                }
            }
        }

        public bool EnableSmoothing
        {
            get => _enableSmoothing;
            set
            {
                if (_enableSmoothing != value)
                {
                    _enableSmoothing = value;
                    OnPropertyChanged();
                    if (!value)
                    {
                        _movementBuffer.Clear();
                    }
                }
            }
        }

        public int SmoothingLevel
        {
            get => _smoothingLevel;
            set
            {
                if (_smoothingLevel != value)
                {
                    _smoothingLevel = Math.Max(1, Math.Min(value, 10));
                    OnPropertyChanged();
                    _movementBuffer.Clear(); // 清空缓冲区以适应新的平滑等级
                }
            }
        }

        public bool EnableNonLinearCurve
        {
            get => _enableNonLinearCurve;
            set
            {
                if (_enableNonLinearCurve != value)
                {
                    _enableNonLinearCurve = value;
                    OnPropertyChanged();
                }
            }
        }

        public double NonLinearCurvePower
        {
            get => _nonLinearCurvePower;
            set
            {
                if (_nonLinearCurvePower != value)
                {
                    _nonLinearCurvePower = Math.Max(1.0, Math.Min(value, 3.0));
                    OnPropertyChanged();
                }
            }
        }

        public double DeadZone
        {
            get => _deadZone;
            set
            {
                if (_deadZone != value)
                {
                    _deadZone = Math.Max(0.0, Math.Min(value, 20.0));
                    OnPropertyChanged();
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

            // 初始化连接状态检查定时器
            InitializeConnectionCheck();
            
            // 初始化状态检查定时器
            InitializeStateCheck();

            // 加载设置
            LoadSettings();
        }

        private void InitializeConnectionCheck()
        {
            _connectionCheckTimer = new DispatcherTimer();
            _connectionCheckTimer.Interval = TimeSpan.FromMilliseconds(CONNECTION_CHECK_INTERVAL_MS);
            _connectionCheckTimer.Tick += ConnectionCheckTimer_Tick;
            _connectionCheckTimer.Start();
        }

        private async void ConnectionCheckTimer_Tick(object? sender, object e)
        {
            if (!_isConnected && !_isConnecting && !_isReconnecting && _lastConnectedDeviceAddress != 0)
            {
                // 检查是否需要等待重连延迟
                if ((DateTime.Now - _lastConnectionAttempt).TotalMilliseconds < RECONNECT_DELAY_MS)
                {
                    return;
                }

                // 检查重连次数是否超过限制
                if (_reconnectAttempts >= MAX_RECONNECT_ATTEMPTS)
                {
                    StatusMessage = "重连次数超过限制，请手动重新连接";
                    _connectionCheckTimer?.Stop();
                    return;
                }

                await TryReconnectAsync();
            }
        }

        private async Task TryReconnectAsync()
        {
            if (_isReconnecting) return;

            _isReconnecting = true;
            _reconnectAttempts++;
            _lastConnectionAttempt = DateTime.Now;

            try
            {
                StatusMessage = $"正在尝试重新连接... (第 {_reconnectAttempts} 次)";
                await ConnectAsync(_lastConnectedDeviceAddress);
            }
            catch (Exception ex)
            {
                StatusMessage = $"重连失败: {ex.Message}";
                System.Diagnostics.Debug.WriteLine($"重连失败: {ex}");
            }
            finally
            {
                _isReconnecting = false;
            }
        }

        private void InitializeStateCheck()
        {
            _stateCheckTimer = new DispatcherTimer();
            _stateCheckTimer.Interval = TimeSpan.FromSeconds(1);
            _stateCheckTimer.Tick += (s, e) =>
            {
                MonitorInputState();
            };
            _stateCheckTimer.Start();
        }

        private void MonitorInputState()
        {
            try
            {
                var now = DateTime.Now;
                if (_leftButtonPressed || _rightButtonPressed)
                {
                    if ((now - _lastInputTime).TotalMilliseconds > INPUT_TIMEOUT_MS)
                    {
                        System.Diagnostics.Debug.WriteLine("检测到按键可能卡住，强制释放");
                        ForceReleaseAllButtons();
                        _leftButtonPressed = false;
                        _rightButtonPressed = false;
                    }
                }
                _lastInputTime = now;
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"监控输入状态异常: {ex}");
            }
        }

        private void ForceReleaseAllButtons()
        {
            try
            {
                var inputSimulator = (WindowsInputSimulator)_inputSimulator;
                inputSimulator.SimulateMouseButtonEx(false, WindowsInputSimulator.MouseButtons.Left);
                inputSimulator.SimulateMouseButtonEx(false, WindowsInputSimulator.MouseButtons.Right);
                inputSimulator.SimulateMouseButtonEx(false, WindowsInputSimulator.MouseButtons.Middle);
                System.Diagnostics.Debug.WriteLine("已强制释放所有按键");
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"强制释放按键异常: {ex}");
            }
        }

        private async void LoadSettings()
        {
            await _settingsService.LoadSettingsAsync();
            MouseSensitivity = _settingsService.MouseSensitivity;
            IsMouseEnabled = _settingsService.IsMouseEnabled;
            IsKeyboardEnabled = _settingsService.IsKeyboardEnabled;
            IsControlEnabled = _settingsService.IsControlEnabled;
            UseNaturalScrolling = _settingsService.UseNaturalScrolling;
            InvertYAxis = _settingsService.InvertYAxis;
            EnableAutoCalibration = _settingsService.EnableAutoCalibration;
            EnableSmoothing = _settingsService.EnableSmoothing;
            SmoothingLevel = _settingsService.SmoothingLevel;
            EnableNonLinearCurve = _settingsService.EnableNonLinearCurve;
            NonLinearCurvePower = _settingsService.NonLinearCurvePower;
            DeadZone = _settingsService.DeadZone;
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
            _lastConnectedDeviceAddress = deviceAddress;
            _lastConnectionAttempt = DateTime.Now;

            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
                await _controllerService.InitializeAsync();
                _reconnectAttempts = 0; // 连接成功后重置重连计数
                _connectionCheckTimer?.Start(); // 确保定时器在连接成功后启动
            }
            catch (Exception ex)
            {
                StatusMessage = $"连接失败: {ex.Message}";
                System.Diagnostics.Debug.WriteLine($"连接失败: {ex}");
                throw;
            }
            finally
            {
                IsConnecting = false;
            }
        }

        public void Disconnect()
        {
            try
            {
                _bluetoothService.Disconnect();
                StatusMessage = "已断开连接";
                _lastConnectedDeviceAddress = 0;
                _reconnectAttempts = 0;
                _connectionCheckTimer?.Stop();
                _stateCheckTimer?.Stop();
                
                // 确保释放所有按键
                ForceReleaseAllButtons();
                _leftButtonPressed = false;
                _rightButtonPressed = false;
                _movementBuffer.Clear();
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"断开连接异常: {ex}");
            }
        }

        private void BluetoothService_ConnectionStatusChanged(object? sender, BluetoothConnectionStatus status)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                IsConnected = status == BluetoothConnectionStatus.Connected;
                
                if (!IsConnected)
                {
                    // 断开连接时，确保释放所有按键
                    ForceReleaseAllButtons();
                    _leftButtonPressed = false;
                    _rightButtonPressed = false;
                    _movementBuffer.Clear();
                }
                
                // 更新状态消息
                if (IsConnected)
                {
                    StatusMessage = "已连接";
                    _reconnectAttempts = 0;
                    _connectionCheckTimer?.Start();
                }
                else
                {
                    if (_isReconnecting)
                    {
                        StatusMessage = $"连接断开，正在尝试重新连接... (第 {_reconnectAttempts} 次)";
                    }
                    else
                    {
                        StatusMessage = "连接已断开";
                    }
                }
                
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
            if (!_enableSmoothing)
            {
                return (x, y);
            }

            var now = DateTime.Now;

            // 如果距离上次移动时间太长，清空缓冲区
            if ((now - _lastMovementTime).TotalMilliseconds > MOVEMENT_TIMEOUT_MS)
            {
                _movementBuffer.Clear();
            }

            _movementBuffer.Enqueue((x, y));
            _lastMovementTime = now;

            // 根据平滑等级调整缓冲区大小
            while (_movementBuffer.Count > _smoothingLevel)
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
                
                // 应用Y轴反转设置
                if (!_invertYAxis)
                {
                    rawNormalizedY = -rawNormalizedY;
                }
                
                System.Diagnostics.Debug.WriteLine($"[未校准归一化] 中心点偏移: ({deltaX:F2}, {deltaY:F2}) => 归一化: ({rawNormalizedX:F2}, {rawNormalizedY:F2})");
                return (rawNormalizedX, rawNormalizedY);
            }

            // 使用校准中心点
            // 计算相对于中心点的偏移
            double calibratedDeltaX = rawX - _calibrationData.CenterX;
            double calibratedDeltaY = rawY - _calibrationData.CenterY;

            // 应用死区
            if (Math.Abs(calibratedDeltaX) < _deadZone)
                calibratedDeltaX = 0;
            if (Math.Abs(calibratedDeltaY) < _deadZone)
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

            // 应用Y轴反转设置
            if (!_invertYAxis)
            {
                normalizedY = -normalizedY;
            }

            // 应用非线性曲线，使小幅度移动更精确
            if (_enableNonLinearCurve)
            {
                normalizedX = Math.Sign(normalizedX) * Math.Pow(Math.Abs(normalizedX), _nonLinearCurvePower);
                normalizedY = Math.Sign(normalizedY) * Math.Pow(Math.Abs(normalizedY), _nonLinearCurvePower);
            }

            // 应用灵敏度
            normalizedX *= _mouseSensitivity;
            normalizedY *= _mouseSensitivity;
            
            System.Diagnostics.Debug.WriteLine($"[校准后] X={normalizedX:F2}, Y={normalizedY:F2}");

            return (normalizedX, normalizedY);
        }

        private void ProcessTouchpadMovement(ControllerData data)
        {
            try
            {
                if (!_isMouseEnabled || !_isControlEnabled || _isCalibrating)
                    return;
                    
                if (!data.TouchpadTouched)
                {
                    _movementBuffer.Clear(); // 清空缓冲
                    RecordTouchpadHistory(0, 0, false);
                    return;
                }

                // 应用校准
                var (calibratedX, calibratedY) = ApplyCalibration(data.AxisX, data.AxisY);

                // 应用平滑处理
                var (smoothX, smoothY) = SmoothMovement(calibratedX, calibratedY);

                // 计算最终的鼠标移动
                const double MOVEMENT_SCALE = 4.0;
                
                double adaptiveScale = MOVEMENT_SCALE * (0.5 + Math.Min(1.0, Math.Sqrt(smoothX * smoothX + smoothY * smoothY)));
                
                int finalDeltaX = (int)(smoothX * adaptiveScale);
                int finalDeltaY = (int)(smoothY * adaptiveScale);

                RecordTouchpadHistory(smoothX, smoothY, data.TouchpadButton);

                if (CheckAbnormalMovement(finalDeltaX, finalDeltaY))
                {
                    TriggerAutoCalibration();
                    return;
                }
                
                if (Math.Abs(finalDeltaX) > 0 || Math.Abs(finalDeltaY) > 0)
                {
                    if (data.TouchpadButton)
                    {
                        int wheelDelta = (int)(finalDeltaY * 2);
                        if (_useNaturalScrolling)
                        {
                            wheelDelta = -wheelDelta;
                        }
                        _inputSimulator.SimulateWheelMovement(wheelDelta);
                    }
                    else
                    {
                        _inputSimulator.SimulateMouseMovement(finalDeltaX, finalDeltaY);
                    }
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"处理触摸板移动异常: {ex}");
                _movementBuffer.Clear();
            }
        }

        private void HandleButtonInput(ControllerData data)
        {
            try
            {
                if (!IsControlEnabled || _isCalibrating) return;

                var inputSimulator = (WindowsInputSimulator)_inputSimulator;
                
                if (IsMouseEnabled)
                {
                    // 检测按键状态变化并更新状态
                    if (data.TriggerButton != _rightButtonPressed)
                    {
                        _rightButtonPressed = data.TriggerButton;
                        inputSimulator.SimulateMouseButtonEx(data.TriggerButton, WindowsInputSimulator.MouseButtons.Right);
                        _lastInputTime = DateTime.Now;
                    }
                    
                    if (data.TouchpadButton != _leftButtonPressed)
                    {
                        _leftButtonPressed = data.TouchpadButton;
                        inputSimulator.SimulateMouseButtonEx(data.TouchpadButton, WindowsInputSimulator.MouseButtons.Left);
                        _lastInputTime = DateTime.Now;
                    }
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
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"按键处理异常: {ex}");
                ForceReleaseAllButtons();
                _leftButtonPressed = false;
                _rightButtonPressed = false;
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
            UseNaturalScrolling = _settingsService.UseNaturalScrolling;
            InvertYAxis = _settingsService.InvertYAxis;
            EnableAutoCalibration = _settingsService.EnableAutoCalibration;
            EnableSmoothing = _settingsService.EnableSmoothing;
            SmoothingLevel = _settingsService.SmoothingLevel;
            EnableNonLinearCurve = _settingsService.EnableNonLinearCurve;
            NonLinearCurvePower = _settingsService.NonLinearCurvePower;
            DeadZone = _settingsService.DeadZone;
        }

        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            StatusMessage = "已应用触摸板校准数据";
        }

        private bool CheckAbnormalMovement(double deltaX, double deltaY)
        {
            // 如果禁用了自动校准，直接返回false
            if (!_enableAutoCalibration)
            {
                return false;
            }

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