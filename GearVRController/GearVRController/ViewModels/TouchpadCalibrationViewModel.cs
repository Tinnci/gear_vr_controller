using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using GearVRController.Models;
using GearVRController.Events;

namespace GearVRController.ViewModels
{
    /// <summary>
    /// TouchpadCalibrationViewModel 负责处理 Gear VR 控制器的触摸板校准逻辑。
    /// 它指导用户完成多步校准过程（包括圆周运动、中心点和方向校准），
    /// 收集数据，计算校准参数，并向其他组件发布校准完成事件。
    /// </summary>
    public class TouchpadCalibrationViewModel : INotifyPropertyChanged
    {
        /// <summary>
        /// 记录触摸板X轴的最小值。
        /// </summary>
        private int _minX = int.MaxValue;
        /// <summary>
        /// 记录触摸板X轴的最大值。
        /// </summary>
        private int _maxX = int.MinValue;
        /// <summary>
        /// 记录触摸板Y轴的最小值。
        /// </summary>
        private int _minY = int.MaxValue;
        /// <summary>
        /// 记录触摸板Y轴的最大值。
        /// </summary>
        private int _maxY = int.MinValue;
        /// <summary>
        /// 计算出的触摸板中心X坐标。
        /// </summary>
        private int _centerX;
        /// <summary>
        /// 计算出的触摸板中心Y坐标。
        /// </summary>
        private int _centerY;
        /// <summary>
        /// 显示给用户的校准状态消息。
        /// </summary>
        private string _statusMessage = "准备开始校准...";
        /// <summary>
        /// 指示校准过程是否正在进行。
        /// </summary>
        private bool _isCalibrating;
        /// <summary>
        /// 当前的校准步骤。
        /// </summary>
        private CalibrationStep _currentStep = CalibrationStep.NotStarted;
        /// <summary>
        /// 存储校准过程中收集和计算出的所有校准数据。
        /// </summary>
        private TouchpadCalibrationData _calibrationData = new TouchpadCalibrationData();
        /// <summary>
        /// 每个方向校准所需的样本数。
        /// </summary>
        private const int SAMPLES_REQUIRED = 10; // 每个方向需要的样本数
        /// <summary>
        /// 检测触摸板移动的阈值。如果位移小于此值，则认为没有有效移动。
        /// </summary>
        private const double MOVEMENT_THRESHOLD = 50; // 移动检测阈值
        /// <summary>
        /// 触摸板X或Y轴的最小有效范围。如果校准范围小于此值，则认为校准无效。
        /// </summary>
        private const double MIN_RANGE_THRESHOLD = 200; // 最小范围阈值
        /// <summary>
        /// 圆周运动校准中所需的最小有效点数。
        /// </summary>
        private const int MIN_CIRCLE_POINTS = 20; // 最小圆周运动点数
        /// <summary>
        /// 用于检测用户无操作的超时时间（毫秒）。
        /// </summary>
        private const int INACTIVITY_TIMEOUT_MS = 1000; // 停止操作超时时间（毫秒）
        /// <summary>
        /// 在某些校准步骤中需要的最小数据点数。
        /// </summary>
        private const int MIN_POINTS_REQUIRED = 5; // 最小需要的点数
        /// <summary>
        /// 自动进入下一步的倒计时值（例如，在中心点校准后）。
        /// </summary>
        private int _autoProceedCountdownValue = 0;
        /// <summary>
        /// 用于检测用户无操作的计时器。
        /// </summary>
        private System.Timers.Timer _inactivityTimer;
        /// <summary>
        /// 最后一次触摸板有活动的时间。
        /// </summary>
        private DateTime _lastActivityTime;
        /// <summary>
        /// 圆周运动校准过程中收集到的有效点数。
        /// </summary>
        private int _circlePointCount = 0;
        /// <summary>
        /// 指示是否可以进入下一个校准步骤。
        /// </summary>
        private bool _canProceedToNextStep = false;
        /// <summary>
        /// 内部跟踪触摸板是否正在被触摸。
        /// </summary>
        private bool _isCurrentlyTouching = false;
        /// <summary>
        /// 记录触摸结束的时间，用于计算触摸结束后的等待时间。
        /// </summary>
        private DateTime _lastTouchEndTime = DateTime.MinValue;
        /// <summary>
        /// 触摸结束后等待的超时时间（毫秒），用于确保触摸完全释放。
        /// </summary>
        private const int TOUCH_END_TIMEOUT_MS = 500; // 触摸结束后等待时间
        /// <summary>
        /// DispatcherQueue 实例，用于在 UI 线程上执行操作。
        /// </summary>
        private Microsoft.UI.Dispatching.DispatcherQueue? _dispatcher;
        /// <summary>
        /// 用于自动进入下一步的倒计时计时器。
        /// </summary>
        private System.Timers.Timer? _countdownTimer;
        /// <summary>
        /// 倒计时的总持续时间（毫秒）。
        /// </summary>
        private const int COUNTDOWN_DURATION_MS = 3000;
        /// <summary>
        /// 倒计时更新的频率（毫秒），影响进度条的平滑度。
        /// </summary>
        private const int COUNTDOWN_INTERVAL_MS = 50; // 更新频率50ms，使进度条更平滑
        /// <summary>
        /// 触摸结束进度条的值。
        /// </summary>
        private int _touchEndProgressValue = 0;
        /// <summary>
        /// 当前校准步骤的指导信息，显示在 UI 上。
        /// </summary>
        private string _currentStepGuidanceMessage = "";
        /// <summary>
        /// 事件聚合器，用于发布校准完成事件。
        /// </summary>
        private readonly Services.Interfaces.IEventAggregator _eventAggregator;

