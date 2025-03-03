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

namespace GearVRController.Services
{
    public class BluetoothService : IBluetoothService
    {
        // 服务和特征值UUID
        private static readonly Guid CONTROLLER_SERVICE_UUID = new Guid("4f63756c-7573-2054-6872-65656d6f7465");
        private static readonly Guid CONTROLLER_SETUP_CHARACTERISTIC_UUID = new Guid("c8c51726-81bc-483b-a052-f7a14ea3d282");
        private static readonly Guid CONTROLLER_DATA_CHARACTERISTIC_UUID = new Guid("c8c51726-81bc-483b-a052-f7a14ea3d281");

        private BluetoothLEDevice? _device;
        private GattCharacteristic? _setupCharacteristic;
        private GattCharacteristic? _dataCharacteristic;
        private ulong _lastConnectedAddress;
        private bool _isReconnecting;
        private readonly SemaphoreSlim _reconnectionSemaphore = new SemaphoreSlim(1, 1);
        private const int MAX_RECONNECTION_ATTEMPTS = 3;
        private const int RECONNECTION_DELAY_MS = 2000;
        private CancellationTokenSource? _connectionCts;

        // 事件处理
        public event EventHandler<ControllerData>? DataReceived;
        public event EventHandler<BluetoothConnectionStatus>? ConnectionStatusChanged;

        public bool IsConnected => _device?.ConnectionStatus == BluetoothConnectionStatus.Connected;

        public async Task ConnectAsync(ulong bluetoothAddress, int timeoutMs = 10000)
        {
            try
            {
                _lastConnectedAddress = bluetoothAddress;
                _connectionCts?.Cancel();
                _connectionCts = new CancellationTokenSource();

                // 添加超时
                using var timeoutCts = new CancellationTokenSource(timeoutMs);
                using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(timeoutCts.Token, _connectionCts.Token);

                _device = await BluetoothLEDevice.FromBluetoothAddressAsync(bluetoothAddress);
                if (_device == null)
                {
                    throw new Exception("无法连接到设备");
                }

                // 注册连接状态变化事件
                _device.ConnectionStatusChanged += Device_ConnectionStatusChanged;

                await InitializeServicesAsync(linkedCts.Token);
            }
            catch (OperationCanceledException)
            {
                throw new TimeoutException("连接超时");
            }
            catch (Exception)
            {
                _device?.Dispose();
                _device = null;
                throw;
            }
        }

        private async void Device_ConnectionStatusChanged(BluetoothLEDevice sender, object args)
        {
            var status = sender.ConnectionStatus;
            ConnectionStatusChanged?.Invoke(this, status);

            if (status == BluetoothConnectionStatus.Disconnected)
            {
                await AttemptReconnectAsync();
            }
        }

        private async Task AttemptReconnectAsync()
        {
            if (_isReconnecting || _lastConnectedAddress == 0)
                return;

            try
            {
                await _reconnectionSemaphore.WaitAsync();
                _isReconnecting = true;

                for (int attempt = 0; attempt < MAX_RECONNECTION_ATTEMPTS; attempt++)
                {
                    try
                    {
                        System.Diagnostics.Debug.WriteLine($"尝试重新连接，第 {attempt + 1} 次");
                        await ConnectAsync(_lastConnectedAddress);
                        System.Diagnostics.Debug.WriteLine("重新连接成功");
                        return;
                    }
                    catch (Exception ex)
                    {
                        System.Diagnostics.Debug.WriteLine($"重新连接失败: {ex.Message}");
                        if (attempt < MAX_RECONNECTION_ATTEMPTS - 1)
                        {
                            await Task.Delay(RECONNECTION_DELAY_MS);
                        }
                    }
                }
            }
            finally
            {
                _isReconnecting = false;
                _reconnectionSemaphore.Release();
            }
        }

