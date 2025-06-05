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
        private readonly ConnectionViewModel _connectionViewModel;
        private string _statusMessage = string.Empty; // Re-introduce settable status message
        private ControllerData _lastControllerData = new ControllerData();
        private const int NumberOfWheelPositions = 64;

        /// <summary>
        /// 存储触摸板校准数据。
        /// </summary>
        private TouchpadCalibrationData? _calibrationData;

        /// <summary>
        /// 指示触摸板校准过程是否正在进行中。
        /// </summary>
        private bool _isCalibrating = false;

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
        private IDisposable? _settingsChangedSubscription;
        private IDisposable? _controllerDataReceivedSubscription;
        private IDisposable? _connectionStatusChangedSubscription;

        /// <summary>
        /// 当 ViewModel 的属性发生变化时触发的事件。
        /// </summary>
        public event PropertyChangedEventHandler? PropertyChanged;

        /// <summary>
        /// 获取当前蓝牙连接状态。
        /// </summary>
        public bool IsConnected => _connectionViewModel.IsConnected;

        /// <summary>
        /// 获取或设置应用程序当前的状态消息，用于 UI 显示。
        /// </summary>
        public string StatusMessage
        {
            get => _statusMessage;
            set => SetProperty(ref _statusMessage, value);
        }

        /// <summary>
        /// 获取或设置是否正在尝试连接蓝牙设备。
        /// </summary>
        public bool IsConnecting => _connectionViewModel.IsConnecting;

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
        /// 获取或设置整体控制是否启用。
        /// </summary>
        public bool IsControlEnabled
        {
            get => _settingsService.IsControlEnabled;
            set
            {
                if (_settingsService.IsControlEnabled != value)
                {
                    _settingsService.IsControlEnabled = value;
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
        /// <param name="settingsService">设置服务，用于加载和保存应用程序设置。</param>
        /// <param name="dispatcherQueue">DispatcherQueue，用于确保 UI 更新在 UI 线程上进行。</param>
        /// <param name="touchpadProcessor">触摸板处理器，用于对原始触摸板坐标进行校准和归一化。</param>
        /// <param name="windowManagerService">窗口管理服务，用于打开和关闭其他应用程序窗口（如校准窗口）。</param>
        /// <param name="eventAggregator">事件聚合器，用于跨组件发布和订阅事件。</param>
        /// <param name="actionExecutionService">动作执行服务，用于根据手势执行预定义的操作。</param>
        /// <param name="logger">日志服务，用于记录日志信息。</param>
        /// <param name="inputHandlerService">输入处理服务，用于处理控制器输入。</param>
        /// <param name="connectionViewModel">连接视图模型，用于管理连接相关的逻辑。</param>
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
            IInputHandlerService inputHandlerService,
            ConnectionViewModel connectionViewModel)
        {
            _bluetoothService = bluetoothService ?? throw new ArgumentNullException(nameof(bluetoothService));
            _controllerService = controllerService ?? throw new ArgumentNullException(nameof(controllerService));
            _settingsService = settingsService ?? throw new ArgumentNullException(nameof(settingsService));
            _dispatcherQueue = dispatcherQueue ?? throw new ArgumentNullException(nameof(dispatcherQueue));
            _touchpadProcessor = touchpadProcessor ?? throw new ArgumentNullException(nameof(touchpadProcessor));
            _windowManagerService = windowManagerService ?? throw new ArgumentNullException(nameof(windowManagerService));
            _eventAggregator = eventAggregator ?? throw new ArgumentNullException(nameof(eventAggregator));
            _actionExecutionService = actionExecutionService ?? throw new ArgumentNullException(nameof(actionExecutionService));
            _logger = logger ?? throw new ArgumentNullException(nameof(logger));
            _inputHandlerService = inputHandlerService ?? throw new ArgumentNullException(nameof(inputHandlerService));
            _connectionViewModel = connectionViewModel ?? throw new ArgumentNullException(nameof(connectionViewModel));

            // Remove direct BluetoothService subscriptions
            // _bluetoothService.DataReceived += BluetoothService_DataReceived;
            // _bluetoothService.ConnectionStatusChanged += BluetoothService_ConnectionStatusChanged;

            // Subscribe to events from EventAggregator that ConnectionViewModel now publishes
            _controllerDataReceivedSubscription = _eventAggregator.Subscribe<ControllerDataReceivedEvent>(OnControllerDataReceived);
            _connectionStatusChangedSubscription = _eventAggregator.Subscribe<ConnectionStatusChangedEvent>(OnConnectionStatusChanged);
            _calibrationCompletedSubscription = _eventAggregator.Subscribe<CalibrationCompletedEvent>(OnCalibrationCompleted);
            _settingsChangedSubscription = _eventAggregator.Subscribe<SettingsChangedEvent>(OnSettingsChanged);

            LoadCalibrationFromSettings();
            _gestureRecognizer = new GestureRecognizer(_settingsService, _dispatcherQueue);
            _gestureRecognizer.GestureDetected += OnGestureDetected;
            RegisterHotKeys();
            _touchpadProcessor.SetCalibrationData(_calibrationData);

            // Initial status message
            StatusMessage = "准备就绪";
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

        private void LoadCalibrationFromSettings()
        {
            _logger.LogInfo("尝试从设置加载校准数据.", nameof(MainViewModel));
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

        private void OnSettingsChanged(SettingsChangedEvent e)
        {
            _logger.LogInfo("接收到设置更改事件，更新相关属性.", nameof(MainViewModel));
            // Update UI-bound properties that reflect settings
            OnPropertyChanged(nameof(IsGestureMode));
            OnPropertyChanged(nameof(IsControlEnabled));
            // Add other properties that are directly bound to UI and reflect settings here
            // For example, if you have a property for ShowTouchpadVisualizer in MainViewModel
            // OnPropertyChanged(nameof(ShowTouchpadVisualizer)); 
            // And update GestureRecognizer config based on updated settings
            _gestureRecognizer.UpdateGestureConfig(_settingsService.GestureConfig);
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
            // Delegate connection logic to ConnectionViewModel
            await _connectionViewModel.ConnectAsync(deviceAddress);
            // MainViewModel will react to connection status changes via EventAggregator subscription
        }

        public void Disconnect()
        {
            // Delegate disconnection logic to ConnectionViewModel
            _connectionViewModel.Disconnect();
            // MainViewModel will react to connection status changes via EventAggregator subscription
        }

        private void OnControllerDataReceived(ControllerDataReceivedEvent e)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                LastControllerData = e.Data;

                if (_settingsService.IsControlEnabled && !_isCalibrating)
                {
                    if (IsGestureMode)
                    {
                        _gestureRecognizer.ProcessTouchpadPoint(new TouchpadPoint
                        {
                            X = e.Data.TouchpadX,
                            Y = e.Data.TouchpadY,
                            IsTouched = e.Data.TouchpadTouched
                        });
                    }
                    else // Relative mode (mouse simulation and button input)
                    {
                        _inputHandlerService.ProcessInput(e.Data);
                    }

                    if (_settingsService.ShowTouchpadVisualizer)
                    {
                        RecordTouchpadHistory(e.Data.TouchpadX, e.Data.TouchpadY, e.Data.TouchpadTouched);
                    }
                }

                ProcessedTouchpadX = e.Data.ProcessedTouchpadX;
                ProcessedTouchpadY = e.Data.ProcessedTouchpadY;

                // Button handling is now delegated to InputHandlerService.ProcessInput(e.Data)
                // if (_settingsService.IsControlEnabled)
                // {
                //     _inputHandlerService.HandleButtonInput(e.Data.ButtonState);
                // }
            });
        }

        private void OnConnectionStatusChanged(ConnectionStatusChangedEvent e)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                // Update MainViewModel's own properties based on ConnectionViewModel's status
                OnPropertyChanged(nameof(IsConnected));
                OnPropertyChanged(nameof(IsConnecting));
                StatusMessage = e.IsConnected ? "设备已连接" : "设备已断开连接"; // Update MainViewModel's status message
                UpdateControlState(); // Re-evaluate control state based on connection status
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
            _settingsService.IsControlEnabled = !_settingsService.IsControlEnabled;
            UpdateControlState();
            StatusMessage = _settingsService.IsControlEnabled ? "控制已启用" : "控制已禁用";
        }

        public void ResetSettings()
        {
            _settingsService.ResetToDefaults();
        }

        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            _touchpadProcessor.SetCalibrationData(_calibrationData);
            StatusMessage = "已应用触摸板校准数据";
        }

        private void UpdateControlState()
        {
            System.Diagnostics.Debug.WriteLine($"[MainViewModel] UpdateControlState 已调用. IsCalibrating: {IsCalibrating}, IsConnecting: {IsConnecting}, IsConnected: {IsConnected}, IsControlEnabled: {_settingsService.IsControlEnabled}");
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
            if (!_settingsService.IsControlEnabled || _isCalibrating) return;

            if (_settingsService.IsGestureMode)
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

        protected virtual void Dispose(bool disposing)
        {
            if (disposing)
            {
                // ... existing code ...

                // Unsubscribe from EventAggregator subscriptions
                _controllerDataReceivedSubscription?.Dispose();
                _connectionStatusChangedSubscription?.Dispose();
                _calibrationCompletedSubscription?.Dispose();
                _settingsChangedSubscription?.Dispose();

                if (_gestureRecognizer != null)
                {
                    _gestureRecognizer.GestureDetected -= OnGestureDetected;
                }
                // ... existing code ...
            }
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }
    }
}