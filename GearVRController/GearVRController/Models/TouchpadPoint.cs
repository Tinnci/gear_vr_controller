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
        public float X { get; set; }

        /// <summary>
        /// Y坐标，范围[-1, 1]
        /// </summary>
        public float Y { get; set; }

        /// <summary>
        /// 是否按下
        /// </summary>
        public bool IsTouched { get; set; }

        /// <summary>
        /// 时间戳
        /// </summary>
        public DateTime Timestamp { get; set; }

        public TouchpadPoint()
        {
            Timestamp = DateTime.Now;
        }

        public TouchpadPoint(float x, float y, bool isTouched)
        {
            X = x;
            Y = y;
            IsTouched = isTouched;
            Timestamp = DateTime.Now;
        }
    }
}