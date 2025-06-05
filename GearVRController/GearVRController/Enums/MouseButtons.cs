namespace GearVRController.Enums
{
    // 鼠标按钮常量
    // 更多信息请参考: https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-mouse_event
    public enum MouseButtons : int
    {
        /// <summary>
        /// Specifies the left mouse button.
        /// </summary>
        Left = 0x0002,
        /// <summary>
        /// Specifies the right mouse button.
        /// </summary>
        Right = 0x0008,
        /// <summary>
        /// Specifies the middle mouse button.
        /// </summary>
        Middle = 0x0020,
        /// <summary>
        /// Specifies the first extended mouse button (XBUTTON1).
        /// </summary>
        XButton1 = 0x0080,
        /// <summary>
        /// Specifies the second extended mouse button (XBUTTON2).
        /// </summary>
        XButton2 = 0x0100
    }
}