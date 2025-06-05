using System;
using System.Threading.Tasks;
using Windows.Devices.Bluetooth;
using Windows.Devices.Bluetooth.GenericAttributeProfile;
using Windows.Storage.Streams;
using GearVRController.Models;
using System.Threading;
using GearVRController.Services.Interfaces;
using GearVRController.Events;

namespace GearVRController.Services
{
    /// <summary>
    /// BluetoothService 负责管理与 Gear VR 控制器的蓝牙低功耗 (BLE) 连接。
    /// 它处理设备的发现、连接、服务和特征值的初始化，以及数据通知的订阅和数据传输。
    /// 实现了 IDisposable 接口以确保在不再需要时正确释放蓝牙资源。
    /// </summary>
    public class BluetoothService : IBluetoothService, IDisposable
    {
        /// <summary>
        /// 当前连接的蓝牙 LE 设备实例。
        /// </summary>
        private BluetoothLEDevice? _device;
        /// <summary>
        /// 用于控制器设置的 GATT 特征值。
        /// </summary>
        private GattCharacteristic? _setupCharacteristic;
        /// <summary>
        /// 用于控制器数据传输的 GATT 特征值（订阅通知）。
        /// </summary>
        private GattCharacteristic? _dataCharacteristic;
        /// <summary>
        /// 最后一次成功连接的蓝牙设备地址。
        /// </summary>
        private ulong _lastConnectedAddress;
        /// <summary>
        /// 指示是否正在进行重连尝试。
        /// </summary>
        private bool _isReconnecting;
        /// <summary>
        /// 用于控制重连尝试的并发信号量。
        /// </summary>
        private readonly SemaphoreSlim _reconnectionSemaphore = new SemaphoreSlim(1, 1);
        /// <summary>
        /// 用于取消正在进行的连接操作的 CancellationTokenSource。
        /// </summary>
        private CancellationTokenSource? _connectionCts;

        private readonly ISettingsService _settingsService;
        private readonly ILogger _logger;
        private readonly IEventAggregator _eventAggregator;
        private readonly IControllerProfile _controllerProfile;

        /// <summary>
        /// 当接收到新的控制器数据时触发的事件。
        /// </summary>
        public event EventHandler<ControllerData>? DataReceived;
        /// <summary>
        /// 当蓝牙连接状态发生变化时触发的事件。
        /// </summary>
        public event EventHandler<BluetoothConnectionStatus>? ConnectionStatusChanged;

        /// <summary>
        /// 获取当前蓝牙连接状态。如果设备已连接则返回 true。
        /// </summary>
        public bool IsConnected => _device?.ConnectionStatus == BluetoothConnectionStatus.Connected;

        /// <summary>
        /// BluetoothService 的构造函数。
        /// </summary>
        /// <param name="settingsService">设置服务，用于获取重连参数等配置。</param>
        /// <param name="logger">日志服务，用于记录日志。</param>
        /// <param name="eventAggregator">事件聚合器服务，用于发布控制器数据事件。</param>
        /// <param name="controllerProfile">控制器配置文件，包含协议相关的常量。</param>
        public BluetoothService(ISettingsService settingsService, ILogger logger, IEventAggregator eventAggregator, IControllerProfile controllerProfile)
        {
            _settingsService = settingsService ?? throw new ArgumentNullException(nameof(settingsService));
            _logger = logger ?? throw new ArgumentNullException(nameof(logger));
            _eventAggregator = eventAggregator ?? throw new ArgumentNullException(nameof(eventAggregator));
            _controllerProfile = controllerProfile ?? throw new ArgumentNullException(nameof(controllerProfile));
        }

