using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services;
using GearVRController.Services.Interfaces;
using Microsoft.UI.Xaml;
using Microsoft.UI.Dispatching;
using Windows.Devices.Bluetooth;
using System.Collections.Generic;
using System.Linq;
using GearVRController.Enums;
using EnumsNS = GearVRController.Enums; // 添加命名空间别名
using System.Diagnostics; // 添加 Debug 命名空间
using GearVRController.Events; // Add this

namespace GearVRController.ViewModels
{
    /// <summary>
    /// MainViewModel 是应用程序的主视图模型，负责协调模型、服务和视图之间的交互。
    /// 它管理蓝牙连接状态、控制器数据处理、用户输入模拟以及应用程序设置。
    /// 实现了 INotifyPropertyChanged 接口，用于通知 UI 属性的变化，并实现了 IDisposable 接口用于资源清理。
    /// </summary>
    public class MainViewModel : INotifyPropertyChanged, IDisposable
    {
        private readonly IBluetoothService _bluetoothService;
        private readonly IControllerService _controllerService;
        private readonly IInputSimulator _inputSimulator;
        private readonly ISettingsService _settingsService;
        private readonly DispatcherQueue _dispatcherQueue;
        private readonly TouchpadProcessor _touchpadProcessor;
        private readonly IInputStateMonitorService _inputStateMonitorService;
        private readonly IWindowManagerService _windowManagerService;
        private readonly IEventAggregator _eventAggregator;
        private readonly IActionExecutionService _actionExecutionService;
        private bool _isConnected;
        private string _statusMessage = string.Empty;
        private ControllerData _lastControllerData = new ControllerData();
        private const int NumberOfWheelPositions = 64;

        /// <summary>
        /// 存储触摸板校准数据。
        /// </summary>
        private TouchpadCalibrationData? _calibrationData;

        // 用户可配置的控制设置
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

        /// <summary>
        /// 指示触摸板校准过程是否正在进行中。
        /// </summary>
        private bool _isCalibrating = false;
        /// <summary>
        /// 指示蓝牙连接过程是否正在进行中。
        /// </summary>
        private bool _isConnecting = false;

        /// <summary>
        /// 内部触发器按钮状态跟踪，用于去抖动。
        /// </summary>
        private bool _isTriggerButtonPressed = false;
        /// <summary>
        /// 内部触摸板按钮状态跟踪，用于去抖动。
        /// </summary>
        private bool _isTouchpadButtonPressed = false;

        /// <summary>
        /// 触摸板按钮按下计数器，用于实现去抖动。
        /// </summary>
        private int _touchpadButtonPressCounter = 0;
        /// <summary>
        /// 触摸板按钮释放计数器，用于实现去抖动。
        /// </summary>
        private int _touchpadButtonReleaseCounter = 0;
        /// <summary>
        /// 触发器按钮按下计数器，用于实现去抖动。
        /// </summary>
        private int _triggerButtonPressCounter = 0;
        /// <summary>
        /// 触发器按钮释放计数器，用于实现去抖动。
        /// </summary>
        private int _triggerButtonReleaseCounter = 0;
        /// <summary>
        /// 按钮去抖动的阈值，表示需要连续多少个数据包来确认按钮状态变化。
        /// </summary>
        private const int BUTTON_DEBOUNCE_THRESHOLD = 2; // Number of consistent packets

        /// <summary>
        /// 内部音量上键状态跟踪，用于边缘检测。
        /// </summary>
        private bool _isVolumeUpHeld = false;
        /// <summary>
        /// 内部音量下键状态跟踪，用于边缘检测。
        /// </summary>
        private bool _isVolumeDownHeld = false;

        /// <summary>
        /// 上次识别到的触摸板手势，用于 UI 可视化。
        /// </summary>
        private EnumsNS.TouchpadGesture _lastGesture = EnumsNS.TouchpadGesture.None;
        /// <summary>
        /// 触摸板历史轨迹点列表，用于 UI 轨迹可视化。
        /// </summary>
        private readonly List<TouchpadPoint> _touchpadHistory = new List<TouchpadPoint>();
        /// <summary>
        /// 触摸板历史轨迹点的最大数量。
        /// </summary>
        private const int MAX_TOUCHPAD_HISTORY = 50;

        /// <summary>
        /// 处理后的触摸板X坐标（归一化）。
        /// </summary>
        private double _processedTouchpadX;
        /// <summary>
        /// 处理后的触摸板Y坐标（归一化）。
        /// </summary>
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

