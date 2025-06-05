using System;
using GearVRController.Models;

namespace GearVRController.Events
{
    public class ControllerDataReceivedEvent : EventArgs
    {
        public ControllerData Data { get; }
        public ControllerDataReceivedEvent(ControllerData data)
        {
            Data = data;
        }
    }
}