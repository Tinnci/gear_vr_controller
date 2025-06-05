using System;
using Microsoft.UI.Dispatching;
using GearVRController.Services.Interfaces;
using System.Diagnostics;
using GearVRController.Enums; // 添加对 MouseButtons 的引用
using Microsoft.UI.Xaml; // Add this for DispatcherTimer
using System.Collections.Generic;
using GearVRController.Events;
using System.Linq; // Added for ToList()

namespace GearVRController.Services
{
    /// <summary>
    /// InputStateMonitorService 负责监控应用程序的输入活动，并在长时间没有输入时强制释放所有模拟的按键。
    /// 这是一种"守护"机制，旨在防止在应用程序意外关闭或蓝牙连接中断时，模拟的鼠标或键盘按键"卡住"。
    /// 它实现了 `IInputStateMonitorService` 接口和 `IDisposable` 接口，以便进行适当的资源清理。
    /// </summary>
    public class InputStateMonitorService : IInputStateMonitorService, IDisposable
    {
        /// <summary>
        /// 输入模拟器，用于强制释放按键。
        /// </summary>
        private readonly IInputSimulator _inputSimulator;
        /// <summary>
        /// DispatcherQueue 实例，用于确保计时器回调在 UI 线程上执行。
        /// </summary>
        private readonly DispatcherQueue _dispatcherQueue; // Used for DispatcherTimer

        /// <summary>
        /// 最后一次检测到输入活动的时间戳。
        /// </summary>
        private DateTime _lastInputTime = DateTime.MinValue;
        /// <summary>
        /// 检测无输入活动超时的毫秒数。如果超过此时间没有输入，将强制释放按键。
        /// </summary>
        private const int INPUT_TIMEOUT_MS = 2000; // 2秒超时
        /// <summary>
        /// 用于定期检查输入状态的计时器。
        /// </summary>
        private DispatcherTimer? _stateCheckTimer;

        private readonly ILogger _logger; // Added ILogger dependency
        private readonly HashSet<VirtualKeyCode> _pressedKeys = new HashSet<VirtualKeyCode>(); // Track pressed keys

        // Event for input timeout
        public event EventHandler<InputTimeoutDetectedEvent>? InputTimeoutDetected;

        /// <summary>
        /// InputStateMonitorService 的构造函数。
        /// </summary>
        /// <param name="inputSimulator">输入模拟器服务，用于模拟按键释放。</param>
        /// <param name="dispatcherQueue">DispatcherQueue 实例，用于调度 UI 线程操作。</param>
        public InputStateMonitorService(IInputSimulator inputSimulator, DispatcherQueue dispatcherQueue, ILogger logger)
        {
            _inputSimulator = inputSimulator;
            _dispatcherQueue = dispatcherQueue;
            _logger = logger; // Initialize logger
        }

        /// <summary>
        /// 启动输入状态监控器。
        /// 初始化并启动一个定时器，该定时器会定期检查是否有输入活动。
        /// </summary>
        public void StartMonitoring() // Renamed from StartMonitor
        {
            if (_stateCheckTimer == null)
            {
                _stateCheckTimer = new DispatcherTimer();
                _stateCheckTimer.Interval = TimeSpan.FromSeconds(1); // Check every second
                _stateCheckTimer.Tick += MonitorInputState_Tick;
            }
            _lastInputTime = DateTime.Now; // Reset timer on start
            _stateCheckTimer.Start();
            _logger.LogInfo("State check timer initialized and started.", nameof(InputStateMonitorService));
        }

        /// <summary>
        /// 定时器触发的回调方法，用于监控输入状态。
        /// </summary>
        /// <param name="sender">事件发送者。</param>
        /// <param name="e">事件参数。</param>
        private void MonitorInputState_Tick(object? sender, object e) // Named event handler
        {
            MonitorInputState();
        }

        /// <summary>
        /// 监控输入状态，并在长时间无输入时强制释放所有按键。
        /// 这是防止按键"卡住"的守护逻辑。
        /// </summary>
        private void MonitorInputState()
        {
            try
            {
                var now = DateTime.Now;
                if ((now - _lastInputTime).TotalMilliseconds > INPUT_TIMEOUT_MS)
                {
                    _logger.LogWarning("检测到长时间无输入，强制释放所有按键 (守护机制)", nameof(InputStateMonitorService));
                    ForceReleaseAllButtons();
                    _lastInputTime = now;
                }
            }
            catch (Exception ex)
            {
                _logger.LogError($"监控输入状态异常.", nameof(InputStateMonitorService), ex);
            }
        }

