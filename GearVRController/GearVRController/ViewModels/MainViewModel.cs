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
using GearVRController.Enums;
using EnumsNS = GearVRController.Enums; // 添加命名空间别名
using System.Diagnostics; // 添加 Debug 命名空间

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
        private bool _enableSmoothing = true;
        private int _smoothingLevel = 3;
        private bool _enableNonLinearCurve = true;
        private double _nonLinearCurvePower = 1.5;
        private double _deadZone = 8.0;

        private bool _isCalibrating = false;
        private bool _isConnecting = false;

        // 在类的字段部分添加触摸板可视化相关的字段
        private EnumsNS.TouchpadGesture _lastGesture = EnumsNS.TouchpadGesture.None; // 使用命名空间别名
        private readonly List<TouchpadPoint> _touchpadHistory = new List<TouchpadPoint>();
        private const int MAX_TOUCHPAD_HISTORY = 50;

        // Add processed touchpad coordinates properties
        private double _processedTouchpadX;
        private double _processedTouchpadY;

        public double ProcessedTouchpadX
        {
            get => _processedTouchpadX;
            private set => SetProperty(ref _processedTouchpadX, value);
        }

        public double ProcessedTouchpadY
        {
            get => _processedTouchpadY;
            private set => SetProperty(ref _processedTouchpadY, value);
        }

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

        private bool _isGestureMode;
        private GestureRecognizer _gestureRecognizer;
        private GestureAction _swipeUpAction = GestureAction.PageUp;
        private GestureAction _swipeDownAction = GestureAction.PageDown;
        private GestureAction _swipeLeftAction = GestureAction.BrowserBack;
        private GestureAction _swipeRightAction = GestureAction.BrowserForward;

        // Add TouchpadProcessor field
        private readonly TouchpadProcessor _touchpadProcessor;

        public event PropertyChangedEventHandler? PropertyChanged;
        public event EventHandler<ControllerData>? ControllerDataReceived;

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
                        // 清空缓冲区以适应新的平滑等级
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
                    // 清空缓冲区以适应新的平滑等级
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

        public bool IsGestureMode
        {
            get => _isGestureMode;
            set
            {
                if (SetProperty(ref _isGestureMode, value))
                {
                    OnPropertyChanged(nameof(IsRelativeMode));
                }
            }
        }

        public bool IsRelativeMode
        {
            get => !_isGestureMode;
            set => IsGestureMode = !value;
        }

        public float GestureSensitivity
        {
            get => _settingsService.GestureSensitivity;
            set
            {
                if (_settingsService.GestureSensitivity != value)
                {
                    _settingsService.GestureSensitivity = value;
                    _gestureRecognizer.UpdateGestureConfig(_settingsService.GestureConfig);
                    OnPropertyChanged();
                }
            }
        }

        public bool ShowGestureHints
        {
            get => _settingsService.ShowGestureHints;
            set
            {
                if (_settingsService.ShowGestureHints != value)
                {
                    _settingsService.ShowGestureHints = value;
                    _gestureRecognizer.UpdateGestureConfig(_settingsService.GestureConfig);
                    OnPropertyChanged();
                }
            }
        }

        public GestureAction SwipeUpAction
        {
            get => _swipeUpAction;
            set
            {
                if (_swipeUpAction != value)
                {
                    _swipeUpAction = value;
                    OnPropertyChanged();
                }
            }
        }

        public GestureAction SwipeDownAction
        {
            get => _swipeDownAction;
            set
            {
                if (_swipeDownAction != value)
                {
                    _swipeDownAction = value;
                    OnPropertyChanged();
                }
            }
        }

        public GestureAction SwipeLeftAction
        {
            get => _swipeLeftAction;
            set
            {
                if (_swipeLeftAction != value)
                {
                    _swipeLeftAction = value;
                    OnPropertyChanged();
                }
            }
        }

        public GestureAction SwipeRightAction
        {
            get => _swipeRightAction;
            set
            {
                if (_swipeRightAction != value)
                {
                    _swipeRightAction = value;
                    OnPropertyChanged();
                }
            }
        }

        public IEnumerable<GestureAction> AvailableGestureActions => Enum.GetValues<GestureAction>();

        public MainViewModel(
            IBluetoothService bluetoothService,
            IControllerService controllerService,
            IInputSimulator inputSimulator,
            ISettingsService settingsService,
            DispatcherQueue dispatcherQueue,
            TouchpadProcessor touchpadProcessor)
        {
            _bluetoothService = bluetoothService;
            _controllerService = controllerService;
            _inputSimulator = inputSimulator;
            _settingsService = settingsService;
            _dispatcherQueue = dispatcherQueue;
            _touchpadProcessor = touchpadProcessor;

            _bluetoothService.ConnectionStatusChanged += BluetoothService_ConnectionStatusChanged;
            _bluetoothService.DataReceived += BluetoothService_DataReceived;
            _controllerService.ControllerDataProcessed += (sender, data) => LastControllerData = data;

            _gestureRecognizer = new GestureRecognizer(_settingsService.GestureConfig, _dispatcherQueue);
            _gestureRecognizer.GestureDetected += OnGestureDetected;

            LoadSettings();
            InitializeConnectionCheck();
            InitializeStateCheck();
            RegisterHotKeys();
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
            EnableSmoothing = _settingsService.EnableSmoothing;
            SmoothingLevel = _settingsService.SmoothingLevel;
            EnableNonLinearCurve = _settingsService.EnableNonLinearCurve;
            NonLinearCurvePower = _settingsService.NonLinearCurvePower;
            DeadZone = _settingsService.DeadZone;
            IsGestureMode = _settingsService.IsGestureMode;
            IsRelativeMode = _settingsService.IsRelativeMode;
            GestureSensitivity = _settingsService.GestureSensitivity;
            ShowGestureHints = _settingsService.ShowGestureHints;
            SwipeUpAction = _settingsService.SwipeUpAction;
            SwipeDownAction = _settingsService.SwipeDownAction;
            SwipeLeftAction = _settingsService.SwipeLeftAction;
            SwipeRightAction = _settingsService.SwipeRightAction;

            // 尝试加载校准数据
            var calibration = _settingsService.LoadCalibrationData();
            if (calibration != null)
            {
                ApplyCalibrationData(calibration);
            }
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
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"断开连接异常: {ex}");
            }
        }

        private void BluetoothService_ConnectionStatusChanged(object? sender, BluetoothConnectionStatus status)
        {
            Debug.WriteLine($"[MainViewModel] 收到 ConnectionStatusChanged 事件. 新状态: {status}"); // 添加日志
            _dispatcherQueue.TryEnqueue(() =>
            {
                Debug.WriteLine($"[MainViewModel] 在 DispatcherQueue 中处理状态更新. 当前 IsConnected (前): {IsConnected}, 新状态: {status}"); // 添加日志
                IsConnected = status == BluetoothConnectionStatus.Connected;
                Debug.WriteLine($"[MainViewModel] IsConnected 已更新为: {IsConnected}"); // 添加日志

                if (!IsConnected)
                {
                    // 断开连接时，确保释放所有按键
                    ForceReleaseAllButtons();
                    _leftButtonPressed = false;
                    _rightButtonPressed = false;
                    Debug.WriteLine("[MainViewModel] 检测到断开连接，已释放按键并清空缓冲区."); // 添加日志
                }

                // 更新状态消息
                if (IsConnected)
                {
                    StatusMessage = "已连接";
                    _reconnectAttempts = 0;
                    _connectionCheckTimer?.Start();
                    Debug.WriteLine("[MainViewModel] 状态消息已更新为: 已连接"); // 添加日志
                }
                else
                {
                    if (_isReconnecting)
                    {
                        StatusMessage = $"连接断开，正在尝试重新连接... (第 {_reconnectAttempts} 次)";
                        Debug.WriteLine($"[MainViewModel] 状态消息已更新为: {StatusMessage}"); // 添加日志
                    }
                    else
                    {
                        StatusMessage = "连接已断开";
                        Debug.WriteLine("[MainViewModel] 状态消息已更新为: 连接已断开"); // 添加日志
                    }
                }

                UpdateControlState();
                // Debug.WriteLine($"[MainViewModel] UpdateControlState 已调用. IsCalibrating: {IsCalibrating}, IsConnecting: {IsConnecting}, IsConnected: {IsConnected}, IsControlEnabled: {IsControlEnabled}"); // 简化日志
            });
        }

        private void BluetoothService_DataReceived(object? sender, ControllerData data)
        {
            // 将数据传递给 UI 线程处理
            _dispatcherQueue.TryEnqueue(() =>
            {
                // System.Diagnostics.Debug.WriteLine($"[MainViewModel] Data Received: AccelX={{data.AccelX}}, GyroZ={{data.GyroZ}}");

                if (!IsControlEnabled)
                {
                    LastControllerData = data; // 即使禁用控制，也更新最新数据
                    ControllerDataReceived?.Invoke(this, data); // 触发事件以便UI更新可视化
                    return;
                }

                // Process controller data
                _controllerService.ProcessControllerData(data);

                // Add these lines:
                var (pX, pY) = _touchpadProcessor.ProcessRawData(data.AxisX, data.AxisY);
                ProcessedTouchpadX = pX;
                ProcessedTouchpadY = pY;

                // Update LastControllerData
                LastControllerData = data;

                // Invoke event for UI updates
                ControllerDataReceived?.Invoke(this, data);

                // Handle button input if mouse is enabled
                if (IsMouseEnabled)
                {
                    HandleButtonInput(data);
                }

                // Process wheel movement
                // 滚轮模拟的Delta值，如果需要，可以基于陀螺仪或加速度计数据来计算
                // For now, we assume a fixed delta for demonstration purposes if a specific button is pressed.
                // In a real scenario, this would come from a continuous input like a scroll wheel or specific gestures.
                // Example: if data.SomeScrollButton is pressed, simulate a scroll.
                // int scrollDelta = 0;
                // if (data.VolumeUpButton) scrollDelta = 1;
                // else if (data.VolumeDownButton) scrollDelta = -1;
                // if (scrollDelta != 0)
                // {
                //     _inputSimulator.SimulateMouseWheel(scrollDelta);
                // }
            });
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

        protected bool SetProperty<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
        {
            if (EqualityComparer<T>.Default.Equals(field, value)) return false;
            field = value;
            OnPropertyChanged(propertyName);
            return true;
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
            EnableSmoothing = _settingsService.EnableSmoothing;
            SmoothingLevel = _settingsService.SmoothingLevel;
            EnableNonLinearCurve = _settingsService.EnableNonLinearCurve;
            NonLinearCurvePower = _settingsService.NonLinearCurvePower;
            DeadZone = _settingsService.DeadZone;
        }

        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            _touchpadProcessor.SetCalibrationData(_calibrationData);
            StatusMessage = "已应用触摸板校准数据";
        }

        private void UpdateControlState()
        {
            // 此方法主要用于在连接/断开或校准状态改变时更新整体状态信息，
            // 不再直接控制 IsMouseEnabled 和 IsKeyboardEnabled 的值。
            // 这些子级控制的启用状态应仅由用户设置决定。

            // 在 ProcessTouchpadMovement 和 HandleButtonInput 中会综合判断
            // IsControlEnabled, IsCalibrating, IsMouseEnabled, IsKeyboardEnabled。

            // 保留此方法，如果未来需要根据连接/校准状态更新其他UI或状态信息。
            System.Diagnostics.Debug.WriteLine($"[MainViewModel] UpdateControlState 已调用. IsCalibrating: {IsCalibrating}, IsConnecting: {IsConnecting}, IsConnected: {IsConnected}, IsControlEnabled: {IsControlEnabled}");
        }

        // 添加记录触摸板历史的方法
        private void RecordTouchpadHistory(double x, double y, bool isPressed)
        {
            if (x == 0 && y == 0 && !isPressed)
                return; // 忽略无意义的数据

            var point = new TouchpadPoint((float)x, (float)y, isPressed);
            _touchpadHistory.Add(point);

            // 维持最大历史点数
            while (_touchpadHistory.Count > MAX_TOUCHPAD_HISTORY)
            {
                _touchpadHistory.RemoveAt(0);
            }
        }

        // 添加清除触摸板历史的方法
        public void ClearTouchpadHistory()
        {
            _touchpadHistory.Clear();
            LastGesture = EnumsNS.TouchpadGesture.None; // 使用命名空间别名
        }

        private void OnGestureDetected(object? sender, GestureDirection direction)
        {
            // 只有控制启用、非校准状态下处理手势
            // 在手势模式下，IsKeyboardEnabled 不影响鼠标移动手势的识别和处理。
            if (!IsControlEnabled || _isCalibrating) return;

            if (IsGestureMode)
            {
                // 手势模式：根据识别到的方向执行预定义动作
                GestureAction action = GestureAction.None;
                switch (direction)
                {
                    case GestureDirection.Up:
                        action = _settingsService.SwipeUpAction;
                        break;
                    case GestureDirection.Down:
                        action = _settingsService.SwipeDownAction;
                        break;
                    case GestureDirection.Left:
                        action = _settingsService.SwipeLeftAction;
                        break;
                    case GestureDirection.Right:
                        action = _settingsService.SwipeRightAction;
                        break;
                }
                ExecuteGestureAction(action);

                // 更新LastGesture以供可视化
                LastGesture = (EnumsNS.TouchpadGesture)(int)direction; // 假设 GestureDirection 可以直接映射到 TouchpadGesture

                System.Diagnostics.Debug.WriteLine($"[MainViewModel] Gesture Mode: Discrete Swipe gesture detected ({direction}), executing action.");
            }
            else // 相对模式，不在这里执行手势动作
            {
                System.Diagnostics.Debug.WriteLine($"[MainViewModel] Relative Mode: Gesture detected ({direction}), no action executed here as continuous movement is handled by ControllerService.");
            }
        }

        private void ExecuteGestureAction(GestureAction action)
        {
            switch (action)
            {
                case GestureAction.PageUp:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.PRIOR);
                    break;
                case GestureAction.PageDown:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.NEXT);
                    break;
                case GestureAction.BrowserBack:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_BACK);
                    break;
                case GestureAction.BrowserForward:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_FORWARD);
                    break;
                case GestureAction.VolumeUp:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_UP);
                    break;
                case GestureAction.VolumeDown:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_DOWN);
                    break;
                case GestureAction.MediaPlayPause:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_PLAY_PAUSE);
                    break;
                case GestureAction.MediaNext:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_NEXT_TRACK);
                    break;
                case GestureAction.MediaPrevious:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_PREV_TRACK);
                    break;
                case GestureAction.Copy:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_C);
                    break;
                case GestureAction.Paste:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_V);
                    break;
                case GestureAction.Undo:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_Z);
                    break;
                case GestureAction.Redo:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_Y);
                    break;
                case GestureAction.SelectAll:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_A);
                    break;
            }
        }

        // 恢复手动校准相关方法
        public void StartManualCalibration()
        {
            IsCalibrating = true;
            StatusMessage = "手动校准已启动，请在触摸板边缘划圈...";
        }

        public void EndCalibration()
        {
            IsCalibrating = false;
            StatusMessage = "校准完成";
        }
    }
}