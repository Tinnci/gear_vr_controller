using System;
using System.Runtime.InteropServices;
using GearVRController.Enums;

namespace GearVRController.Services.Interfaces
{
    /// <summary>
    /// 定义一个输入模拟器服务接口，用于模拟鼠标和键盘输入。
    /// </summary>
    public interface IInputSimulator
    {
        /// <summary>
        /// 模拟鼠标移动。
        /// </summary>
        /// <param name="deltaX">X 轴上的移动量。</param>
        /// <param name="deltaY">Y 轴上的移动量。</param>
        void SimulateMouseMovement(double deltaX, double deltaY);
        /// <summary>
        /// 模拟鼠标按钮按下或释放。
        /// </summary>
        /// <param name="isPressed">如果为 true，则模拟按下；如果为 false，则模拟释放。</param>
        void SimulateMouseButton(bool isPressed);
        /// <summary>
        /// 模拟键盘按键按下。
        /// </summary>
        /// <param name="keyCode">要按下的虚拟键码。</param>
        void SimulateKeyPress(int keyCode);
        /// <summary>
        /// 模拟键盘按键释放。
        /// </summary>
        /// <param name="keyCode">要释放的虚拟键码。</param>
        void SimulateKeyRelease(int keyCode);
        /// <summary>
        /// 模拟鼠标滚轮移动。
        /// </summary>
        /// <param name="delta">滚轮移动量（正值向上滚动，负值向下滚动）。</param>
        void SimulateWheelMovement(int delta);
        /// <summary>
        /// 模拟鼠标按钮（扩展）按下或释放。
        /// </summary>
        /// <param name="isPressed">如果为 true，则模拟按下；如果为 false，则模拟释放。</param>
        /// <param name="button">要模拟的鼠标按钮。</param>
        void SimulateMouseButtonEx(bool isPressed, int button);
        /// <summary>
        /// 模拟带有修饰键的按键组合。
        /// </summary>
        /// <param name="modifier">修饰键（例如：<see cref="VirtualKeyCode.SHIFT"/>, <see cref="VirtualKeyCode.CONTROL"/>）。</param>
        /// <param name="key">要按下的键。</param>
        void SimulateModifiedKeyStroke(VirtualKeyCode modifier, VirtualKeyCode key);
    }
}