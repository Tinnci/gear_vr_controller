using System;
using System.Collections.Generic;
using GearVRController.Enums;
using GearVRController.Models;

namespace GearVRController.Services
{
    public class GestureRecognizer
    {
        private const int GESTURE_SAMPLE_COUNT = 5;
        private const int MIN_GESTURE_DISTANCE = 10;

        private float _sensitivity;
        private readonly Queue<TouchpadPoint> _points = new();
        private TouchpadPoint? _gestureStartPoint;
        private bool _isGestureInProgress;

        public event EventHandler<GestureDirection>? GestureDetected;

        public GestureRecognizer(float sensitivity = 0.3f)
        {
            _sensitivity = Math.Max(0.1f, Math.Min(sensitivity, 1.0f));
            _isGestureInProgress = false;
        }

        public void SetSensitivity(float sensitivity)
        {
            _sensitivity = Math.Clamp(sensitivity, 0.1f, 1.0f);
        }

        public void ProcessTouchpadPoint(TouchpadPoint point)
        {
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
            _gestureStartPoint = point;
            _points.Clear();
            _points.Enqueue(point);
            _isGestureInProgress = true;
        }

        private void UpdateGesture(TouchpadPoint point)
        {
            _points.Enqueue(point);
            if (_points.Count > GESTURE_SAMPLE_COUNT)
            {
                _points.Dequeue();
            }
        }

        private void EndGesture()
        {
            if (_points.Count >= 2 && _gestureStartPoint != null)
            {
                var direction = CalculateGestureDirection();
                if (direction != GestureDirection.None)
                {
                    GestureDetected?.Invoke(this, direction);
                }
            }

            _isGestureInProgress = false;
            _points.Clear();
        }

        private GestureDirection CalculateGestureDirection()
        {
            if (_points.Count < 2 || _gestureStartPoint == null) return GestureDirection.None;

            var lastPoint = _points.ToArray()[_points.Count - 1];
            float dx = lastPoint.X - _gestureStartPoint.X;
            float dy = lastPoint.Y - _gestureStartPoint.Y;

            // 计算手势距离
            float distance = (float)Math.Sqrt(dx * dx + dy * dy);
            if (distance < MIN_GESTURE_DISTANCE * _sensitivity)
            {
                return GestureDirection.None;
            }

            // 判断手势方向
            if (Math.Abs(dx) > Math.Abs(dy))
            {
                return dx > 0 ? GestureDirection.Right : GestureDirection.Left;
            }
            else
            {
                return dy > 0 ? GestureDirection.Down : GestureDirection.Up;
            }
        }
    }
}