        /// <summary>
        /// 获取当前校准步骤。
        /// </summary>
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

        /// <summary>
        /// 获取或设置触摸结束进度条的当前值。
        /// </summary>
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

        /// <summary>
        /// 获取或设置自动进入下一步的倒计时值。
        /// </summary>
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

        /// <summary>
        /// 获取或设置当前校准步骤的指导信息。
        /// </summary>
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

        /// <summary>
        /// 当 ViewModel 的属性发生变化时触发的事件。
        /// </summary>
        public event PropertyChangedEventHandler? PropertyChanged;

        /// <summary>
        /// 获取触摸板X轴的最小值。
        /// </summary>
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

        /// <summary>
        /// 获取触摸板X轴的最大值。
        /// </summary>
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

        /// <summary>
        /// 获取触摸板Y轴的最小值。
        /// </summary>
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

        /// <summary>
        /// 获取触摸板Y轴的最大值。
        /// </summary>
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

        /// <summary>
        /// 获取计算出的触摸板中心X坐标。
        /// </summary>
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

        /// <summary>
        /// 获取计算出的触摸板中心Y坐标。
        /// </summary>
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

        /// <summary>
        /// 获取当前校准状态消息。
        /// </summary>
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

        /// <summary>
        /// 获取指示校准过程是否正在进行的值。
        /// </summary>
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

        /// <summary>
        /// 获取或设置是否可以进入下一个校准步骤。
        /// </summary>
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

        /// <summary>
        /// 获取向上方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData UpDirection => _calibrationData.UpDirection;
        /// <summary>
        /// 获取向下方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData DownDirection => _calibrationData.DownDirection;
        /// <summary>
        /// 获取向左方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData LeftDirection => _calibrationData.LeftDirection;
        /// <summary>
        /// 获取向右方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData RightDirection => _calibrationData.RightDirection;

        /// <summary>
        /// TouchpadCalibrationViewModel 的构造函数。
        /// </summary>
        /// <param name="eventAggregator">事件聚合器，用于发布校准完成事件。</param>
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

        /// <summary>
        /// 处理非活动计时器触发的事件。
        /// 用于监控触摸板的非活动状态，并在超时后触发自动进入下一步的逻辑。
        /// </summary>
        /// <param name="sender">事件发送者。</param>
        /// <param name="e">事件参数。</param>
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

