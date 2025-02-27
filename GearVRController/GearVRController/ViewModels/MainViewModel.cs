using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using GearVRController.Models;
using GearVRController.Services;
using GearVRController.Helpers;
using Microsoft.UI.Xaml;
using Microsoft.UI.Dispatching;

namespace GearVRController.ViewModels
{
    public class MainViewModel : INotifyPropertyChanged
    {
        private readonly BluetoothService _bluetoothService;
        private readonly DispatcherQueue _dispatcherQueue;
        private bool _isConnected;
        private string _statusMessage = string.Empty;
        private ControllerData _lastControllerData = new ControllerData();
        private bool _useWheel;
        private bool _useTouch;
        private int _wheelPosition;
        private const int NumberOfWheelPositions = 64;

        public event PropertyChangedEventHandler? PropertyChanged;

        public bool IsConnected
        {
            get => _isConnected;
            private set
            {
                if (_isConnected != value)
                {
                    _isConnected = value;
                    OnPropertyChanged();
                }
            }
        }

        public string StatusMessage
        {
            get => _statusMessage;
            private set
            {
                if (_statusMessage != value)
                {
                    _statusMessage = value;
                    OnPropertyChanged();
                }
            }
        }

        public ControllerData LastControllerData
        {
            get => _lastControllerData;
            private set
            {
                if (_lastControllerData != value)
                {
                    _lastControllerData = value;
                    OnPropertyChanged();
                }
            }
        }

        public MainViewModel()
        {
            _bluetoothService = new BluetoothService();
            _bluetoothService.DataReceived += BluetoothService_DataReceived;
            _dispatcherQueue = DispatcherQueue.GetForCurrentThread();
            StatusMessage = "准备连接...";
        }

        public async Task ConnectAsync(ulong deviceAddress)
        {
            try
            {
                await _bluetoothService.ConnectAsync(deviceAddress);
                _dispatcherQueue.TryEnqueue(() =>
                {
                    IsConnected = true;
                    StatusMessage = "已连接";
                });
            }
            catch (Exception ex)
            {
                _dispatcherQueue.TryEnqueue(() =>
                {
                    StatusMessage = $"连接失败: {ex.Message}";
                });
            }
        }

        public void Disconnect()
        {
            _bluetoothService.Disconnect();
            _dispatcherQueue.TryEnqueue(() =>
            {
                IsConnected = false;
                StatusMessage = "已断开连接";
            });
        }

        private void BluetoothService_DataReceived(object? sender, ControllerData data)
        {
            if (data == null)
            {
                return;
            }

            _dispatcherQueue.TryEnqueue(() =>
            {
                LastControllerData = data;

                // 处理触摸板输入
                if (data.TouchpadButton)
                {
                    if (data.AxisX >= 90 && data.AxisX <= 240 && data.AxisY >= 90 && data.AxisY <= 240)
                    {
                        _useWheel = !_useWheel;
                        _useTouch = !_useTouch;
                    }
                    else
                    {
                        HandleTouchpadInput(data);
                    }
                }

                // 处理按钮输入
                HandleButtonInput(data);

                // 处理鼠标移动
                if (!_useWheel && !_useTouch)
                {
                    int deltaX = (int)Math.Round((data.AxisX - 157) * 1.2);
                    int deltaY = (int)Math.Round((data.AxisY - 157) * 1.2);
                    if (deltaX != 0 || deltaY != 0)
                    {
                        InputSimulator.MoveMouse(deltaX, deltaY);
                    }
                }
            });
        }

        private void HandleTouchpadInput(ControllerData data)
        {
            if (data == null)
            {
                return;
            }

            if (_useWheel)
            {
                // 计算轮盘位置
                int newWheelPos = CalculateWheelPosition(data.AxisX - 157, data.AxisY - 157);
                if (newWheelPos != _wheelPosition)
                {
                    if ((newWheelPos - _wheelPosition + NumberOfWheelPositions) % NumberOfWheelPositions == 1)
                    {
                        InputSimulator.SendKey(InputSimulator.VK_DOWN);
                    }
                    else if ((_wheelPosition - newWheelPos + NumberOfWheelPositions) % NumberOfWheelPositions == 1)
                    {
                        InputSimulator.SendKey(InputSimulator.VK_UP);
                    }
                    _wheelPosition = newWheelPos;
                }
            }
            else if (_useTouch)
            {
                // 处理触摸板滑动
                if (data.AxisX < 90)
                {
                    InputSimulator.SendKey(InputSimulator.VK_LEFT);
                }
                else if (data.AxisX > 240)
                {
                    InputSimulator.SendKey(InputSimulator.VK_RIGHT);
                }
                else if (data.AxisY < 90)
                {
                    InputSimulator.SendKey(InputSimulator.VK_UP);
                }
                else if (data.AxisY > 240)
                {
                    InputSimulator.SendKey(InputSimulator.VK_DOWN);
                }
            }
        }

        private void HandleButtonInput(ControllerData data)
        {
            if (data == null)
            {
                return;
            }

            if (data.TriggerButton)
            {
                InputSimulator.MouseDown();
            }
            else
            {
                InputSimulator.MouseUp();
            }

            if (data.HomeButton)
            {
                InputSimulator.SendKey(InputSimulator.VK_HOME);
            }

            if (data.BackButton)
            {
                InputSimulator.SendKey(InputSimulator.VK_BACK);
            }

            if (data.VolumeUpButton)
            {
                InputSimulator.SendKey(InputSimulator.VK_VOLUME_UP);
            }

            if (data.VolumeDownButton)
            {
                InputSimulator.SendKey(InputSimulator.VK_VOLUME_DOWN);
            }
        }

        private int CalculateWheelPosition(int x, int y)
        {
            if (x == 0 && y == 0) return -1;
            double angle = Math.Atan2(y, x);
            return (int)Math.Floor((angle + Math.PI) / (2 * Math.PI) * NumberOfWheelPositions);
        }

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
} 