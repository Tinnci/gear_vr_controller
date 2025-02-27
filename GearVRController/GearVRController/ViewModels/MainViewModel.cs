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
        
        // 添加触摸板校准数据
        private TouchpadCalibrationData? _calibrationData;
        
        // 添加新的属性
        private bool _isControlEnabled = true;
        private double _mouseSensitivity = 1.0;
        private bool _isMouseEnabled = true;
        private bool _isKeyboardEnabled = true;

        // 添加异常移动检测相关字段
        private const int ABNORMAL_MOVEMENT_THRESHOLD = 10; // 连续向左上角移动的次数阈值
        private const double DIRECTION_TOLERANCE = 0.3; // 判定为左上角移动的方向容差
        private int _abnormalMovementCount = 0;
        private bool _isAutoCalibrating = false;
        private DateTime _lastAbnormalMovementTime = DateTime.MinValue;
        private const int RESET_INTERVAL_SECONDS = 5; // 重置异常计数的时间间隔

        private bool _isCalibrating = false;
        private bool _isConnecting = false;

        public event PropertyChangedEventHandler? PropertyChanged;
        public event EventHandler<ControllerData>? ControllerDataReceived;
        public event EventHandler? AutoCalibrationRequired;

        public bool IsControlEnabled
        {
            get => _isControlEnabled;
            set
            {
                if (_isControlEnabled != value)
                {
                    _isControlEnabled = value;
                    OnPropertyChanged();
                }
            }
        }

        public double MouseSensitivity
        {
            get => _mouseSensitivity;
            set
            {
                if (_mouseSensitivity != value)
                {
                    _mouseSensitivity = Math.Max(0.1, Math.Min(value, 2.0)); // 限制灵敏度范围在0.1到2.0之间
                    OnPropertyChanged();
                }
            }
        }

        public bool IsMouseEnabled
        {
            get => _isMouseEnabled;
            set
            {
                if (_isMouseEnabled != value)
                {
                    _isMouseEnabled = value;
                    OnPropertyChanged();
                }
            }
        }

        public bool IsKeyboardEnabled
        {
            get => _isKeyboardEnabled;
            set
            {
                if (_isKeyboardEnabled != value)
                {
                    _isKeyboardEnabled = value;
                    OnPropertyChanged();
                }
            }
        }

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

        public bool IsCalibrating
        {
            get => _isCalibrating;
            private set
            {
                if (_isCalibrating != value)
                {
                    _isCalibrating = value;
                    OnPropertyChanged();
                    // 当校准状态改变时，更新控制状态
                    UpdateControlState();
                }
            }
        }

        public bool IsConnecting
        {
            get => _isConnecting;
            private set
            {
                if (_isConnecting != value)
                {
                    _isConnecting = value;
                    OnPropertyChanged();
                    // 当连接状态改变时，更新控制状态
                    UpdateControlState();
                }
            }
        }

        public MainViewModel()
        {
            _bluetoothService = new BluetoothService();
            _bluetoothService.DataReceived += BluetoothService_DataReceived;
            _dispatcherQueue = DispatcherQueue.GetForCurrentThread();
            StatusMessage = "准备连接...";

            // 注册热键处理
            RegisterHotKeys();
        }

        private void RegisterHotKeys()
        {
            // 这里可以添加热键注册逻辑
            // 由于WinUI 3的限制，我们可能需要使用原生Windows API来实现
            // 暂时可以通过UI按钮来控制
        }

        public async Task ConnectAsync(ulong deviceAddress)
        {
            try
            {
                IsConnecting = true;
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
            finally
            {
                IsConnecting = false;
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
                // 触发控制器数据接收事件
                ControllerDataReceived?.Invoke(this, data);

                // 如果正在校准或控制被禁用，直接返回
                if (IsCalibrating || !_isControlEnabled)
                {
                    return;
                }

                LastControllerData = data;

                // 处理触摸板输入
                if (IsMouseEnabled && !_useWheel && !_useTouch && !_isAutoCalibrating)
                {
                    // 添加平滑处理和死区
                    const int deadZone = 10;
                    
                    // 使用校准数据或默认值
                    int centerX = _calibrationData?.CenterX ?? 512;
                    int centerY = _calibrationData?.CenterY ?? 512;
                    
                    // 处理触摸板移动（不需要按下，只要触摸即可）
                    double deltaX = (data.AxisX - centerX);
                    double deltaY = (data.AxisY - centerY);

                    // 如果有校准数据，使用方向校准来改进移动检测
                    if (_calibrationData != null)
                    {
                        double magnitude = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);
                        if (magnitude > deadZone)
                        {
                            // 归一化向量
                            double normalizedX = deltaX / magnitude;
                            double normalizedY = deltaY / magnitude;

                            // 计算与各个方向向量的点积
                            double upDot = normalizedX * _calibrationData.UpDirection.AverageX + 
                                         normalizedY * _calibrationData.UpDirection.AverageY;
                            double downDot = normalizedX * _calibrationData.DownDirection.AverageX + 
                                           normalizedY * _calibrationData.DownDirection.AverageY;
                            double leftDot = normalizedX * _calibrationData.LeftDirection.AverageX + 
                                           normalizedY * _calibrationData.LeftDirection.AverageY;
                            double rightDot = normalizedX * _calibrationData.RightDirection.AverageX + 
                                            normalizedY * _calibrationData.RightDirection.AverageY;

                            // 根据点积调整移动方向
                            double adjustedX = deltaX;
                            double adjustedY = deltaY;

                            if (Math.Abs(leftDot) > Math.Abs(rightDot) && Math.Abs(leftDot) > 0.7)
                            {
                                // 向左移动
                                adjustedX = -magnitude;
                                adjustedY = 0;
                            }
                            else if (Math.Abs(rightDot) > Math.Abs(leftDot) && Math.Abs(rightDot) > 0.7)
                            {
                                // 向右移动
                                adjustedX = magnitude;
                                adjustedY = 0;
                            }

                            if (Math.Abs(upDot) > Math.Abs(downDot) && Math.Abs(upDot) > 0.7)
                            {
                                // 向上移动
                                adjustedY = -magnitude;
                                if (Math.Abs(leftDot) <= 0.7 && Math.Abs(rightDot) <= 0.7)
                                {
                                    adjustedX = 0;
                                }
                            }
                            else if (Math.Abs(downDot) > Math.Abs(upDot) && Math.Abs(downDot) > 0.7)
                            {
                                // 向下移动
                                adjustedY = magnitude;
                                if (Math.Abs(leftDot) <= 0.7 && Math.Abs(rightDot) <= 0.7)
                                {
                                    adjustedX = 0;
                                }
                            }

                            deltaX = adjustedX;
                            deltaY = adjustedY;
                        }

                        // 使用校准范围调整灵敏度
                        double rangeX = _calibrationData.MaxX - _calibrationData.MinX;
                        double rangeY = _calibrationData.MaxY - _calibrationData.MinY;
                        
                        // 将移动距离标准化到[-1, 1]范围
                        deltaX = deltaX / (rangeX / 2);
                        deltaY = deltaY / (rangeY / 2);
                    }

                    // 检测异常移动
                    if (CheckAbnormalMovement(deltaX, deltaY))
                    {
                        // 如果检测到异常移动，触发自动校准
                        TriggerAutoCalibration();
                        return;
                    }

                    // 应用死区
                    if (Math.Abs(deltaX) < deadZone / 100.0) deltaX = 0;
                    if (Math.Abs(deltaY) < deadZone / 100.0) deltaY = 0;

                    // 应用平滑和灵敏度
                    if (deltaX != 0 || deltaY != 0)
                    {
                        deltaX = Math.Sign(deltaX) * Math.Pow(Math.Abs(deltaX), 1.5) * 20 * _mouseSensitivity;
                        deltaY = Math.Sign(deltaY) * Math.Pow(Math.Abs(deltaY), 1.5) * 20 * _mouseSensitivity;
                        InputSimulator.MoveMouse((int)deltaX, (int)deltaY);
                    }

                    // 处理触摸板点击（按下触发左键）
                    if (data.TouchpadButton)
                    {
                        InputSimulator.MouseDown();
                    }
                    else
                    {
                        InputSimulator.MouseUp();
                    }
                }
                else if (_useWheel || _useTouch)
                {
                    HandleTouchpadInput(data);
                }

                // 处理其他按钮输入
                if (IsKeyboardEnabled)
                {
                    HandleButtonInput(data);
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
                // 扳机键可以作为右键
                InputSimulator.MouseEvent(InputSimulator.MOUSEEVENTF_RIGHTDOWN);
            }
            else
            {
                InputSimulator.MouseEvent(InputSimulator.MOUSEEVENTF_RIGHTUP);
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

        public void ToggleControl()
        {
            _isControlEnabled = !_isControlEnabled;
            UpdateControlState();
            StatusMessage = _isControlEnabled ? "控制已启用" : "控制已禁用";
        }

        public void ResetSettings()
        {
            MouseSensitivity = 1.0;
            IsMouseEnabled = true;
            IsKeyboardEnabled = true;
            _useWheel = false;
            _useTouch = false;
        }

        public void ApplyCalibrationData(TouchpadCalibrationData calibrationData)
        {
            _calibrationData = calibrationData;
            StatusMessage = "已应用触摸板校准数据";
        }

        private bool CheckAbnormalMovement(double deltaX, double deltaY)
        {
            // 检查是否需要重置计数器
            if ((DateTime.Now - _lastAbnormalMovementTime).TotalSeconds > RESET_INTERVAL_SECONDS)
            {
                _abnormalMovementCount = 0;
            }

            // 计算移动方向
            double magnitude = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);
            if (magnitude < 1) // 如果移动太小，不计入
            {
                return false;
            }

            // 归一化方向向量
            double normalizedX = deltaX / magnitude;
            double normalizedY = deltaY / magnitude;

            // 检查是否向左上方移动（大约-135度方向）
            // 理想的左上方向量是 (-0.707, -0.707)
            if (normalizedX < -0.707 - DIRECTION_TOLERANCE || normalizedX > -0.707 + DIRECTION_TOLERANCE ||
                normalizedY < -0.707 - DIRECTION_TOLERANCE || normalizedY > -0.707 + DIRECTION_TOLERANCE)
            {
                // 如果不是向左上方移动，重置计数
                _abnormalMovementCount = 0;
                return false;
            }

            // 更新计数和时间戳
            _abnormalMovementCount++;
            _lastAbnormalMovementTime = DateTime.Now;

            // 如果连续向左上角移动次数超过阈值，返回true
            return _abnormalMovementCount >= ABNORMAL_MOVEMENT_THRESHOLD;
        }

        private void TriggerAutoCalibration()
        {
            if (_isAutoCalibrating)
            {
                return;
            }

            _isAutoCalibrating = true;
            StatusMessage = "检测到异常移动，正在启动自动校准...";
            AutoCalibrationRequired?.Invoke(this, EventArgs.Empty);
        }

        public void StartAutoCalibration()
        {
            IsCalibrating = true;
            _isAutoCalibrating = true;
            StatusMessage = "自动校准已启动，请在触摸板边缘划圈...";
        }

        public void StartManualCalibration()
        {
            IsCalibrating = true;
            StatusMessage = "手动校准已启动，请在触摸板边缘划圈...";
        }

        public void EndCalibration()
        {
            IsCalibrating = false;
            _isAutoCalibrating = false;
            _abnormalMovementCount = 0;
            StatusMessage = "校准完成";
        }

        private void UpdateControlState()
        {
            // 在以下情况下禁用控制：
            // 1. 正在校准
            // 2. 正在连接
            // 3. 未连接
            // 4. 用户手动禁用
            bool shouldBeEnabled = !_isCalibrating && 
                                 !_isConnecting && 
                                 _isConnected && 
                                 _isControlEnabled;

            // 更新鼠标和键盘控制状态
            IsMouseEnabled = shouldBeEnabled && _isMouseEnabled;
            IsKeyboardEnabled = shouldBeEnabled && _isKeyboardEnabled;
        }
    }
} 