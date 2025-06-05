using System;
using System.Runtime.InteropServices;
using GearVRController.Services.Interfaces;
using GearVRController.Enums;

namespace GearVRController.Services
{
    /// <summary>
    /// WindowsInputSimulator 实现了 IInputSimulator 接口，用于模拟 Windows 操作系统中的鼠标和键盘输入。
    /// 它通过调用 Win32 API 中的 SendInput 函数来模拟各种输入事件，例如鼠标移动、按键按下/释放和滚轮滚动。
    /// </summary>
    public class WindowsInputSimulator : IInputSimulator
    {
        #region Win32 API Structures
        /// <summary>
        /// 定义鼠标事件的数据结构，用于 SendInput 函数。
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        private struct MOUSEINPUT
        {
            /// <summary>
            /// 鼠标的相对或绝对X坐标。 (鼠标事件的水平位移量)
            /// </summary>
            public int dx;
            /// <summary>
            /// 鼠标的相对或绝对Y坐标。 (鼠标事件的垂直位移量)
            /// </summary>
            public int dy;
            /// <summary>
            /// 鼠标按钮的状态或滚轮的滚动量。 (鼠标事件相关数据)
            /// </summary>
            public uint mouseData;
            /// <summary>
            /// 鼠标事件的标志位，指示事件类型（例如，移动、左键按下、滚轮滚动）。
            /// </summary>
            public uint dwFlags;
            /// <summary>
            /// 事件的时间戳。 (时间戳)
            /// </summary>
            public uint time;
            /// <summary>
            /// 额外的与输入事件相关联的值。 (额外信息)
            /// </summary>
            public IntPtr dwExtraInfo;
        }

        /// <summary>
        /// 定义键盘事件的数据结构，用于 SendInput 函数。
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        private struct KEYBDINPUT
        {
            /// <summary>
            /// 虚拟键码（VK_CODE）。
            /// </summary>
            public ushort wVk;
            /// <summary>
            /// 硬件扫描码。
            /// </summary>
            public ushort wScan;
            /// <summary>
            /// 键盘事件的标志位。
            /// </summary>
            public uint dwFlags;
            /// <summary>
            /// 事件的时间戳。
            /// </summary>
            public uint time;
            /// <summary>
            /// 额外的与输入事件相关联的值。
            /// </summary>
            public IntPtr dwExtraInfo;
        }

        /// <summary>
        /// 定义硬件事件的数据结构，用于 SendInput 函数。
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        private struct HARDWAREINPUT
        {
            /// <summary>
            /// 硬件消息。
            /// </summary>
            public uint uMsg;
            /// <summary>
            /// 硬件消息的低位参数。
            /// </summary>
            public ushort wParamL;
            /// <summary>
            /// 硬件消息的高位参数。
            /// </summary>
            public ushort wParamH;
        }

        /// <summary>
        /// 联合体，包含 MOUSEINPUT, KEYBDINPUT, HARDWAREINPUT 结构，用于 SendInput 函数。
        /// 允许 SendInput 接受不同类型的输入事件。
        /// </summary>
        [StructLayout(LayoutKind.Explicit)]
        private struct InputUnion
        {
            [FieldOffset(0)] public MOUSEINPUT mi;
            [FieldOffset(0)] public KEYBDINPUT ki;
            [FieldOffset(0)] public HARDWAREINPUT hi;
        }

        /// <summary>
        /// 定义 SendInput 函数使用的通用输入事件结构。
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        private struct INPUT
        {
            /// <summary>
            /// 输入事件的类型（例如，鼠标、键盘、硬件）。
            /// </summary>
            public uint type;
            /// <summary>
            /// 包含特定输入类型数据的联合体。
            /// </summary>
            public InputUnion u;
        }
        #endregion