        /// <summary>
        /// 异步连接到指定的蓝牙 LE 设备。
        /// </summary>
        /// <param name="bluetoothAddress">要连接的蓝牙设备的地址。</param>
        /// <param name="timeoutMs">连接超时时间（毫秒）。</param>
        /// <returns>表示异步连接操作的任务。</param>
        /// <exception cref="TimeoutException">如果连接在指定时间内超时则抛出。</exception>
        /// <exception cref="Exception">连接过程中发生其他错误时抛出。</exception>
        public async Task ConnectAsync(ulong bluetoothAddress, int timeoutMs = 10000)
        {
            _logger.LogInfo($"开始连接到设备地址: {bluetoothAddress}", nameof(BluetoothService));
            try
            {
                _lastConnectedAddress = bluetoothAddress;
                _connectionCts?.Cancel();
                _connectionCts = new CancellationTokenSource();

                // Add timeout
                using var timeoutCts = new CancellationTokenSource(timeoutMs);
                using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(timeoutCts.Token, _connectionCts.Token);

                _logger.LogInfo($"尝试从地址获取设备: {bluetoothAddress}", nameof(BluetoothService));
                _device = await BluetoothLEDevice.FromBluetoothAddressAsync(bluetoothAddress);

                if (_device == null)
                {
                    _logger.LogError($"无法从地址获取设备: {bluetoothAddress}", nameof(BluetoothService));
                    throw new Exception("无法连接到设备");
                }
                _logger.LogInfo($"成功获取到设备对象. ConnectionStatus: {_device.ConnectionStatus}", nameof(BluetoothService));

                // Register connection status change event
                _device.ConnectionStatusChanged += Device_ConnectionStatusChanged;

                await InitializeServicesAsync(linkedCts.Token);

                // After successful connection, trigger ConnectionStatusChanged event again to ensure MainViewModel updates status
                // This can help rule out event loss or delay issues
                Device_ConnectionStatusChanged(_device, null);

                _logger.LogInfo($"连接到设备 {bluetoothAddress} 成功.", nameof(BluetoothService));
            }
            catch (OperationCanceledException ex)
            {
                _logger.LogError($"连接到设备超时: {bluetoothAddress}", nameof(BluetoothService), ex);
                throw new TimeoutException("连接超时");
            }
            catch (Exception ex)
            {
                _logger.LogError($"连接到设备 {bluetoothAddress} 失败.", nameof(BluetoothService), ex);
                _device?.Dispose();
                _device = null;
                throw;
            }
        }

        /// <summary>
        /// 处理蓝牙设备连接状态变化的事件。
        /// 当设备断开连接时，尝试重新连接。
        /// </summary>
        /// <param name="sender">事件发送者（BluetoothLEDevice 实例）。</param>
        /// <param name="args">事件参数。</param>
        private async void Device_ConnectionStatusChanged(BluetoothLEDevice sender, object? args)
        {
            var status = sender.ConnectionStatus;
            _logger.LogInfo($"ConnectionStatusChanged 事件触发. 新状态: {status}", nameof(BluetoothService));
            ConnectionStatusChanged?.Invoke(this, status);

            if (status == BluetoothConnectionStatus.Disconnected)
            {
                _logger.LogWarning("检测到断开连接，尝试重新连接...", nameof(BluetoothService));
                await AttemptReconnectAsync();
            }
        }

        /// <summary>
        /// 尝试重新连接到上次连接的蓝牙设备。
        /// </summary>
        /// <returns>表示异步重连操作的任务。</returns>
        private async Task AttemptReconnectAsync()
        {
            if (_isReconnecting || _lastConnectedAddress == 0)
            {
                if (_isReconnecting) _logger.LogInfo("正在重连，跳过新的重连尝试.", nameof(BluetoothService));
                if (_lastConnectedAddress == 0) _logger.LogWarning("无上次连接地址，跳过重连.", nameof(BluetoothService));
                return;
            }

            _logger.LogInfo("开始尝试重新连接流程.", nameof(BluetoothService));
            try
            {
                await _reconnectionSemaphore.WaitAsync();
                _isReconnecting = true;

                for (int attempt = 0; attempt < _settingsService.MaxReconnectAttempts; attempt++)
                {
                    try
                    {
                        _logger.LogInfo($"尝试重新连接，第 {attempt + 1} 次，地址: {_lastConnectedAddress}", nameof(BluetoothService));
                        await ConnectAsync(_lastConnectedAddress);
                        _logger.LogInfo("重新连接成功", nameof(BluetoothService));
                        return;
                    }
                    catch (Exception ex)
                    {
                        _logger.LogError($"重新连接失败.", nameof(BluetoothService), ex);
                        if (attempt < _settingsService.MaxReconnectAttempts - 1)
                        {
                            _logger.LogInfo($"等待 {_settingsService.ReconnectDelayMs}ms 后再次尝试.", nameof(BluetoothService));
                            await Task.Delay(_settingsService.ReconnectDelayMs);
                        }
                    }
                }
                _logger.LogWarning($"达到最大重连次数 ({_settingsService.MaxReconnectAttempts})，停止重连.", nameof(BluetoothService));
            }
            finally
            {
                _isReconnecting = false;
                _reconnectionSemaphore.Release();
                _logger.LogInfo("尝试重新连接流程结束.", nameof(BluetoothService));
            }
        }

