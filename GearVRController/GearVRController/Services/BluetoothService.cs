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
                // 解析数据
                data.AxisX = (((byteArray[54] & 0xF) << 6) + ((byteArray[55] & 0xFC) >> 2)) & 0x3FF;
                data.AxisY = (((byteArray[55] & 0x3) << 8) + ((byteArray[56] & 0xFF) >> 0)) & 0x3FF;

                // 确保其他数据索引也在有效范围内
                if (byteArray.Length >= 16) // 加速度计和陀螺仪数据
                {
                    data.AccelX = (int)(((byteArray[4] << 8) + byteArray[5]) * 10000.0 * 9.80665 / 2048.0);
                    data.AccelY = (int)(((byteArray[6] << 8) + byteArray[7]) * 10000.0 * 9.80665 / 2048.0);
                    data.AccelZ = (int)(((byteArray[8] << 8) + byteArray[9]) * 10000.0 * 9.80665 / 2048.0);
                    data.GyroX = (int)(((byteArray[10] << 8) + byteArray[11]) * 10000.0 * 0.017453292 / 14.285);
                    data.GyroY = (int)(((byteArray[12] << 8) + byteArray[13]) * 10000.0 * 0.017453292 / 14.285);
                    data.GyroZ = (int)(((byteArray[14] << 8) + byteArray[15]) * 10000.0 * 0.017453292 / 14.285);
                }

                if (byteArray.Length >= 38) // 磁力计数据
                {
                    data.MagnetX = (int)(((byteArray[32] << 8) + byteArray[33]) * 0.06);
                    data.MagnetY = (int)(((byteArray[34] << 8) + byteArray[35]) * 0.06);
                    data.MagnetZ = (int)(((byteArray[36] << 8) + byteArray[37]) * 0.06);
                }

                if (byteArray.Length >= 59) // 按钮状态
                {
                    data.TriggerButton = (byteArray[58] & 1) == 1;
                    data.HomeButton = (byteArray[58] & 2) == 2;
                    data.BackButton = (byteArray[58] & 4) == 4;
                    data.TouchpadButton = (byteArray[58] & 8) == 8;
                    data.VolumeUpButton = (byteArray[58] & 16) == 16;
                    data.VolumeDownButton = (byteArray[58] & 32) == 32;
                    data.NoButton = (byteArray[58] & 64) == 64;
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
            _connectionCts?.Dispose();
            _connectionCts = null;

            if (_dataCharacteristic != null)
            {
                _dataCharacteristic.ValueChanged -= DataCharacteristic_ValueChanged;
            }

            if (_device != null)
            {
                _device.ConnectionStatusChanged -= Device_ConnectionStatusChanged;
                _device.Dispose();
                _device = null;
            }

            _setupCharacteristic = null;
            _dataCharacteristic = null;
            _lastConnectedAddress = 0;
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