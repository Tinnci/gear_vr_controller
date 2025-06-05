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
    public class InputStateMonitorService : IInputStateMonitorService, IDisposable
    {
        private readonly IInputSimulator _inputSimulator;
        private readonly DispatcherQueue _dispatcherQueue; // Used for DispatcherTimer

        private DateTime _lastInputTime = DateTime.MinValue;
        private const int INPUT_TIMEOUT_MS = 5000; // 5秒超时
        private DispatcherTimer? _stateCheckTimer;

        public InputStateMonitorService(IInputSimulator inputSimulator, DispatcherQueue dispatcherQueue)
        {
            _inputSimulator = inputSimulator;
            _dispatcherQueue = dispatcherQueue;
        }

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

        private void MonitorInputState_Tick(object? sender, object e) // Named event handler
        {
            MonitorInputState();
        }

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

        public void NotifyInputActivity()
        {
            _lastInputTime = DateTime.Now;
            // Debug.WriteLine("[InputStateMonitorService] Input activity notified, timer reset."); // Too verbose
        }

        // Implement IDisposable
        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

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