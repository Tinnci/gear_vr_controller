using GearVRController.ViewModels;
using System;

namespace GearVRController.Events
{
    public class CalibrationCompletedEvent : EventArgs
    {
        public TouchpadCalibrationData? CalibrationData { get; }
        public bool IsSuccess { get; }

        public CalibrationCompletedEvent(TouchpadCalibrationData? data, bool isSuccess)
        {
            CalibrationData = data;
            IsSuccess = isSuccess;
        }
    }
}