            _dispatcher?.TryEnqueue(() =>
            {
                TouchEndProgressValue = newProgress;

                if (newProgress == 100 && CurrentStep == CalibrationStep.CircularMotion && _circlePointCount >= MIN_CIRCLE_POINTS)
                {
                    CanProceedToNextStep = true;
                    StatusMessage = "圆周运动校准完成！触摸板已抬起。请点击'下一步'继续。";
                }
                else if (newProgress == 100 && CurrentStep == CalibrationStep.Center && _calibrationData.SampleCount > 0)
                {
                    CanProceedToNextStep = true;
                    StatusMessage = "中心点校准完成！触摸板已抬起。请点击'下一步'继续。";
                }
                else if (newProgress == 100 && CurrentStep >= CalibrationStep.Up && CurrentStep <= CalibrationStep.Right && GetCurrentDirectionData()?.SampleCount >= SAMPLES_REQUIRED)
                {
                    CanProceedToNextStep = true;
                    StatusMessage = $"当前方向校准完成！触摸板已抬起。请点击'下一步'继续。 ({GetCurrentDirectionData()?.SampleCount} 点)";
                }
                else
                {
                    CanProceedToNextStep = false;
                }
            });

            if ((DateTime.Now - _lastActivityTime).TotalMilliseconds > INACTIVITY_TIMEOUT_MS)
            {
                _dispatcher?.TryEnqueue(() =>
                {
                    if (CurrentStep == CalibrationStep.Center && _calibrationData.SampleCount > 0 && !CanProceedToNextStep)
                    {
                        // 如果在中心校准且有数据但无法自动进入下一步（例如，未抬起手指），则强制进入下一步
                        ProceedToNextStep();
                    }
                    else if (CurrentStep >= CalibrationStep.Up && CurrentStep <= CalibrationStep.Right && GetCurrentDirectionData()?.SampleCount > 0 && !CanProceedToNextStep)
                    {
                        ProceedToNextStep();
                    }
                });
            }
        }

        /// <summary>
        /// 根据当前校准步骤，验证收集的数据并决定是否可以进入下一步。
        /// 此方法会在每次触摸板活动停止或手动点击下一步时触发，以检查当前步骤的数据是否有效。
        /// </summary>
        private void ValidateAndProceed()
        {
            bool isValid = false;
            string validationMessage = "";

            switch (CurrentStep)
            {
                case CalibrationStep.CircularMotion:
                    if (_circlePointCount >= MIN_CIRCLE_POINTS && (DateTime.Now - _lastTouchEndTime).TotalMilliseconds >= TOUCH_END_TIMEOUT_MS)
                    {
                        // Check if the overall range is large enough for a circular motion
                        if ((_maxX - _minX) > MIN_RANGE_THRESHOLD && (_maxY - _minY) > MIN_RANGE_THRESHOLD)
                        {
                            isValid = true;
                            StatusMessage = "圆周运动校准成功！";
                        }
                        else
                        {
                            validationMessage = "检测到的运动范围太小。请确保在触摸板边缘完整划圈。";
                            StatusMessage = validationMessage;
                            ResetCalibrationData(); // Reset for re-attempt
                            CurrentStep = CalibrationStep.CircularMotion; // Stay on this step
                            CanProceedToNextStep = false; // Cannot proceed if invalid
                        }
                    }
                    else
                    {
                        validationMessage = $"需要至少 {MIN_CIRCLE_POINTS} 个圆周点和触摸板抬起。当前点数: {_circlePointCount}";
                        StatusMessage = validationMessage;
                    }
                    break;
                case CalibrationStep.Center:
                    if (_calibrationData.SampleCount > 0 && (DateTime.Now - _lastTouchEndTime).TotalMilliseconds >= TOUCH_END_TIMEOUT_MS)
                    {
                        isValid = true;
                        _centerX = (int)_calibrationData.AverageX;
                        _centerY = (int)_calibrationData.AverageY;
                        StatusMessage = $"中心点校准成功！中心点: ({_centerX}, {_centerY})";
                    }
                    else
                    {
                        validationMessage = "需要触摸中心点。";
                        StatusMessage = validationMessage;
                    }
                    break;
                case CalibrationStep.Up:
                case CalibrationStep.Down:
                case CalibrationStep.Left:
                case CalibrationStep.Right:
                    var currentDirectionData = GetCurrentDirectionData();
                    if (currentDirectionData != null && currentDirectionData.SampleCount >= SAMPLES_REQUIRED && (DateTime.Now - _lastTouchEndTime).TotalMilliseconds >= TOUCH_END_TIMEOUT_MS)
                    {
                        if (ValidateDirectionalCalibration(currentDirectionData))
                        {
                            isValid = true;
                            StatusMessage = $"{CurrentStep} 方向校准成功！ ({currentDirectionData.SampleCount} 点)";
                        }
                        else
                        {
                            validationMessage = $"检测到的 {CurrentStep} 方向运动不符合预期。请确保沿单一方向平稳滑动。";
                            StatusMessage = validationMessage;
                            currentDirectionData.Reset(); // Reset current direction data for re-attempt
                            CanProceedToNextStep = false; // Cannot proceed if invalid
                        }
                    }
                    else
                    {
                        validationMessage = $"需要至少 {SAMPLES_REQUIRED} 个 {CurrentStep} 方向的样本和触摸板抬起。当前样本数: {currentDirectionData?.SampleCount ?? 0}";
                        StatusMessage = validationMessage;
                    }
                    break;
            }

            CanProceedToNextStep = isValid; // Update button state
        }