        /// <summary>
        /// 初始化蓝牙服务和特征值。
        /// 发现控制器服务及其设置和数据特征值。
        /// </summary>
        /// <param name="cancellationToken">用于取消操作的 CancellationToken。</param>
        /// <returns>表示异步操作的任务。</returns>
        /// <exception cref="Exception">如果设备未连接或未找到必要的服务/特征值则抛出。</exception>
        private async Task InitializeServicesAsync(CancellationToken cancellationToken)
        {
            _logger.LogInfo("开始 InitializeServicesAsync.", nameof(BluetoothService));
            if (_device == null)
            {
                _logger.LogError("InitializeServicesAsync: 设备未连接.", nameof(BluetoothService));
                throw new Exception("设备未连接");
            }

            var result = await _device.GetGattServicesAsync();
            cancellationToken.ThrowIfCancellationRequested();

            if (result.Status != GattCommunicationStatus.Success)
            {
                _logger.LogError($"GetGattServicesAsync 失败: {result.Status}", nameof(BluetoothService));
                throw new Exception("无法获取GATT服务");
            }
            _logger.LogInfo($"成功获取 GATT 服务 ({result.Services.Count} 个).", nameof(BluetoothService));

            foreach (var service in result.Services)
            {
                if (service.Uuid == _controllerProfile.ControllerServiceUuid)
                {
                    var characteristicsResult = await service.GetCharacteristicsAsync();
                    cancellationToken.ThrowIfCancellationRequested();

                    if (characteristicsResult.Status == GattCommunicationStatus.Success)
                    {
                        foreach (var characteristic in characteristicsResult.Characteristics)
                        {
                            if (characteristic.Uuid == _controllerProfile.ControllerSetupCharacteristicUuid)
                            {
                                _setupCharacteristic = characteristic;
                            }
                            else if (characteristic.Uuid == _controllerProfile.ControllerDataCharacteristicUuid)
                            {
                                _dataCharacteristic = characteristic;
                                cancellationToken.ThrowIfCancellationRequested();
                            }
                        }
                    }
                    else
                    {
                        _logger.LogError($"GetCharacteristicsAsync 失败: {characteristicsResult.Status}", nameof(BluetoothService));
                        throw new Exception("无法获取特征值。");
                    }
                    break; // Found the service, no need to check others
                }
            }

            if (_setupCharacteristic == null)
            {
                _logger.LogError("无法找到 Setup Characteristic.", nameof(BluetoothService));
                throw new Exception("无法找到 Setup Characteristic。");
            }

            if (_dataCharacteristic == null)
            {
                _logger.LogError("无法找到 Data Characteristic.", nameof(BluetoothService));
                throw new Exception("无法找到 Data Characteristic。");
            }
            _logger.LogInfo("成功找到所有特征值.", nameof(BluetoothService));


            await SubscribeToNotificationsAsync();
            await InitializeControllerAsync();
            await OptimizeConnectionParametersAsync();
        }