        #region Win32 API Constants
        /// <summary>
        /// 输入事件类型：鼠标。
        /// </summary>
        private const int INPUT_MOUSE = 0;
        /// <summary>
        /// 输入事件类型：键盘。
        /// </summary>
        private const int INPUT_KEYBOARD = 1;
        /// <summary>
        /// 鼠标事件标志：鼠标移动。
        /// </summary>
        private const uint MOUSEEVENTF_MOVE = 0x0001;
        /// <summary>
        /// 鼠标事件标志：鼠标左键按下。
        /// </summary>
        private const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
        /// <summary>
        /// 鼠标事件标志：鼠标左键抬起。
        /// </summary>
        private const uint MOUSEEVENTF_LEFTUP = 0x0004;
        /// <summary>
        /// 鼠标事件标志：鼠标右键按下。
        /// </summary>
        private const uint MOUSEEVENTF_RIGHTDOWN = 0x0008;
        /// <summary>
        /// 鼠标事件标志：鼠标右键抬起。
        /// </summary>
        private const uint MOUSEEVENTF_RIGHTUP = 0x0010;
        /// <summary>
        /// 鼠标事件标志：鼠标中键按下。
        /// </summary>
        private const uint MOUSEEVENTF_MIDDLEDOWN = 0x0020;
        /// <summary>
        /// 鼠标事件标志：鼠标中键抬起。
        /// </summary>
        private const uint MOUSEEVENTF_MIDDLEUP = 0x0040;
        /// <summary>
        /// 鼠标事件标志：鼠标X键1按下 (用于XButton1/XButton2)。
        /// </summary>
        private const uint MOUSEEVENTF_XBUTTONDOWN = 0x0080; // For XButton1/XButton2
        /// <summary>
        /// 鼠标事件标志：鼠标X键1抬起 (用于XButton1/XButton2)。
        /// </summary>
        private const uint MOUSEEVENTF_XBUTTONUP = 0x0100;   // For XButton1/XButton2
        /// <summary>
        /// 鼠标事件标志：滚轮滚动。
        /// </summary>
        private const uint MOUSEEVENTF_WHEEL = 0x0800;
        /// <summary>
        /// 鼠标事件标志：dx和dy为绝对坐标。
        /// </summary>
        private const uint MOUSEEVENTF_ABSOLUTE = 0x8000;

        /// <summary>
        /// 键盘事件标志：扩展键。
        /// </summary>
        private const uint KEYEVENTF_EXTENDEDKEY = 0x0001;
        /// <summary>
        /// 键盘事件标志：键抬起。
        /// </summary>
        private const uint KEYEVENTF_KEYUP = 0x0002;
        /// <summary>
        /// 键盘事件标志：Unicode字符。
        /// </summary>
        private const uint KEYEVENTF_UNICODE = 0x0004;
        /// <summary>
        /// 键盘事件标志：扫描码。
        /// </summary>
        private const uint KEYEVENTF_SCANCODE = 0x0008;
        #endregion

        #region Win32 API Methods
        /// <summary>
        /// 发送模拟的输入事件到输入流。
        /// </summary>
        /// <param name="nInputs">要插入的事件结构的数量。</param>
        /// <param name="pInputs">指向 INPUT 结构体数组的指针，每个结构体代表一个输入事件。</param>
        /// <param name="cbSize">INPUT 结构体的大小（以字节为单位）。</param>
        /// <returns>成功插入输入队列的事件数量。</returns>
        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);

        /// <summary>
        /// 检索指定系统指标的值。
        /// </summary>
        /// <param name="nIndex">要检索的系统指标。</param>
        /// <returns>指定系统指标的值。</returns>
        [DllImport("user32.dll")]
        private static extern int GetSystemMetrics(int nIndex);

        /// <summary>
        /// 检索光标的屏幕坐标。
        /// </summary>
        /// <param name="lpPoint">接收光标屏幕坐标的 POINT 结构体。</param>
        /// <returns>如果成功检索到位置，则为非零值；否则为零。</returns>
        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool GetCursorPos(out POINT lpPoint);

        /// <summary>
        /// 定义一个点结构，包含X和Y坐标。
        /// </summary>
        [StructLayout(LayoutKind.Sequential)]
        private struct POINT
        {
            /// <summary>
            /// X坐标。
            /// </summary>
            public int X;
            /// <summary>
            /// Y坐标。
            /// </summary>
            public int Y;
        }
        #endregion

        /// <summary>
        /// 模拟鼠标相对移动。
        /// </summary>
        /// <param name="deltaX">X轴上的移动量。</param>
        /// <param name="deltaY">Y轴上的移动量。</param>
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

        /// <summary>
        /// 模拟鼠标左键的按下或释放。
        /// </summary>
        /// <param name="isPressed">如果为 true，则模拟按下；如果为 false，则模拟释放。</param>
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

        /// <summary>
        /// 模拟指定鼠标按钮的按下或释放，支持额外的鼠标按钮（XButton1/XButton2）。
        /// </summary>
        /// <param name="isPressed">如果为 true，则模拟按下；如果为 false，则模拟释放。</param>
        /// <param name="button">要模拟的鼠标按钮的整数值（对应 MouseButtons 枚举）。</param>
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

        /// <summary>
        /// 模拟鼠标滚轮的滚动。
        /// </summary>
        /// <param name="delta">滚动的量。正值表示向上滚动，负值表示向下滚动。</param>
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

