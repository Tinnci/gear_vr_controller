using System;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services.Interfaces;
using System.Collections.Generic;
using System.Linq;
using GearVRController.Events;

namespace GearVRController.Services
{
    /// <summary>
    /// ControllerService 负责处理来自 Gear VR 控制器的原始数据，并根据应用程序设置将其转换为可模拟的输入事件。
    /// 它集成了触摸板处理、手势识别和输入模拟功能。
    /// </summary>
    public class ControllerService : IControllerService, IDisposable
    {
        private readonly IBluetoothService _bluetoothService;
        private readonly ISettingsService _settingsService;
        private readonly TouchpadProcessor _touchpadProcessor;
        private readonly ILogger _logger;
        private readonly IEventAggregator _eventAggregator;
        private IDisposable _dataSubscription;

        /// <summary>
        /// 当控制器数据被处理完成后触发的事件。
        /// </summary>
        public event EventHandler<ControllerData>? ControllerDataProcessed;

        /// <summary>
        /// ControllerService 的构造函数。
        /// 通过依赖注入接收所有必要的服务。
        /// </summary>
        /// <param name="bluetoothService">蓝牙服务，用于发送命令到控制器。</param>
        /// <param name="settingsService">设置服务，用于获取应用程序的各项配置。</param>
        /// <param name="touchpadProcessor">触摸板处理器，用于处理原始触摸板坐标。</param>
        /// <param name="logger">日志服务，用于记录日志。</param>
        /// <param name="eventAggregator">事件聚合器，用于订阅控制器数据事件。</param>
        public ControllerService(IBluetoothService bluetoothService, ISettingsService settingsService, TouchpadProcessor touchpadProcessor, ILogger logger, IEventAggregator eventAggregator)
        {
            _bluetoothService = bluetoothService ?? throw new ArgumentNullException(nameof(bluetoothService));
            _settingsService = settingsService ?? throw new ArgumentNullException(nameof(settingsService));
            _touchpadProcessor = touchpadProcessor ?? throw new ArgumentNullException(nameof(touchpadProcessor));
            _logger = logger ?? throw new ArgumentNullException(nameof(logger));
            _eventAggregator = eventAggregator ?? throw new ArgumentNullException(nameof(eventAggregator));

            // Subscribe to ControllerDataReceivedEvent
            _dataSubscription = _eventAggregator.Subscribe<ControllerDataReceivedEvent>(e => ProcessControllerData(e.Data));
        }

        // public Task InitializeAsync()
        // {
        //     // 初始化控制器
        //     return Task.CompletedTask;
        // }

        /// <summary>
        /// 异步发送命令到控制器。
        /// </summary>
        /// <param name="command">要发送的命令字节数组。</param>
        /// <param name="repeat">重复发送命令的次数（默认为1）。</param>
        /// <returns>表示异步操作的任务。</returns>
        public Task SendCommandAsync(byte[] command, int repeat = 1)
        {
            // 发送命令到控制器
            return _bluetoothService.SendDataAsync(command, repeat);
        }

        /// <summary>
        /// 处理从蓝牙服务接收到的原始控制器数据。
        /// 根据应用程序设置区分手势模式和相对模式，并进行相应的处理。
        /// </summary>
        /// <param name="data">接收到的控制器数据。</param>
        public void ProcessControllerData(ControllerData data)
        {
            if (data == null)
                return;

            // Validate data validity
            if (!ValidateControllerData(data))
                return;

            // Preprocess data (e.g., Y-axis inversion, calculate touch status)
            PreprocessControllerData(data);

            // ControllerService now only preprocesses data and publishes it.
            // MainViewModel (or InputHandlerService) will handle gesture/relative mode logic.

            // Trigger data processing completed event (data is always passed out)
            OnControllerDataProcessed(data);
        }