        private async Task InitializeServicesAsync(CancellationToken cancellationToken)
        {
            if (_device == null)
            {
                throw new Exception("设备未连接");
            }

            var result = await _device.GetGattServicesAsync();
            cancellationToken.ThrowIfCancellationRequested();

            if (result.Status != GattCommunicationStatus.Success)
            {
                throw new Exception("无法获取GATT服务");
            }

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
                                await SubscribeToNotificationsAsync();
                                cancellationToken.ThrowIfCancellationRequested();
                            }
                        }
                    }
                }
            }

            if (_setupCharacteristic == null || _dataCharacteristic == null)
            {
                throw new Exception("未找到必要的特征值");
            }

            await InitializeControllerAsync();
        }

        private async Task SubscribeToNotificationsAsync()
        {
            if (_dataCharacteristic == null)
            {
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
                throw new Exception("无法订阅通知");
            }
        }

        private async Task InitializeControllerAsync()
        {
            // 发送初始化命令
            await SendCommandAsync(new byte[] { 0x01, 0x00 }, repeat: 3);
            await SendCommandAsync(new byte[] { 0x06, 0x00 }, repeat: 1);
            await SendCommandAsync(new byte[] { 0x07, 0x00 }, repeat: 1);
            await SendCommandAsync(new byte[] { 0x08, 0x00 }, repeat: 3);
        }

        private async Task SendCommandAsync(byte[] command, int repeat = 1)
        {
            if (_setupCharacteristic == null)
            {
                throw new Exception("设置特征值未初始化");
            }

            for (int i = 0; i < repeat; i++)
            {
                var buffer = command.AsBuffer();
                await _setupCharacteristic.WriteValueAsync(buffer);
            }
        }

        private void DataCharacteristic_ValueChanged(GattCharacteristic sender, GattValueChangedEventArgs args)
        {
            if (args?.CharacteristicValue == null)
            {
                return;
            }

            var data = new ControllerData();
            var reader = DataReader.FromBuffer(args.CharacteristicValue);
            var byteArray = new byte[args.CharacteristicValue.Length];
            reader.ReadBytes(byteArray);

            // 添加数据长度检查和日志
            if (byteArray.Length < 57) // 需要至少57字节的数据
            {
                System.Diagnostics.Debug.WriteLine($"收到的数据长度不足: {byteArray.Length} 字节");
                // 可以选择记录接收到的数据内容
                System.Diagnostics.Debug.WriteLine($"数据内容: {BitConverter.ToString(byteArray)}");
                return;
            }

            try
            {
                // 解析触摸板数据 - 增加安全检查
                if (byteArray.Length >= 57)
                {
                    try {
                        // 解析原始值（0-1023范围）
                        int rawAxisX = (((byteArray[54] & 0xF) << 6) + ((byteArray[55] & 0xFC) >> 2)) & 0x3FF;
                        int rawAxisY = (((byteArray[55] & 0x3) << 8) + ((byteArray[56] & 0xFF) >> 0)) & 0x3FF;
                        
                        // 根据实际测量范围调整映射
                        // 先直接记录原始值，便于观察实际范围
                        data.AxisX = rawAxisX;
                        data.AxisY = rawAxisY;
                        
                        System.Diagnostics.Debug.WriteLine($"[测量模式] 触摸板原始数据: X={rawAxisX}, Y={rawAxisY}");
                        
                        // 注释掉之前的映射代码，直到确认实际范围
                        // 将0-1023范围映射到0-315范围
                        // data.AxisX = rawAxisX * 315 / 1023;
                        // data.AxisY = rawAxisY * 315 / 1023;
                        
                        // System.Diagnostics.Debug.WriteLine($"触摸板原始数据: X={rawAxisX}, Y={rawAxisY} => 映射后: X={data.AxisX}, Y={data.AxisY}");
                    }
                    catch (Exception ex) {
                        System.Diagnostics.Debug.WriteLine($"触摸板数据解析错误: {ex.Message}");
                        // 设置默认值
                        data.AxisX = 0;
                        data.AxisY = 0;
                    }
                }

                // 确保其他数据索引也在有效范围内
                if (byteArray.Length >= 16) // 加速度计和陀螺仪数据
                {
                    try {
                        // 使用short类型进行转换，避免溢出
                        short accelXRaw = (short)((byteArray[4] << 8) | byteArray[5]);
                        short accelYRaw = (short)((byteArray[6] << 8) | byteArray[7]);
                        short accelZRaw = (short)((byteArray[8] << 8) | byteArray[9]);
                        short gyroXRaw = (short)((byteArray[10] << 8) | byteArray[11]);
                        short gyroYRaw = (short)((byteArray[12] << 8) | byteArray[13]);
                        short gyroZRaw = (short)((byteArray[14] << 8) | byteArray[15]);
                        
                        // 转换为物理单位
                        data.AccelX = accelXRaw * 9.80665f / 2048.0f;
                        data.AccelY = accelYRaw * 9.80665f / 2048.0f;
                        data.AccelZ = accelZRaw * 9.80665f / 2048.0f;
                        data.GyroX = gyroXRaw * 0.017453292f / 14.285f;
                        data.GyroY = gyroYRaw * 0.017453292f / 14.285f;
                        data.GyroZ = gyroZRaw * 0.017453292f / 14.285f;
                    }
                    catch (Exception ex) {
                        System.Diagnostics.Debug.WriteLine($"IMU数据解析错误: {ex.Message}");
                    }
                }

                if (byteArray.Length >= 38) // 磁力计数据
                {
                    try {
                        short magXRaw = (short)((byteArray[32] << 8) | byteArray[33]);
                        short magYRaw = (short)((byteArray[34] << 8) | byteArray[35]);
                        short magZRaw = (short)((byteArray[36] << 8) | byteArray[37]);
                        
                        data.MagnetX = (int)(magXRaw * 0.06);
                        data.MagnetY = (int)(magYRaw * 0.06);
                        data.MagnetZ = (int)(magZRaw * 0.06);
                    }
                    catch (Exception ex) {
                        System.Diagnostics.Debug.WriteLine($"磁力计数据解析错误: {ex.Message}");
                    }
                }

                if (byteArray.Length >= 59) // 按钮状态
                {
                    try {
                        byte buttonState = byteArray[58];
                        data.TriggerButton = (buttonState & 1) == 1;
                        data.HomeButton = (buttonState & 2) == 2;
                        data.BackButton = (buttonState & 4) == 4;
                        data.TouchpadButton = (buttonState & 8) == 8;
                        data.VolumeUpButton = (buttonState & 16) == 16;
                        data.VolumeDownButton = (buttonState & 32) == 32;
                        data.NoButton = (buttonState & 64) == 64;
                        data.TouchpadTouched = data.AxisX != 0 || data.AxisY != 0;
                    }
                    catch (Exception ex) {
                        System.Diagnostics.Debug.WriteLine($"按钮状态解析错误: {ex.Message}");
                    }
                }

                // 记录成功解析的数据
                System.Diagnostics.Debug.WriteLine($"成功解析数据: AxisX={data.AxisX}, AxisY={data.AxisY}");

                DataReceived?.Invoke(this, data);
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"数据解析错误: {ex.Message}");
                System.Diagnostics.Debug.WriteLine($"数据长度: {byteArray.Length}");
                System.Diagnostics.Debug.WriteLine($"数据内容: {BitConverter.ToString(byteArray)}");
            }
        }

        public void Disconnect()
        {
            _connectionCts?.Cancel();
            _device?.Dispose();
            _device = null;
            _setupCharacteristic = null;
            _dataCharacteristic = null;
        }

        public async Task SendDataAsync(byte[] data, int repeat = 1)
        {
            if (!IsConnected || _setupCharacteristic == null)
                throw new InvalidOperationException("设备未连接");

            try
            {
                // 创建数据缓冲区
                using var writer = new DataWriter();
                writer.WriteBytes(data);

                // 重复发送指定次数
                for (int i = 0; i < repeat; i++)
                {
                    await _setupCharacteristic.WriteValueAsync(writer.DetachBuffer());
                    if (repeat > 1 && i < repeat - 1)
                    {
                        await Task.Delay(50); // 在重复发送之间添加小延迟
                    }
                }
            }
            catch (Exception ex)
            {
                throw new Exception($"发送数据失败: {ex.Message}", ex);
            }
        }

        ~BluetoothService()
        {
            Disconnect();
            _reconnectionSemaphore.Dispose();
        }

        // 用于测试的模拟数据方法
        public void SimulateData(ControllerData data)
        {
            DataReceived?.Invoke(this, data);
        }
    }
}