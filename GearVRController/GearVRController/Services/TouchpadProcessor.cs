using System;
using GearVRController.ViewModels;

namespace GearVRController.Services
{
    /// <summary>
    /// TouchpadProcessor 负责处理 Gear VR 控制器的原始触摸板数据（0-1023范围），
    /// 并将其转换为校准后的归一化坐标（-1到1范围）。
    /// 它支持使用校准数据来精确映射触摸板输入，如果未提供校准数据，则使用默认的归一化方式。
    /// </summary>
    public class TouchpadProcessor
    {
        /// <summary>
        /// 存储触摸板校准数据。如果为null，则使用默认的归一化方式。
        /// </summary>
        private TouchpadCalibrationData? _calibrationData;

        /// <summary>
        /// 未校准时触摸板X/Y轴的默认中心值 ((0 + 1023) / 2)。
        /// </summary>
        private const double DEFAULT_CENTER = 511.5; // (0 + 1023) / 2
        /// <summary>
        /// 未校准时触摸板X/Y轴的默认一半范围 (1023 / 2)。
        /// </summary>
        private const double DEFAULT_HALF_RANGE = 511.5; // 1023 / 2

        /// <summary>
        /// 设置或更新触摸板的校准数据。
        /// 如果提供非null的校准数据，后续的 `ProcessRawData` 调用将使用这些数据进行校准。
        /// 如果设置为null，则恢复到默认的归一化处理。
        /// </summary>
        /// <param name="calibrationData">要设置的 TouchpadCalibrationData 对象，或null以重置。</param>
        public void SetCalibrationData(TouchpadCalibrationData? calibrationData)
        {
            _calibrationData = calibrationData;
        }

        /// <summary>
        /// 处理原始触摸板数据（0-1023范围），并返回归一化和校准后的坐标（-1到1）。
        /// 如果存在校准数据，则使用校准数据进行精确映射；否则，使用默认的中心和范围进行归一化。
        /// 归一化后的X轴范围是 [-1, 1]（左到右），Y轴范围是 [-1, 1]（下到上，因UI坐标系通常Y轴向下，故在此处翻转）。
        /// </summary>
        /// <param name="rawX">来自控制器的原始X坐标。</param>
        /// <param name="rawY">来自控制器的原始Y坐标。</param>
        /// <returns>一个包含处理后的X和Y坐标的元组。</returns>
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
                // 使用总范围的一半作为缩放基准，确保分母不会过小，避免除以零或产生极大值
                double xScale = (_calibrationData.MaxX - _calibrationData.MinX) / 2.0;
                double yScale = (_calibrationData.MaxY - _calibrationData.MinY) / 2.0;

                // 避免除以零或过小的数值，确保至少为1.0
                xScale = Math.Max(1.0, xScale);
                yScale = Math.Max(1.0, yScale);

                // 归一化坐标，并限制在[-1, 1]范围，Y轴进行翻转以符合通常的笛卡尔坐标系（Y轴向上）
                processedX = Math.Max(-1.0, Math.Min(1.0, deltaX / xScale));
                processedY = Math.Max(-1.0, Math.Min(1.0, -deltaY / yScale)); // Y轴翻转

                System.Diagnostics.Debug.WriteLine($"[TouchpadProcessor] Calibrated: deltaX={deltaX:F2}, deltaY={deltaY:F2}, xScale={xScale:F2}, yScale={yScale:F2}, processedX={processedX:F2}, processedY={processedY:F2}");
            }
            else
            {
                // 如果没有校准数据，使用简单的归一化（假设0-1023范围），并Y轴翻转
                processedX = Math.Max(-1.0, Math.Min(1.0, (rawX - DEFAULT_CENTER) / DEFAULT_HALF_RANGE));
                processedY = Math.Max(-1.0, Math.Min(1.0, -(rawY - DEFAULT_CENTER) / DEFAULT_HALF_RANGE)); // Y轴翻转
                System.Diagnostics.Debug.WriteLine($"[TouchpadProcessor] Uncalibrated: processedX={processedX:F2}, processedY={processedY:F2}");
            }

            return (processedX, processedY);
        }
    }
}