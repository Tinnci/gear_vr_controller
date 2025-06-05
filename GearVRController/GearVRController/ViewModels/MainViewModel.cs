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
        private readonly TouchpadProcessor _touchpadProcessor;
        private readonly IInputStateMonitorService _inputStateMonitorService;
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

        private bool _isGestureMode;
        private GestureRecognizer _gestureRecognizer;
        private GestureAction _swipeUpAction = GestureAction.PageUp;
        private GestureAction _swipeDownAction = GestureAction.PageDown;
        private GestureAction _swipeLeftAction = GestureAction.BrowserBack;
        private GestureAction _swipeRightAction = GestureAction.BrowserForward;

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
                    _settingsService.IsControlEnabled = _isControlEnabled;
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
                    _settingsService.MouseSensitivity = _mouseSensitivity;
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
                    _settingsService.IsMouseEnabled = _isMouseEnabled;
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
                    _settingsService.IsKeyboardEnabled = _isKeyboardEnabled;
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
                    _settingsService.UseNaturalScrolling = _useNaturalScrolling;
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
                    _settingsService.InvertYAxis = _invertYAxis;
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
                    _settingsService.EnableSmoothing = _enableSmoothing;
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
                    _settingsService.SmoothingLevel = _smoothingLevel;
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
                    _settingsService.EnableNonLinearCurve = _enableNonLinearCurve;
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
                    _settingsService.NonLinearCurvePower = _nonLinearCurvePower;
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
                    _settingsService.DeadZone = _deadZone;
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
                    _settingsService.IsGestureMode = _isGestureMode;
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
                    _settingsService.SwipeUpAction = _swipeUpAction;
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
                    _settingsService.SwipeDownAction = _swipeDownAction;
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
                    _settingsService.SwipeLeftAction = _swipeLeftAction;
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
                    _settingsService.SwipeRightAction = _swipeRightAction;
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
            TouchpadProcessor touchpadProcessor,
            IInputStateMonitorService inputStateMonitorService)
        {
            _bluetoothService = bluetoothService;
            _controllerService = controllerService;
            _inputSimulator = inputSimulator;
            _settingsService = settingsService;
            _dispatcherQueue = dispatcherQueue;
            _touchpadProcessor = touchpadProcessor;
            _inputStateMonitorService = inputStateMonitorService;

            _bluetoothService.ConnectionStatusChanged += BluetoothService_ConnectionStatusChanged;
            _bluetoothService.DataReceived += BluetoothService_DataReceived;
            _controllerService.ControllerDataProcessed += (sender, data) => LastControllerData = data;

            _gestureRecognizer = new GestureRecognizer(_settingsService, _dispatcherQueue);
            _gestureRecognizer.GestureDetected += OnGestureDetected;

            LoadSettings();
            _inputStateMonitorService.Initialize(); // Initialize the input state monitor
            RegisterHotKeys();
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

            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
                await _controllerService.InitializeAsync();
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
                _inputStateMonitorService.StopMonitor(); // Stop the input state monitor

                // 确保释放所有按键
                _inputStateMonitorService.ForceReleaseAllButtons();
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
                    _inputStateMonitorService.ForceReleaseAllButtons();
                    Debug.WriteLine("[MainViewModel] 检测到断开连接，已释放按键并清空缓冲区."); // 添加日志
                }

                // 更新状态消息
                if (IsConnected)
                {
                    StatusMessage = "已连接";
                    Debug.WriteLine("[MainViewModel] 状态消息已更新为: 已连接"); // 添加日志
                }
                else
                {
                    StatusMessage = "连接已断开，正在尝试重新连接..."; // Simplified status message
                    Debug.WriteLine($"[MainViewModel] 状态消息已更新为: {StatusMessage}"); // 添加日志
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
                    _inputStateMonitorService.UpdateInputState(data.TriggerButton, data.TouchpadButton, IsControlEnabled, _isCalibrating);
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
                _inputStateMonitorService.ForceReleaseAllButtons();
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
            _settingsService.IsControlEnabled = _isControlEnabled;
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