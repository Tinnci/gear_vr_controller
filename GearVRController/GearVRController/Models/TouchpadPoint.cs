using System;

namespace GearVRController.Models
{
    /// <summary>
    /// 触摸板上的点位置记录
    /// </summary>
    public class TouchpadPoint
    {
        /// <summary>
        /// X坐标，范围[-1, 1]
        /// </summary>
        public double X { get; set; }
        
        /// <summary>
        /// Y坐标，范围[-1, 1]
        /// </summary>
        public double Y { get; set; }
        
        /// <summary>
        /// 是否按下
        /// </summary>
        public bool IsPressed { get; set; }
        
        /// <summary>
        /// 时间戳
        /// </summary>
        public DateTime Timestamp { get; set; }

        public TouchpadPoint(double x, double y, bool isPressed)
        {
            X = x;
            Y = y;
            IsPressed = isPressed;
            Timestamp = DateTime.Now;
        }
    }

    /// <summary>
    /// 触摸板手势类型
    /// </summary>
    public enum TouchpadGesture
    {
        /// <summary>
        /// 无手势
        /// </summary>
        None,
        
        /// <summary>
        /// 向上滑动
        /// </summary>
        SwipeUp,
        
        /// <summary>
        /// 向下滑动
        /// </summary>
        SwipeDown,
        
        /// <summary>
        /// 向左滑动
        /// </summary>
        SwipeLeft,
        
        /// <summary>
        /// 向右滑动
        /// </summary>
        SwipeRight,
        
        /// <summary>
        /// 点击
        /// </summary>
        Tap,
        
        /// <summary>
        /// 长按
        /// </summary>
        LongPress
    }
} 