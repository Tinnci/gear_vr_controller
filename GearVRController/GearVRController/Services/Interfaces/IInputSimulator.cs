using System;
using System.Runtime.InteropServices;
using GearVRController.Enums;

namespace GearVRController.Services.Interfaces
{
    public interface IInputSimulator
    {
        void SimulateMouseMovement(double deltaX, double deltaY);
        void SimulateMouseButton(bool isPressed);
        void SimulateKeyPress(int keyCode);
        void SimulateKeyRelease(int keyCode);
        void SimulateWheelMovement(int delta);
        void SimulateMouseButtonEx(bool isPressed, int button);
        void SimulateModifiedKeyStroke(VirtualKeyCode modifier, VirtualKeyCode key);
    }
}