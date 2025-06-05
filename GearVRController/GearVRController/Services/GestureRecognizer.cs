using System;
using System.Collections.Generic;
using GearVRController.Enums;
using GearVRController.Models;
using Microsoft.UI.Dispatching;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    /// <summary>
    /// GestureRecognizer 类负责识别来自触摸板的各种手势（例如滑动）。
    /// 它收集触摸板点数据，并在触摸序列结束后分析这些数据以检测预定义的手势。
    /// 识别到的手势会通过事件发布。
    /// </summary>
    public class GestureRecognizer
    {
        /// <summary>
        /// 用于手势识别的触摸点样本数量。
        /// </summary>
        private const int GESTURE_SAMPLE_COUNT = 5;
        /// <summary>
        /// 识别为有效手势所需的最小距离阈值。
        /// </summary>
        private const float MIN_GESTURE_DISTANCE = 0.2f;

        /// <summary>
        /// 当前的手势配置，包括灵敏度等参数。
        /// </summary>
        private GestureConfig _gestureConfig;
        /// <summary>
        /// DispatcherQueue 实例，用于在 UI 线程上触发手势检测事件。
        /// </summary>
        private readonly DispatcherQueue _dispatcherQueue;
        /// <summary>
        /// 存储触摸板历史轨迹点的队列，用于手势分析。
        /// </summary>
        private readonly Queue<TouchpadPoint> _points = new();
        /// <summary>
        /// 手势开始时的第一个触摸点。
        /// </summary>
        private TouchpadPoint? _gestureStartPoint;
        /// <summary>
        /// 指示当前是否有手势正在进行中。
        /// </summary>
        private bool _isGestureInProgress;

        /// <summary>
        /// 当检测到手势时触发的事件。
        /// </summary>
        public event EventHandler<GestureDirection>? GestureDetected;

        /// <summary>
        /// GestureRecognizer 类的构造函数。
        /// </summary>
        /// <param name="settingsService">设置服务，用于获取手势配置。</param>
        /// <param name="dispatcherQueue">DispatcherQueue，用于确保手势检测事件在 UI 线程上触发。</param>
        public GestureRecognizer(ISettingsService settingsService, DispatcherQueue dispatcherQueue)
        {
            _gestureConfig = settingsService.GestureConfig;
            _dispatcherQueue = dispatcherQueue;
            _isGestureInProgress = false;
        }

        /// <summary>
        /// 更新手势识别器的配置参数，例如手势灵敏度。
        /// </summary>
        /// <param name="gestureConfig">新的手势配置对象。</param>
        public void UpdateGestureConfig(GestureConfig gestureConfig)
        {
            _gestureConfig = gestureConfig;
        }

        /// <summary>
        /// 处理接收到的单个触摸板点。
        /// 根据触摸状态（按下或抬起），启动、更新或结束手势识别过程。
        /// </summary>
        /// <param name="point">当前的触摸板点，包含坐标和触摸状态。</param>
        public void ProcessTouchpadPoint(TouchpadPoint point)
        {
            // System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] ProcessTouchpadPoint: Received point ({point.X:F2}, {point.Y:F2}), IsTouched: {point.IsTouched}");
            if (!_isGestureInProgress && point.IsTouched)
            {
                StartGesture(point);
            }
            else if (_isGestureInProgress)
            {
                if (point.IsTouched)
                {
                    UpdateGesture(point);
                }
                else
                {
                    EndGesture();
                }
            }
        }

        /// <summary>
        /// 启动一个新的手势识别过程。
        /// 记录手势的起始点，并清空历史点队列。
        /// </summary>
        /// <param name="point">手势开始时的触摸板点。</param>
        private void StartGesture(TouchpadPoint point)
        {
            // System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] StartGesture: Starting gesture at ({point.X:F2}, {point.Y:F2})");
            _gestureStartPoint = point;
            _points.Clear();
            _points.Enqueue(point);
            _isGestureInProgress = true;
        }

        /// <summary>
        /// 在手势进行中更新手势数据。
        /// 将新的触摸点添加到队列中，并保持队列大小不超过预设的样本数量。
        /// </summary>
        /// <param name="point">当前接收到的触摸板点。</param>
        private void UpdateGesture(TouchpadPoint point)
        {
            // System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] UpdateGesture: Updating gesture with point ({point.X:F2}, {point.Y:F2})");
            _points.Enqueue(point);
            if (_points.Count > GESTURE_SAMPLE_COUNT)
            {
                _points.Dequeue();
            }
        }

        /// <summary>
        /// 结束当前的手势识别过程。
        /// 如果收集到的点足够，则尝试计算并识别手势方向，并通过事件发布结果。
        /// </summary>
        private void EndGesture()
        {
            System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] EndGesture: Ending gesture. Point count: {_points.Count}");
            if (_points.Count >= 2 && _gestureStartPoint != null)
            {
                var direction = CalculateGestureDirection();
                if (direction != GestureDirection.None)
                {
                    System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] EndGesture: Gesture detected: {direction}");
                    _dispatcherQueue.TryEnqueue(() => GestureDetected?.Invoke(this, direction));
                }
                else
                {
                    System.Diagnostics.Debug.WriteLine("[GestureRecognizer] EndGesture: No significant gesture detected.");
                }
            }

            _isGestureInProgress = false;
            _points.Clear();
        }

        /// <summary>
        /// 根据收集到的触摸点数据计算手势方向。
        /// 分析起始点和结束点之间的位移和角度，以确定手势属于哪个方向（上、下、左、右）。
        /// </summary>
        /// <returns>识别到的手势方向，如果没有识别到有效手势则返回 `GestureDirection.None`。</returns>
        private GestureDirection CalculateGestureDirection()
        {
            // System.Diagnostics.Debug.WriteLine("[GestureRecognizer] CalculateGestureDirection: Calculating direction.");
            if (_points.Count < 2 || _gestureStartPoint == null) return GestureDirection.None;

            var pointsArray = _points.ToArray();
            var startPoint = pointsArray[0];
            var endPoint = pointsArray[_points.Count - 1];

            float dx = endPoint.X - startPoint.X;
            float dy = endPoint.Y - startPoint.Y;

            float distance = (float)Math.Sqrt(dx * dx + dy * dy);
            float minGestureDistanceThreshold = MIN_GESTURE_DISTANCE * _gestureConfig.Sensitivity;

            // System.Diagnostics.Debug.WriteLine("[GestureRecognizer] CalculateGestureDirection: " +
            //                                    "Start: (" + startPoint.X.ToString("F2") + ", " + startPoint.Y.ToString("F2") + "), " +
            //                                    "End: (" + endPoint.X.ToString("F2") + ", " + endPoint.Y.ToString("F2") + "), " +
            //                                    "Delta: (" + dx.ToString("F2") + ", " + dy.ToString("F2") + "), " +
            //                                    "Distance: " + distance.ToString("F2") + ", " +
            //                                    "MinThreshold: " + minGestureDistanceThreshold.ToString("F2"));

            if (distance < minGestureDistanceThreshold)
            {
                return GestureDirection.None;
            }

            double angle = Math.Atan2(dy, dx);
            double degrees = angle * 180 / Math.PI;

            if (degrees < 0)
                degrees += 360;

            // System.Diagnostics.Debug.WriteLine("[GestureRecognizer] CalculateGestureDirection: Calculated degrees = " + degrees.ToString("F2"));

            const double tolerance = 60 / 2.0;

            if (degrees >= (360 - tolerance) || degrees < tolerance)
            {
                return GestureDirection.Right;
            }
            else if (degrees >= (90 - tolerance) && degrees < (90 + tolerance))
            {
                return GestureDirection.Down;
            }
            else if (degrees >= (180 - tolerance) && degrees < (180 + tolerance))
            {
                return GestureDirection.Left;
            }
            else if (degrees >= (270 - tolerance) && degrees < (270 + tolerance))
            {
                return GestureDirection.Up;
            }
            else
            {
                return GestureDirection.None;
            }
        }
    }
}