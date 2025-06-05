using GearVRController.Enums;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class ActionExecutionService : IActionExecutionService
    {
        private readonly IInputSimulator _inputSimulator;

        public ActionExecutionService(IInputSimulator inputSimulator)
        {
            _inputSimulator = inputSimulator;
        }

        public void ExecuteAction(GestureAction action)
        {
            switch (action)
            {
                case GestureAction.PageUp:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.PRIOR);
                    break;
                case GestureAction.PageDown:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.NEXT);
                    break;
                case GestureAction.BrowserBack:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_BACK);
                    break;
                case GestureAction.BrowserForward:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_FORWARD);
                    break;
                case GestureAction.VolumeUp:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_UP);
                    break;
                case GestureAction.VolumeDown:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_DOWN);
                    break;
                case GestureAction.MediaPlayPause:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_PLAY_PAUSE);
                    break;
                case GestureAction.MediaNext:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_NEXT_TRACK);
                    break;
                case GestureAction.MediaPrevious:
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_PREV_TRACK);
                    break;
                case GestureAction.Copy:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_C);
                    break;
                case GestureAction.Paste:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_V);
                    break;
                case GestureAction.Undo:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_Z);
                    break;
                case GestureAction.Redo:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_Y);
                    break;
                case GestureAction.SelectAll:
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_A);
                    break;
            }
        }
    }
}