using GearVRController.Enums;
using GearVRController.Services.Interfaces;
using WindowsInput.Native;

namespace GearVRController.Services
{
    public class MonitoredInputSimulator : IInputSimulator
    {
        private readonly IInputSimulator _realSimulator;
        private readonly IInputStateMonitorService _monitor;

        public MonitoredInputSimulator(WindowsInputSimulator realSimulator, IInputStateMonitorService monitor)
        {
            _realSimulator = realSimulator;
            _monitor = monitor;
        }

        public void SimulateKeyDown(int keyCode)
        {
            _realSimulator.SimulateKeyDown(keyCode);
            _monitor.AddPressedKey((VirtualKeyCode)keyCode);
        }

        public void SimulateKeyUp(int keyCode)
        {
            _realSimulator.SimulateKeyUp(keyCode);
            _monitor.RemovePressedKey((VirtualKeyCode)keyCode);
        }

        public void SimulateKeyRelease(int keyCode)
        {
            _realSimulator.SimulateKeyRelease(keyCode);
            _monitor.RemovePressedKey((VirtualKeyCode)keyCode);
        }

        public void SimulateModifiedKeyStroke(VirtualKeyCode modifier, VirtualKeyCode key)
        {
            _realSimulator.SimulateModifiedKeyStroke(modifier, key);
            // For simplicity, we are not tracking modified key strokes in the monitor separately
            // as individual key down/up are handled, and this is a combination.
        }

        public void SimulateKeyPress(int keyCode)
        {
            _realSimulator.SimulateKeyPress(keyCode);
            // KeyPress already includes down and up, so monitor will handle internally if needed
        }

        public void SimulateMouseMovement(double deltaX, double deltaY)
        {
            _realSimulator.SimulateMouseMovement(deltaX, deltaY);
        }

        public void SimulateMouseButton(bool isPressed)
        {
            _realSimulator.SimulateMouseButton(isPressed);
        }

        public void SimulateMouseButtonEx(bool isPressed, int buttonCode)
        {
            _realSimulator.SimulateMouseButtonEx(isPressed, buttonCode);
            // For extended mouse buttons, you might need to map them to VirtualKeyCode if tracking is desired.
            // For simplicity, we are not adding tracking for extended mouse buttons here.
        }

        public void SimulateWheelMovement(int delta)
        {
            _realSimulator.SimulateWheelMovement(delta);
        }
    }
}