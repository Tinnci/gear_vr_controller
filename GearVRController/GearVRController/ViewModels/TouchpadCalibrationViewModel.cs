using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using GearVRController.Models;
using GearVRController.Events;

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
        private int _autoProceedCountdownValue = 0; // 自动进入下一步的倒计时值
        private System.Timers.Timer _inactivityTimer;
        private DateTime _lastActivityTime;
        private int _circlePointCount = 0;
        private bool _canProceedToNextStep = false;
        private bool _isCurrentlyTouching = false;
        private DateTime _lastTouchEndTime = DateTime.MinValue;
        private const int TOUCH_END_TIMEOUT_MS = 500; // 触摸结束后等待时间
        private Microsoft.UI.Dispatching.DispatcherQueue? _dispatcher;
        private System.Timers.Timer? _countdownTimer;
        private const int COUNTDOWN_DURATION_MS = 3000;
        private const int COUNTDOWN_INTERVAL_MS = 50; // 更新频率50ms，使进度条更平滑
        private int _touchEndProgressValue = 0;
        private string _currentStepGuidanceMessage = "";
        private readonly Services.Interfaces.IEventAggregator _eventAggregator;

        public CalibrationStep CurrentStep
        {
            get => _currentStep;
            private set
            {
                if (_currentStep != value)
                {
                    _currentStep = value;
                    OnPropertyChanged();
                    UpdateCurrentStepGuidanceMessage();
                }
            }
        }

        public int TouchEndProgressValue
        {
            get => _touchEndProgressValue;
            private set
            {
                if (_touchEndProgressValue != value)
                {
                    _touchEndProgressValue = value;
                    OnPropertyChanged();
                }
            }
        }

        public int AutoProceedCountdownValue
        {
            get => _autoProceedCountdownValue;
            private set
            {
                if (_autoProceedCountdownValue != value)
                {
                    _autoProceedCountdownValue = value;
                    OnPropertyChanged();
                }
            }
        }

        public string CurrentStepGuidanceMessage
        {
            get => _currentStepGuidanceMessage;
            private set
            {
                if (_currentStepGuidanceMessage != value)
                {
                    _currentStepGuidanceMessage = value;
                    OnPropertyChanged();
                }
            }
        }

        public event PropertyChangedEventHandler? PropertyChanged;

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

        public DirectionalCalibrationData UpDirection => _calibrationData.UpDirection;
        public DirectionalCalibrationData DownDirection => _calibrationData.DownDirection;
        public DirectionalCalibrationData LeftDirection => _calibrationData.LeftDirection;
        public DirectionalCalibrationData RightDirection => _calibrationData.RightDirection;

        public TouchpadCalibrationViewModel(Services.Interfaces.IEventAggregator eventAggregator)
        {
            _eventAggregator = eventAggregator;
            _inactivityTimer = new System.Timers.Timer(100);
            _inactivityTimer.Elapsed += InactivityTimer_Elapsed;
            _lastActivityTime = DateTime.Now;

            if (Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread() != null)
            {
                _dispatcher = Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread();
            }
            UpdateCurrentStepGuidanceMessage();
        }

        private void InactivityTimer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
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

            // Always update progress value
            TouchEndProgressValue = newProgress;

            if (newProgress >= 100 && CurrentStep != CalibrationStep.NotStarted && CurrentStep != CalibrationStep.Completed && !isTouching)
            {
                // 如果校准停止且持续一段时间没有触摸，则尝试自动进行下一步
                _dispatcher?.TryEnqueue(() =>
                {
                    ValidateAndProceed();
                });
            }
        }

        private void ValidateAndProceed()
        {
            if (CurrentStep == CalibrationStep.CircularMotion)
            {
                if (_circlePointCount >= MIN_CIRCLE_POINTS &&
                    (_maxX - _minX) >= MIN_RANGE_THRESHOLD &&
                    (_maxY - _minY) >= MIN_RANGE_THRESHOLD)
                {
                    StatusMessage = "圆形运动校准完成。";
                    CanProceedToNextStep = true;
                    // 自动开始倒计时进入下一步
                    StartCountdown(() => ProceedToNextStep());
                }
                else
                {
                    StatusMessage = "圆形运动数据不足或范围太小。请继续在触摸板边缘划圈。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start(); // Continue monitoring
                }
            }
            else if (CurrentStep == CalibrationStep.Center)
            {
                if (_calibrationData.CenterX != 0 && _calibrationData.CenterY != 0)
                {
                    StatusMessage = "中心点校准完成。";
                    CanProceedToNextStep = true;
                    StartCountdown(() => ProceedToNextStep());
                }
                else
                {
                    StatusMessage = "中心点数据不足。请触摸并按住触摸板中心。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start();
                }
            }
            else if (CurrentStep >= CalibrationStep.Up && CurrentStep <= CalibrationStep.Right)
            {
                DirectionalCalibrationData? currentDirection = GetCurrentDirectionData();
                if (currentDirection != null && ValidateDirectionalCalibration(currentDirection))
                {
                    StatusMessage = $"{CurrentStep} 方向校准完成。";
                    CanProceedToNextStep = true;
                    StartCountdown(() => MoveToNextDirectionalStep());
                }
                else
                {
                    StatusMessage = $"{CurrentStep} 方向数据不足。请沿指定方向滑动。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start();
                }
            }
        }

        public void StartCalibration()
        {
            ResetCalibrationData();
            IsCalibrating = true;
            CurrentStep = CalibrationStep.CircularMotion;
            StatusMessage = "请在触摸板边缘划圈以校准边界。";
            _inactivityTimer.Start(); // Start monitoring inactivity
        }

        public void ProcessTouchpadData(ControllerData data)
        {
            // Reset inactivity timer on any touch activity
            _lastActivityTime = DateTime.Now;
            _isCurrentlyTouching = data.TouchpadTouched;
            if (!_isCurrentlyTouching) // If finger just lifted, record end time
            {
                _lastTouchEndTime = DateTime.Now;
            }

            if (!IsCalibrating) return;

            switch (CurrentStep)
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
            if (!data.TouchpadTouched) return;

            // Update min/max raw coordinates
            MinX = Math.Min(MinX, data.AxisX);
            MaxX = Math.Max(MaxX, data.AxisX);
            MinY = Math.Min(MinY, data.AxisY);
            MaxY = Math.Max(MaxY, data.AxisY);

            if (IsValidCirclePoint(data))
            {
                _circlePointCount++;
                StatusMessage = $"正在收集圆形运动数据... ({_circlePointCount}/{MIN_CIRCLE_POINTS})";
            }
            else
            {
                StatusMessage = "请确保在触摸板边缘划圈。";
            }

            // 动态调整CanProceedToNextStep
            CanProceedToNextStep = _circlePointCount >= MIN_CIRCLE_POINTS &&
                                   (_maxX - _minX) >= MIN_RANGE_THRESHOLD &&
                                   (_maxY - _minY) >= MIN_RANGE_THRESHOLD;
        }

        private bool IsValidCirclePoint(ControllerData data)
        {
            // 判断点是否在接近边界的区域
            // 这里可以定义一个环形区域，例如距离中心一定距离且在某个宽度范围内
            // 简单示例：判断点是否接近四个边界
            bool nearLeft = Math.Abs(data.AxisX - _minX) < MOVEMENT_THRESHOLD;
            bool nearRight = Math.Abs(data.AxisX - _maxX) < MOVEMENT_THRESHOLD;
            bool nearTop = Math.Abs(data.AxisY - _minY) < MOVEMENT_THRESHOLD;
            bool nearBottom = Math.Abs(data.AxisY - _maxY) < MOVEMENT_THRESHOLD;

            // 如果点在某个边界附近，则认为是有效的圆周运动点
            return nearLeft || nearRight || nearTop || nearBottom;
        }

        public void ProceedToNextStep()
        {
            _inactivityTimer.Stop(); // Stop inactivity timer when manually proceeding
            StopCountdown();

            switch (CurrentStep)
            {
                case CalibrationStep.CircularMotion:
                    CurrentStep = CalibrationStep.Center;
                    StatusMessage = "请触摸并按住触摸板中心。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start(); // Restart for next step
                    break;
                case CalibrationStep.Center:
                    CurrentStep = CalibrationStep.Up;
                    StatusMessage = "请向上滑动触摸板。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start();
                    break;
                case CalibrationStep.Up:
                    CurrentStep = CalibrationStep.Down;
                    StatusMessage = "请向下滑动触摸板。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start();
                    break;
                case CalibrationStep.Down:
                    CurrentStep = CalibrationStep.Left;
                    StatusMessage = "请向左滑动触摸板。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start();
                    break;
                case CalibrationStep.Left:
                    CurrentStep = CalibrationStep.Right;
                    StatusMessage = "请向右滑动触摸板。";
                    CanProceedToNextStep = false;
                    _inactivityTimer.Start();
                    break;
                case CalibrationStep.Right:
                    FinishCalibration();
                    break;
            }
        }

        private void ProcessCenterCalibration(ControllerData data)
        {
            if (!data.TouchpadTouched) return;

            // 收集中心点数据（假设触摸板中心是所有触摸点的平均值）
            // 可以根据需要收集多个样本并取平均值
            if (_calibrationData.SampleCount < SAMPLES_REQUIRED)
            {
                _calibrationData.AddSample(data.AxisX, data.AxisY);
                _centerX = (int)_calibrationData.AverageX;
                _centerY = (int)_calibrationData.AverageY;
                StatusMessage = $"正在收集中心点数据... ({_calibrationData.SampleCount}/{SAMPLES_REQUIRED})";
            }
            else
            {
                // 完成中心点校准
                StatusMessage = "中心点数据收集完成。";
                CanProceedToNextStep = true;
            }
        }

        private void ProcessDirectionalCalibration(ControllerData data)
        {
            if (!data.TouchpadTouched) return;

            DirectionalCalibrationData? currentDirection = GetCurrentDirectionData();
            if (currentDirection == null) return;

            // 在这里添加收集方向性校准数据的逻辑
            // 示例：收集每次触摸的起点和终点，计算平均向量
            // 假设我们只需要收集SAMPLES_REQUIRED个有效滑动样本
            if (currentDirection.SampleCount < SAMPLES_REQUIRED)
            {
                // 这里需要更复杂的逻辑来判断是否为"有效滑动"
                // 暂时只收集所有触摸点，这可能不准确，需要后续改进
                currentDirection.AddSample(data.AxisX, data.AxisY);
                StatusMessage = $"正在收集 {CurrentStep} 方向数据... ({currentDirection.SampleCount}/{SAMPLES_REQUIRED})";
            }
            else
            {
                StatusMessage = $"{CurrentStep} 方向数据收集完成。";
                CanProceedToNextStep = true;
            }
        }

        private DirectionalCalibrationData? GetCurrentDirectionData()
        {
            return CurrentStep switch
            {
                CalibrationStep.Up => _calibrationData.UpDirection,
                CalibrationStep.Down => _calibrationData.DownDirection,
                CalibrationStep.Left => _calibrationData.LeftDirection,
                CalibrationStep.Right => _calibrationData.RightDirection,
                _ => null,
            };
        }

        private bool IsValidDirectionalMovement(double deltaX, double deltaY)
        {
            // 判断是否是有效方向性移动的逻辑
            // 例如：检查移动的幅度是否超过阈值，以及方向是否与当前校准方向一致
            return Math.Sqrt(deltaX * deltaX + deltaY * deltaY) > MOVEMENT_THRESHOLD;
        }

        private bool ValidateDirectionalCalibration(DirectionalCalibrationData direction)
        {
            // 验证方向性校准数据是否有效
            // 例如：检查样本数是否足够，或者平均幅度是否达到要求
            return direction.SampleCount >= SAMPLES_REQUIRED && direction.Magnitude > MIN_RANGE_THRESHOLD; // 示例检查
        }

        private string GetDirectionalGuidance()
        {
            return CurrentStep switch
            {
                CalibrationStep.Up => "请向上滑动触摸板，确保从中心向顶部滑动。",
                CalibrationStep.Down => "请向下滑动触摸板，确保从中心向底部滑动。",
                CalibrationStep.Left => "请向左滑动触摸板，确保从中心向左侧滑动。",
                CalibrationStep.Right => "请向右滑动触摸板，确保从中心向右侧滑动。",
                _ => "",
            };
        }

        private void MoveToNextDirectionalStep()
        {
            ProceedToNextStep(); // Reuse existing logic
        }

        public void FinishCalibration()
        {
            _inactivityTimer.Stop();
            StopCountdown(); // Ensure countdown is stopped

            IsCalibrating = false;
            CurrentStep = CalibrationStep.Completed;
            StatusMessage = "校准完成！数据已保存。";
            // 在这里触发校准完成事件，并传递校准数据
            _eventAggregator.Publish(new CalibrationCompletedEvent(_calibrationData));
        }

        public void CancelCalibration()
        {
            _inactivityTimer.Stop();
            StopCountdown(); // Ensure countdown is stopped

            IsCalibrating = false;
            CurrentStep = CalibrationStep.NotStarted;
            StatusMessage = "校准已取消。";
            ResetCalibrationData();
        }

        public void ResetCalibration()
        {
            _inactivityTimer.Stop();
            StopCountdown(); // Ensure countdown is stopped
            ResetCalibrationData();
            IsCalibrating = false;
            CurrentStep = CalibrationStep.NotStarted;
            StatusMessage = "校准数据已重置。";
            UpdateCurrentStepGuidanceMessage(); // Reset guidance message as well
        }

        private void ResetCalibrationData()
        {
            _minX = int.MaxValue;
            _maxX = int.MinValue;
            _minY = int.MaxValue;
            _maxY = int.MinValue;
            _centerX = 0;
            _centerY = 0;
            _circlePointCount = 0;
            _isCurrentlyTouching = false;
            _lastTouchEndTime = DateTime.MinValue;
            _calibrationData.Reset(); // Use the Reset method of TouchpadCalibrationData
            TouchEndProgressValue = 0;
            AutoProceedCountdownValue = 0;
        }

        public void Dispose()
        {
            _inactivityTimer?.Stop();
            _inactivityTimer?.Dispose();
            _countdownTimer?.Stop(); // Null check before stopping
            _countdownTimer?.Dispose(); // Null check before disposing
        }

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

        private void StartCountdown(Action onComplete)
        {
            StopCountdown(); // Stop any existing countdown
            AutoProceedCountdownValue = COUNTDOWN_DURATION_MS / 1000; // Display initial seconds

            _countdownTimer = new System.Timers.Timer(COUNTDOWN_INTERVAL_MS);
            double remainingTime = COUNTDOWN_DURATION_MS;

            _countdownTimer.Elapsed += (sender, e) =>
            {
                remainingTime -= COUNTDOWN_INTERVAL_MS;
                _dispatcher?.TryEnqueue(() =>
                {
                    if (remainingTime <= 0)
                    {
                        StopCountdown();
                        onComplete?.Invoke();
                    }
                    else
                    {
                        // 更新倒计时显示，精确到秒
                        AutoProceedCountdownValue = (int)Math.Ceiling(remainingTime / 1000);
                    }
                });
            };
            _countdownTimer.Start();
        }

        private void StopCountdown()
        {
            _countdownTimer?.Stop();
            _countdownTimer?.Dispose();
            _countdownTimer = null; // Clear the timer reference
            AutoProceedCountdownValue = 0; // Reset display
        }

        private void UpdateCurrentStepGuidanceMessage()
        {
            CurrentStepGuidanceMessage = CurrentStep switch
            {
                CalibrationStep.NotStarted => "点击 '开始校准' 按钮开始。",
                CalibrationStep.CircularMotion => "请在触摸板边缘划圈，以便系统学习您的触摸边界。",
                CalibrationStep.Center => "请用一根手指触摸并按住触摸板中心，直到倒计时结束。",
                CalibrationStep.Up => "请将手指从触摸板中心向上滑动。",
                CalibrationStep.Down => "请将手指从触摸板中心向下滑动。",
                CalibrationStep.Left => "请将手指从触摸板中心向左滑动。",
                CalibrationStep.Right => "请将手指从触摸板中心向右滑动。",
                CalibrationStep.Completed => "校准已完成。",
                _ => "",
            };
            StatusMessage = CurrentStepGuidanceMessage; // Also set status message
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

        // Properties for center calibration
        public double AverageX { get; private set; }
        public double AverageY { get; private set; }
        public int SampleCount { get; private set; }

        public void AddSample(int x, int y)
        {
            double newCount = SampleCount + 1;
            AverageX = (AverageX * SampleCount + x) / newCount;
            AverageY = (AverageY * SampleCount + y) / newCount;
            SampleCount = (int)newCount;
        }

        public void Reset()
        {
            MinX = int.MaxValue;
            MaxX = int.MinValue;
            MinY = int.MaxValue;
            MaxY = int.MinValue;
            CenterX = 0;
            CenterY = 0;
            AverageX = 0;
            AverageY = 0;
            SampleCount = 0;
            UpDirection.Reset();
            DownDirection.Reset();
            LeftDirection.Reset();
            RightDirection.Reset();
        }

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
        CircularMotion,
        Center,
        Up,
        Down,
        Left,
        Right,
        Completed
    }
}