        /// <summary>
        /// 模拟键盘按键的按下（Key Down）事件。
        /// </summary>
        /// <param name="keyCode">要模拟的虚拟键码（VirtualKeyCode）。</param>
        public void SimulateKeyPress(int keyCode)
        {
            try
            {
                var input = new INPUT
                {
                    type = INPUT_KEYBOARD,
                    u = new InputUnion
                    {
                        ki = new KEYBDINPUT
                        {
                            wVk = (ushort)keyCode,
                            wScan = 0,
                            dwFlags = 0, // Key down
                            time = 0,
                            dwExtraInfo = IntPtr.Zero
                        }
                    }
                };

                if (SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT))) == 0)
                {
                    int error = Marshal.GetLastWin32Error();
                    System.Diagnostics.Debug.WriteLine($"按键按下模拟失败: {error}");
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"按键按下模拟异常: {ex}");
            }
        }

        /// <summary>
        /// 模拟键盘按键释放。
        /// </summary>
        /// <param name="keyCode">要释放的虚拟键码。</param>
        public void SimulateKeyRelease(int keyCode)
        {
            var input = new INPUT
            {
                type = INPUT_KEYBOARD,
                u = new InputUnion
                {
                    ki = new KEYBDINPUT
                    {
                        wVk = (ushort)keyCode,
                        dwFlags = KEYEVENTF_KEYUP,
                        time = 0,
                        dwExtraInfo = IntPtr.Zero
                    }
                }
            };
            SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT)));
            System.Diagnostics.Debug.WriteLine($"[WindowsInputSimulator] Simulated key up: {keyCode}");
        }

        /// <summary>
        /// 模拟键盘按键按下（不释放）。
        /// </summary>
        /// <param name="keyCode">要按下的虚拟键码。</param>
        public void SimulateKeyDown(int keyCode)
        {
            var input = new INPUT
            {
                type = INPUT_KEYBOARD,
                u = new InputUnion
                {
                    ki = new KEYBDINPUT
                    {
                        wVk = (ushort)keyCode,
                        dwFlags = 0, // No KEYEVENTF_KEYUP flag
                        time = 0,
                        dwExtraInfo = IntPtr.Zero
                    }
                }
            };
            SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT)));
            System.Diagnostics.Debug.WriteLine($"[WindowsInputSimulator] Simulated key down: {keyCode}");
        }

        /// <summary>
        /// 模拟键盘按键释放（不按下）。
        /// </summary>
        /// <param name="keyCode">要释放的虚拟键码。</param>
        public void SimulateKeyUp(int keyCode)
        {
            var input = new INPUT
            {
                type = INPUT_KEYBOARD,
                u = new InputUnion
                {
                    ki = new KEYBDINPUT
                    {
                        wVk = (ushort)keyCode,
                        dwFlags = KEYEVENTF_KEYUP,
                        time = 0,
                        dwExtraInfo = IntPtr.Zero
                    }
                }
            };
            SendInput(1, new INPUT[] { input }, Marshal.SizeOf(typeof(INPUT)));
            System.Diagnostics.Debug.WriteLine($"[WindowsInputSimulator] Simulated key up: {keyCode}");
        }

        /// <summary>
        /// 模拟带有修饰键的按键组合。
        /// 例如，Ctrl + C。
        /// </summary>
        /// <param name="modifier">修饰键的虚拟键码（例如，VirtualKeyCode.CONTROL）。</param>
        /// <param name="key">普通键的虚拟键码（例如，VirtualKeyCode.VK_C）。</param>
        public void SimulateModifiedKeyStroke(VirtualKeyCode modifier, VirtualKeyCode key)
        {
            try
            {
                // 按下修饰键
                SimulateKeyPress((int)modifier);

                // 按下主键
                SimulateKeyPress((int)key);

                // 释放主键
                SimulateKeyRelease((int)key);

                // 释放修饰键
                SimulateKeyRelease((int)modifier);
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"组合键模拟异常: {ex}");
            }
        }

        /// <summary>
        /// 强制释放所有可能被模拟的键。
        /// 通常在应用程序关闭或断开连接时调用，以避免"卡住"的键状态。
        /// </summary>
        public void ForceReleaseAllButtons()
        {
            // 对于鼠标按键，模拟所有按钮的释放
            SimulateMouseButtonEx(false, (int)MouseButtons.Left);
            SimulateMouseButtonEx(false, (int)MouseButtons.Right);
            SimulateMouseButtonEx(false, (int)MouseButtons.Middle);
            SimulateMouseButtonEx(false, (int)MouseButtons.XButton1);
            SimulateMouseButtonEx(false, (int)MouseButtons.XButton2);

            // 对于键盘按键，目前无法知道哪些键被按下，所以只释放一些常用键或依赖于 InputStateMonitorService 的处理
            // 这里的实现可能需要改进，如果需要更精确的"全部释放"，则需要跟踪当前按下的键
            // 作为简化的安全措施，可以释放一些最常用的控制键
            SimulateKeyRelease((int)VirtualKeyCode.CONTROL);
            SimulateKeyRelease((int)VirtualKeyCode.VK_MENU); // Alt
            SimulateKeyRelease((int)VirtualKeyCode.VK_SHIFT);
            SimulateKeyRelease((int)VirtualKeyCode.VK_LWIN); // Windows Key
            SimulateKeyRelease((int)VirtualKeyCode.VK_RWIN);

            // 如果有其他模拟的特殊键，也应该在这里添加释放逻辑
        }
    }
}