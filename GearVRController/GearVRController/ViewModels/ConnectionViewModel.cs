using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services.Interfaces;
using Microsoft.UI.Dispatching;
using Windows.Devices.Bluetooth;
using GearVRController.Events;
using System.Collections.Generic;

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
            if (IsConnecting) return;

            IsConnecting = true;
            StatusMessage = "正在连接...";
            _logger.LogInfo($"尝试连接到设备：{deviceAddress}");

            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
                IsConnected = _bluetoothService.IsConnected;
                StatusMessage = IsConnected ? "已连接" : "连接失败";
                _logger.LogInfo($"设备连接成功: {deviceAddress}");
            }
            catch (TimeoutException)
            {
                StatusMessage = "连接超时";
                _logger.LogError($"连接到设备 {deviceAddress} 超时。", nameof(ConnectionViewModel));
            }
            catch (Exception ex)
            {
                StatusMessage = $"连接失败: {ex.Message}";
                _logger.LogError($"连接到设备 {deviceAddress} 失败: {ex.Message}", nameof(ConnectionViewModel), ex);
            }
            finally
            {
                IsConnecting = false;
            }
        }

        public void Disconnect()
        {
            if (!IsConnected && !IsConnecting) return;

            _bluetoothService.Disconnect();
            IsConnected = _bluetoothService.IsConnected;
            StatusMessage = "已手动断开设备连接。";
            _logger.LogInfo("已手动断开设备连接。");
        }

        private void BluetoothService_ConnectionStatusChanged(object? sender, BluetoothConnectionStatus status)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                IsConnected = status == BluetoothConnectionStatus.Connected;
                StatusMessage = IsConnected ? "已连接" : "已断开";
                _logger.LogInfo($"蓝牙连接状态改变：{status}");
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