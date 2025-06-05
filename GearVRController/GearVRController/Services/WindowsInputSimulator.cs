using System;
using System.Runtime.InteropServices;
using GearVRController.Services.Interfaces;
using GearVRController.Enums;

namespace GearVRController.Services
{
    public class WindowsInputSimulator : IInputSimulator
    {
        #region Win32 API Structures
        [StructLayout(LayoutKind.Sequential)]
        private struct MOUSEINPUT
        {
            public int dx;
            public int dy;
            public uint mouseData;
            public uint dwFlags;
            public uint time;
            public IntPtr dwExtraInfo;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct KEYBDINPUT
        {
            public ushort wVk;
            public ushort wScan;
            public uint dwFlags;
            public uint time;
            public IntPtr dwExtraInfo;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct HARDWAREINPUT
        {
            public uint uMsg;
            public ushort wParamL;
            public ushort wParamH;
        }

        [StructLayout(LayoutKind.Explicit)]
        private struct InputUnion
        {
            [FieldOffset(0)] public MOUSEINPUT mi;
            [FieldOffset(0)] public KEYBDINPUT ki;
            [FieldOffset(0)] public HARDWAREINPUT hi;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct INPUT
        {
            public uint type;
            public InputUnion u;
        }
        #endregion

        #region Win32 API Constants
        private const int INPUT_MOUSE = 0;
        private const int INPUT_KEYBOARD = 1;
        private const uint MOUSEEVENTF_MOVE = 0x0001;
        private const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
        private const uint MOUSEEVENTF_LEFTUP = 0x0004;
        private const uint MOUSEEVENTF_RIGHTDOWN = 0x0008;
        private const uint MOUSEEVENTF_RIGHTUP = 0x0010;
        private const uint MOUSEEVENTF_MIDDLEDOWN = 0x0020;
        private const uint MOUSEEVENTF_MIDDLEUP = 0x0040;
        private const uint MOUSEEVENTF_XBUTTONDOWN = 0x0080; // For XButton1/XButton2
        private const uint MOUSEEVENTF_XBUTTONUP = 0x0100;   // For XButton1/XButton2
        private const uint MOUSEEVENTF_WHEEL = 0x0800;
        private const uint MOUSEEVENTF_ABSOLUTE = 0x8000;

        private const uint KEYEVENTF_EXTENDEDKEY = 0x0001;
        private const uint KEYEVENTF_KEYUP = 0x0002;
        private const uint KEYEVENTF_UNICODE = 0x0004;
        private const uint KEYEVENTF_SCANCODE = 0x0008;
        #endregion

        #region Win32 API Methods
        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);

        [DllImport("user32.dll")]
        private static extern int GetSystemMetrics(int nIndex);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool GetCursorPos(out POINT lpPoint);

        [StructLayout(LayoutKind.Sequential)]
        private struct POINT
        {
            public int X;
            public int Y;
        }
        #endregion

        public void SimulateMouseMovement(double deltaX, double deltaY)
        {
            try
            {
                var input = new INPUT
                {
                    type = INPUT_MOUSE,
                    u = new InputUnion
                    {
                        mi = new MOUSEINPUT
                        {
                            dx = (int)deltaX,
                            dy = (int)deltaY,
                            dwFlags = MOUSEEVENTF_MOVE,
                            time = 0,
                            mouseData = 0,
                            dwExtraInfo = IntPtr.Zero
                        }
                    }
                };

                if (SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT))) == 0)
                {
                    int error = Marshal.GetLastWin32Error();
                    System.Diagnostics.Debug.WriteLine($"鼠标移动模拟失败: {error}");
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"鼠标移动模拟异常: {ex}");
            }
        }

