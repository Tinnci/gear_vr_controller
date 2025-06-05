using GearVRController.Enums;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    /// <summary>
    /// ActionExecutionService 负责根据指定的手势动作（GestureAction）执行相应的系统级操作。
    /// 它利用 `IInputSimulator` 接口来模拟鼠标和键盘输入，从而实现各种预定义的功能，
    /// 例如翻页、浏览器导航、音量控制以及常见的编辑操作（剪切、复制、粘贴等）。
    /// </summary>
    public class ActionExecutionService : IActionExecutionService
    {
        /// <summary>
        /// 输入模拟器实例，用于执行模拟的鼠标和键盘操作。
        /// </summary>
        private readonly IInputSimulator _inputSimulator;

        /// <summary>
        /// ActionExecutionService 的构造函数。
        /// </summary>
        /// <param name="inputSimulator">输入模拟器服务，用于模拟按键和鼠标操作。</param>
        public ActionExecutionService(IInputSimulator inputSimulator)
        {
            _inputSimulator = inputSimulator;
        }

        /// <summary>
        /// 根据给定的手势动作枚举值执行相应的系统操作。
        /// 此方法通过调用 `IInputSimulator` 来模拟键盘按键或组合键。
        /// </summary>
        /// <param name="action">要执行的手势动作。</param>
        public void ExecuteAction(GestureAction action)
        {
            switch (action)
            {
                case GestureAction.PageUp:
                    // 模拟 "Page Up" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.PRIOR);
                    break;
                case GestureAction.PageDown:
                    // 模拟 "Page Down" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.NEXT);
                    break;
                case GestureAction.BrowserBack:
                    // 模拟浏览器 "后退" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_BACK);
                    break;
                case GestureAction.BrowserForward:
                    // 模拟浏览器 "前进" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.BROWSER_FORWARD);
                    break;
                case GestureAction.VolumeUp:
                    // 模拟 "音量加" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_UP);
                    break;
                case GestureAction.VolumeDown:
                    // 模拟 "音量减" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.VOLUME_DOWN);
                    break;
                case GestureAction.MediaPlayPause:
                    // 模拟 "媒体播放/暂停" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_PLAY_PAUSE);
                    break;
                case GestureAction.MediaNext:
                    // 模拟 "下一曲" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_NEXT_TRACK);
                    break;
                case GestureAction.MediaPrevious:
                    // 模拟 "上一曲" 键按下
                    _inputSimulator.SimulateKeyPress((int)VirtualKeyCode.MEDIA_PREV_TRACK);
                    break;
                case GestureAction.Copy:
                    // 模拟 "Ctrl + C" 组合键
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_C);
                    break;
                case GestureAction.Paste:
                    // 模拟 "Ctrl + V" 组合键
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_V);
                    break;
                case GestureAction.Undo:
                    // 模拟 "Ctrl + Z" 组合键
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_Z);
                    break;
                case GestureAction.Redo:
                    // 模拟 "Ctrl + Y" 组合键
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_Y);
                    break;
                case GestureAction.SelectAll:
                    // 模拟 "Ctrl + A" 组合键
                    _inputSimulator.SimulateModifiedKeyStroke(VirtualKeyCode.CONTROL, VirtualKeyCode.VK_A);
                    break;
            }
        }
    }
}