        /// <summary>
        /// 用于管理校准完成事件订阅的 Disposable 对象，确保在 ViewModel 销毁时取消订阅。
        /// </summary>
        private IDisposable? _calibrationCompletedSubscription;

        /// <summary>
        /// 当 ViewModel 的属性发生变化时触发的事件。
        /// </summary>
        public event PropertyChangedEventHandler? PropertyChanged;
        /// <summary>
        /// 当控制器接收到新数据并处理后触发的事件。
        /// </summary>
        public event EventHandler<ControllerData>? ControllerDataReceived;

        /// <summary>
        /// 获取或设置是否启用对控制器输入的整体控制（鼠标、键盘等）。
        /// 如果禁用，控制器数据将仅用于可视化，而不会模拟任何输入。
        /// </summary>
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

        /// <summary>
        /// 获取或设置鼠标灵敏度。
        /// 值越大，鼠标移动越快。范围限制在0.1到2.0之间。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否启用鼠标输入模拟。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否启用键盘输入模拟。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否使用自然滚动（即向上滑动触摸板向下滚动页面）。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否反转触摸板 Y 轴的移动方向。
        /// </summary>
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

        /// <summary>
        /// 获取当前蓝牙连接状态。
        /// </summary>
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

        /// <summary>
        /// 获取或设置应用程序当前的状态消息，用于 UI 显示。
        /// </summary>
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

        /// <summary>
        /// 获取最后一次接收到的控制器数据。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否正在进行触摸板校准。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否正在尝试连接蓝牙设备。
        /// </summary>
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

        /// <summary>
        /// 获取最后识别到的触摸板手势。
        /// </summary>
        public EnumsNS.TouchpadGesture LastGesture
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

        /// <summary>
        /// 获取触摸板历史轨迹点的只读列表，用于可视化。
        /// </summary>
        public IReadOnlyList<TouchpadPoint> TouchpadHistory => _touchpadHistory.AsReadOnly();

        /// <summary>
        /// 获取当前应用的触摸板校准数据。
        /// </summary>
        public TouchpadCalibrationData? CalibrationData => _calibrationData;

        /// <summary>
        /// 获取或设置是否启用鼠标移动平滑处理。
        /// </summary>
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

        /// <summary>
        /// 获取或设置鼠标移动平滑的等级。值越大，平滑效果越明显。
        /// 范围限制在1到10之间。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否启用非线性曲线来调整鼠标移动。
        /// 非线性曲线可以使小范围移动更精确，大范围移动更迅速。
        /// </summary>
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

        /// <summary>
        /// 获取或设置非线性曲线的幂次。值越大，曲线的非线性程度越高。
        /// 范围限制在1.0到3.0之间。
        /// </summary>
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

        /// <summary>
        /// 获取或设置触摸板的死区大小。在死区内的移动将被忽略。
        /// 范围限制在0.0到20.0之间。
        /// </summary>
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

        /// <summary>
        /// 获取或设置当前是否处于手势模式。
        /// 在手势模式下，触摸板输入用于识别离散手势；否则用于模拟连续鼠标移动（相对模式）。
        /// </summary>
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

        /// <summary>
        /// 获取或设置当前是否处于相对模式（与手势模式相反）。
        /// </summary>
        public bool IsRelativeMode
        {
            get => !_isGestureMode;
            set => IsGestureMode = !value;
        }

        /// <summary>
        /// 获取或设置手势识别的灵敏度。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否在 UI 上显示手势提示。
        /// </summary>
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

        /// <summary>
        /// 获取所有可用的手势动作列表。
        /// </summary>
        public IEnumerable<GestureAction> AvailableGestureActions => Enum.GetValues<GestureAction>();

