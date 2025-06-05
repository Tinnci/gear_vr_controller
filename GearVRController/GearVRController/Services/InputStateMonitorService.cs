using System;
using Microsoft.UI.Dispatching;
using GearVRController.Services.Interfaces;
using System.Diagnostics;
using GearVRController.Enums; // 添加对 MouseButtons 的引用
// using WindowsInput.Native; // Not needed if VirtualKeys are merged
using Microsoft.UI.Xaml; // Add this for DispatcherTimer
// using WindowsInput; // Not needed if direct WindowsInputSimulator reference is removed

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
        private const int INPUT_TIMEOUT_MS = 5000; // 5秒超时
        /// <summary>
        /// 用于定期检查输入状态的计时器。
        /// </summary>
        private DispatcherTimer? _stateCheckTimer;

        /// <summary>
        /// InputStateMonitorService 的构造函数。
        /// </summary>
        /// <param name="inputSimulator">输入模拟器服务，用于模拟按键释放。</param>
        /// <param name="dispatcherQueue">DispatcherQueue 实例，用于调度 UI 线程操作。</param>
        public InputStateMonitorService(IInputSimulator inputSimulator, DispatcherQueue dispatcherQueue)
        {
            _inputSimulator = inputSimulator;
            _dispatcherQueue = dispatcherQueue;
        }

        /// <summary>
        /// 启动输入状态监控器。
        /// 初始化并启动一个定时器，该定时器会定期检查是否有输入活动。
        /// </summary>
        public void StartMonitor() // Renamed from Initialize
        {
            if (_stateCheckTimer == null)
            {
                _stateCheckTimer = new DispatcherTimer();
                _stateCheckTimer.Interval = TimeSpan.FromSeconds(1); // Check every second
                _stateCheckTimer.Tick += MonitorInputState_Tick; // Use a named method for easy unsubscription
            }
            _lastInputTime = DateTime.Now; // Reset timer on start
            _stateCheckTimer.Start();
            Debug.WriteLine("[InputStateMonitorService] State check timer initialized and started.");
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
                    System.Diagnostics.Debug.WriteLine("检测到长时间无输入，强制释放所有按键 (守护机制)");
                    ForceReleaseAllButtons();
                    // Reset last input time to avoid continuous releases if no input is ever received again
                    _lastInputTime = now;
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"监控输入状态异常: {ex}");
            }
        }

        /// <summary>
        /// 强制释放所有模拟的鼠标按键。
        /// </summary>
        public void ForceReleaseAllButtons()
        {
            try
            {
                _inputSimulator.SimulateMouseButtonEx(false, (int)MouseButtons.Left);
                _inputSimulator.SimulateMouseButtonEx(false, (int)MouseButtons.Right);
                _inputSimulator.SimulateMouseButtonEx(false, (int)MouseButtons.Middle);
                System.Diagnostics.Debug.WriteLine("已强制释放所有按键");
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"强制释放按键异常: {ex}");
            }
        }

        /// <summary>
        /// 停止输入状态监控器并清理相关资源。
        /// 停止计时器并取消其事件订阅。
        /// </summary>
        public void StopMonitor()
        {
            if (_stateCheckTimer != null)
            {
                _stateCheckTimer.Stop();
                // 取消订阅Tick事件
                _stateCheckTimer.Tick -= MonitorInputState_Tick; // Unsubscribe
                _stateCheckTimer = null; // Help with garbage collection
            }
            Debug.WriteLine("[InputStateMonitorService] State check timer stopped and cleaned up.");
        }

        /// <summary>
        /// 通知监控器有输入活动发生。
        /// 此方法应在每次检测到用户输入时调用，以重置不活动计时器。
        /// </summary>
        public void NotifyInputActivity()
        {
            _lastInputTime = DateTime.Now;
            // Debug.WriteLine("[InputStateMonitorService] Input activity notified, timer reset."); // Too verbose
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
                // Dispose managed state (managed objects)
                StopMonitor(); // Stop and clean up the timer
            }
            // Free unmanaged resources (unmanaged objects) and override finalizer
            // Set large fields to null
        }
    }
}