        /// <summary>
        /// 启动触摸板校准过程。
        /// 将校准步骤重置为 NotStarted，并开始监控活动。
        /// </summary>
        public void StartCalibration()
        {
            ResetCalibrationData();
            CurrentStep = CalibrationStep.CircularMotion;
            IsCalibrating = true;
            _inactivityTimer.Start(); // 启动不活动计时器
            _lastActivityTime = DateTime.Now;
            CanProceedToNextStep = false;
            StatusMessage = "请在触摸板边缘划圈以校准范围。";
            _circlePointCount = 0; // 重置圆周点计数
        }

        /// <summary>
        /// 处理从控制器接收到的触摸板数据。
        /// 根据当前的校准步骤，将数据分发给不同的处理方法。
        /// </summary>
        /// <param name="data">包含触摸板信息的控制器数据。</param>
        public void ProcessTouchpadData(ControllerData data)
        {
            _isCurrentlyTouching = data.TouchpadTouched;
            if (!data.TouchpadTouched)
            {
                _lastTouchEndTime = DateTime.Now; // 记录触摸结束时间
            }
            else
            {
                _lastActivityTime = DateTime.Now; // 更新最后活动时间
            }

            if (!IsCalibrating) return; // 只有在校准模式下才处理数据

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

        /// <summary>
        /// 处理圆周运动校准步骤的触摸板数据。
        /// 收集触摸板的最小/最大X/Y值以确定其有效范围。
        /// </summary>
        /// <param name="data">当前的控制器数据。</param>
        private void ProcessCircularMotion(ControllerData data)
        {
            if (!data.TouchpadTouched) return;

            _lastActivityTime = DateTime.Now; // 更新最后活动时间

            int currentX = data.AxisX;
            int currentY = data.AxisY;

            MinX = Math.Min(MinX, currentX);
            MaxX = Math.Max(MaxX, currentX);
            MinY = Math.Min(MinY, currentY);
            MaxY = Math.Max(MaxY, currentY);

            if (IsValidCirclePoint(data))
            {
                _circlePointCount++;
                // System.Diagnostics.Debug.WriteLine($"[Calibration] Circle Point Count: {{_circlePointCount}}");
            }

            // 检查是否已经收集到足够的点并且触摸板已抬起
            // 自动进入下一步的逻辑由 InactivityTimer_Elapsed 触发 ValidateAndProceed 决定
        }

        /// <summary>
        /// 判断当前触摸板点是否是有效圆周运动点。
        /// 检查点是否在触摸板边缘区域且有足够的移动。
        /// </summary>
        /// <param name="data">当前的控制器数据。</param>
        /// <returns>如果是有效圆周点则返回 true，否则返回 false。</returns>
        private bool IsValidCirclePoint(ControllerData data)
        {
            int currentX = data.AxisX;
            int currentY = data.AxisY;

            // 定义边缘区域的阈值 (例如，距离边缘X%以内)
            const int edgeThreshold = 100; // 根据实际触摸板范围调整

            bool isAtEdgeX = currentX <= edgeThreshold || currentX >= (1023 - edgeThreshold);
            bool isAtEdgeY = currentY <= edgeThreshold || currentY >= (1023 - edgeThreshold);

            // 确保点在边缘区域内，并且当前移动量超过阈值
            // 这里的移动量检测应该更复杂，可能需要与上一个点比较
            // 简化为只要在边缘区就算有效
            return (isAtEdgeX || isAtEdgeY);
        }

        /// <summary>
        /// 推进到下一个校准步骤。
        /// 在验证当前步骤的数据有效性后，此方法会更新 `CurrentStep` 并重置相关数据以准备下一个校准阶段。
        /// </summary>
        public void ProceedToNextStep()
        {
            StopCountdown(); // 停止自动进入下一步的倒计时

            ValidateAndProceed(); // 在进入下一步前验证当前步骤

            if (CanProceedToNextStep)
            {
                switch (CurrentStep)
                {
                    case CalibrationStep.CircularMotion:
                        CurrentStep = CalibrationStep.Center;
                        _calibrationData.MinX = MinX;
                        _calibrationData.MaxX = MaxX;
                        _calibrationData.MinY = MinY;
                        _calibrationData.MaxY = MaxY;
                        _calibrationData.Reset(); // Reset for center calibration
                        CanProceedToNextStep = false;
                        break;
                    case CalibrationStep.Center:
                        CurrentStep = CalibrationStep.Up;
                        _calibrationData.CenterX = _centerX;
                        _calibrationData.CenterY = _centerY;
                        _calibrationData.UpDirection.Reset(); // Reset for up calibration
                        CanProceedToNextStep = false;
                        break;
                    case CalibrationStep.Up:
                        CurrentStep = CalibrationStep.Down;
                        _calibrationData.DownDirection.Reset(); // Reset for down calibration
                        CanProceedToNextStep = false;
                        break;
                    case CalibrationStep.Down:
                        CurrentStep = CalibrationStep.Left;
                        _calibrationData.LeftDirection.Reset(); // Reset for left calibration
                        CanProceedToNextStep = false;
                        break;
                    case CalibrationStep.Left:
                        CurrentStep = CalibrationStep.Right;
                        _calibrationData.RightDirection.Reset(); // Reset for right calibration
                        CanProceedToNextStep = false;
                        break;
                    case CalibrationStep.Right:
                        CurrentStep = CalibrationStep.Completed;
                        FinishCalibration(); // Finish after all directions
                        break;
                    case CalibrationStep.NotStarted:
                    case CalibrationStep.Completed:
                        // No action needed or already handled by StartCalibration
                        break;
                }
            }
            UpdateCurrentStepGuidanceMessage(); // Update guidance after step change
        }

        /// <summary>
        /// 处理中心点校准步骤的触摸板数据。
        /// 收集触摸板中心区域的样本点，用于计算中心坐标。
        /// </summary>
        /// <param name="data">当前的控制器数据。</param>
        private void ProcessCenterCalibration(ControllerData data)
        {
            if (!data.TouchpadTouched) return;

            _lastActivityTime = DateTime.Now; // 更新最后活动时间

            // 假设中心点校准是用户触摸触摸板中心位置
            // 收集中心点周围的数据
            _calibrationData.AddSample(data.AxisX, data.AxisY);

            if (_calibrationData.SampleCount >= SAMPLES_REQUIRED)
            {
                // 达到样本数量后，自动触发 ValidateAndProceed 逻辑
                // 具体进入下一步的逻辑由 InactivityTimer_Elapsed 触发 ValidateAndProceed 决定
            }
        }

        /// <summary>
        /// 处理方向性校准步骤的触摸板数据（上、下、左、右）。
        /// 收集特定方向的移动样本，用于确定该方向的校准参数。
        /// </summary>
        /// <param name="data">当前的控制器数据。</param>
        private void ProcessDirectionalCalibration(ControllerData data)
        {
            if (!data.TouchpadTouched) return;

            _lastActivityTime = DateTime.Now; // 更新最后活动时间

            var currentDirectionData = GetCurrentDirectionData();
            if (currentDirectionData == null) return; // Should not happen if CurrentStep is valid

            // 计算与校准中心点的增量
            double deltaX = data.AxisX - _centerX;
            double deltaY = data.AxisY - _centerY;

            if (IsValidDirectionalMovement(deltaX, deltaY))
            {
                currentDirectionData.AddSample(deltaX, deltaY);
                // System.Diagnostics.Debug.WriteLine($"[Calibration] {CurrentStep} Sample Count: {currentDirectionData.SampleCount}");
            }

            if (currentDirectionData.SampleCount >= SAMPLES_REQUIRED)
            {
                // 达到样本数量后，自动触发 ValidateAndProceed 逻辑
                // 具体进入下一步的逻辑由 InactivityTimer_Elapsed 触发 ValidateAndProceed 决定
            }
        }

        /// <summary>
        /// 获取当前校准步骤对应的方向性校准数据对象。
        /// </summary>
        /// <returns>当前步骤的 DirectionalCalibrationData 对象，如果当前步骤不是方向校准则返回 null。</returns>
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

        /// <summary>
        /// 判断当前移动增量是否为有效方向性移动。
        /// </summary>
        /// <param name="deltaX">X轴方向的位移增量。</param>
        /// <param name="deltaY">Y轴方向的位移增量。</param>
        /// <returns>如果是有效方向性移动则返回 true，否则返回 false。</returns>
        private bool IsValidDirectionalMovement(double deltaX, double deltaY)
        {
            // 简单判断是否有显著移动
            return Math.Abs(deltaX) > MOVEMENT_THRESHOLD || Math.Abs(deltaY) > MOVEMENT_THRESHOLD;
        }

        /// <summary>
        /// 验证方向性校准数据是否有效。
        /// 检查收集到的样本数量和移动幅度是否符合预期。
        /// </summary>
        /// <param name="direction">要验证的方向性校准数据。</param>
        /// <returns>如果校准数据有效则返回 true，否则返回 false。</returns>
        private bool ValidateDirectionalCalibration(DirectionalCalibrationData direction)
        {
            if (direction.SampleCount < SAMPLES_REQUIRED) return false;
            // 可以添加更多逻辑来验证移动是否确实在一个方向上
            // 例如，检查平均X或Y增量是否显著
            return direction.Magnitude > MIN_RANGE_THRESHOLD; // 假设幅度大于某个阈值表示有效移动
        }

        /// <summary>
        /// 根据当前校准步骤获取对应的指导信息。
        /// </summary>
        /// <returns>当前步骤的指导信息字符串。</returns>
        private string GetDirectionalGuidance()
        {
            return CurrentStep switch
            {
                CalibrationStep.Up => "请在触摸板上方向上平稳滑动。",
                CalibrationStep.Down => "请在触摸板上方向下平稳滑动。",
                CalibrationStep.Left => "请在触摸板上方向左平稳滑动。",
                CalibrationStep.Right => "请在触摸板上方向右平稳滑动。",
                _ => "请进行方向校准。"
            };
        }

        /// <summary>
        /// 移动到下一个方向校准步骤。
        /// </summary>
        private void MoveToNextDirectionalStep()
        {
            // This method is primarily handled by ProceedToNextStep now, but kept if needed for other flows.
        }

        /// <summary>
        /// 完成校准过程。
        /// 发布校准完成事件，并停止所有计时器。
        /// </summary>
        public void FinishCalibration()
        {
            IsCalibrating = false;
            StatusMessage = "校准完成！数据已保存。";
            _inactivityTimer.Stop();
            StopCountdown();
            CanProceedToNextStep = false;
            _eventAggregator.Publish(new CalibrationCompletedEvent(_calibrationData));
        }

        /// <summary>
        /// 取消校准过程。
        /// 重置所有校准数据和状态，并停止计时器。
        /// </summary>
        public void CancelCalibration()
        {
            ResetCalibrationData();
            IsCalibrating = false;
            CurrentStep = CalibrationStep.NotStarted;
            StatusMessage = "校准已取消。";
            _inactivityTimer.Stop();
            StopCountdown();
            CanProceedToNextStep = false;
        }

        /// <summary>
        /// 重置整个校准过程。
        /// </summary>
        public void ResetCalibration()
        {
            ResetCalibrationData();
            CurrentStep = CalibrationStep.NotStarted;
            IsCalibrating = false;
            StatusMessage = "校准已重置。请点击'开始校准'重新开始。";
            _inactivityTimer.Stop();
            StopCountdown();
            CanProceedToNextStep = false;
        }

        /// <summary>
        /// 重置内部校准数据和状态。
        /// </summary>
        private void ResetCalibrationData()
        {
            _minX = int.MaxValue;
            _maxX = int.MinValue;
            _minY = int.MaxValue;
            _maxY = int.MinValue;
            _centerX = 0;
            _centerY = 0;
            _circlePointCount = 0;
            _calibrationData.Reset();
        }

        /// <summary>
        /// 释放 TouchpadCalibrationViewModel 实例持有的资源。
        /// 包括停止并释放计时器。
        /// </summary>
        public void Dispose()
        {
            _inactivityTimer.Stop();
            _inactivityTimer.Elapsed -= InactivityTimer_Elapsed;
            _inactivityTimer.Dispose();
            _countdownTimer?.Stop();
            _countdownTimer?.Dispose();
        }

        /// <summary>
        /// 当属性值改变时触发 PropertyChanged 事件。
        /// </summary>
        /// <param name="propertyName">发生变化的属性名称。如果为 null，则由编译器自动填充。</param>
        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

        /// <summary>
        /// 启动一个倒计时，并在倒计时结束后执行指定的操作。
        /// </summary>
        /// <param name="onComplete">倒计时完成后要执行的 Action。</param>
        private void StartCountdown(Action onComplete)
        {
            StopCountdown(); // Stop any existing countdown

            _countdownTimer = new System.Timers.Timer(COUNTDOWN_INTERVAL_MS);
            DateTime startTime = DateTime.Now;
            _countdownTimer.Elapsed += (s, e) =>
            {
                _dispatcher?.TryEnqueue(() =>
                {
                    double elapsedMs = (DateTime.Now - startTime).TotalMilliseconds;
                    AutoProceedCountdownValue = (int)Math.Max(0, COUNTDOWN_DURATION_MS - elapsedMs);

                    if (AutoProceedCountdownValue <= 0)
                    {
                        StopCountdown();
                        onComplete?.Invoke();
                    }
                });
            };
            _countdownTimer.Start();
        }

        /// <summary>
        /// 停止当前正在进行的倒计时。
        /// </summary>
        private void StopCountdown()
        {
            if (_countdownTimer != null)
            {
                _countdownTimer.Stop();
                _countdownTimer.Dispose();
                _countdownTimer = null;
                AutoProceedCountdownValue = 0;
            }
        }

        /// <summary>
        /// 根据当前校准步骤更新显示给用户的指导信息。
        /// </summary>
        private void UpdateCurrentStepGuidanceMessage()
        {
            CurrentStepGuidanceMessage = CurrentStep switch
            {
                CalibrationStep.NotStarted => "点击'开始校准'按钮启动触摸板校准。",
                CalibrationStep.CircularMotion => "请用手指在触摸板边缘划一个大圈，以校准触摸板的边界和范围。完成后请抬起手指，系统将自动进入下一步，或手动点击'下一步'按钮。",
                CalibrationStep.Center => "请用手指轻触并按住触摸板中心点，收集足够的数据以精确确定中心。完成后请抬起手指，系统将自动进入下一步，或手动点击'下一步'按钮。",
                CalibrationStep.Up => "请在触摸板上方向上平稳滑动，收集向上方向的校准数据。完成后请抬起手指，系统将自动进入下一步，或手动点击'下一步'按钮。",
                CalibrationStep.Down => "请在触摸板上方向下平稳滑动，收集向下方向的校准数据。完成后请抬起手指，系统将自动进入下一步，或手动点击'下一步'按钮。",
                CalibrationStep.Left => "请在触摸板上方向左平稳滑动，收集向左方向的校准数据。完成后请抬起手指，系统将自动进入下一步，或手动点击'下一步'按钮。",
                CalibrationStep.Right => "请在触摸板上方向右平稳滑动，收集向右方向的校准数据。完成后请抬起手指，系统将自动进入下一步，或手动点击'下一步'按钮。",
                CalibrationStep.Completed => "恭喜！触摸板校准已完成。您现在可以开始使用控制器。",
                _ => "未知校准步骤。"
            };
        }
    }

