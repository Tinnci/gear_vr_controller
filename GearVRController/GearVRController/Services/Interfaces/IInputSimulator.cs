namespace GearVRController.Services.Interfaces
{
    public interface IInputSimulator
    {
        void SimulateMouseMovement(double deltaX, double deltaY);
        void SimulateMouseButton(bool isPressed);
        void SimulateKeyPress(int keyCode);
        void SimulateKeyRelease(int keyCode);
        void SimulateWheelMovement(int delta);
    }
}