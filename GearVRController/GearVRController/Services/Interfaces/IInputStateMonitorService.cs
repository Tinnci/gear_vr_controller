using System.Threading.Tasks;

namespace GearVRController.Services.Interfaces
{
    public interface IInputStateMonitorService
    {
        void StartMonitor();
        void ForceReleaseAllButtons();
        void StopMonitor();
        void NotifyInputActivity();
        // Additional methods or properties if needed by other services/viewmodels
    }
}