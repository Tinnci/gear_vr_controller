namespace GearVRController.Enums
{
    public enum VirtualKeyCode
    {
        // 定义 Win32 API 虚拟键代码
        // 更多信息请参考: https://docs.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
        VK_LBUTTON = 0x01, // 鼠标左键
        VK_RBUTTON = 0x02, // 鼠标右键
        VK_CANCEL = 0x03,
        VK_MBUTTON = 0x04, // 鼠标中键
        VK_XBUTTON1 = 0x05, // X1鼠标按钮
        VK_XBUTTON2 = 0x06, // X2鼠标按钮

        VK_BACK = 0x08, // BACKSPACE键
        VK_TAB = 0x09, // TAB键
        VK_CLEAR = 0x0C, // CLEAR键
        VK_RETURN = 0x0D, // ENTER键
        VK_SHIFT = 0x10, // SHIFT键
        CONTROL = 0x11, // CTRL键
        VK_MENU = 0x12, // ALT键
        VK_PAUSE = 0x13, // PAUSE键
        VK_CAPITAL = 0x14, // CAPS LOCK键
        VK_KANA = 0x15, // IME Kana/Hangul模式
        VK_HANGUL = 0x15, // IME Hangul模式
        VK_IME_ON = 0x16, // IME On
        VK_JUNJA = 0x17, // IME Junja模式
        VK_FINAL = 0x18, // IME Final模式
        VK_HANJA = 0x19, // IME Hanja/Kanji模式
        VK_KANJI = 0x19, // IME Kanji模式
        VK_IME_OFF = 0x1A, // IME Off
        VK_ESCAPE = 0x1B, // ESC键
        VK_CONVERT = 0x1C, // IME Convert
        VK_NONCONVERT = 0x1D, // IME NonConvert
        VK_ACCEPT = 0x1E, // IME Accept
        VK_MODECHANGE = 0x1F, // IME Mode change request
        VK_SPACE = 0x20, // SPACEBAR
        PRIOR = 0x21, // PAGE UP键
        NEXT = 0x22, // PAGE DOWN键
        VK_END = 0x23, // END键
        VK_HOME = 0x24, // HOME键
        VK_LEFT = 0x25, // LEFT ARROW键
        VK_UP = 0x26, // UP ARROW键
        VK_RIGHT = 0x27, // RIGHT ARROW键
        VK_DOWN = 0x28, // DOWN ARROW键
        VK_SELECT = 0x29, // SELECT键
        VK_PRINT = 0x2A, // PRINT键
        VK_EXECUTE = 0x2B, // EXECUTE键
        VK_SNAPSHOT = 0x2C, // PRINT SCREEN键
        VK_INSERT = 0x2D, // INS键
        VK_DELETE = 0x2E, // DEL键
        VK_HELP = 0x2F, // HELP键

        VK_0 = 0x30, // 0键
        VK_1 = 0x31, // 1键
        VK_2 = 0x32, // 2键
        VK_3 = 0x33, // 3键
        VK_4 = 0x34, // 4键
        VK_5 = 0x35, // 5键
        VK_6 = 0x36, // 6键
        VK_7 = 0x37, // 7键
        VK_8 = 0x38, // 8键
        VK_9 = 0x39, // 9键

        VK_A = 0x41, // A键
        VK_B = 0x42, // B键
        VK_C = 0x43, // C键
        VK_D = 0x44, // D键
        VK_E = 0x45, // E键
        VK_F = 0x46, // F键
        VK_G = 0x47, // G键
        VK_H = 0x48, // H键
        VK_I = 0x49, // I键
        VK_J = 0x4A, // J键
        VK_K = 0x4B, // K键
        VK_L = 0x4C, // L键
        VK_M = 0x4D, // M键
        VK_N = 0x4E, // N键
        VK_O = 0x4F, // O键
        VK_P = 0x50, // P键
        VK_Q = 0x51, // Q键
        VK_R = 0x52, // R键
        VK_S = 0x53, // S键
        VK_T = 0x54, // T键
        VK_U = 0x55, // U键
        VK_V = 0x56, // V键
        VK_W = 0x57, // W键
        VK_X = 0x58, // X键
        VK_Y = 0x59, // Y键
        VK_Z = 0x5A, // Z键

        VK_LWIN = 0x5B, // 左Windows键
        VK_RWIN = 0x5C, // 右Windows键
        VK_APPS = 0x5D, // 应用程序键
        VK_SLEEP = 0x5F, // 电脑睡眠键
        VK_NUMPAD0 = 0x60, // 小键盘0
        VK_NUMPAD1 = 0x61, // 小键盘1
        VK_NUMPAD2 = 0x62, // 小键盘2
        VK_NUMPAD3 = 0x63, // 小键盘3
        VK_NUMPAD4 = 0x64, // 小键盘4
        VK_NUMPAD5 = 0x65, // 小键盘5
        VK_NUMPAD6 = 0x66, // 小键盘6
        VK_NUMPAD7 = 0x67, // 小键盘7
        VK_NUMPAD8 = 0x68, // 小键盘8
        VK_NUMPAD9 = 0x69, // 小键盘9
        VK_MULTIPLY = 0x6A, // 乘号键
        VK_ADD = 0x6B, // 加号键
        VK_SEPARATOR = 0x6C, // 分隔符键
        VK_SUBTRACT = 0x6D, // 减号键
        VK_DECIMAL = 0x6E, // 小数点键
        VK_DIVIDE = 0x6F, // 除号键
        VK_F1 = 0x70, // F1键
        VK_F2 = 0x71, // F2键
        VK_F3 = 0x72, // F3键
        VK_F4 = 0x73, // F4键
        VK_F5 = 0x74, // F5键
        VK_F6 = 0x75, // F6键
        VK_F7 = 0x76, // F7键
        VK_F8 = 0x77, // F8键
        VK_F9 = 0x78, // F9键
        VK_F10 = 0x79, // F10键
        VK_F11 = 0x7A, // F11键
        VK_F12 = 0x7B, // F12键
        VK_F13 = 0x7C, // F13键
        VK_F14 = 0x7D, // F14键
        VK_F15 = 0x7E, // F15键
        VK_F16 = 0x7F, // F16键
        VK_F17 = 0x80, // F17键
        VK_F18 = 0x81, // F18键
        VK_F19 = 0x82, // F19键
        VK_F20 = 0x83, // F20键
        VK_F21 = 0x84, // F21键
        VK_F22 = 0x85, // F22键
        VK_F23 = 0x86, // F23键
        VK_F24 = 0x87, // F24键

        VK_NUMLOCK = 0x90, // NUM LOCK键
        VK_SCROLL = 0x91, // SCROLL LOCK键
        VK_OEM_NEC_EQUAL = 0x92, // =键
        VK_OEM_FJ_JISHO = 0x92, // 'Jisho' (Dictionary)键
        VK_OEM_FJ_MASSHOU = 0x93, // 'Masshou' (Erase)键
        VK_OEM_FJ_TOUROKU = 0x94, // 'Touroku' (Register)键
        VK_OEM_FJ_LOYA = 0x95, // 'Loya' (Left)键
        VK_OEM_FJ_ROYA = 0x96, // 'Roya' (Right)键

        VK_LSHIFT = 0xA0, // 左SHIFT键
        VK_RSHIFT = 0xA1, // 右SHIFT键
        VK_LCONTROL = 0xA2, // 左CTRL键
        VK_RCONTROL = 0xA3, // 右CTRL键
        VK_LMENU = 0xA4, // 左ALT键
        VK_RMENU = 0xA5, // 右ALT键

        BROWSER_BACK = 0xA6, // 浏览器后退键
        BROWSER_FORWARD = 0xA7, // 浏览器前进键
        VK_BROWSER_REFRESH = 0xA8, // 浏览器刷新键
        VK_BROWSER_STOP = 0xA9, // 浏览器停止键
        VK_BROWSER_SEARCH = 0xAA, // 浏览器搜索键
        VK_BROWSER_FAVORITES = 0xAB, // 浏览器收藏夹键
        VK_BROWSER_HOME = 0xAC, // 浏览器主页键

        VOLUME_MUTE = 0xAD, // 静音键
        VOLUME_DOWN = 0xAE, // 音量减键
        VOLUME_UP = 0xAF, // 音量加键
        MEDIA_NEXT_TRACK = 0xB0, // 下一曲键
        MEDIA_PREV_TRACK = 0xB1, // 上一曲键
        MEDIA_STOP = 0xB2, // 停止键
        MEDIA_PLAY_PAUSE = 0xB3, // 播放/暂停键
        VK_LAUNCH_MAIL = 0xB4, // 邮件启动键
        VK_LAUNCH_MEDIA_SELECT = 0xB5, // 媒体选择启动键
        VK_LAUNCH_APP1 = 0xB6, // 应用程序1启动键
        VK_LAUNCH_APP2 = 0xB7, // 应用程序2启动键

        VK_OEM_1 = 0xBA, // 用于OEM特定目的的按键 (通常是 ';:')
        VK_OEM_PLUS = 0xBB, // PLUS键
        VK_OEM_COMMA = 0xBC, // 逗号键
        VK_OEM_MINUS = 0xBD, // 减号键
        VK_OEM_PERIOD = 0xBE, // 句号键
        VK_OEM_2 = 0xBF, // 用于OEM特定目的的按键 (通常是 '/?')
        VK_OEM_3 = 0xC0, // 用于OEM特定目的的按键 (通常是 '`~')

        VK_OEM_4 = 0xDB, // 用于OEM特定目的的按键 (通常是 '[{')
        VK_OEM_5 = 0xDC, // 用于OEM特定目的的按键 (通常是 '\|')
        VK_OEM_6 = 0xDD, // 用于OEM特定目的的按键 (通常是 ']}')
        VK_OEM_7 = 0xDE, // 用于OEM特定目的的按键 (通常是 '单引号/双引号')
        VK_OEM_8 = 0xDF, // 用于OEM特定目的的按键

        VK_OEM_AX = 0xE1, // AX键
        VK_OEM_102 = 0xE2, // "<>" or "\|" on RT 102-key keyboard
        VK_ICO_HELP = 0xE3, // 帮助图标
        VK_ICO_00 = 0xE4, // 00图标
        VK_PROCESSKEY = 0xE5, // PROCESS键
        VK_ICO_CLEAR = 0xE6, // 清除图标

        VK_PACKET = 0xE7, // Used to pass Unicode characters as if they were keystrokes.

        VK_OEM_RESET = 0xE9, // OEM重置
        VK_OEM_JUMP = 0xEA, // OEM跳转
        VK_OEM_PA1 = 0xEB, // OEM PA1
        VK_OEM_PA2 = 0xEC, // OEM PA2
        VK_OEM_PA3 = 0xED, // OEM PA3
        VK_OEM_WSCTRL = 0xEE, // OEM WSCTRL
        VK_OEM_CUSEL = 0xEF, // OEM CUSEL
        VK_OEM_ATTN = 0xF0, // OEM ATTN
        VK_OEM_FINISH = 0xF1, // OEM FINISH
        VK_OEM_COPY = 0xF2, // OEM COPY
        VK_OEM_AUTO = 0xF3, // OEM AUTO
        VK_OEM_ENLW = 0xF4, // OEM ENLW
        VK_OEM_BACKTAB = 0xF5, // OEM BACKTAB

        VK_ATTN = 0xF6, // ATTN键
        VK_CRSEL = 0xF7, // CRSEL键
        VK_EXSEL = 0xF8, // EXSEL键
        VK_EREOF = 0xF9, // EREOF键
        VK_PLAY = 0xFA, // PLAY键
        VK_ZOOM = 0xFB, // ZOOM键
        VK_NONAME = 0xFC, // 无名键
        VK_PA1 = 0xFD, // PA1键
        VK_OEM_CLEAR = 0xFE // OEM清除键
    }
}