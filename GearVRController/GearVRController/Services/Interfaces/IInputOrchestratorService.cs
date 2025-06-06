using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using GearVRController.Models;

namespace GearVRController.Services.Interfaces
{
    public interface IInputOrchestratorService
    {
        void ProcessControllerData(ControllerData data, bool isCalibrating, bool isControlEnabled);
    }
}