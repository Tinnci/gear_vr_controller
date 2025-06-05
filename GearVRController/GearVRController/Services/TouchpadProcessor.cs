using System;
using GearVRController.ViewModels;

namespace GearVRController.Services
{
    public class TouchpadProcessor
    {
        private TouchpadCalibrationData? _calibrationData;

        private const double DEFAULT_CENTER = 511.5; // (0 + 1023) / 2
        private const double DEFAULT_HALF_RANGE = 511.5; // 1023 / 2

        public void SetCalibrationData(TouchpadCalibrationData? calibrationData)
        {
            _calibrationData = calibrationData;
        }

        /// <summary>
        /// Processes raw touchpad data (0-1023 range) and returns normalized and calibrated coordinates (-1 to 1).
        /// </summary>
        /// <param name="rawX">Raw X coordinate from controller.</param>
        /// <param name="rawY">Raw Y coordinate from controller.</param>
        /// <returns>A tuple containing the processed X and Y coordinates.</returns>
        public (double processedX, double processedY) ProcessRawData(int rawX, int rawY)
        {
            System.Diagnostics.Debug.WriteLine($"[TouchpadProcessor] ProcessRawData: rawX={rawX}, rawY={rawY}");
            double processedX = 0;
            double processedY = 0;

            if (_calibrationData != null)
            {
                System.Diagnostics.Debug.WriteLine($"[TouchpadProcessor] Calibration Data: MinX={_calibrationData.MinX}, MaxX={_calibrationData.MaxX}, MinY={_calibrationData.MinY}, MaxY={_calibrationData.MaxY}, CenterX={_calibrationData.CenterX}, CenterY={_calibrationData.CenterY}");

                // 计算相对于中心点的偏移
                double deltaX = rawX - _calibrationData.CenterX;
                double deltaY = rawY - _calibrationData.CenterY;

                // 计算归一化系数
                // 确保分母不会过小，避免除以零或产生极大值
                double xScale = (_calibrationData.MaxX - _calibrationData.MinX) / 2.0; // 使用总范围的一半作为缩放基准
                double yScale = (_calibrationData.MaxY - _calibrationData.MinY) / 2.0; // 使用总范围的一半作为缩放基准

                // 避免除以零或过小的数值
                xScale = Math.Max(1.0, xScale);
                yScale = Math.Max(1.0, yScale);

                // 归一化坐标
                processedX = Math.Max(-1.0, Math.Min(1.0, deltaX / xScale));
                processedY = Math.Max(-1.0, Math.Min(1.0, -deltaY / yScale)); // Y轴翻转

                System.Diagnostics.Debug.WriteLine($"[TouchpadProcessor] Calibrated: deltaX={deltaX:F2}, deltaY={deltaY:F2}, xScale={xScale:F2}, yScale={yScale:F2}, processedX={processedX:F2}, processedY={processedY:F2}");
            }
            else
            {
                // If no calibration data, use simple normalization (assuming 0-1023 range)
                processedX = Math.Max(-1.0, Math.Min(1.0, (rawX - DEFAULT_CENTER) / DEFAULT_HALF_RANGE));
                processedY = Math.Max(-1.0, Math.Min(1.0, -(rawY - DEFAULT_CENTER) / DEFAULT_HALF_RANGE)); // Y轴翻转
                System.Diagnostics.Debug.WriteLine($"[TouchpadProcessor] Uncalibrated: processedX={processedX:F2}, processedY={processedY:F2}");
            }

            return (processedX, processedY);
        }
    }
}