using System;
using System.Collections.Generic;
using GearVRController.Enums;
using GearVRController.Models;
using Microsoft.UI.Dispatching;

namespace GearVRController.Services
{
    public class GestureRecognizer
    {
        private const int GESTURE_SAMPLE_COUNT = 5;
        private const float MIN_GESTURE_DISTANCE = 0.2f;

        private GestureConfig _gestureConfig;
        private readonly DispatcherQueue _dispatcherQueue;
        private readonly Queue<TouchpadPoint> _points = new();
        private TouchpadPoint? _gestureStartPoint;
        private bool _isGestureInProgress;

        public event EventHandler<GestureDirection>? GestureDetected;

        public GestureRecognizer(GestureConfig gestureConfig, DispatcherQueue dispatcherQueue)
        {
            _gestureConfig = gestureConfig;
            _dispatcherQueue = dispatcherQueue;
            _isGestureInProgress = false;
        }

        public void UpdateGestureConfig(GestureConfig gestureConfig)
        {
            _gestureConfig = gestureConfig;
        }

        public void ProcessTouchpadPoint(TouchpadPoint point)
        {
            // System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] ProcessTouchpadPoint: Received point ({{point.X:F2}}, {{point.Y:F2}}), IsTouched: {{point.IsTouched}}");
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

        private void StartGesture(TouchpadPoint point)
        {
            // System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] StartGesture: Starting gesture at ({{point.X:F2}}, {{point.Y:F2}})");
            _gestureStartPoint = point;
            _points.Clear();
            _points.Enqueue(point);
            _isGestureInProgress = true;
        }

        private void UpdateGesture(TouchpadPoint point)
        {
            // System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] UpdateGesture: Updating gesture with point ({{point.X:F2}}, {{point.Y:F2}})");
            _points.Enqueue(point);
            if (_points.Count > GESTURE_SAMPLE_COUNT)
            {
                _points.Dequeue();
            }
        }

        private void EndGesture()
        {
            System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] EndGesture: Ending gesture. Point count: {{_points.Count}}");
            if (_points.Count >= 2 && _gestureStartPoint != null)
            {
                var direction = CalculateGestureDirection();
                if (direction != GestureDirection.None)
                {
                    System.Diagnostics.Debug.WriteLine($"[GestureRecognizer] EndGesture: Gesture detected: {{direction}}");
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

        private GestureDirection CalculateGestureDirection()
        {
            System.Diagnostics.Debug.WriteLine("[GestureRecognizer] CalculateGestureDirection: Calculating direction.");
            if (_points.Count < 2 || _gestureStartPoint == null) return GestureDirection.None;

            var pointsArray = _points.ToArray();
            var startPoint = pointsArray[0];
            var endPoint = pointsArray[_points.Count - 1];

            float dx = endPoint.X - startPoint.X;
            float dy = endPoint.Y - startPoint.Y;

            float distance = (float)Math.Sqrt(dx * dx + dy * dy);
            float minGestureDistanceThreshold = MIN_GESTURE_DISTANCE * _gestureConfig.Sensitivity;

            System.Diagnostics.Debug.WriteLine("[GestureRecognizer] CalculateGestureDirection: " +
                                               "Start: (" + startPoint.X.ToString("F2") + ", " + startPoint.Y.ToString("F2") + "), " +
                                               "End: (" + endPoint.X.ToString("F2") + ", " + endPoint.Y.ToString("F2") + "), " +
                                               "Delta: (" + dx.ToString("F2") + ", " + dy.ToString("F2") + "), " +
                                               "Distance: " + distance.ToString("F2") + ", " +
                                               "MinThreshold: " + minGestureDistanceThreshold.ToString("F2"));

            if (distance < minGestureDistanceThreshold)
            {
                return GestureDirection.None;
            }

            double angle = Math.Atan2(dy, dx);
            double degrees = angle * 180 / Math.PI;

            if (degrees < 0)
                degrees += 360;

            System.Diagnostics.Debug.WriteLine("[GestureRecognizer] CalculateGestureDirection: Calculated degrees = " + degrees.ToString("F2"));

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