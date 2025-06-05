using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services.Interfaces;
using Microsoft.UI.Dispatching;
using Windows.Devices.Bluetooth;
using GearVRController.Events;

namespace GearVRController.ViewModels
{
    public class ConnectionViewModel : INotifyPropertyChanged, IDisposable
    {
        private readonly IBluetoothService _bluetoothService;
        private readonly DispatcherQueue _dispatcherQueue;
        private readonly IEventAggregator _eventAggregator;
        private readonly ILogger _logger;

        private bool _isConnected;
        private bool _isConnecting = false;
        private string _statusMessage = string.Empty;

        public event PropertyChangedEventHandler? PropertyChanged;

        public bool IsConnected
        {
            get => _isConnected;
            private set => SetProperty(ref _isConnected, value);
        }

        public bool IsConnecting
        {
            get => _isConnecting;
            private set => SetProperty(ref _isConnecting, value);
        }

        public string StatusMessage
        {
            get => _statusMessage;
            set => SetProperty(ref _statusMessage, value);
        }

        public ConnectionViewModel(
            IBluetoothService bluetoothService,
            DispatcherQueue dispatcherQueue,
            IEventAggregator eventAggregator,
            ILogger logger)
        {
            _bluetoothService = bluetoothService ?? throw new ArgumentNullException(nameof(bluetoothService));
            _dispatcherQueue = dispatcherQueue ?? throw new ArgumentNullException(nameof(dispatcherQueue));
            _eventAggregator = eventAggregator ?? throw new ArgumentNullException(nameof(eventAggregator));
            _logger = logger ?? throw new ArgumentNullException(nameof(logger));

            _bluetoothService.ConnectionStatusChanged += BluetoothService_ConnectionStatusChanged;
            _bluetoothService.DataReceived += BluetoothService_DataReceived;

            StatusMessage = "未连接";
        }

        public async Task ConnectAsync(ulong deviceAddress)
        {
            if (IsConnecting || IsConnected)
            {
                _logger.LogWarning($"ConnectAsync called but already connecting or connected. IsConnecting: {IsConnecting}, IsConnected: {IsConnected}");
                return;
            }

            IsConnecting = true;
            StatusMessage = "正在连接...";
            _logger.LogInformation($"尝试连接到设备：{deviceAddress}");

            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
                // 连接状态会在 ConnectionStatusChanged 事件中更新
            }
            catch (Exception ex)
            {
                _logger.LogError($"连接失败：{ex.Message}");
                StatusMessage = $"连接失败: {ex.Message}";
                IsConnected = false;
            }
            finally
            {
                IsConnecting = false; // 连接尝试结束，无论是成功还是失败
            }
        }

        public void Disconnect()
        {
            if (!IsConnected && !IsConnecting)
            {
                _logger.LogWarning("Disconnect called but not connected or connecting.");
                return;
            }

            _bluetoothService.Disconnect();
            // 连接状态会在 ConnectionStatusChanged 事件中更新
            StatusMessage = "已断开";
            IsConnected = false;
            IsConnecting = false;
            _logger.LogInformation("已手动断开设备连接。");
        }

        private void BluetoothService_ConnectionStatusChanged(object? sender, BluetoothConnectionStatus status)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                IsConnected = status == BluetoothConnectionStatus.Connected;
                StatusMessage = IsConnected ? "已连接" : "已断开";
                _logger.LogInformation($"蓝牙连接状态改变：{status}");
                _eventAggregator.Publish(new ConnectionStatusChangedEvent(IsConnected));
            });
        }

        private void BluetoothService_DataReceived(object? sender, ControllerData data)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                // ConnectionViewModel 仅转发数据接收事件，不直接处理数据
                _eventAggregator.Publish(new ControllerDataReceivedEvent(data));
            });
        }

        protected void OnPropertyChanged([CallerMemberName] string? propertyName = null)
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

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (disposing)
            {
                if (_bluetoothService != null)
                {
                    _bluetoothService.ConnectionStatusChanged -= BluetoothService_ConnectionStatusChanged;
                    _bluetoothService.DataReceived -= BluetoothService_DataReceived;
                }
            }
        }
    }
}