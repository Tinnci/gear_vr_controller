using System;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Threading.Tasks;
using Windows.Devices.Bluetooth;
using Windows.Devices.Bluetooth.GenericAttributeProfile;
using Windows.Devices.Enumeration;
using Windows.Storage.Streams;
using GearVRController.Models;
using System.Threading;
using GearVRController.Services.Interfaces;
using System.Diagnostics;
using System.IO;

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
        /// Gear VR 控制器服务的 UUID。
        /// </summary>
        private static readonly Guid CONTROLLER_SERVICE_UUID = new Guid("4f63756c-7573-2054-6872-65656d6f7465");
        /// <summary>
        /// 用于控制器设置的特征值 UUID。
        /// </summary>
        private static readonly Guid CONTROLLER_SETUP_CHARACTERISTIC_UUID = new Guid("c8c51726-81bc-483b-a052-f7a14ea3d282");
        /// <summary>
        /// 用于控制器数据传输的特征值 UUID。
        /// </summary>
        private static readonly Guid CONTROLLER_DATA_CHARACTERISTIC_UUID = new Guid("c8c51726-81bc-483b-a052-f7a14ea3d281");

        // 控制器初始化命令常量
        /// <summary>
        /// 初始化序列的第一步命令。
        /// </summary>
        private static readonly byte[] CMD_INIT_1 = new byte[] { 0x01, 0x00 };
        /// <summary>
        /// 初始化序列的第二步命令。
        /// </summary>
        private static readonly byte[] CMD_INIT_2 = new byte[] { 0x06, 0x00 };
        /// <summary>
        /// 初始化序列的第三步命令。
        /// </summary>
        private static readonly byte[] CMD_INIT_3 = new byte[] { 0x07, 0x00 };
        /// <summary>
        /// 初始化序列的第四步命令。
        /// </summary>
        private static readonly byte[] CMD_INIT_4 = new byte[] { 0x08, 0x00 };
        /// <summary>
        /// 用于优化 BLE 连接参数（例如间隔时间）的命令。
        /// </summary>
        private static readonly byte[] CMD_OPTIMIZE_CONNECTION = new byte[] { 0x0A, 0x02 };

        // 数据包结构常量
        /// <summary>
        /// 期望的控制器数据包长度。
        /// </summary>
        private const int EXPECTED_PACKET_LENGTH = 60;
        /// <summary>
        /// 触摸板X坐标在数据包中的偏移量。
        /// </summary>
        private const int TOUCHPAD_X_OFFSET = 54;
        /// <summary>
        /// 触摸板Y坐标在数据包中的偏移量。
        /// </summary>
        private const int TOUCHPAD_Y_OFFSET = 56;
        /// <summary>
        /// 按钮状态在数据包中的偏移量。
        /// </summary>
        private const int BUTTON_STATE_OFFSET = 2;
        /// <summary>
        /// 加速度计X轴数据在数据包中的偏移量。
        /// </summary>
        private const int ACCEL_X_OFFSET = 6;
        /// <summary>
        /// 加速度计Y轴数据在数据包中的偏移量。
        /// </summary>
        private const int ACCEL_Y_OFFSET = 8;
        /// <summary>
        /// 加速度计Z轴数据在数据包中的偏移量。
        /// </summary>
        private const int ACCEL_Z_OFFSET = 10;
        /// <summary>
        /// 陀螺仪X轴数据在数据包中的偏移量。
        /// </summary>
        private const int GYRO_X_OFFSET = 12;
        /// <summary>
        /// 陀螺仪Y轴数据在数据包中的偏移量。
        /// </summary>
        private const int GYRO_Y_OFFSET = 14;
        /// <summary>
        /// 陀螺仪Z轴数据在数据包中的偏移量。
        /// </summary>
        private const int GYRO_Z_OFFSET = 16;

        // 按钮位掩码
        /// <summary>
        /// 触摸板按钮的位掩码。
        /// </summary>
        private const byte TOUCHPAD_BUTTON_MASK = 0b00000001;
        /// <summary>
        /// Home 按钮的位掩码。
        /// </summary>
        private const byte HOME_BUTTON_MASK = 0b00000010;
        /// <summary>
        /// 扳机按钮的位掩码。
        /// </summary>
        private const byte TRIGGER_BUTTON_MASK = 0b00000100;
        /// <summary>
        /// 返回按钮的位掩码。
        /// </summary>
        private const byte BACK_BUTTON_MASK = 0b00001000;
        /// <summary>
        /// 音量上键的位掩码。
        /// </summary>
        private const byte VOLUME_UP_BUTTON_MASK = 0b00010000;
        /// <summary>
        /// 音量下键的位掩码。
        /// </summary>
        private const byte VOLUME_DOWN_BUTTON_MASK = 0b00100000;

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
        public BluetoothService(ISettingsService settingsService)
        {
            _settingsService = settingsService;
        }

        /// <summary>
        /// 异步连接到指定的蓝牙 LE 设备。
        /// </summary>
        /// <param name="bluetoothAddress">要连接的蓝牙设备的地址。</param>
        /// <param name="timeoutMs">连接超时时间（毫秒）。</param>
        /// <returns>表示异步连接操作的任务。</returns>
        /// <exception cref="TimeoutException">如果连接在指定时间内超时则抛出。</exception>
        /// <exception cref="Exception">连接过程中发生其他错误时抛出。</exception>
        public async Task ConnectAsync(ulong bluetoothAddress, int timeoutMs = 10000)
        {
            Debug.WriteLine($"[BluetoothService] 开始连接到设备地址: {bluetoothAddress}");
            try
            {
                _lastConnectedAddress = bluetoothAddress;
                _connectionCts?.Cancel();
                _connectionCts = new CancellationTokenSource();

                // 添加超时
                using var timeoutCts = new CancellationTokenSource(timeoutMs);
                using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(timeoutCts.Token, _connectionCts.Token);

                Debug.WriteLine($"[BluetoothService] 尝试从地址获取设备: {bluetoothAddress}");
                _device = await BluetoothLEDevice.FromBluetoothAddressAsync(bluetoothAddress);

                if (_device == null)
                {
                    Debug.WriteLine($"[BluetoothService] 无法从地址获取设备: {bluetoothAddress}");
                    throw new Exception("无法连接到设备");
                }
                Debug.WriteLine($"[BluetoothService] 成功获取到设备对象. ConnectionStatus: {_device.ConnectionStatus}");

                // 注册连接状态变化事件
                _device.ConnectionStatusChanged += Device_ConnectionStatusChanged;

                await InitializeServicesAsync(linkedCts.Token);

                // 连接成功后，再次触发 ConnectionStatusChanged 事件，确保 MainViewModel 更新状态
                // 这可以帮助排除事件丢失或延迟的问题
                Device_ConnectionStatusChanged(_device, null);

                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 连接到设备 {bluetoothAddress} 成功.");
            }
            catch (OperationCanceledException)
            {
                Debug.WriteLine($"[BluetoothService] 连接到设备超时: {bluetoothAddress}");
                throw new TimeoutException("连接超时");
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"[BluetoothService] 连接到设备 {bluetoothAddress} 失败: {ex.Message}");
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
            Debug.WriteLine($"[BluetoothService] ConnectionStatusChanged 事件触发. 新状态: {status}");
            ConnectionStatusChanged?.Invoke(this, status);

            if (status == BluetoothConnectionStatus.Disconnected)
            {
                Debug.WriteLine("[BluetoothService] 检测到断开连接，尝试重新连接...");
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
                if (_isReconnecting) Debug.WriteLine("[BluetoothService] 正在重连，跳过新的重连尝试.");
                if (_lastConnectedAddress == 0) Debug.WriteLine("[BluetoothService] 无上次连接地址，跳过重连.");
                return;
            }

            Debug.WriteLine("[BluetoothService] 开始尝试重新连接流程.");
            try
            {
                await _reconnectionSemaphore.WaitAsync();
                _isReconnecting = true;

                for (int attempt = 0; attempt < _settingsService.MaxReconnectAttempts; attempt++)
                {
                    try
                    {
                        System.Diagnostics.Debug.WriteLine($"[BluetoothService] 尝试重新连接，第 {attempt + 1} 次，地址: {_lastConnectedAddress}");
                        await ConnectAsync(_lastConnectedAddress);
                        System.Diagnostics.Debug.WriteLine("[BluetoothService] 重新连接成功");
                        return;
                    }
                    catch (Exception ex)
                    {
                        System.Diagnostics.Debug.WriteLine($"[BluetoothService] 重新连接失败: {ex.Message}");
                        if (attempt < _settingsService.MaxReconnectAttempts - 1)
                        {
                            System.Diagnostics.Debug.WriteLine($"[BluetoothService] 等待 {_settingsService.ReconnectDelayMs}ms 后再次尝试.");
                            await Task.Delay(_settingsService.ReconnectDelayMs);
                        }
                    }
                }
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 达到最大重连次数 ({_settingsService.MaxReconnectAttempts})，停止重连.");
            }
            finally
            {
                _isReconnecting = false;
                _reconnectionSemaphore.Release();
                Debug.WriteLine("[BluetoothService] 尝试重新连接流程结束.");
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
            Debug.WriteLine("[BluetoothService] 开始 InitializeServicesAsync.");
            if (_device == null)
            {
                Debug.WriteLine("[BluetoothService] InitializeServicesAsync: 设备未连接.");
                throw new Exception("设备未连接");
            }

            var result = await _device.GetGattServicesAsync();
            cancellationToken.ThrowIfCancellationRequested();

            if (result.Status != GattCommunicationStatus.Success)
            {
                Debug.WriteLine($"[BluetoothService] GetGattServicesAsync 失败: {result.Status}");
                throw new Exception("无法获取GATT服务");
            }
            Debug.WriteLine($"[BluetoothService] 成功获取 GATT 服务 ({result.Services.Count} 个).");

            foreach (var service in result.Services)
            {
                if (service.Uuid == CONTROLLER_SERVICE_UUID)
                {
                    var characteristicsResult = await service.GetCharacteristicsAsync();
                    cancellationToken.ThrowIfCancellationRequested();

                    if (characteristicsResult.Status == GattCommunicationStatus.Success)
                    {
                        foreach (var characteristic in characteristicsResult.Characteristics)
                        {
                            if (characteristic.Uuid == CONTROLLER_SETUP_CHARACTERISTIC_UUID)
                            {
                                _setupCharacteristic = characteristic;
                            }
                            else if (characteristic.Uuid == CONTROLLER_DATA_CHARACTERISTIC_UUID)
                            {
                                _dataCharacteristic = characteristic;
                                cancellationToken.ThrowIfCancellationRequested();
                            }
                        }
                    }
                }
            }

            if (_setupCharacteristic == null || _dataCharacteristic == null)
            {
                System.Diagnostics.Debug.WriteLine("[BluetoothService] 未找到必要的特征值.");
                throw new Exception("未找到必要的特征值");
            }
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 已找到所有必要的特征值.");

            await InitializeControllerAsync();
            await SubscribeToNotificationsAsync();
        }

        /// <summary>
        /// 订阅数据特征值的通知。
        /// 这使得控制器能够向应用程序发送实时数据更新。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        /// <exception cref="Exception">如果数据特征值未初始化或订阅失败则抛出。</exception>
        private async Task SubscribeToNotificationsAsync()
        {
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 开始 SubscribeToNotificationsAsync.");
            if (_dataCharacteristic == null)
            {
                Debug.WriteLine("[BluetoothService] SubscribeToNotificationsAsync: 数据特征值未初始化.");
                throw new Exception("数据特征值未初始化");
            }

            // 启用通知
            var status = await _dataCharacteristic.WriteClientCharacteristicConfigurationDescriptorAsync(
                GattClientCharacteristicConfigurationDescriptorValue.Notify);

            if (status == GattCommunicationStatus.Success)
            {
                _dataCharacteristic.ValueChanged += DataCharacteristic_ValueChanged;
            }
            else
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 订阅数据特征值通知失败: {status}");
                throw new Exception($"无法订阅数据特征值通知: {status}");
            }
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 订阅数据特征值通知成功.");
        }

        /// <summary>
        /// 初始化控制器，发送一系列初始化命令。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        /// <exception cref="Exception">如果设置特征值未初始化或命令发送失败则抛出。</exception>
        private async Task InitializeControllerAsync()
        {
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 开始 InitializeControllerAsync.");
            if (_setupCharacteristic == null)
            {
                Debug.WriteLine("[BluetoothService] InitializeControllerAsync: 设置特征值未初始化.");
                throw new Exception("设置特征值未初始化");
            }

            // 发送初始化命令序列
            await SendCommandAsync(CMD_INIT_1);
            await Task.Delay(50); // 短暂延迟，确保命令被控制器处理
            await SendCommandAsync(CMD_INIT_2);
            await Task.Delay(50);
            await SendCommandAsync(CMD_INIT_3);
            await Task.Delay(50);
            await SendCommandAsync(CMD_INIT_4);
            await Task.Delay(50);

            // 优化连接参数
            await OptimizeConnectionParametersAsync();

            System.Diagnostics.Debug.WriteLine("[BluetoothService] 控制器初始化完成.");
        }

        /// <summary>
        /// 优化 BLE 连接参数，以提高数据传输效率。
        /// </summary>
        /// <returns>表示异步操作的任务。</returns>
        /// <exception cref="Exception">如果设置特征值未初始化或命令发送失败则抛出。</exception>
        private async Task OptimizeConnectionParametersAsync()
        {
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 尝试优化连接参数.");
            if (_setupCharacteristic == null)
            {
                Debug.WriteLine("[BluetoothService] OptimizeConnectionParametersAsync: 设置特征值未初始化.");
                throw new Exception("设置特征值未初始化");
            }

            await SendCommandAsync(CMD_OPTIMIZE_CONNECTION);
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 连接参数优化命令已发送.");
        }

        /// <summary>
        /// 异步发送数据到数据特征值。
        /// </summary>
        /// <param name="data">要发送的数据字节数组。</param>
        /// <param name="repeat">重复发送数据的次数（默认为1）。</param>
        /// <returns>表示异步操作的任务。</returns>
        /// <exception cref="Exception">如果数据特征值未初始化或数据写入失败则抛出。</exception>
        public async Task SendDataAsync(byte[] data, int repeat = 1)
        {
            if (_dataCharacteristic == null)
            {
                throw new Exception("数据特征值未初始化，无法发送数据");
            }

            var writer = new DataWriter();
            writer.WriteBytes(data);
            var buffer = writer.DetachBuffer();

            for (int i = 0; i < repeat; i++)
            {
                var status = await _dataCharacteristic.WriteValueAsync(buffer);
                if (status != GattCommunicationStatus.Success)
                {
                    System.Diagnostics.Debug.WriteLine($"[BluetoothService] 发送数据失败: {status}");
                    throw new Exception($"发送数据失败: {status}");
                }
            }
            Debug.WriteLine($"[BluetoothService] 数据已发送 (重复 {repeat} 次).");
        }

        /// <summary>
        /// 异步发送命令到设置特征值。
        /// </summary>
        /// <param name="command">要发送的命令字节数组。</param>
        /// <param name="repeat">重复发送命令的次数（默认为1）。</param>
        /// <returns>表示异步操作的任务。</returns>
        private async Task SendCommandAsync(byte[] command, int repeat = 1)
        {
            if (_setupCharacteristic == null)
            {
                throw new Exception("设置特征值未初始化，无法发送命令");
            }
            var writer = new DataWriter();
            writer.WriteBytes(command);
            var buffer = writer.DetachBuffer();
            for (int i = 0; i < repeat; i++)
            {
                var status = await _setupCharacteristic.WriteValueAsync(buffer);
                if (status != GattCommunicationStatus.Success)
                {
                    System.Diagnostics.Debug.WriteLine($"[BluetoothService] 发送命令失败: {status}");
                    throw new Exception($"发送命令失败: {status}");
                }
            }
        }

        /// <summary>
        /// 断开与当前连接的蓝牙设备的连接，并释放相关资源。
        /// 取消事件订阅，停止重连尝试，并清理 BluetoothLEDevice 实例。
        /// </summary>
        public void Disconnect()
        {
            try
            {
                if (_device != null)
                {
                    // Unsubscribe from events
                    _device.ConnectionStatusChanged -= Device_ConnectionStatusChanged;
                    if (_dataCharacteristic != null)
                    {
                        _dataCharacteristic.ValueChanged -= DataCharacteristic_ValueChanged;
                    }

                    _device.Dispose();
                    _device = null;
                }

                _setupCharacteristic = null;
                _dataCharacteristic = null;
                _lastConnectedAddress = 0; // Clear last connected address
                _isReconnecting = false;

                // Dispose CancellationTokenSource
                _connectionCts?.Cancel();
                _connectionCts?.Dispose();
                _connectionCts = null;

                Debug.WriteLine("[BluetoothService] 设备已断开连接并清理资源.");
                ConnectionStatusChanged?.Invoke(this, BluetoothConnectionStatus.Disconnected); // Explicitly notify disconnected
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 断开连接异常: {ex}");
            }
        }

        /// <summary>
        /// 处理数据特征值的数据变化通知。
        /// 从接收到的数据中解析控制器状态，并触发 DataReceived 事件。
        /// </summary>
        /// <param name="sender">事件发送者。</param>
        /// <param name="args">包含新数据值的事件参数。</param>
        private void DataCharacteristic_ValueChanged(GattCharacteristic sender, GattValueChangedEventArgs args)
        {
            var reader = DataReader.FromBuffer(args.CharacteristicValue);
            var byteArray = new byte[reader.UnconsumedBufferLength];
            reader.ReadBytes(byteArray);

            // 异步处理数据以避免阻塞 UI 线程或蓝牙回调。
            Task.Run(() => ProcessDataAsync(byteArray));
        }

        /// <summary>
        /// 异步处理原始控制器数据包，解析出控制器数据模型。
        /// </summary>
        /// <param name="byteArray">包含原始控制器数据的数据包。</param>
        private void ProcessDataAsync(byte[] byteArray)
        {
            if (byteArray.Length != EXPECTED_PACKET_LENGTH)
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 接收到的数据包长度不符合预期: {byteArray.Length} (预期 {EXPECTED_PACKET_LENGTH})。数据包将被丢弃。");
                return; // 直接丢弃不符合长度要求的数据包
            }

            try
            {
                // 解析按钮状态
                byte buttonState = byteArray[BUTTON_STATE_OFFSET];
                bool touchpadButton = (buttonState & TOUCHPAD_BUTTON_MASK) != 0;
                bool homeButton = (buttonState & HOME_BUTTON_MASK) != 0;
                bool triggerButton = (buttonState & TRIGGER_BUTTON_MASK) != 0;
                bool backButton = (buttonState & BACK_BUTTON_MASK) != 0;
                bool volumeUpButton = (buttonState & VOLUME_UP_BUTTON_MASK) != 0;
                bool volumeDownButton = (buttonState & VOLUME_DOWN_BUTTON_MASK) != 0;

                // 解析触摸板坐标（假设为ushort，需要转换）
                // 注意：Gear VR 控制器触摸板的原始范围通常是 0-1023
                ushort touchpadXRaw = BitConverter.ToUInt16(byteArray, TOUCHPAD_X_OFFSET);
                ushort touchpadYRaw = BitConverter.ToUInt16(byteArray, TOUCHPAD_Y_OFFSET);

                // 解析加速度计数据（假设为short，需要转换）
                short accelXRaw = BitConverter.ToInt16(byteArray, ACCEL_X_OFFSET);
                short accelYRaw = BitConverter.ToInt16(byteArray, ACCEL_Y_OFFSET);
                short accelZRaw = BitConverter.ToInt16(byteArray, ACCEL_Z_OFFSET);

                // 解析陀螺仪数据（假设为short，需要转换）
                short gyroXRaw = BitConverter.ToInt16(byteArray, GYRO_X_OFFSET);
                short gyroYRaw = BitConverter.ToInt16(byteArray, GYRO_Y_OFFSET);
                short gyroZRaw = BitConverter.ToInt16(byteArray, GYRO_Z_OFFSET);

                // 创建 ControllerData 模型
                var controllerData = new ControllerData
                {
                    TouchpadButton = touchpadButton,
                    HomeButton = homeButton,
                    TriggerButton = triggerButton,
                    BackButton = backButton,
                    VolumeUpButton = volumeUpButton,
                    VolumeDownButton = volumeDownButton,
                    AxisX = touchpadXRaw,
                    AxisY = touchpadYRaw,
                    AccelX = accelXRaw,
                    AccelY = accelYRaw,
                    AccelZ = accelZRaw,
                    GyroX = gyroXRaw,
                    GyroY = gyroYRaw,
                    GyroZ = gyroZRaw
                };

                // 触发数据接收事件
                DataReceived?.Invoke(this, controllerData);
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 处理数据包时发生错误: {ex.Message}");
            }
        }

        /// <summary>
        /// 模拟接收控制器数据，用于测试或调试。
        /// </summary>
        /// <param name="data">要模拟的控制器数据。</param>
        public void SimulateData(ControllerData data)
        {
            DataReceived?.Invoke(this, data);
        }

        /// <summary>
        /// 释放 BluetoothService 实例所持有的所有托管和非托管资源。
        /// </summary>
        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        /// <summary>
        /// 释放 BluetoothService 实例所持有的资源。
        /// </summary>
        /// <param name="disposing">如果为 true，则释放托管资源；否则只释放非托管资源。</param>
        protected virtual void Dispose(bool disposing)
        {
            if (disposing)
            {
                // 释放托管状态 (托管对象)
                Disconnect(); // 调用 Disconnect 来处理事件取消订阅和设备释放
                _reconnectionSemaphore.Dispose(); // 释放 SemaphoreSlim 资源
            }
            // 释放非托管资源 (非托管对象) 并重写终结器
            // 将大型字段设置为 null
        }
    }
}