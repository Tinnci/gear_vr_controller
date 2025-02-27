using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using GearVRController.Models;

namespace GearVRController.ViewModels
{
    public class TouchpadCalibrationViewModel : INotifyPropertyChanged
    {
        private int _minX = int.MaxValue;
        private int _maxX = int.MinValue;
        private int _minY = int.MaxValue;
        private int _maxY = int.MinValue;
        private int _centerX;
        private int _centerY;
        private string _statusMessage = "准备开始校准...";
        private bool _isCalibrating;
        private CalibrationStep _currentStep = CalibrationStep.NotStarted;
        private TouchpadCalibrationData _calibrationData = new TouchpadCalibrationData();
        private const int SAMPLES_REQUIRED = 10; // 每个方向需要的样本数
        private const double MOVEMENT_THRESHOLD = 50; // 移动检测阈值
        private const double MIN_RANGE_THRESHOLD = 200; // 最小范围阈值
        private const int MIN_CIRCLE_POINTS = 20; // 最小圆周运动点数
        private const int INACTIVITY_TIMEOUT_MS = 1000; // 停止操作超时时间（毫秒）
        private const int MIN_POINTS_REQUIRED = 5; // 最小需要的点数
        private int _progressValue = 0; // 进度值
        private System.Timers.Timer _inactivityTimer;
        private DateTime _lastActivityTime;
        private int _circlePointCount = 0;
        private bool _canProceedToNextStep = false;
        private bool _isCurrentlyTouching = false;
        private DateTime _lastTouchEndTime = DateTime.MinValue;
        private const int TOUCH_END_TIMEOUT_MS = 500; // 触摸结束后等待时间
        private Microsoft.UI.Dispatching.DispatcherQueue? _dispatcher;

        public event PropertyChangedEventHandler? PropertyChanged;
        public event EventHandler<TouchpadCalibrationData>? CalibrationCompleted;

        public int MinX
        {
            get => _minX;
            private set
            {
                if (_minX != value)
                {
                    _minX = value;
                    OnPropertyChanged();
                }
            }
        }

        public int MaxX
        {
            get => _maxX;
            private set
            {
                if (_maxX != value)
                {
                    _maxX = value;
                    OnPropertyChanged();
                }
            }
        }

        public int MinY
        {
            get => _minY;
            private set
            {
                if (_minY != value)
                {
                    _minY = value;
                    OnPropertyChanged();
                }
            }
        }

        public int MaxY
        {
            get => _maxY;
            private set
            {
                if (_maxY != value)
                {
                    _maxY = value;
                    OnPropertyChanged();
                }
            }
        }

        public int CenterX
        {
            get => _centerX;
            private set
            {
                if (_centerX != value)
                {
                    _centerX = value;
                    OnPropertyChanged();
                }
            }
        }