        /// <summary>
        /// 强制释放所有模拟的鼠标按键。
        /// </summary>
        public void ForceReleaseAllButtons()
        {
            if (_pressedKeys.Count == 0) return; // Only proceed if there are keys to release
            try
            {
                var releasedKeys = new List<VirtualKeyCode>();
                foreach (var keyCode in _pressedKeys.ToList()) // ToList to avoid modification during iteration
                {
                    if (Enum.IsDefined(typeof(MouseButtons), (int)keyCode))
                    {
                        // It's a mouse button
                        _inputSimulator.SimulateMouseButtonEx(false, (int)keyCode);
                        _logger.LogInfo($"强制释放鼠标按键: {keyCode}", nameof(InputStateMonitorService));
                    }
                    else
                    {
                        // It's a keyboard key
                        _inputSimulator.SimulateKeyRelease((int)keyCode);
                        _logger.LogInfo($"强制释放键盘按键: {keyCode}", nameof(InputStateMonitorService));
                    }
                    releasedKeys.Add(keyCode);
                }
                _pressedKeys.Clear();
                _logger.LogInfo($"已强制释放所有跟踪的 {releasedKeys.Count} 个按键.", nameof(InputStateMonitorService));
                InputTimeoutDetected?.Invoke(this, new InputTimeoutDetectedEvent(releasedKeys));
            }
            catch (Exception ex)
            {
                _logger.LogError($"强制释放按键异常.", nameof(InputStateMonitorService), ex);
            }
        }

        /// <summary>
        /// 停止输入状态监控器并清理相关资源。
        /// 停止计时器并取消其事件订阅。
        /// </summary>
        public void StopMonitoring() // Renamed from StopMonitor
        {
            if (_stateCheckTimer != null)
            {
                _stateCheckTimer.Stop();
                _stateCheckTimer.Tick -= MonitorInputState_Tick;
                _stateCheckTimer = null;
            }
            _logger.LogInfo("State check timer stopped and cleaned up.", nameof(InputStateMonitorService));
        }

        /// <summary>
        /// 通知监控器有输入活动发生。
        /// 此方法应在每次检测到用户输入时调用，以重置不活动计时器。
        /// </summary>
        public void NotifyInputActivity()
        {
            _lastInputTime = DateTime.Now;
            // _logger.LogInfo("Input activity notified, timer reset.", nameof(InputStateMonitorService)); // Too verbose for continuous activity
        }

        /// <summary>
        /// 跟踪当前按下的键，以便在超时时释放。
        /// </summary>
        /// <param name="keyCode">按下的键的虚拟键码。</param>
        public void AddPressedKey(VirtualKeyCode keyCode)
        {
            _pressedKeys.Add(keyCode);
            _logger.LogInfo($"跟踪按下的键: {keyCode}. 当前跟踪数量: {_pressedKeys.Count}", nameof(InputStateMonitorService));
        }

        /// <summary>
        /// 从跟踪列表中移除已释放的键。
        /// </summary>
        /// <param name="keyCode">释放的键的虚拟键码。</param>
        public void RemovePressedKey(VirtualKeyCode keyCode)
        {
            _pressedKeys.Remove(keyCode);
            _logger.LogInfo($"移除已释放的键: {keyCode}. 当前跟踪数量: {_pressedKeys.Count}", nameof(InputStateMonitorService));
        }

        /// <summary>
        /// 注册一个全局热键。此实现仅作日志记录，不实际注册。
        /// </summary>
        /// <param name="keyCode">要注册的虚拟键码。</param>
        /// <param name="action">热键触发时执行的动作。</param>
        public void RegisterHotKey(VirtualKeyCode keyCode, Action action)
        {
            _logger.LogWarning($"热键注册功能未完全实现。键码: {keyCode}. 动作将被记录，但不会实际注册全局热键。", nameof(InputStateMonitorService));
            // 在实际应用中，这里需要调用P/Invoke来注册全局热键。
            // 为简化目的，目前只记录日志。
        }

        /// <summary>
        /// 释放由 InputStateMonitorService 实例占用的非托管资源。
        /// </summary>
        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        /// <summary>
        /// 释放托管和（可选）非托管资源。
        /// </summary>
        /// <param name="disposing">如果为 true，表示是从 `Dispose()` 方法调用，应释放托管和非托管资源；
        /// 如果为 false，表示从终结器调用，仅释放非托管资源。</param>
        protected virtual void Dispose(bool disposing)
        {
            if (disposing)
            {
                StopMonitoring(); // Stop and clean up the timer
                ForceReleaseAllButtons(); // Ensure any pressed keys are released on dispose
                _pressedKeys.Clear();
            }
        }
    }
}