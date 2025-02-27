using System;

namespace GearVRController.Models
{
    public class ControllerData
    {
        // 轴数据
        public int AxisX { get; set; }
        public int AxisY { get; set; }

        // 加速度数据
        public int AccelX { get; set; }
        public int AccelY { get; set; }
        public int AccelZ { get; set; }

        // 陀螺仪数据
        public int GyroX { get; set; }
        public int GyroY { get; set; }
        public int GyroZ { get; set; }

        // 磁力计数据
        public int MagnetX { get; set; }
        public int MagnetY { get; set; }
        public int MagnetZ { get; set; }

        // 按钮状态
        public bool TriggerButton { get; set; }
        public bool HomeButton { get; set; }
        public bool BackButton { get; set; }
        public bool TouchpadButton { get; set; }
        public bool VolumeUpButton { get; set; }
        public bool VolumeDownButton { get; set; }
        public bool NoButton { get; set; }

        // 时间戳
        public DateTime Timestamp { get; set; } = DateTime.Now;
    }
} 