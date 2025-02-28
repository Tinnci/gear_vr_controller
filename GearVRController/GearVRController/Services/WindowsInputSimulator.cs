using System;
using System.Runtime.InteropServices;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class WindowsInputSimulator : IInputSimulator
    {
        [DllImport("user32.dll")]
        private static extern void mouse_event(int dwFlags, int dx, int dy, int cButtons, int dwExtraInfo);

        [DllImport("user32.dll")]
        private static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);

        // 鼠标事件常量
        private const int MOUSEEVENTF_MOVE = 0x0001;
        private const int MOUSEEVENTF_LEFTDOWN = 0x0002;
        private const int MOUSEEVENTF_LEFTUP = 0x0004;
        private const int MOUSEEVENTF_RIGHTDOWN = 0x0008;
        private const int MOUSEEVENTF_RIGHTUP = 0x0010;
        private const int MOUSEEVENTF_MIDDLEDOWN = 0x0020;
        private const int MOUSEEVENTF_MIDDLEUP = 0x0040;
        private const int MOUSEEVENTF_WHEEL = 0x0800;
        private const int MOUSEEVENTF_ABSOLUTE = 0x8000;

        // 键盘事件常量
        private const int KEYEVENTF_EXTENDEDKEY = 0x0001;
        private const int KEYEVENTF_KEYUP = 0x0002;

        public void SimulateMouseMovement(double deltaX, double deltaY)
        {
            mouse_event(MOUSEEVENTF_MOVE, (int)deltaX, (int)deltaY, 0, 0);
        }

        public void SimulateMouseButton(bool isPressed)
        {
            if (isPressed)
            {
                mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            }
            else
            {
                mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
            }
        }

        // 扩展方法，用于支持不同类型的鼠标按钮
        public void SimulateMouseButtonEx(bool isPressed, int button = MOUSEEVENTF_LEFTDOWN)
        {
            if (isPressed)
            {
                mouse_event(button, 0, 0, 0, 0);
            }
            else
            {
                // 根据按下的按钮类型选择对应的释放事件
                int upEvent = button switch
                {
                    MOUSEEVENTF_LEFTDOWN => MOUSEEVENTF_LEFTUP,
                    MOUSEEVENTF_RIGHTDOWN => MOUSEEVENTF_RIGHTUP,
                    MOUSEEVENTF_MIDDLEDOWN => MOUSEEVENTF_MIDDLEUP,
                    _ => MOUSEEVENTF_LEFTUP
                };
                mouse_event(upEvent, 0, 0, 0, 0);
            }
        }

        public void SimulateKeyPress(int keyCode)
        {
            keybd_event((byte)keyCode, 0, 0, 0);
            keybd_event((byte)keyCode, 0, KEYEVENTF_KEYUP, 0);
        }

        public void SimulateKeyRelease(int keyCode)
        {
            keybd_event((byte)keyCode, 0, KEYEVENTF_KEYUP, 0);
        }

        public void SimulateWheelMovement(int delta)
        {
            mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta, 0);
        }

        // 公开常用的虚拟键码
        public static class VirtualKeys
        {
            public const byte VK_VOLUME_MUTE = 0xAD;
            public const byte VK_VOLUME_DOWN = 0xAE;
            public const byte VK_VOLUME_UP = 0xAF;
            public const byte VK_LEFT = 0x25;
            public const byte VK_UP = 0x26;
            public const byte VK_RIGHT = 0x27;
            public const byte VK_DOWN = 0x28;
            public const byte VK_BACK = 0xA6;
            public const byte VK_HOME = 0xAC;
        }

        // 公开鼠标按钮常量
        public static class MouseButtons
        {
            public const int Left = MOUSEEVENTF_LEFTDOWN;
            public const int Right = MOUSEEVENTF_RIGHTDOWN;
            public const int Middle = MOUSEEVENTF_MIDDLEDOWN;
        }
    }
}