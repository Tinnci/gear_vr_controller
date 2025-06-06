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
using Microsoft.UI.Xaml.Controls; // Add this for InfoBarSeverity

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
        private readonly ConnectionViewModel _connectionViewModel;
        private readonly IInputOrchestratorService _inputOrchestratorService;
        private StatusInfo? _statusMessage; // Changed from string to StatusInfo?
        private bool _isStatusOpen; // Added for InfoBar visibility
        private string _title = "Gear VR Controller"; // Added for Window Title Bar
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
        private IDisposable? _gestureExecutedSubscription;

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
        public StatusInfo? StatusMessage
        {
            get => _statusMessage;
            set
            {
                if (SetProperty(ref _statusMessage, value))
                {
                    OnPropertyChanged(nameof(StatusTitle)); // Notify for StatusTitle
                    OnPropertyChanged(nameof(StatusSeverity)); // Notify for StatusSeverity
                    IsStatusOpen = value != null; // Update IsStatusOpen based on StatusMessage presence
                }
            }
        }

        /// <summary>
        /// 获取或设置 InfoBar 的打开状态。
        /// </summary>
        public bool IsStatusOpen
        {
            get => _isStatusOpen;
            private set => SetProperty(ref _isStatusOpen, value);
        }

        /// <summary>
        /// 获取 InfoBar 的标题，对应 StatusMessage 的 Message 属性。
        /// </summary>
        public string StatusTitle => _statusMessage?.Message ?? string.Empty;

        /// <summary>
        /// 获取 InfoBar 的严重级别，对应 StatusMessage 的 Severity 属性。
        /// </summary>
        public InfoBarSeverity StatusSeverity => _statusMessage?.Severity ?? InfoBarSeverity.Informational;

        /// <summary>
        /// 获取应用程序的标题。
        /// </summary>
        public string Title
        {
            get => _title;
            set => SetProperty(ref _title, value);
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

        public IEnumerable<GestureAction> AvailableGestureActions => Enum.GetValues<GestureAction>();

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
            ConnectionViewModel connectionViewModel,
            IInputOrchestratorService inputOrchestratorService)
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
            _connectionViewModel = connectionViewModel ?? throw new ArgumentNullException(nameof(connectionViewModel));
            _inputOrchestratorService = inputOrchestratorService ?? throw new ArgumentNullException(nameof(inputOrchestratorService));

            LoadCalibrationFromSettings();
            RegisterHotKeys();
            _touchpadProcessor.SetCalibrationData(_calibrationData);

            // Initial status message
            StatusMessage = new StatusInfo("请连接您的Gear VR控制器。", InfoBarSeverity.Informational);

            UpdateControlState();

            // Subscribe to EventAggregator events
            _controllerDataReceivedSubscription = _eventAggregator.Subscribe<ControllerDataReceivedEvent>(OnControllerDataReceived);
            _connectionStatusChangedSubscription = _eventAggregator.Subscribe<ConnectionStatusChangedEvent>(OnConnectionStatusChanged);
            _calibrationCompletedSubscription = _eventAggregator.Subscribe<CalibrationCompletedEvent>(OnCalibrationCompleted);
            _settingsChangedSubscription = _eventAggregator.Subscribe<SettingsChangedEvent>(OnSettingsChanged);
            _gestureExecutedSubscription = _eventAggregator.Subscribe<GestureExecutedEvent>(OnGestureExecuted); // Subscribe to new event

            // Initialize UI related properties
            _isGestureMode = _settingsService.IsGestureMode;
            OnPropertyChanged(nameof(IsRelativeMode));
            OnPropertyChanged(nameof(GestureSensitivity));
            OnPropertyChanged(nameof(ShowGestureHints));
            OnPropertyChanged(nameof(IsControlEnabled));
            OnPropertyChanged(nameof(Title)); // Notify for initial title
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
            // _gestureRecognizer.UpdateGestureConfig(_settingsService.GestureConfig);
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

                _inputOrchestratorService.ProcessControllerData(e.Data, _isCalibrating, _settingsService.IsControlEnabled);

                if (_settingsService.ShowTouchpadVisualizer)
                {
                    RecordTouchpadHistory(e.Data.TouchpadX, e.Data.TouchpadY, e.Data.TouchpadTouched);
                }

                ProcessedTouchpadX = e.Data.ProcessedTouchpadX;
                ProcessedTouchpadY = e.Data.ProcessedTouchpadY;
            });
        }

        private void OnConnectionStatusChanged(ConnectionStatusChangedEvent e)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                OnPropertyChanged(nameof(IsConnecting));
                OnPropertyChanged(nameof(IsConnected));
                UpdateControlState(); // Update control state based on connection

                if (e.IsConnected)
                {
                    StatusMessage = new StatusInfo("控制器已连接。", InfoBarSeverity.Success);
                }
                else
                {
                    StatusMessage = new StatusInfo("控制器已断开连接。", InfoBarSeverity.Warning);
                }
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
            OnPropertyChanged(nameof(IsControlEnabled));
            UpdateControlState();

            if (_settingsService.IsControlEnabled)
            {
                StatusMessage = new StatusInfo("控制已启用。", InfoBarSeverity.Success);
            }
            else
            {
                StatusMessage = new StatusInfo("控制已禁用。", InfoBarSeverity.Warning);
            }
        }

        public void ResetSettings()
        {
            _settingsService.ResetToDefaults();
        }

        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            _settingsService.SaveCalibrationData(calibrationData);
            OnPropertyChanged(nameof(CalibrationData));
            _touchpadProcessor.SetCalibrationData(calibrationData);
            StatusMessage = new StatusInfo("触摸板校准数据已保存。", InfoBarSeverity.Success);
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

        public void StartManualCalibration()
        {
            IsCalibrating = true;
            // _touchpadProcessor.StartCalibration(); // Removed as calibration is managed by TouchpadCalibrationViewModel
            StatusMessage = new StatusInfo("开始手动校准触摸板。", InfoBarSeverity.Informational);
        }

        public void EndCalibration()
        {
            IsCalibrating = false;
            // _touchpadProcessor.EndCalibration(); // Removed as calibration is managed by TouchpadCalibrationViewModel
            StatusMessage = new StatusInfo("触摸板校准已完成。", InfoBarSeverity.Success);
        }

        private void OnGestureExecuted(GestureExecutedEvent e)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                LastGesture = (EnumsNS.TouchpadGesture)(int)e.DetectedDirection;
            });
        }

        protected virtual void Dispose(bool disposing)
        {
            if (disposing)
            {
                // Unsubscribe from EventAggregator subscriptions
                _controllerDataReceivedSubscription?.Dispose();
                _connectionStatusChangedSubscription?.Dispose();
                _calibrationCompletedSubscription?.Dispose();
                _settingsChangedSubscription?.Dispose();
                _gestureExecutedSubscription?.Dispose(); // Dispose new subscription
            }
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }
    }
}