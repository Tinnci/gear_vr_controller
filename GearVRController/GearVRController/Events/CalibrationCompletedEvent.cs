using GearVRController.Models;
using GearVRController.ViewModels;

namespace GearVRController.Events
{
    public class CalibrationCompletedEvent
    {
        public TouchpadCalibrationData CalibrationData { get; }

        public CalibrationCompletedEvent(TouchpadCalibrationData data)
        {
            CalibrationData = data;
        }
    }
}