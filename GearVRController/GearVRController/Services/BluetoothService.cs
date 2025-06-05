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
    public class BluetoothService : IBluetoothService
    {
        // 服务和特征值UUID
        private static readonly Guid CONTROLLER_SERVICE_UUID = new Guid("4f63756c-7573-2054-6872-65656d6f7465");
        private static readonly Guid CONTROLLER_SETUP_CHARACTERISTIC_UUID = new Guid("c8c51726-81bc-483b-a052-f7a14ea3d282");
        private static readonly Guid CONTROLLER_DATA_CHARACTERISTIC_UUID = new Guid("c8c51726-81bc-483b-a052-f7a14ea3d281");

        // 控制器初始化命令常量
        // 这些命令通常用于设置控制器模式、报告频率或启用/禁用特定传感器。
        private static readonly byte[] CMD_INIT_1 = new byte[] { 0x01, 0x00 }; // 可能是初始化序列的第一步
        private static readonly byte[] CMD_INIT_2 = new byte[] { 0x06, 0x00 }; // 可能是初始化序列的第二步
        private static readonly byte[] CMD_INIT_3 = new byte[] { 0x07, 0x00 }; // 可能是初始化序列的第三步
        private static readonly byte[] CMD_INIT_4 = new byte[] { 0x08, 0x00 }; // 可能是初始化序列的第四步
        private static readonly byte[] CMD_OPTIMIZE_CONNECTION = new byte[] { 0x0A, 0x02 }; // 用于优化BLE连接参数，例如间隔时间

        // Define constants for data packet structure
        private const int EXPECTED_PACKET_LENGTH = 60;
        private const int TOUCHPAD_X_OFFSET = 54; // Assuming from previous code
        private const int TOUCHPAD_Y_OFFSET = 56; // Assuming from previous code
        private const int BUTTON_STATE_OFFSET = 2; // Example offset, verify with actual data spec
        private const int ACCEL_X_OFFSET = 6;
        private const int ACCEL_Y_OFFSET = 8;
        private const int ACCEL_Z_OFFSET = 10;
        private const int GYRO_X_OFFSET = 12;
        private const int GYRO_Y_OFFSET = 14;
        private const int GYRO_Z_OFFSET = 16;

        // Button bitmasks (example, verify with actual data spec)
        private const byte TOUCHPAD_BUTTON_MASK = 0b00000001; // Example
        private const byte HOME_BUTTON_MASK = 0b00000010;     // Example
        private const byte TRIGGER_BUTTON_MASK = 0b00000100;  // Example
        private const byte BACK_BUTTON_MASK = 0b00001000;     // Example
        private const byte VOLUME_UP_BUTTON_MASK = 0b00010000;    // Example
        private const byte VOLUME_DOWN_BUTTON_MASK = 0b00100000;  // Example

        private BluetoothLEDevice? _device;
        private GattCharacteristic? _setupCharacteristic;
        private GattCharacteristic? _dataCharacteristic;
        private ulong _lastConnectedAddress;
        private bool _isReconnecting;
        private readonly SemaphoreSlim _reconnectionSemaphore = new SemaphoreSlim(1, 1);
        private CancellationTokenSource? _connectionCts;

        private readonly ISettingsService _settingsService;

        // 事件处理
        public event EventHandler<ControllerData>? DataReceived;
        public event EventHandler<BluetoothConnectionStatus>? ConnectionStatusChanged;

        public bool IsConnected => _device?.ConnectionStatus == BluetoothConnectionStatus.Connected;

        public BluetoothService(ISettingsService settingsService)
        {
            _settingsService = settingsService;
        }

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
                // System.Diagnostics.Debug.WriteLine($"[BluetoothService] 已注册 ConnectionStatusChanged 事件."); // 简化日志

                await InitializeServicesAsync(linkedCts.Token);
                // System.Diagnostics.Debug.WriteLine($"[BluetoothService] InitializeServicesAsync 完成."); // 简化日志

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
            // System.Diagnostics.Debug.WriteLine("[BluetoothService] InitializeServicesAsync 完成."); // 简化日志
        }

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
                Debug.WriteLine("[BluetoothService] 成功订阅数据通知.");
            }
            else
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 订阅数据通知失败: {status}");
                throw new Exception($"订阅数据通知失败: {status}");
            }
        }

        private async Task InitializeControllerAsync()
        {
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 开始 InitializeControllerAsync.");
            if (_setupCharacteristic == null)
            {
                Debug.WriteLine("[BluetoothService] InitializeControllerAsync: 设置特征值未初始化.");
                throw new Exception("设置特征值未初始化");
            }

            // 发送初始化命令
            await SendDataAsync(CMD_INIT_1, 5); // 发送第一个初始化命令，重复5次以确保送达
            await Task.Delay(100); // 短暂延迟
            await SendDataAsync(CMD_INIT_2); // 发送第二个初始化命令
            await SendDataAsync(CMD_INIT_3); // 发送第三个初始化命令
            await SendDataAsync(CMD_INIT_4); // 发送第四个初始化命令

            System.Diagnostics.Debug.WriteLine("[BluetoothService] 控制器初始化命令已发送.");

            // 优化连接参数 (可选)
            await OptimizeConnectionParametersAsync();

            System.Diagnostics.Debug.WriteLine("[BluetoothService] 控制器初始化完成.");
        }

        private async Task OptimizeConnectionParametersAsync()
        {
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 尝试优化连接参数.");
            if (_setupCharacteristic == null)
            {
                Debug.WriteLine("[BluetoothService] OptimizeConnectionParametersAsync: 设置特征值未初始化.");
                return; // 或者抛出异常，取决于错误处理策略
            }

            // 发送优化连接参数命令
            await SendDataAsync(CMD_OPTIMIZE_CONNECTION);
            System.Diagnostics.Debug.WriteLine("[BluetoothService] 优化连接参数命令已发送.");
        }

        public async Task SendDataAsync(byte[] data, int repeat = 1)
        {
            if (_setupCharacteristic == null)
            {
                System.Diagnostics.Debug.WriteLine("[BluetoothService] SendDataAsync: 设置特征值未初始化，无法发送数据.");
                return; // 或者抛出异常
            }

            for (int i = 0; i < repeat; i++)
            {
                try
                {
                    var writer = new DataWriter();
                    writer.WriteBytes(data);
                    var status = await _setupCharacteristic.WriteValueAsync(writer.DetachBuffer());

                    if (status != GattCommunicationStatus.Success)
                    {
                        System.Diagnostics.Debug.WriteLine($"[BluetoothService] 发送数据失败: {status}");
                        // 可以选择在这里抛出异常或进行其他错误处理
                    }
                    else
                    {
                        // System.Diagnostics.Debug.WriteLine("数据发送成功"); // 简化日志
                    }
                }
                catch (Exception ex)
                {
                    System.Diagnostics.Debug.WriteLine($"[BluetoothService] 发送数据异常: {ex.Message}");
                }
            }
        }

        // 添加 SendCommandAsync 方法以供内部调用
        private async Task SendCommandAsync(byte[] command, int repeat = 1)
        {
            await SendDataAsync(command, repeat);
        }

        public void Disconnect()
        {
            _connectionCts?.Cancel(); // 取消任何正在进行的连接或重连尝试
            _connectionCts?.Dispose();
            _connectionCts = null;

            if (_dataCharacteristic != null)
            {
                _dataCharacteristic.ValueChanged -= DataCharacteristic_ValueChanged;
                _dataCharacteristic = null;
            }

            if (_device != null)
            {
                _device.ConnectionStatusChanged -= Device_ConnectionStatusChanged;
                _device.Dispose();
                _device = null;
            }
            Debug.WriteLine("[BluetoothService] 设备已断开连接并清理资源.");
            ConnectionStatusChanged?.Invoke(this, BluetoothConnectionStatus.Disconnected);
        }

        private void DataCharacteristic_ValueChanged(GattCharacteristic sender, GattValueChangedEventArgs args)
        {
            // System.Diagnostics.Debug.WriteLine("[BluetoothService] 收到数据通知."); // 简化日志

            var reader = DataReader.FromBuffer(args.CharacteristicValue);
            var byteArray = new byte[args.CharacteristicValue.Length];
            reader.ReadBytes(byteArray);

            // 为了避免阻塞BLE回调线程，将数据处理卸载到Task.Run
            Task.Run(() => ProcessDataAsync(byteArray));
        }

        private void ProcessDataAsync(byte[] byteArray)
        {
            if (byteArray.Length < EXPECTED_PACKET_LENGTH)
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 接收到的数据包长度不足. 预期: {EXPECTED_PACKET_LENGTH}, 实际: {byteArray.Length}");
                return; // 或者根据需要抛出异常
            }

            try
            {
                using (var stream = new MemoryStream(byteArray))
                using (var reader = new BinaryReader(stream))
                {
                    var data = new ControllerData
                    {
                        Timestamp = DateTimeOffset.Now.ToUnixTimeMilliseconds(),
                        // PacketLength = byteArray.Length // PacketLength 不存在于 ControllerData 中
                    };

                    // 解析加速度计和陀螺仪数据 (Signed 16-bit integers)
                    // 假设数据在特定偏移量，并且是小端字节序
                    reader.BaseStream.Seek(ACCEL_X_OFFSET, SeekOrigin.Begin);
                    data.AccelX = reader.ReadInt16();
                    reader.BaseStream.Seek(ACCEL_Y_OFFSET, SeekOrigin.Begin);
                    data.AccelY = reader.ReadInt16();
                    reader.BaseStream.Seek(ACCEL_Z_OFFSET, SeekOrigin.Begin);
                    data.AccelZ = reader.ReadInt16();

                    reader.BaseStream.Seek(GYRO_X_OFFSET, SeekOrigin.Begin);
                    data.GyroX = reader.ReadInt16();
                    reader.BaseStream.Seek(GYRO_Y_OFFSET, SeekOrigin.Begin);
                    data.GyroY = reader.ReadInt16();
                    reader.BaseStream.Seek(GYRO_Z_OFFSET, SeekOrigin.Begin);
                    data.GyroZ = reader.ReadInt16();

                    // 解析触摸板数据 (Unsigned 16-bit integers)
                    // 注意：Gear VR Controller 的触摸板数据通常在 [0, 1023] 范围内
                    // 这里读取为 int，后续在 TouchpadProcessor 中会进行归一化和校准
                    reader.BaseStream.Seek(TOUCHPAD_X_OFFSET, SeekOrigin.Begin);
                    data.TouchpadX = reader.ReadUInt16();
                    reader.BaseStream.Seek(TOUCHPAD_Y_OFFSET, SeekOrigin.Begin);
                    data.TouchpadY = reader.ReadUInt16();

                    // 解析按钮状态
                    // 假设按钮状态在一个字节中，每个位代表一个按钮
                    reader.BaseStream.Seek(BUTTON_STATE_OFFSET, SeekOrigin.Begin);
                    byte buttonStates = reader.ReadByte();
                    data.TouchpadButton = (buttonStates & TOUCHPAD_BUTTON_MASK) != 0;
                    data.HomeButton = (buttonStates & HOME_BUTTON_MASK) != 0;
                    data.TriggerButton = (buttonStates & TRIGGER_BUTTON_MASK) != 0;
                    data.BackButton = (buttonStates & BACK_BUTTON_MASK) != 0;
                    data.VolumeUpButton = (buttonStates & VOLUME_UP_BUTTON_MASK) != 0;
                    data.VolumeDownButton = (buttonStates & VOLUME_DOWN_BUTTON_MASK) != 0;

                    // 清除注释掉的电池电量读取代码
                    // reader.BaseStream.Seek(BATTERY_LEVEL_OFFSET, SeekOrigin.Begin);
                    // byte batteryLevel = reader.ReadByte();
                    // data.BatteryLevel = batteryLevel;

                    DataReceived?.Invoke(this, data);
                }
            }
            catch (EndOfStreamException ex)
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 数据包不完整，无法解析: {ex.Message}");
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"[BluetoothService] 解析数据包时发生错误: {ex.Message}");
            }
        }

        public void SimulateData(ControllerData data)
        {
            // This method is for testing/simulation purposes
            DataReceived?.Invoke(this, data);
        }
    }
}