        /// <summary>
        /// MainViewModel 的构造函数。
        /// 通过依赖注入接收所有必要的服务。
        /// </summary>
        /// <param name="bluetoothService">蓝牙服务，用于管理蓝牙连接和数据传输。</param>
        /// <param name="controllerService">控制器服务，用于处理原始控制器数据并转换为有意义的输入。</param>
        /// <param name="inputSimulator">输入模拟器，用于模拟鼠标和键盘输入。</param>
        /// <param name="settingsService">设置服务，用于加载和保存应用程序设置。</param>
        /// <param name="dispatcherQueue">DispatcherQueue，用于确保 UI 更新在 UI 线程上进行。</param>
        /// <param name="touchpadProcessor">触摸板处理器，用于对原始触摸板坐标进行校准和归一化。</param>
        /// <param name="inputStateMonitorService">输入状态监控服务，用于监控用户输入活动并强制释放按键。</param>
        /// <param name="windowManagerService">窗口管理服务，用于打开和关闭其他应用程序窗口（如校准窗口）。</param>
        /// <param name="eventAggregator">事件聚合器，用于跨组件发布和订阅事件。</param>
        /// <param name="actionExecutionService">动作执行服务，用于根据手势执行预定义的操作。</param>
        public MainViewModel(
            IBluetoothService bluetoothService,
            IControllerService controllerService,
            IInputSimulator inputSimulator,
            ISettingsService settingsService,
            DispatcherQueue dispatcherQueue,
            TouchpadProcessor touchpadProcessor,
            IInputStateMonitorService inputStateMonitorService,
            IWindowManagerService windowManagerService,
            IEventAggregator eventAggregator,
            IActionExecutionService actionExecutionService)
        {
            _bluetoothService = bluetoothService;
            _controllerService = controllerService;
            _inputSimulator = inputSimulator;
            _settingsService = settingsService;
            _dispatcherQueue = dispatcherQueue;
            _touchpadProcessor = touchpadProcessor;
            _inputStateMonitorService = inputStateMonitorService;
            _windowManagerService = windowManagerService; // Assign
            _eventAggregator = eventAggregator; // Assign
            _actionExecutionService = actionExecutionService; // Assign

            _bluetoothService.ConnectionStatusChanged += BluetoothService_ConnectionStatusChanged;
            _bluetoothService.DataReceived += BluetoothService_DataReceived;
            _controllerService.ControllerDataProcessed += (sender, data) => LastControllerData = data;

            _gestureRecognizer = new GestureRecognizer(_settingsService, _dispatcherQueue);
            _gestureRecognizer.GestureDetected += OnGestureDetected;

            LoadSettings();
            _inputStateMonitorService.StartMonitor();
            RegisterHotKeys();

            // Subscribe to calibration completion event
            _calibrationCompletedSubscription = _eventAggregator.Subscribe<CalibrationCompletedEvent>(OnCalibrationCompleted);
        }

        /// <summary>
        /// 处理触摸板校准完成事件。
        /// 当校准过程结束后，应用校准数据并关闭校准窗口。
        /// </summary>
        /// <param name="e">包含校准数据的事件参数。</param>
        private void OnCalibrationCompleted(CalibrationCompletedEvent e)
        {
            ApplyCalibrationData(e.CalibrationData);
            EndCalibration();
            _windowManagerService.CloseTouchpadCalibrationWindow(); // Close the window
        }

        /// <summary>
        /// 异步加载应用程序设置。
        /// </summary>
        private async void LoadSettings()
        {
            await _settingsService.LoadSettingsAsync();
            ApplyLoadedSettings();
        }

        /// <summary>
        /// 注册应用程序的热键。
        /// TODO: 由于 WinUI 3 的限制，可能需要使用原生 Windows API 来实现。目前通过 UI 按钮控制。
        /// </summary>
        private void RegisterHotKeys()
        {
            // 这里可以添加热键注册逻辑
            // 由于WinUI 3的限制，我们可能需要使用原生Windows API来实现
            // 暂时可以通过UI按钮来控制
        }

