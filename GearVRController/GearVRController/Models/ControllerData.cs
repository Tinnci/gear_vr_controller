using System;

namespace GearVRController.Models
{
    public class ControllerData
    {
        // 轴数据
        public int AxisX { get; set; }
        public int AxisY { get; set; }

        // 加速度数据 (Currently not used for core logic, only for display/debug)
        // public float AccelX { get; set; }
        // public float AccelY { get; set; }
        // public float AccelZ { get; set; }

        // 陀螺仪数据 (Currently not used for core logic, only for display/debug)
        // public float GyroX { get; set; }
        // public float GyroY { get; set; }
        // public float GyroZ { get; set; }

        // 磁力计数据 (Currently not used for core logic, only for display/debug)
        // public int MagnetX { get; set; }
        // public int MagnetY { get; set; }
        // public int MagnetZ { get; set; }

        // 按钮状态
        public bool TriggerButton { get; set; }
        public bool TriggerPressed { get; set; }
        public bool HomeButton { get; set; }
        public bool BackButton { get; set; }
        public bool TouchpadButton { get; set; }
        public bool TouchpadTouched { get; set; }
        public bool VolumeUpButton { get; set; }
        public bool VolumeDownButton { get; set; }
        public bool NoButton { get; set; }

        // 原始触摸板坐标
        public ushort TouchpadX { get; set; }
        public ushort TouchpadY { get; set; }

        // 处理后的触摸板坐标 (归一化到 [-1, 1])
        public double ProcessedTouchpadX { get; set; }
        public double ProcessedTouchpadY { get; set; }

        // 时间戳 (Unix 毫秒)
        public long Timestamp { get; set; }
    }
}