        public int CenterY
        {
            get => _centerY;
            private set
            {
                if (_centerY != value)
                {
                    _centerY = value;
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

        public bool IsCalibrating
        {
            get => _isCalibrating;
            private set
            {
                if (_isCalibrating != value)
                {
                    _isCalibrating = value;
                    OnPropertyChanged();
                }
            }
        }

        public bool CanProceedToNextStep
        {
            get => _canProceedToNextStep;
            private set
            {
                if (_canProceedToNextStep != value)
                {
                    _canProceedToNextStep = value;
                    OnPropertyChanged();
                }
            }
        }

        public int ProgressValue
        {
            get => _progressValue;
            private set
            {
                if (_progressValue != value)
                {
                    _progressValue = value;
                    OnPropertyChanged();
                }
            }
        }

        public TouchpadCalibrationViewModel()
        {
            _inactivityTimer = new System.Timers.Timer(100); // 每100ms检查一次
            _inactivityTimer.Elapsed += InactivityTimer_Elapsed;
            _lastActivityTime = DateTime.Now;
            
            // 确保在构造函数中获取UI线程的调度器
            if (Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread() != null)
            {
                _dispatcher = Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread();
            }
        }

        private void InactivityTimer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            // 检查是否真正停止触摸
            bool isTouching = _isCurrentlyTouching;
            var timeSinceLastTouch = (DateTime.Now - _lastTouchEndTime).TotalMilliseconds;
            int newProgress;

            if (isTouching)
            {
                newProgress = 0;
            }
            else if (timeSinceLastTouch < TOUCH_END_TIMEOUT_MS)
            {
                newProgress = (int)Math.Min(100, (timeSinceLastTouch / TOUCH_END_TIMEOUT_MS) * 100);
            }
            else
            {
                newProgress = 100;
            }

            var dispatcher = Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread();
            if (dispatcher != null)
            {
                dispatcher.TryEnqueue(() =>
                {
                    ProgressValue = newProgress;
                    if (newProgress >= 100 && !isTouching && !CanProceedToNextStep)
                    {
                        ValidateAndProceed();
                    }
                });
            }
        }

        private void ValidateAndProceed()
        {
            switch (_currentStep)
            {
                case CalibrationStep.CircularMotion:
                    if (_circlePointCount >= MIN_POINTS_REQUIRED)
                    {
                        CanProceedToNextStep = true;
                        StatusMessage = "边界校准完成，请点击\"下一步\"继续...";
                        _inactivityTimer.Stop();
                    }
                    break;
                case CalibrationStep.Up:
                case CalibrationStep.Down:
                case CalibrationStep.Left:
                case CalibrationStep.Right:
                    var currentDirection = GetCurrentDirectionData();
                    if (currentDirection != null && currentDirection.SampleCount >= MIN_POINTS_REQUIRED)
                    {
                        if (ValidateDirectionalCalibration(currentDirection))
                        {
                            CanProceedToNextStep = true;
                            StatusMessage = "方向校准完成，请点击\"下一步\"继续...";
                            _inactivityTimer.Stop();
                        }
                    }
                    break;
            }
        }

        public void StartCalibration()
        {
            IsCalibrating = true;
            _currentStep = CalibrationStep.CircularMotion;
            StatusMessage = "第1步：请在触摸板边缘顺时针划一个完整的圆...";
            ResetCalibrationData();
            CanProceedToNextStep = false;
            _circlePointCount = 0;
            ProgressValue = 0;
            _lastActivityTime = DateTime.Now;
            _inactivityTimer.Start();
        }

        public void ProcessTouchpadData(ControllerData data)
        {
            if (!IsCalibrating) return;

            // 检测触摸状态变化
            bool isTouching = data.TouchpadButton || (data.AxisX != 0 || data.AxisY != 0); // 根据实际控制器数据调整判断条件
            if (isTouching != _isCurrentlyTouching)
            {
                _isCurrentlyTouching = isTouching;
                if (!isTouching)
                {
                    _lastTouchEndTime = DateTime.Now;
                }
            }

            switch (_currentStep)
            {
                case CalibrationStep.CircularMotion:
                    ProcessCircularMotion(data);
                    break;
                case CalibrationStep.Center:
                    ProcessCenterCalibration(data);
                    break;
                case CalibrationStep.Up:
                case CalibrationStep.Down:
                case CalibrationStep.Left:
                case CalibrationStep.Right:
                    ProcessDirectionalCalibration(data);
                    break;
            }
        }

        private void ProcessCircularMotion(ControllerData data)
        {
            // 更新最后活动时间
            _lastActivityTime = DateTime.Now;
            ProgressValue = 0;

            // 更新边界值
            _calibrationData.MinX = Math.Min(_calibrationData.MinX, data.AxisX);
            _calibrationData.MaxX = Math.Max(_calibrationData.MaxX, data.AxisX);
            _calibrationData.MinY = Math.Min(_calibrationData.MinY, data.AxisY);
            _calibrationData.MaxY = Math.Max(_calibrationData.MaxY, data.AxisY);

            // 更新属性通知
            MinX = _calibrationData.MinX;
            MaxX = _calibrationData.MaxX;
            MinY = _calibrationData.MinY;
            MaxY = _calibrationData.MaxY;

            // 计数有效的圆周运动点
            if (IsValidCirclePoint(data))
            {
                _circlePointCount++;
            }

            // 检查是否满足最小范围要求
            double rangeX = MaxX - MinX;
            double rangeY = MaxY - MinY;
            bool hasValidRange = rangeX >= MIN_RANGE_THRESHOLD && rangeY >= MIN_RANGE_THRESHOLD;

            if (!hasValidRange)
            {
                StatusMessage = "第1步：请在触摸板边缘划圈...\\n需要更大的移动范围";
            }
            else
            {
                StatusMessage = $"第1步：请在触摸板边缘划圈...\\n已收集 {_circlePointCount} 个有效点\\n停止操作后将自动完成";
            }
        }

        private bool IsValidCirclePoint(ControllerData data)
        {
            // 检查点是否在边缘区域
            double centerX = (MaxX + MinX) / 2.0;
            double centerY = (MaxY + MinY) / 2.0;
            double radiusX = (MaxX - MinX) / 2.0;
            double radiusY = (MaxY - MinY) / 2.0;

            if (radiusX == 0 || radiusY == 0) return false;

            // 计算点到中心的归一化距离
            double dx = (data.AxisX - centerX) / radiusX;
            double dy = (data.AxisY - centerY) / radiusY;
            double distance = Math.Sqrt(dx * dx + dy * dy);

            // 如果点在边缘区域（距离接近1），则认为是有效点
            return distance > 0.8;
        }

        public void ProceedToNextStep()
        {
            if (!CanProceedToNextStep) return;

            switch (_currentStep)
            {
                case CalibrationStep.CircularMotion:
                    _currentStep = CalibrationStep.Center;
                    StatusMessage = "第2步：请点击触摸板中心点";
                    CanProceedToNextStep = false;
                    break;
                case CalibrationStep.Center:
                    _currentStep = CalibrationStep.Up;
                    StatusMessage = "第3步：请在触摸板上从中心向上滑动多次";
                    CanProceedToNextStep = false;
                    break;
                case CalibrationStep.Up:
                case CalibrationStep.Down:
                case CalibrationStep.Left:
                case CalibrationStep.Right:
                    MoveToNextDirectionalStep();
                    break;
            }
        }

        private void ProcessCenterCalibration(ControllerData data)
        {
            if (data.TouchpadButton)
            {
                // 验证中心点是否在合理范围内
                double centerX = (MaxX + MinX) / 2.0;
                double centerY = (MaxY + MinY) / 2.0;
                double maxDistance = Math.Min(MaxX - MinX, MaxY - MinY) * 0.3; // 允许的最大偏差

                if (Math.Abs(data.AxisX - centerX) <= maxDistance && 
                    Math.Abs(data.AxisY - centerY) <= maxDistance)
                {
                    _calibrationData.CenterX = data.AxisX;
                    _calibrationData.CenterY = data.AxisY;
                    CenterX = _calibrationData.CenterX;
                    CenterY = _calibrationData.CenterY;
                    CanProceedToNextStep = true;
                    StatusMessage = "中心点已记录，请点击\"下一步\"继续...";
                }
                else
                {
                    StatusMessage = "所选中心点偏离预期太远，请重试...";
                }
            }
        }

        private void ProcessDirectionalCalibration(ControllerData data)
        {
            // 更新最后活动时间
            _lastActivityTime = DateTime.Now;
            ProgressValue = 0;

            // 计算相对于中心点的位移
            double deltaX = data.AxisX - _calibrationData.CenterX;
            double deltaY = data.AxisY - _calibrationData.CenterY;
            double magnitude = Math.Sqrt(deltaX * deltaX + deltaY * deltaY);

            // 如果移动距离太小，忽略
            if (magnitude < MOVEMENT_THRESHOLD)
            {
                return;
            }

            // 根据当前步骤更新相应的方向数据
            DirectionalCalibrationData? currentDirection = GetCurrentDirectionData();
            if (currentDirection != null && IsValidDirectionalMovement(deltaX, deltaY))
            {
                currentDirection.AddSample(deltaX / magnitude, deltaY / magnitude);

                // 更新进度消息
                StatusMessage = $"当前步骤进度：已收集 {currentDirection.SampleCount} 个有效点\\n" +
                              GetDirectionalGuidance() + "\\n停止操作后将自动完成";
            }
        }

        private DirectionalCalibrationData? GetCurrentDirectionData()
        {
            return _currentStep switch
            {
                CalibrationStep.Up => _calibrationData.UpDirection,
                CalibrationStep.Down => _calibrationData.DownDirection,
                CalibrationStep.Left => _calibrationData.LeftDirection,
                CalibrationStep.Right => _calibrationData.RightDirection,
                _ => null
            };
        }

        private bool IsValidDirectionalMovement(double deltaX, double deltaY)
        {
            // 根据当前步骤检查移动方向是否正确
            double angle = Math.Atan2(deltaY, deltaX);
            const double tolerance = Math.PI / 4; // 45度容差

            return _currentStep switch
            {
                CalibrationStep.Up => Math.Abs(angle + Math.PI/2) < tolerance, // -90度
                CalibrationStep.Down => Math.Abs(angle - Math.PI/2) < tolerance, // 90度
                CalibrationStep.Left => Math.Abs(angle + Math.PI) < tolerance || Math.Abs(angle - Math.PI) < tolerance, // 180度
                CalibrationStep.Right => Math.Abs(angle) < tolerance, // 0度
                _ => false
            };
        }

        private bool ValidateDirectionalCalibration(DirectionalCalibrationData direction)
        {
            // 检查采样的一致性
            const double consistencyThreshold = 0.2; // 允许的方差阈值
            
            // 由于我们只存储了平均值，这里使用一个简化的检查
            double expectedMagnitude = 1.0; // 归一化向量的期望长度
            double actualMagnitude = Math.Sqrt(direction.AverageX * direction.AverageX + 
                                             direction.AverageY * direction.AverageY);

            return Math.Abs(actualMagnitude - expectedMagnitude) < consistencyThreshold;
        }

        private string GetDirectionalGuidance()
        {
            return _currentStep switch
            {
                CalibrationStep.Up => "请保持垂直向上滑动",
                CalibrationStep.Down => "请保持垂直向下滑动",
                CalibrationStep.Left => "请保持水平向左滑动",
                CalibrationStep.Right => "请保持水平向右滑动",
                _ => string.Empty
            };
        }

        private void MoveToNextDirectionalStep()
        {
            switch (_currentStep)
            {
                case CalibrationStep.Up:
                    _currentStep = CalibrationStep.Down;
                    StatusMessage = "第4步：请在触摸板上从中心向下滑动多次";
                    break;
                case CalibrationStep.Down:
                    _currentStep = CalibrationStep.Left;
                    StatusMessage = "第5步：请在触摸板上从中心向左滑动多次";
                    break;
                case CalibrationStep.Left:
                    _currentStep = CalibrationStep.Right;
                    StatusMessage = "第6步：请在触摸板上从中心向右滑动多次";
                    break;
                case CalibrationStep.Right:
                    _currentStep = CalibrationStep.Completed;
                    StatusMessage = "校准完成！";
                    FinishCalibration();
                    break;
            }
            CanProceedToNextStep = false;
        }

        public void FinishCalibration()
        {
            if (!IsCalibrating) return;

            IsCalibrating = false;
            StatusMessage = "校准完成";

            CalibrationCompleted?.Invoke(this, _calibrationData);
        }

        public void CancelCalibration()
        {
            IsCalibrating = false;
            StatusMessage = "校准已取消";
            ResetCalibrationData();
        }

        private void ResetCalibrationData()
        {
            _calibrationData = new TouchpadCalibrationData
            {
                MinX = int.MaxValue,
                MaxX = int.MinValue,
                MinY = int.MaxValue,
                MaxY = int.MinValue,
                CenterX = 0,
                CenterY = 0
            };
            
            MinX = _calibrationData.MinX;
            MaxX = _calibrationData.MaxX;
            MinY = _calibrationData.MinY;
            MaxY = _calibrationData.MaxY;
            CenterX = _calibrationData.CenterX;
            CenterY = _calibrationData.CenterY;
        }

        public void Dispose()
        {
            _inactivityTimer?.Dispose();
        }

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            if (_dispatcher != null)
            {
                _dispatcher.TryEnqueue(() =>
                {
                    PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
                });
            }
        }
    }