        /// <summary>
        /// 连接到指定的蓝牙设备。
        /// </summary>
        /// <param name="deviceAddress">要连接的蓝牙设备的地址。</param>
        /// <returns>表示异步连接操作的任务。</returns>
        /// <exception cref="Exception">连接失败时抛出。</exception>
        public async Task ConnectAsync(ulong deviceAddress)
        {
            if (_bluetoothService.IsConnected) return;

            IsConnecting = true;
            StatusMessage = "正在连接...";

            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
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

        /// <summary>
        /// 断开与当前连接的蓝牙设备的连接。
        /// 停止输入状态监控，并强制释放所有模拟的按键。
        /// </summary>
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

        /// <summary>
        /// 处理蓝牙连接状态变化的事件。
        /// 当蓝牙服务报告连接状态变化时，此方法会更新 ViewModel 的连接状态和状态消息。
        /// 并在断开连接时，强制释放所有模拟的按键。
        /// </summary>
        /// <param name="sender">事件发送者。</param>
        /// <param name="status">新的蓝牙连接状态。</param>
        private void BluetoothService_ConnectionStatusChanged(object? sender, BluetoothConnectionStatus status)
        {
            Debug.WriteLine($"[MainViewModel] 收到 ConnectionStatusChanged 事件. 新状态: {status}");
            _dispatcherQueue.TryEnqueue(() =>
            {
                Debug.WriteLine($"[MainViewModel] 在 DispatcherQueue 中处理状态更新. 当前 IsConnected (前): {IsConnected}, 新状态: {status}");
                IsConnected = status == BluetoothConnectionStatus.Connected;
                Debug.WriteLine($"[MainViewModel] IsConnected 已更新为: {IsConnected}");

                if (!IsConnected)
                {
                    // 断开连接时，确保释放所有按键
                    _inputStateMonitorService.ForceReleaseAllButtons();
                    Debug.WriteLine("[MainViewModel] 检测到断开连接，已释放按键并清空缓冲区.");
                }

                // 更新状态消息
                if (IsConnected)
                {
                    StatusMessage = "已连接";
                    Debug.WriteLine("[MainViewModel] 状态消息已更新为: 已连接");
                }
                else
                {
                    StatusMessage = "连接已断开，正在尝试重新连接..."; // Simplified status message
                    Debug.WriteLine($"[MainViewModel] 状态消息已更新为: {StatusMessage}");
                }

                UpdateControlState();
            });
        }

        /// <summary>
        /// 处理从蓝牙服务接收到的控制器数据。
        /// 此方法在 UI 线程上处理数据，更新 LastControllerData，并根据控制启用状态模拟输入。
        /// </summary>
        /// <param name="sender">事件发送者。</param>
        /// <param name="data">接收到的控制器数据。</param>
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

                // Process controller data (this will update ProcessedTouchpadX/Y in data object)
                _controllerService.ProcessControllerData(data);

                // Update processed touchpad coordinates from data (now populated by ControllerService)
                ProcessedTouchpadX = data.ProcessedTouchpadX;
                ProcessedTouchpadY = data.ProcessedTouchpadY;

                // Record touchpad history for visualization
                RecordTouchpadHistory(data.ProcessedTouchpadX, data.ProcessedTouchpadY, data.TouchpadTouched);

                // Update LastControllerData
                LastControllerData = data;

                // Invoke event for UI updates
                ControllerDataReceived?.Invoke(this, data);

                // Notify InputStateMonitorService of activity
                _inputStateMonitorService.NotifyInputActivity();

                // Handle button input if mouse is enabled
                if (IsMouseEnabled)
                {
                    HandleButtonInput(data);
                }
            });
        }

        /// <summary>
        /// 处理控制器按钮输入，并根据设置模拟鼠标或键盘操作。
        /// 包含去抖动逻辑以防止重复触发。
        /// </summary>
        /// <param name="data">当前的控制器数据，包含按钮状态。</param>
        private void HandleButtonInput(ControllerData data)
        {
            try
            {
                if (!IsControlEnabled || _isCalibrating) return;

                if (IsMouseEnabled)
                {
                    // 右键 (TriggerButton) 去抖动处理
                    if (data.TriggerButton)
                    {
                        _triggerButtonReleaseCounter = 0;
                        if (!_isTriggerButtonPressed)
                        {
                            _triggerButtonPressCounter++;
                            if (_triggerButtonPressCounter >= BUTTON_DEBOUNCE_THRESHOLD)
                            {
                                _isTriggerButtonPressed = true;
                                _inputSimulator.SimulateMouseButtonEx(true, (int)EnumsNS.MouseButtons.Right);
                                Debug.WriteLine($"[MainViewModel] Right button state changed to: pressed (debounced)");
                                _inputStateMonitorService.NotifyInputActivity();
                                _triggerButtonPressCounter = 0; // Reset counter
                            }
                        }
                    }
                    else // data.TriggerButton 为 false
                    {
                        _triggerButtonPressCounter = 0;
                        if (_isTriggerButtonPressed)
                        {
                            _triggerButtonReleaseCounter++;
                            if (_triggerButtonReleaseCounter >= BUTTON_DEBOUNCE_THRESHOLD)
                            {
                                _isTriggerButtonPressed = false;
                                _inputSimulator.SimulateMouseButtonEx(false, (int)EnumsNS.MouseButtons.Right);
                                Debug.WriteLine($"[MainViewModel] Right button state changed to: released (debounced)");
                                _inputStateMonitorService.NotifyInputActivity();
                                _triggerButtonReleaseCounter = 0; // Reset counter
                            }
                        }
                    }

                    // 左键 (TouchpadButton) 去抖动处理
                    if (data.TouchpadButton)
                    {
                        _touchpadButtonReleaseCounter = 0;
                        if (!_isTouchpadButtonPressed)
                        {
                            _touchpadButtonPressCounter++;
                            if (_touchpadButtonPressCounter >= BUTTON_DEBOUNCE_THRESHOLD)
                            {
                                _isTouchpadButtonPressed = true;
                                _inputSimulator.SimulateMouseButtonEx(true, (int)EnumsNS.MouseButtons.Left);
                                Debug.WriteLine($"[MainViewModel] Left button state changed to: pressed (debounced)");
                                _inputStateMonitorService.NotifyInputActivity();
                                _touchpadButtonPressCounter = 0; // Reset counter
                            }
                        }
                    }
                    else // data.TouchpadButton 为 false
                    {
                        _touchpadButtonPressCounter = 0;
                        if (_isTouchpadButtonPressed)
                        {
                            _touchpadButtonReleaseCounter++;
                            if (_touchpadButtonReleaseCounter >= BUTTON_DEBOUNCE_THRESHOLD)
                            {
                                _isTouchpadButtonPressed = false;
                                _inputSimulator.SimulateMouseButtonEx(false, (int)EnumsNS.MouseButtons.Left);
                                Debug.WriteLine($"[MainViewModel] Left button state changed to: released (debounced)");
                                _inputStateMonitorService.NotifyInputActivity();
                                _touchpadButtonReleaseCounter = 0; // Reset counter
                            }
                        }
                    }
                }

                if (IsKeyboardEnabled)
                {
                    // Home Button
                    if (data.HomeButton)
                    {
                        _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VK_HOME);
                        _inputStateMonitorService.NotifyInputActivity();
                    }

                    // Back Button
                    if (data.BackButton)
                    {
                        _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VK_BACK);
                        _inputStateMonitorService.NotifyInputActivity();
                    }

                    // 音量上键 (上升沿检测)
                    if (data.VolumeUpButton)
                    {
                        if (!_isVolumeUpHeld)
                        {
                            _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_UP);
                            _inputStateMonitorService.NotifyInputActivity();
                            _isVolumeUpHeld = true;
                            Debug.WriteLine($"[MainViewModel] Volume Up pressed");
                        }
                    }
                    else
                    {
                        _isVolumeUpHeld = false;
                    }

                    // 音量下键 (上升沿检测)
                    if (data.VolumeDownButton)
                    {
                        if (!_isVolumeDownHeld)
                        {
                            _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_DOWN);
                            _inputStateMonitorService.NotifyInputActivity();
                            _isVolumeDownHeld = true;
                            Debug.WriteLine($"[MainViewModel] Volume Down pressed");
                        }
                    }
                    else
                    {
                        _isVolumeDownHeld = false;
                    }
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"按键处理异常: {ex}");
                _inputStateMonitorService.ForceReleaseAllButtons();
            }
        }

        /// <summary>
        /// 当属性值改变时触发 PropertyChanged 事件。
        /// </summary>
        /// <param name="propertyName">发生变化的属性名称。如果为 null，则由编译器自动填充。</param>
        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

        /// <summary>
        /// 用于设置属性值的辅助方法，并在值实际改变时触发 PropertyChanged 事件。
        /// </summary>
        /// <typeparam name="T">属性的类型。</typeparam>
        /// <param name="field">属性的后端字段引用。</param>
        /// <param name="value">要设置的新值。</param>
        /// <param name="propertyName">发生变化的属性名称。如果为 null，则由编译器自动填充。</param>
        /// <returns>如果属性值发生变化，则返回 true；否则返回 false。</returns>
        protected bool SetProperty<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
        {
            if (EqualityComparer<T>.Default.Equals(field, value)) return false;
            field = value;
            OnPropertyChanged(propertyName);
            return true;
        }

        /// <summary>
        /// 切换控制的启用/禁用状态。
        /// </summary>
        public void ToggleControl()
        {
            _isControlEnabled = !_isControlEnabled;
            _settingsService.IsControlEnabled = _isControlEnabled;
            UpdateControlState();
            StatusMessage = _isControlEnabled ? "控制已启用" : "控制已禁用";
        }

        /// <summary>
        /// 将所有设置重置为默认值并重新应用。
        /// </summary>
        public void ResetSettings()
        {
            _settingsService.ResetToDefaults();
            ApplyLoadedSettings(); // Apply default settings after resetting
        }

        /// <summary>
        /// 应用新的触摸板校准数据。
        /// </summary>
        /// <param name="calibrationData">要应用的触摸板校准数据。</param>
        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            _touchpadProcessor.SetCalibrationData(_calibrationData);
            StatusMessage = "已应用触摸板校准数据";
        }

        /// <summary>
        /// 根据当前连接、校准和控制启用状态更新内部控制状态。
        /// 此方法主要用于同步内部状态，并不直接控制鼠标或键盘的启用状态，
        /// 这些子级控制的启用状态由用户设置决定。
        /// </summary>
        private void UpdateControlState()
        {
            System.Diagnostics.Debug.WriteLine($"[MainViewModel] UpdateControlState 已调用. IsCalibrating: {IsCalibrating}, IsConnecting: {IsConnecting}, IsConnected: {IsConnected}, IsControlEnabled: {IsControlEnabled}");
        }

        /// <summary>
        /// 记录触摸板的历史轨迹点，用于可视化。
        /// 忽略 (0,0) 且未触摸的无效数据点。
        /// </summary>
        /// <param name="x">处理后的触摸板X坐标。</param>
        /// <param name="y">处理后的触摸板Y坐标。</param>
        /// <param name="isPressed">触摸板是否被按下。</param>
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

        /// <summary>
        /// 清除触摸板历史轨迹和当前手势信息。
        /// </summary>
        public void ClearTouchpadHistory()
        {
            _touchpadHistory.Clear();
            LastGesture = EnumsNS.TouchpadGesture.None;
        }

        /// <summary>
        /// 处理手势识别器检测到的手势。
        /// 在手势模式下，根据识别到的方向执行预定义动作，并更新手势可视化。
        /// </summary>
        /// <param name="sender">事件发送者。</param>
        /// <param name="direction">识别到的手势方向。</param>
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
                LastGesture = (EnumsNS.TouchpadGesture)(int)direction;

                System.Diagnostics.Debug.WriteLine($"[MainViewModel] Gesture Mode: Discrete Swipe gesture detected ({{direction}}), executing action.");
            }
            else // 相对模式，不在这里执行手势动作
            {
                System.Diagnostics.Debug.WriteLine($"[MainViewModel] Relative Mode: Gesture detected ({{direction}}), no action executed here as continuous movement is handled by ControllerService.");
            }
        }

        /// <summary>
        /// 执行指定的手势动作。
        /// </summary>
        /// <param name="action">要执行的动作。</param>
        private void ExecuteGestureAction(GestureAction action)
        {
            _actionExecutionService.ExecuteAction(action);
        }

        /// <summary>
        /// 启动手动触摸板校准过程。
        /// </summary>
        public void StartManualCalibration()
        {
            IsCalibrating = true;
            StatusMessage = "手动校准已启动，请在触摸板边缘划圈...";
        }

        /// <summary>
        /// 结束触摸板校准过程。
        /// </summary>
        public void EndCalibration()
        {
            IsCalibrating = false;
            StatusMessage = "校准完成";
        }

        /// <summary>
        /// 释放 ViewModel 持有的资源，取消事件订阅，防止内存泄漏。
        /// </summary>
        public void Dispose()
        {
            _bluetoothService.ConnectionStatusChanged -= BluetoothService_ConnectionStatusChanged;
            _bluetoothService.DataReceived -= BluetoothService_DataReceived;
            if (_controllerService != null)
            {
                _controllerService.ControllerDataProcessed -= (sender, data) => LastControllerData = data;
            }
            if (_gestureRecognizer != null)
            {
                _gestureRecognizer.GestureDetected -= OnGestureDetected;
            }
            _calibrationCompletedSubscription?.Dispose();

            // 确保释放所有模拟的按键，以防应用在断开连接时意外退出。
            _inputStateMonitorService.ForceReleaseAllButtons();
            _inputStateMonitorService.StopMonitor(); // 停止输入状态监控器
        }

        /// <summary>
        /// 应用从设置服务加载的设置到 ViewModel 的属性。
        /// 确保 UI 能够反映最新的配置。
        /// </summary>
        private void ApplyLoadedSettings()
        {
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
            // IsRelativeMode is derived from IsGestureMode, no direct setting needed
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
    }
}