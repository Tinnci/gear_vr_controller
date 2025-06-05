using System.Threading.Tasks;

namespace GearVRController.Services.Interfaces
{
    public interface IInputStateMonitorService
    {
        void Initialize();
        void ForceReleaseAllButtons();
        void StopMonitor();
        void UpdateInputState(bool triggerButton, bool touchpadButton, bool isControlEnabled, bool isCalibrating);
        // Additional methods or properties if needed by other services/viewmodels
    }
}