    /// <summary>
    /// 表示触摸板的整体校准数据。
    /// 包括触摸板的边界 (Min/Max X/Y)，中心点 (Center X/Y)，以及四个方向的校准数据。
    /// </summary>
    public class TouchpadCalibrationData
    {
        /// <summary>
        /// 触摸板X轴的最小原始值。
        /// </summary>
        public int MinX { get; set; }
        /// <summary>
        /// 触摸板X轴的最大原始值。
        /// </summary>
        public int MaxX { get; set; }
        /// <summary>
        /// 触摸板Y轴的最小原始值。
        /// </summary>
        public int MinY { get; set; }
        /// <summary>
        /// 触摸板Y轴的最大原始值。
        /// </summary>
        public int MaxY { get; set; }
        /// <summary>
        /// 触摸板中心点X坐标的原始值。
        /// </summary>
        public int CenterX { get; set; }
        /// <summary>
        /// 触摸板中心点Y坐标的原始值。
        /// </summary>
        public int CenterY { get; set; }

        /// <summary>
        /// 在中心点校准过程中收集的X坐标平均值。
        /// </summary>
        public double AverageX { get; private set; }
        /// <summary>
        /// 在中心点校准过程中收集的Y坐标平均值。
        /// </summary>
        public double AverageY { get; private set; }
        /// <summary>
        /// 在中心点校准过程中收集的样本数量。
        /// </summary>
        public int SampleCount { get; private set; }

