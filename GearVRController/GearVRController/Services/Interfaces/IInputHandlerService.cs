using GearVRController.Models;

namespace GearVRController.Services.Interfaces
{
    public interface IInputHandlerService
    {
        void ProcessInput(ControllerData data);
    }
}