        /// <summary>
        /// 订阅控制器数据特征值的通知。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        private async Task SubscribeToNotificationsAsync()
        {
            if (_dataCharacteristic == null)
            {
                _logger.LogError("Data Characteristic 未初始化。", nameof(BluetoothService));
                throw new InvalidOperationException("Data Characteristic 未初始化。");
            }

            // 设置通知属性
            var status = await _dataCharacteristic.WriteClientCharacteristicConfigurationDescriptorAsync(GattClientCharacteristicConfigurationDescriptorValue.Notify);
            if (status != GattCommunicationStatus.Success)
            {
                _logger.LogError($"无法订阅 Data Characteristic 通知: {status}", nameof(BluetoothService));
                throw new Exception("无法订阅数据通知。");
            }
            _dataCharacteristic.ValueChanged += DataCharacteristic_ValueChanged;
            _logger.LogInfo("已订阅 Data Characteristic 通知。", nameof(BluetoothService));
        }

        /// <summary>
        /// 发送初始化命令到控制器。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        private async Task InitializeControllerAsync()
        {
            _logger.LogInfo("发送控制器初始化命令...", nameof(BluetoothService));
            await SendCommandAsync(_controllerProfile.CmdInit1);
            await Task.Delay(_controllerProfile.CommandDelayMs); // 等待响应
            await SendCommandAsync(_controllerProfile.CmdInit2);
            await Task.Delay(_controllerProfile.CommandDelayMs);
            await SendCommandAsync(_controllerProfile.CmdInit3);
            await Task.Delay(_controllerProfile.CommandDelayMs);
            await SendCommandAsync(_controllerProfile.CmdInit4);
            await Task.Delay(_controllerProfile.CommandDelayMs);
            _logger.LogInfo("控制器初始化命令发送完成。", nameof(BluetoothService));
        }

        /// <summary>
        /// 尝试优化蓝牙连接参数。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        private async Task OptimizeConnectionParametersAsync()
        {
            _logger.LogInfo("尝试优化连接参数...", nameof(BluetoothService));
            await SendCommandAsync(_controllerProfile.CmdOptimizeConnection);
            _logger.LogInfo("连接参数优化命令已发送。", nameof(BluetoothService));
        }

        /// <summary>
        /// 异步发送原始字节数据到设置特征值。
        /// </summary>
        /// <param name="data">要发送的字节数组。</param>
        /// <param name="repeat">发送次数。</param>
        /// <returns>表示异步操作的任务。</returns>
        public async Task SendDataAsync(byte[] data, int repeat = 1)
        {
            if (_setupCharacteristic == null)
            {
                _logger.LogError("Setup Characteristic 未初始化。", nameof(BluetoothService));
                throw new InvalidOperationException("Setup Characteristic 未初始化。");
            }

            for (int i = 0; i < repeat; i++)
            {
                var writer = new DataWriter();
                writer.WriteBytes(data);
                var status = await _setupCharacteristic.WriteValueAsync(writer.DetachBuffer());
                if (status != GattCommunicationStatus.Success)
                {
                    _logger.LogError($"发送数据失败: {status}", nameof(BluetoothService));
                    throw new Exception($"发送数据失败: {status}");
                }
                _logger.LogDebug($"数据已发送: {BitConverter.ToString(data)}", nameof(BluetoothService));
            }
        }

        /// <summary>
        /// 异步发送控制器命令到设置特征值。
        /// 这是 SendDataAsync 的私有包装器，用于内部命令。
        /// </summary>
        /// <param name="command">要发送的命令字节数组。</param>
        /// <param name="repeat">发送次数。</param>
        /// <returns>表示异步操作的任务。</returns>
        private Task SendCommandAsync(byte[] command, int repeat = 1)
        {
            return SendDataAsync(command, repeat);
        }

        /// <summary>
        /// 断开与当前连接的蓝牙 LE 设备的连接。
        /// </summary>
        public void Disconnect()
        {
            _connectionCts?.Cancel(); // Cancel any ongoing connection attempts
            _connectionCts?.Dispose();
            _connectionCts = null;

            if (_device != null)
            {
                _device.ConnectionStatusChanged -= Device_ConnectionStatusChanged;

                if (_dataCharacteristic != null)
                {
                    _dataCharacteristic.ValueChanged -= DataCharacteristic_ValueChanged;
                    _dataCharacteristic = null;
                }

                _setupCharacteristic = null;
                _device.Dispose();
                _device = null;
                _logger.LogInfo("已断开蓝牙设备连接并释放资源。", nameof(BluetoothService));
            }
        }

