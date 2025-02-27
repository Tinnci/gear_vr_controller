using System;
using System.Runtime.InteropServices;

namespace GearVRController.Helpers
{
    public static class InputSimulator
    {
        [DllImport("user32.dll")]
        private static extern void mouse_event(int dwFlags, int dx, int dy, int cButtons, int dwExtraInfo);

        [DllImport("user32.dll")]
        private static extern int SetCursorPos(int x, int y);

        [DllImport("user32.dll")]
        private static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);

        // 鼠标事件常量
        private const int MOUSEEVENTF_MOVE = 0x0001;
        public const int MOUSEEVENTF_LEFTDOWN = 0x0002;
        public const int MOUSEEVENTF_LEFTUP = 0x0004;
        public const int MOUSEEVENTF_RIGHTDOWN = 0x0008;
        public const int MOUSEEVENTF_RIGHTUP = 0x0010;
        public const int MOUSEEVENTF_MIDDLEDOWN = 0x0020;
        public const int MOUSEEVENTF_MIDDLEUP = 0x0040;
        private const int MOUSEEVENTF_WHEEL = 0x0800;
        private const int MOUSEEVENTF_ABSOLUTE = 0x8000;

        // 键盘事件常量
        private const int KEYEVENTF_EXTENDEDKEY = 0x0001;
        private const int KEYEVENTF_KEYUP = 0x0002;

        // 虚拟键码
        public const byte VK_VOLUME_MUTE = 0xAD;
        public const byte VK_VOLUME_DOWN = 0xAE;
        public const byte VK_VOLUME_UP = 0xAF;
        public const byte VK_LEFT = 0x25;
        public const byte VK_UP = 0x26;
        public const byte VK_RIGHT = 0x27;
        public const byte VK_DOWN = 0x28;
        public const byte VK_BACK = 0xA6;
        public const byte VK_HOME = 0xAC;

        public static void MoveMouse(int dx, int dy)
        {
            mouse_event(MOUSEEVENTF_MOVE, dx, dy, 0, 0);
        }

        public static void SetMousePosition(int x, int y)
        {
            SetCursorPos(x, y);
        }

        public static void MouseDown()
        {
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        }

        public static void MouseUp()
        {
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        }

        public static void MouseEvent(int mouseEvent)
        {
            mouse_event(mouseEvent, 0, 0, 0, 0);
        }

        public static void SendKey(byte keyCode, bool isExtended = false)
        {
            uint flags = isExtended ? (uint)KEYEVENTF_EXTENDEDKEY : 0;
            keybd_event(keyCode, 0, flags, 0);
            keybd_event(keyCode, 0, flags | KEYEVENTF_KEYUP, 0);
        }

        public static void KeyDown(byte keyCode, bool isExtended = false)
        {
            uint flags = isExtended ? (uint)KEYEVENTF_EXTENDEDKEY : 0;
            keybd_event(keyCode, 0, flags, 0);
        }

        public static void KeyUp(byte keyCode, bool isExtended = false)
        {
            uint flags = isExtended ? (uint)KEYEVENTF_EXTENDEDKEY : 0;
            keybd_event(keyCode, 0, flags | KEYEVENTF_KEYUP, 0);
        }
    }
} 