        /// <summary>
        /// Validate the validity of controller data.
        /// </summary>
        /// <param name="data">The controller data to validate.</param>
        /// <returns>True if the data is valid, otherwise false.</returns>
        private bool ValidateControllerData(ControllerData data)
        {
            // Check touchpad data range (using raw value range 0-1023)
            if (data.AxisX < 0 || data.AxisX > 1023 || data.AxisY < 0 || data.AxisY > 1023)
            {
                _logger.LogWarning($"触摸板数据超出预期范围: X={data.AxisX}, Y={data.AxisY}", nameof(ControllerService));
                // Don't return false, continue processing, as data may sometimes be temporarily out of range but still needs to be processed
                // If strict filtering is required, false can be returned here
            }

            // Check accelerometer data (currently not used for core logic)
            // if (float.IsNaN(data.AccelX) || float.IsNaN(data.AccelY) || float.IsNaN(data.AccelZ))
            //     return false;

            // Check gyroscope data (currently not used for core logic)
            // if (float.IsNaN(data.GyroX) || float.IsNaN(data.GyroY) || float.IsNaN(data.GyroZ))
            //     return false;

            return true;
        }

        /// <summary>
        /// Preprocesses raw controller data before further processing.
        /// This includes applying Y-axis inversion and determining touchpad touch status.
        /// </summary>
        /// <param name="data">The ControllerData object to preprocess.</param>
        private void PreprocessControllerData(ControllerData data)
        {
            data.Timestamp = DateTimeOffset.Now.ToUnixTimeMilliseconds();

            // Apply Y-axis inversion (if enabled)
            if (_settingsService.InvertYAxis)
            {
                // Invert Y-axis in the 0-1023 range to match user preference
                data.AxisY = 1023 - data.AxisY;
            }

            // Get processed touchpad coordinates for more precise touch detection
            var (processedX, processedY) = _touchpadProcessor.ProcessRawData(data.AxisX, data.AxisY);
            data.ProcessedTouchpadX = processedX;
            data.ProcessedTouchpadY = processedY;

            // Handle touch status - update judgment logic to avoid misjudgment
            // Condition for touchpad being touched: touchpad button is pressed, or absolute values of processed touchpad X/Y coordinates exceed preset normalized touch threshold
            System.Diagnostics.Debug.WriteLine($"[ControllerService] PreprocessControllerData: Raw AxisX={{data.AxisX}}, AxisY={{data.AxisY}}, TouchpadButton={{data.TouchpadButton}}");
            data.TouchpadTouched = data.TouchpadButton ||
                                  (Math.Abs(processedX) > _settingsService.ProcessedTouchThreshold ||
                                   Math.Abs(processedY) > _settingsService.ProcessedTouchThreshold);
            System.Diagnostics.Debug.WriteLine($"[ControllerService] PreprocessControllerData: Processed TouchpadX={{processedX:F2}}, Processed TouchpadY={{processedY:F2}}, Calculated TouchpadTouched={{data.TouchpadTouched}}");

            // Handle button states, determine if any button is pressed
            data.NoButton = !data.TriggerButton && !data.HomeButton &&
                           !data.BackButton && !data.TouchpadButton &&
                           !data.VolumeUpButton && !data.VolumeDownButton;
        }

        /// <summary>
        /// Handles wheel movement and adjusts direction based on "natural scrolling" settings.
        /// </summary>
        /// <param name="delta">The raw wheel scroll amount.</param>
        /// <returns>The adjusted wheel scroll amount.</returns>
        public int ProcessWheelMovement(int delta)
        {
            // If natural scrolling is enabled, reverse the scroll direction
            if (_settingsService.UseNaturalScrolling)
            {
                return -delta;
            }
            return delta;
        }

        /// <summary>
        /// Triggers the ControllerDataProcessed event.
        /// </summary>
        /// <param name="data">The ControllerData object that was processed.</param>
        protected virtual void OnControllerDataProcessed(ControllerData data)
        {
            ControllerDataProcessed?.Invoke(this, data);
        }

        public void Dispose()
        {
            _dataSubscription?.Dispose();
            // Removed disposal logic for _inputSimulator and _gestureRecognizer
        }
    }
}