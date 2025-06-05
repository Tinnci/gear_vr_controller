namespace GearVRController.Services.Interfaces
{
    public interface IWindowManagerService
    {
        void OpenTouchpadCalibrationWindow();
        void CloseTouchpadCalibrationWindow();
        void OpenTouchpadVisualizerWindow();
        void CloseTouchpadVisualizerWindow();
    }
}