        /// <summary>
        /// 处理蓝牙 GATT 特征值的数据变化通知。
        /// 解析接收到的字节数据并更新 ControllerData。
        /// </summary>
        /// <param name="sender">事件发送者（GattCharacteristic 实例）。</param>
        /// <param name="args">包含新数据值的事件参数。</param>
        private void DataCharacteristic_ValueChanged(GattCharacteristic sender, GattValueChangedEventArgs args)
        {
            var reader = DataReader.FromBuffer(args.CharacteristicValue);
            var byteArray = new byte[args.CharacteristicValue.Length];
            reader.ReadBytes(byteArray);

            // Dispatch to UI thread to process the data and update UI
            ProcessDataAsync(byteArray);
        }

        /// <summary>
        /// 异步处理接收到的原始字节数组数据，将其解析为 ControllerData 对象，并触发 DataReceived 事件。
        /// </summary>
        /// <param name="byteArray">接收到的原始数据字节数组。</param>
        private void ProcessDataAsync(byte[] byteArray)
        {
            if (byteArray.Length != _controllerProfile.ExpectedPacketLength)
            {
                _logger.LogWarning($"接收到异常长度的数据包 ({byteArray.Length} bytes). 期望长度: {_controllerProfile.ExpectedPacketLength} bytes.", nameof(BluetoothService));
                return; // Optionally handle incomplete packets or log an error
            }

            // Parse data packet
            var controllerData = new ControllerData
            {
                TouchpadX = BitConverter.ToUInt16(byteArray, _controllerProfile.TouchpadXOffset),
                TouchpadY = BitConverter.ToUInt16(byteArray, _controllerProfile.TouchpadYOffset),

                // Parse button state
                // Button state byte position in packet
                TriggerButton = (byteArray[_controllerProfile.ButtonStateOffset] & _controllerProfile.TriggerButtonMask) != 0,
                HomeButton = (byteArray[_controllerProfile.ButtonStateOffset] & _controllerProfile.HomeButtonMask) != 0,
                BackButton = (byteArray[_controllerProfile.ButtonStateOffset] & _controllerProfile.BackButtonMask) != 0,
                TouchpadButton = (byteArray[_controllerProfile.ButtonStateOffset] & _controllerProfile.TouchpadButtonMask) != 0,
                VolumeUpButton = (byteArray[_controllerProfile.ButtonStateOffset] & _controllerProfile.VolumeUpButtonMask) != 0,
                VolumeDownButton = (byteArray[_controllerProfile.ButtonStateOffset] & _controllerProfile.VolumeDownButtonMask) != 0,

                // Accelerometer data
                AccelX = BitConverter.ToSingle(byteArray, _controllerProfile.AccelXOffset),
                AccelY = BitConverter.ToSingle(byteArray, _controllerProfile.AccelYOffset),
                AccelZ = BitConverter.ToSingle(byteArray, _controllerProfile.AccelZOffset),

                // Gyroscope data
                GyroX = BitConverter.ToSingle(byteArray, _controllerProfile.GyroXOffset),
                GyroY = BitConverter.ToSingle(byteArray, _controllerProfile.GyroYOffset),
                GyroZ = BitConverter.ToSingle(byteArray, _controllerProfile.GyroZOffset),
            };

            // Trigger the DataReceived event
            DataReceived?.Invoke(this, controllerData);
        }

        /// <summary>
        /// Simulate receiving controller data, for testing purposes.
        /// </summary>
        /// <param name="data">The ControllerData to simulate.</param>
        public void SimulateData(ControllerData data)
        {
            DataReceived?.Invoke(this, data);
        }

        /// <summary>
        /// Releases the resources occupied by the BluetoothService instance.
        /// </summary>
        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        /// <summary>
        /// Releases the unmanaged resources occupied by the BluetoothService instance。
        /// </summary>
        /// <param name="disposing">True to release both managed and unmanaged resources; false to release only unmanaged resources.</param>
        protected virtual void Dispose(bool disposing)
        {
            if (disposing)
            {
                Disconnect();
                _reconnectionSemaphore.Dispose();
            }
        }
    }
}