        /// <summary>
        /// 添加一个样本点到中心点校准数据中。
        /// </summary>
        /// <param name="x">样本点的X坐标。</param>
        /// <param name="y">样本点的Y坐标。</param>
        public void AddSample(int x, int y)
        {
            AverageX = (AverageX * SampleCount + x) / (SampleCount + 1);
            AverageY = (AverageY * SampleCount + y) / (SampleCount + 1);
            SampleCount++;
        }

        /// <summary>
        /// 重置中心点校准数据。
        /// </summary>
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

        /// <summary>
        /// 向上方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData UpDirection { get; set; } = new DirectionalCalibrationData();
        /// <summary>
        /// 向下方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData DownDirection { get; set; } = new DirectionalCalibrationData();
        /// <summary>
        /// 向左方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData LeftDirection { get; set; } = new DirectionalCalibrationData();
        /// <summary>
        /// 向右方向的校准数据。
        /// </summary>
        public DirectionalCalibrationData RightDirection { get; set; } = new DirectionalCalibrationData();

    }

    /// <summary>
    /// 表示单个方向的触摸板校准数据（例如，向上滑动）。
    /// 包含该方向的平均位移和幅度。
    /// </summary>
    public class DirectionalCalibrationData
    {
        /// <summary>
        /// 该方向的平均X轴位移。
        /// </summary>
        public double AverageX { get; set; }
        /// <summary>
        /// 该方向的平均Y轴位移。
        /// </summary>
        public double AverageY { get; set; }
        /// <summary>
        /// 该方向的移动幅度。
        /// </summary>
        public double Magnitude { get; set; }
        /// <summary>
        /// 收集到的样本数量。
        /// </summary>
        public int SampleCount { get; set; }

