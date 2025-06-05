using System;
using System.Collections.Generic;
using GearVRController.Enums;

namespace GearVRController.Events
{
    public class InputTimeoutDetectedEvent : EventArgs
    {
        public List<VirtualKeyCode> ReleasedKeys { get; }

        public InputTimeoutDetectedEvent(List<VirtualKeyCode> releasedKeys)
        {
            ReleasedKeys = releasedKeys;
        }
    }
}