using System;
using GearVRController.ViewModels;

namespace GearVRController.Services
{
    public class TouchpadProcessor
    {
        private TouchpadCalibrationData? _calibrationData;

        public void SetCalibrationData(TouchpadCalibrationData? calibrationData)
        {
            _calibrationData = calibrationData;
        }

        /// <summary>
        /// Processes raw touchpad data (0-315 range) and returns normalized and calibrated coordinates (-1 to 1).
        /// </summary>
        /// <param name="rawX">Raw X coordinate from controller.</param>
        /// <param name="rawY">Raw Y coordinate from controller.</param>
        /// <returns>A tuple containing the processed X and Y coordinates.</returns>
        public (double processedX, double processedY) ProcessRawData(int rawX, int rawY)
        {
            double processedX = 0;
            double processedY = 0;

            if (_calibrationData != null)
            {
                // 计算相对于中心点的偏移
                double deltaX = rawX - _calibrationData.CenterX;
                double deltaY = rawY - _calibrationData.CenterY;

                // 计算归一化系数
                double xScale = deltaX > 0 ?
                    Math.Max(10, _calibrationData.MaxX - _calibrationData.CenterX) :
                    Math.Max(10, _calibrationData.CenterX - _calibrationData.MinX);

                double yScale = deltaY > 0 ?
                    Math.Max(10, _calibrationData.MaxY - _calibrationData.CenterY) :
                    Math.Max(10, _calibrationData.CenterY - _calibrationData.MinY);

                // 归一化坐标
                processedX = Math.Max(-1.0, Math.Min(1.0, deltaX / xScale));
                processedY = Math.Max(-1.0, Math.Min(1.0, -deltaY / yScale)); // Y轴翻转
            }
            else
            {
                // 如果没有校准数据，使用简单的归一化方法 ( assuming center is 157.5 and max radius is 157.5)
                processedX = Math.Max(-1.0, Math.Min(1.0, (rawX - 157.5) / 157.5));
                processedY = Math.Max(-1.0, Math.Min(1.0, -(rawY - 157.5) / 157.5)); // Y轴翻转
            }

            return (processedX, processedY);
        }
    }
}