    public class TouchpadCalibrationData
    {
        public int MinX { get; set; }
        public int MaxX { get; set; }
        public int MinY { get; set; }
        public int MaxY { get; set; }
        public int CenterX { get; set; }
        public int CenterY { get; set; }

        // 添加方向校准数据
        public DirectionalCalibrationData UpDirection { get; set; } = new DirectionalCalibrationData();
        public DirectionalCalibrationData DownDirection { get; set; } = new DirectionalCalibrationData();
        public DirectionalCalibrationData LeftDirection { get; set; } = new DirectionalCalibrationData();
        public DirectionalCalibrationData RightDirection { get; set; } = new DirectionalCalibrationData();
    }

    public class DirectionalCalibrationData
    {
        public double AverageX { get; set; }
        public double AverageY { get; set; }
        public double Magnitude { get; set; }
        public int SampleCount { get; set; }

        public void AddSample(double x, double y)
        {
            double newCount = SampleCount + 1;
            AverageX = (AverageX * SampleCount + x) / newCount;
            AverageY = (AverageY * SampleCount + y) / newCount;
            Magnitude = Math.Sqrt(AverageX * AverageX + AverageY * AverageY);
            SampleCount = (int)newCount;
        }

        public void Reset()
        {
            AverageX = 0;
            AverageY = 0;
            Magnitude = 0;
            SampleCount = 0;
        }
    }

    public enum CalibrationStep
    {
        NotStarted,
        CircularMotion,    // 圆周运动校准边界
        Center,           // 中心点校准
        Up,              // 向上滑动
        Down,            // 向下滑动
        Left,            // 向左滑动
        Right,           // 向右滑动
        Completed
    }
} 