        /// <summary>
        /// 添加一个样本到方向性校准数据中。
        /// </summary>
        /// <param name="x">样本的X轴位移。</param>
        /// <param name="y">样本的Y轴位移。</param>
        public void AddSample(double x, double y)
        {
            AverageX = (AverageX * SampleCount + x) / (SampleCount + 1);
            AverageY = (AverageY * SampleCount + y) / (SampleCount + 1);
            SampleCount++;
            Magnitude = Math.Sqrt(AverageX * AverageX + AverageY * AverageY); // Update magnitude after adding sample
        }

        /// <summary>
        /// 重置方向性校准数据。
        /// </summary>
        public void Reset()
        {
            AverageX = 0;
            AverageY = 0;
            Magnitude = 0;
            SampleCount = 0;
        }
    }

    /// <summary>
    /// 定义触摸板校准过程中的各个步骤。
    /// </summary>
    public enum CalibrationStep
    {
        /// <summary>
        /// 校准未开始。
        /// </summary>
        NotStarted,
        /// <summary>
        /// 圆周运动校准步骤，用于确定触摸板的边界。
        /// </summary>
        CircularMotion,
        /// <summary>
        /// 中心点校准步骤，用于确定触摸板的精确中心。
        /// </summary>
        Center,
        /// <summary>
        /// 向上方向校准步骤。
        /// </summary>
        Up,
        /// <summary>
        /// 向下方向校准步骤。
        /// </summary>
        Down,
        /// <summary>
        /// 向左方向校准步骤。
        /// </summary>
        Left,
        /// <summary>
        /// 向右方向校准步骤。
        /// </summary>
        Right,
        /// <summary>
        /// 校准已完成。
        /// </summary>
        Completed
    }
}