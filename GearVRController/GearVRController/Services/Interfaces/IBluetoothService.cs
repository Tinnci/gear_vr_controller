using System;
using System.Threading.Tasks;
using Windows.Devices.Bluetooth;
using GearVRController.Models;

namespace GearVRController.Services.Interfaces
{
    public interface IBluetoothService
    {
        bool IsConnected { get; }
        event EventHandler<ControllerData> DataReceived;
        event EventHandler<BluetoothConnectionStatus> ConnectionStatusChanged;
        Task ConnectAsync(ulong bluetoothAddress, int timeoutMs = 10000);
        void Disconnect();
        Task SendDataAsync(byte[] data, int repeat = 1);
    }
}