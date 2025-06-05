using System.Threading.Tasks;
using GearVRController.Models;
using System;

namespace GearVRController.Services.Interfaces
{
    public interface IControllerService
    {
        event EventHandler<ControllerData>? ControllerDataProcessed;
        Task SendCommandAsync(byte[] command, int repeat = 1);
        void ProcessControllerData(ControllerData data);
    }
}