        public void SimulateMouseButton(bool isPressed)
        {
            try
            {
                var input = new INPUT
                {
                    type = INPUT_MOUSE,
                    u = new InputUnion
                    {
                        mi = new MOUSEINPUT
                        {
                            dwFlags = isPressed ? MOUSEEVENTF_LEFTDOWN : MOUSEEVENTF_LEFTUP,
                            time = 0,
                            mouseData = 0,
                            dwExtraInfo = IntPtr.Zero
                        }
                    }
                };

                if (SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT))) == 0)
                {
                    int error = Marshal.GetLastWin32Error();
                    System.Diagnostics.Debug.WriteLine($"鼠标按键模拟失败: {error}");
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"鼠标按键模拟异常: {ex}");
            }
        }

        public void SimulateMouseButtonEx(bool isPressed, int button)
        {
            try
            {
                uint flags;
                uint mouseData = 0; // For XBUTTON1/XBUTTON2

                switch ((MouseButtons)button) // 强制转换为新的 MouseButtons 枚举
                {
                    case MouseButtons.Left:
                        flags = isPressed ? MOUSEEVENTF_LEFTDOWN : MOUSEEVENTF_LEFTUP;
                        break;
                    case MouseButtons.Right:
                        flags = isPressed ? MOUSEEVENTF_RIGHTDOWN : MOUSEEVENTF_RIGHTUP;
                        break;
                    case MouseButtons.Middle:
                        flags = isPressed ? MOUSEEVENTF_MIDDLEDOWN : MOUSEEVENTF_MIDDLEUP;
                        break;
                    case MouseButtons.XButton1:
                        flags = isPressed ? MOUSEEVENTF_XBUTTONDOWN : MOUSEEVENTF_XBUTTONUP;
                        mouseData = 0x0001; // XBUTTON1
                        break;
                    case MouseButtons.XButton2:
                        flags = isPressed ? MOUSEEVENTF_XBUTTONDOWN : MOUSEEVENTF_XBUTTONUP;
                        mouseData = 0x0002; // XBUTTON2
                        break;
                    default:
                        flags = isPressed ? MOUSEEVENTF_LEFTDOWN : MOUSEEVENTF_LEFTUP;
                        break;
                }

                var input = new INPUT
                {
                    type = INPUT_MOUSE,
                    u = new InputUnion
                    {
                        mi = new MOUSEINPUT
                        {
                            dwFlags = flags,
                            time = 0,
                            mouseData = mouseData, // 使用 mouseData 来传递 XBUTTON 值
                            dwExtraInfo = IntPtr.Zero
                        }
                    }
                };

                if (SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT))) == 0)
                {
                    int error = Marshal.GetLastWin32Error();
                    System.Diagnostics.Debug.WriteLine($"扩展鼠标按键模拟失败: {error}");
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"扩展鼠标按键模拟异常: {ex}");
            }
        }

        public void SimulateWheelMovement(int delta)
        {
            try
            {
                var input = new INPUT
                {
                    type = INPUT_MOUSE,
                    u = new InputUnion
                    {
                        mi = new MOUSEINPUT
                        {
                            mouseData = (uint)delta,
                            dwFlags = MOUSEEVENTF_WHEEL,
                            time = 0,
                            dwExtraInfo = IntPtr.Zero
                        }
                    }
                };

                if (SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT))) == 0)
                {
                    int error = Marshal.GetLastWin32Error();
                    System.Diagnostics.Debug.WriteLine($"滚轮移动模拟失败: {error}");
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"滚轮移动模拟异常: {ex}");
            }
        }

        public void SimulateKeyPress(int keyCode)
        {
            try
            {
                // 按键按下
                var inputDown = new INPUT
                {
                    type = INPUT_KEYBOARD,
                    u = new InputUnion
                    {
                        ki = new KEYBDINPUT
                        {
                            wVk = (ushort)keyCode,
                            wScan = 0,
                            dwFlags = 0,
                            time = 0,
                            dwExtraInfo = IntPtr.Zero
                        }
                    }
                };

                SendInput(1, new INPUT[] { inputDown }, Marshal.SizeOf(typeof(INPUT)));
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"按键按下模拟异常: {ex}");
            }
        }

        public void SimulateKeyRelease(int keyCode)
        {
            try
            {
                // 按键抬起
                var inputUp = new INPUT
                {
                    type = INPUT_KEYBOARD,
                    u = new InputUnion
                    {
                        ki = new KEYBDINPUT
                        {
                            wVk = (ushort)keyCode,
                            wScan = 0,
                            dwFlags = KEYEVENTF_KEYUP,
                            time = 0,
                            dwExtraInfo = IntPtr.Zero
                        }
                    }
                };

                SendInput(1, new INPUT[] { inputUp }, Marshal.SizeOf(typeof(INPUT)));
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"按键抬起模拟异常: {ex}");
            }
        }

        public void SimulateModifiedKeyStroke(VirtualKeyCode modifier, VirtualKeyCode key)
        {
            try
            {
                // 按下修饰键
                SimulateKeyPress((int)modifier);

                // 按下主键
                SimulateKeyPress((int)key);

                // 抬起主键
                SimulateKeyRelease((int)key);

                // 抬起修饰键
                SimulateKeyRelease((int)modifier);
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"组合键模拟异常: {ex}");
            }
        }
    }
}