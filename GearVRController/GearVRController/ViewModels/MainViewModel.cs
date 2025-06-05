using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services;
using GearVRController.Services.Interfaces;
using Microsoft.UI.Dispatching;
using System.Collections.Generic;
using GearVRController.Enums;
using EnumsNS = GearVRController.Enums; // 添加命名空间别名
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
        private readonly ISettingsService _settingsService;
        private readonly DispatcherQueue _dispatcherQueue;
        private readonly TouchpadProcessor _touchpadProcessor;
        private readonly IWindowManagerService _windowManagerService;
        private readonly IEventAggregator _eventAggregator;
        private readonly IActionExecutionService _actionExecutionService;
        private readonly ILogger _logger;
        private readonly IInputHandlerService _inputHandlerService;
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
        /// 获取或设置按钮去抖动阈值。
        /// </summary>
        public int ButtonDebounceThreshold
        {
            get => _settingsService.ButtonDebounceThreshold;
            set
            {
                if (_settingsService.ButtonDebounceThreshold != value)
                {
                    _settingsService.ButtonDebounceThreshold = value;
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
                    _gestureRecognizer.UpdateGestureConfig(_settingsService.GestureConfig);
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
        /// 鼠标灵敏度的最小允许值。
        /// </summary>
        private const double MOUSE_SENSITIVITY_MIN = 0.1;
        /// <summary>
        /// 鼠标灵敏度的最大允许值。
        /// </summary>
        private const double MOUSE_SENSITIVITY_MAX = 2.0;
        /// <summary>
        /// 平滑等级的最小允许值。
        /// </summary>
        private const int SMOOTHING_LEVEL_MIN = 1;
        /// <summary>
        /// 平滑等级的最大允许值。
        /// </summary>
        private const int SMOOTHING_LEVEL_MAX = 10;
        /// <summary>
        /// 非线性曲线幂次的最小允许值。
        /// </summary>
        private const double NON_LINEAR_CURVE_POWER_MIN = 1.0;
        /// <summary>
        /// 非线性曲线幂次的最大允许值。
        /// </summary>
        private const double NON_LINEAR_CURVE_POWER_MAX = 3.0;
        /// <summary>
        /// 死区的最小允许值。
        /// </summary>
        private const double DEAD_ZONE_MIN = 0.0;
        /// <summary>
        /// 死区的最大允许值。
        /// </summary>
        private const double DEAD_ZONE_MAX = 20.0;

        /// <summary>
        /// MainViewModel 的构造函数。
        /// 通过依赖注入接收所有必要的服务。
        /// </summary>
        /// <param name="bluetoothService">蓝牙服务，用于管理蓝牙连接和数据传输。</param>
        /// <param name="controllerService">控制器服务，用于处理原始控制器数据并转换为有意义的输入。</param>
        /// <param name="settingsService">设置服务，用于加载和保存应用程序设置。</param>
        /// <param name="dispatcherQueue">DispatcherQueue，用于确保 UI 更新在 UI 线程上进行。</param>
        /// <param name="touchpadProcessor">触摸板处理器，用于对原始触摸板坐标进行校准和归一化。</param>
        /// <param name="windowManagerService">窗口管理服务，用于打开和关闭其他应用程序窗口（如校准窗口）。</param>
        /// <param name="eventAggregator">事件聚合器，用于跨组件发布和订阅事件。</param>
        /// <param name="actionExecutionService">动作执行服务，用于根据手势执行预定义的操作。</param>
        /// <param name="logger">日志服务，用于记录日志信息。</param>
        /// <param name="inputHandlerService">输入处理服务，用于处理控制器输入。</param>
        public MainViewModel(
            IBluetoothService bluetoothService,
            IControllerService controllerService,
            ISettingsService settingsService,
            DispatcherQueue dispatcherQueue,
            TouchpadProcessor touchpadProcessor,
            IWindowManagerService windowManagerService,
            IEventAggregator eventAggregator,
            IActionExecutionService actionExecutionService,
            ILogger logger,
            IInputHandlerService inputHandlerService)
        {
            _bluetoothService = bluetoothService;
            _controllerService = controllerService;
            _settingsService = settingsService;
            _dispatcherQueue = dispatcherQueue;
            _touchpadProcessor = touchpadProcessor;
            _windowManagerService = windowManagerService;
            _eventAggregator = eventAggregator;
            _actionExecutionService = actionExecutionService;
            _logger = logger;
            _inputHandlerService = inputHandlerService;

            _bluetoothService.DataReceived += BluetoothService_DataReceived;
            _bluetoothService.ConnectionStatusChanged += BluetoothService_ConnectionStatusChanged;
            _calibrationCompletedSubscription = _eventAggregator.Subscribe<CalibrationCompletedEvent>(OnCalibrationCompleted);
            _gestureRecognizer = new GestureRecognizer(_settingsService, _dispatcherQueue);
            _gestureRecognizer.GestureDetected += OnGestureDetected;
            LoadSettings();
            RegisterHotKeys();
            _logger.LogInfo("MainViewModel initialized.", nameof(MainViewModel));
        }

        private void OnCalibrationCompleted(CalibrationCompletedEvent e)
        {
            _logger.LogInfo($"Calibration completed event received. Success: {e.IsSuccess}", nameof(MainViewModel));
            if (e.IsSuccess && e.CalibrationData != null)
            {
                _calibrationData = e.CalibrationData;
                OnPropertyChanged(nameof(CalibrationData));
                _logger.LogInfo($"Applied new calibration data: MinX={_calibrationData.MinX}, MaxX={_calibrationData.MaxX}", nameof(MainViewModel));
            }
            IsCalibrating = false;
        }

        private void LoadSettings()
        {
            _logger.LogInfo("加载应用程序设置.", nameof(MainViewModel));
            try
            {
                IsControlEnabled = _settingsService.IsControlEnabled;
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
                GestureSensitivity = _settingsService.GestureSensitivity;
                ShowGestureHints = _settingsService.ShowGestureHints;
                SwipeUpAction = _settingsService.SwipeUpAction;
                SwipeDownAction = _settingsService.SwipeDownAction;
                SwipeLeftAction = _settingsService.SwipeLeftAction;
                SwipeRightAction = _settingsService.SwipeRightAction;

                var loadedCalibrationData = _settingsService.LoadCalibrationData();
                if (loadedCalibrationData != null)
                {
                    ApplyCalibrationData(loadedCalibrationData);
                    _logger.LogInfo("已加载并应用保存的校准数据.", nameof(MainViewModel));
                }
                else
                {
                    _logger.LogInfo("未找到保存的校准数据，使用默认设置.", nameof(MainViewModel));
                }
            }
            catch (Exception ex)
            {
                _logger.LogError("加载设置时发生错误.", nameof(MainViewModel), ex);
                StatusMessage = "加载设置失败：" + ex.Message;
            }
        }

        private void RegisterHotKeys()
        {
            _logger.LogInfo("注册热键.", nameof(MainViewModel));
            try
            {
                // _settingsService.RegisterHotKey(Enums.VirtualKeyCode.MEDIA_PLAY_PAUSE, () => ToggleControl());
                _logger.LogInfo("播放/暂停键热键已注册.", nameof(MainViewModel));
            }
            catch (Exception ex)
            {
                _logger.LogError("注册热键时发生错误.", nameof(MainViewModel), ex);
            }
        }

        public async Task ConnectAsync(ulong deviceAddress)
        {
            if (IsConnecting) return; // Prevent multiple connection attempts
            IsConnecting = true;
            StatusMessage = "正在连接...";
            _logger.LogInfo($"尝试连接到设备: {deviceAddress}", nameof(MainViewModel));

            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
                StatusMessage = "连接成功！";
                _logger.LogInfo($"成功连接到设备: {deviceAddress}", nameof(MainViewModel));
            }
            catch (TimeoutException ex)
            {
                StatusMessage = "连接超时。请确保控制器已开启并处于配对模式。";
                _logger.LogError($"连接超时到设备: {deviceAddress}", nameof(MainViewModel), ex);
            }
            catch (Exception ex)
            {
                StatusMessage = $"连接失败: {ex.Message}";
                _logger.LogError($"连接到设备 {deviceAddress} 失败.", nameof(MainViewModel), ex);
            }
            finally
            {
                IsConnecting = false;
            }
        }

        public void Disconnect()
        {
            if (!IsConnected && !IsConnecting) return; // Only disconnect if connected or trying to connect
            _logger.LogInfo("断开与设备的连接.", nameof(MainViewModel));
            try
            {
                _bluetoothService.Disconnect();
                StatusMessage = "已断开连接";
                _logger.LogInfo("设备已成功断开连接.", nameof(MainViewModel));
            }
            catch (Exception ex)
            {
                _logger.LogError("断开连接时发生错误.", nameof(MainViewModel), ex);
                StatusMessage = $"断开连接失败: {ex.Message}";
            }
        }

        private void BluetoothService_DataReceived(object? sender, ControllerData data)
        {
            if (!IsConnected || IsConnecting)
            {
                return;
            }

            // Process button inputs (Trigger, Volume Up/Down, Back, Touchpad Click)
            _inputHandlerService.ProcessInput(data);

            // Process touchpad data for mouse movement or gestures
            if (IsGestureMode)
            {
                _gestureRecognizer.ProcessTouchpadPoint(new TouchpadPoint(data.TouchpadX, data.TouchpadY, data.TouchpadTouched));
            }

            _dispatcherQueue.TryEnqueue(() =>
            {
                LastControllerData = data;
                ProcessedTouchpadX = data.ProcessedTouchpadX;
                ProcessedTouchpadY = data.ProcessedTouchpadY;

                RecordTouchpadHistory(data.ProcessedTouchpadX, data.ProcessedTouchpadY, data.TouchpadTouched);
            });
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
            ApplyLoadedSettings(); // Apply default settings after resetting
        }

        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            _touchpadProcessor.SetCalibrationData(_calibrationData);
            StatusMessage = "已应用触摸板校准数据";
        }

        private void UpdateControlState()
        {
            System.Diagnostics.Debug.WriteLine($"[MainViewModel] UpdateControlState 已调用. IsCalibrating: {IsCalibrating}, IsConnecting: {IsConnecting}, IsConnected: {IsConnected}, IsControlEnabled: {IsControlEnabled}");
        }

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

        public void ClearTouchpadHistory()
        {
            _touchpadHistory.Clear();
            LastGesture = EnumsNS.TouchpadGesture.None;
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
                LastGesture = (EnumsNS.TouchpadGesture)(int)direction;

                System.Diagnostics.Debug.WriteLine($"[MainViewModel] Gesture Mode: Discrete Swipe gesture detected ({{direction}}), executing action.");
            }
            else // 相对模式，不在这里执行手势动作
            {
                System.Diagnostics.Debug.WriteLine($"[MainViewModel] Relative Mode: Gesture detected ({{direction}}), no action executed here as continuous movement is handled by ControllerService.");
            }
        }

        private void ExecuteGestureAction(GestureAction action)
        {
            _actionExecutionService.ExecuteAction(action);
        }

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

        public void Dispose()
        {
            _bluetoothService.DataReceived -= BluetoothService_DataReceived;
            _bluetoothService.ConnectionStatusChanged -= BluetoothService_ConnectionStatusChanged;

            if (_controllerService != null)
            {
                // Removed _controllerService.Dispose(); as IControllerService does not have a Dispose method.
            }
            // _inputSimulator is no longer directly used here, its logic moved to InputHandlerService
            // _inputStateMonitorService is no longer directly used here, its logic moved to InputHandlerService

            if (_calibrationCompletedSubscription != null)
            {
                _calibrationCompletedSubscription.Dispose();
            }
            _gestureRecognizer.GestureDetected -= OnGestureDetected;

            // Ensure all simulated keys are released in case the app exits unexpectedly when disconnected.
            // Removed _inputStateMonitorService.ForceReleaseAllButtons();
            // Removed _inputStateMonitorService.StopMonitoring(); // Stop input state monitor
            // Removed InputStateMonitorService_InputTimeoutDetected method
        }

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

        private void BluetoothService_ConnectionStatusChanged(object? sender, Windows.Devices.Bluetooth.BluetoothConnectionStatus status)
        {
            // 确保在UI线程上更新属性
            _dispatcherQueue.TryEnqueue(() =>
            {
                IsConnected = (status == Windows.Devices.Bluetooth.BluetoothConnectionStatus.Connected);

                // 也可以在这里更新状态文本
                if (IsConnected)
                {
                    // 你可以设置一个更明确的状态消息
                    // StatusMessage = "设备已连接"; 
                }
                else
                {
                    StatusMessage = "设备已断开连接";
                }
            });
        }
    }
}