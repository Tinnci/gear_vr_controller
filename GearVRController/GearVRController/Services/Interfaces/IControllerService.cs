using System.Threading.Tasks;
using GearVRController.Models;

namespace GearVRController.Services.Interfaces
{
    public interface IControllerService
    {
        Task InitializeAsync();
        Task SendCommandAsync(byte[] command, int repeat = 1);
        void ProcessControllerData(ControllerData data);
    }
}