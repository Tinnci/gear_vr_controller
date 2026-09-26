//! Internationalization (i18n) and 4-Language Localization Service
//!
//! Provides automatic system language detection (Chinese, English, Japanese, Korean)
//! and localized UI string dictionaries for both WinUI 3 and egui frontends.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Auto,
    SimplifiedChinese,
    English,
    Japanese,
    Korean,
}

impl Language {
    #[allow(dead_code)]
    pub const ALL: [Language; 5] = [
        Language::Auto,
        Language::SimplifiedChinese,
        Language::English,
        Language::Japanese,
        Language::Korean,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Auto => "Auto (System Default)",
            Language::SimplifiedChinese => "简体中文 (Simplified Chinese)",
            Language::English => "English",
            Language::Japanese => "日本語 (Japanese)",
            Language::Korean => "한국어 (Korean)",
        }
    }

    /// Resolves `Auto` to the detected system language
    pub fn resolve(&self) -> Language {
        match self {
            Language::Auto => detect_system_language(),
            specific => *specific,
        }
    }

    /// Returns the localized string dictionary for this language
    pub fn strings(&self) -> &'static I18nStrings {
        match self.resolve() {
            Language::SimplifiedChinese => &ZH_CN_STRINGS,
            Language::Japanese => &JA_JP_STRINGS,
            Language::Korean => &KO_KR_STRINGS,
            _ => &EN_US_STRINGS,
        }
    }
}

/// Detects current Windows user UI language via Win32 API
#[cfg(target_os = "windows")]
pub fn detect_system_language() -> Language {
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    let lcid = unsafe { GetUserDefaultUILanguage() };
    let primary_lang = lcid & 0x03FF;
    match primary_lang {
        0x04 => Language::SimplifiedChinese, // Chinese (Simplified / Traditional / HK / SG)
        0x11 => Language::Japanese,          // Japanese
        0x12 => Language::Korean,            // Korean
        _ => Language::English,              // Default fallback to English
    }
}

#[cfg(not(target_os = "windows"))]
pub fn detect_system_language() -> Language {
    Language::English
}

pub struct I18nStrings {
    pub app_title: &'static str,
    pub app_subtitle: &'static str,

    // Navigation
    pub nav_pane_title: &'static str,
    pub nav_dashboard: &'static str,
    pub nav_calibration: &'static str,
    pub nav_settings: &'static str,
    pub nav_diagnostics: &'static str,

    // Status Infobar
    pub status_connected: &'static str,
    pub status_ready: &'static str,
    pub status_connecting: &'static str,
    pub status_negotiating: &'static str,
    pub status_disconnected: &'static str,
    pub status_no_link: &'static str,
    pub status_error: &'static str,

    // Dashboard: Connection Card
    pub conn_card_title: &'static str,
    pub conn_card_desc: &'static str,
    pub address_placeholder: &'static str,
    pub connect_button: &'static str,
    pub disconnect_button: &'static str,
    pub scan_button: &'static str,
    pub stop_scan_button: &'static str,

    // Dashboard: Mode Card
    pub mode_card_title: &'static str,
    pub mode_card_desc: &'static str,
    pub mode_air_mouse: &'static str,
    pub mode_trackpad: &'static str,
    pub mode_presenter: &'static str,
    pub active_prefix: &'static str,

    // Dashboard: Telemetry Card
    pub telemetry_card_title: &'static str,
    pub telemetry_card_desc: &'static str,
    pub telemetry_awaiting: &'static str,
    pub buttons_idle: &'static str,
    pub timestamp_no_tx: &'static str,

    // Calibration
    pub touch_cal_title: &'static str,
    pub touch_cal_desc: &'static str,
    pub touch_cal_status: &'static str,
    pub imu_cal_title: &'static str,
    pub imu_cal_desc: &'static str,
    pub imu_cal_status: &'static str,
    pub imu_cal_filter: &'static str,

    // Settings
    pub language_card_title: &'static str,
    pub language_card_desc: &'static str,
    pub anti_sleep_title: &'static str,
    pub anti_sleep_desc: &'static str,
    pub auto_profile_title: &'static str,
    pub auto_profile_desc: &'static str,
    pub tray_title: &'static str,
    pub tray_desc: &'static str,

    // Diagnostics
    pub imu_diag_title: &'static str,
    pub imu_diag_desc: &'static str,
    pub bt_recovery_title: &'static str,
    pub bt_recovery_desc: &'static str,
    pub bt_troubleshoot_hint: &'static str,
    pub open_bt_settings: &'static str,
}

// ----------------- English (Default) -----------------
pub static EN_US_STRINGS: I18nStrings = I18nStrings {
    app_title: "Samsung Gear VR Controller",
    app_subtitle: "Universal Windows Bluetooth Input Driver & Motion Translation Service",

    nav_pane_title: "Gear VR Controller",
    nav_dashboard: "Dashboard",
    nav_calibration: "Calibration",
    nav_settings: "Settings",
    nav_diagnostics: "Diagnostics",

    status_connected: "Connected",
    status_ready: "Ready for motion and button input",
    status_connecting: "Connecting",
    status_negotiating: "Negotiating Bluetooth Low Energy GATT link...",
    status_disconnected: "Disconnected",
    status_no_link: "No active controller link",
    status_error: "Connection Error",

    conn_card_title: "Bluetooth Controller Link",
    conn_card_desc: "Pair and manage low-latency connection to Samsung Gear VR Controller",
    address_placeholder: "Bluetooth Address (e.g. 2C41A1001234)",
    connect_button: "Connect",
    disconnect_button: "Disconnect",
    scan_button: "Scan Devices",
    stop_scan_button: "Stop Scan",

    mode_card_title: "Active Control Profile",
    mode_card_desc: "Choose motion translation model and input behavior",
    mode_air_mouse: "Air Mouse",
    mode_trackpad: "Trackpad",
    mode_presenter: "Presenter",
    active_prefix: "[ Active: {} ]",

    telemetry_card_title: "Real-Time Input Telemetry",
    telemetry_card_desc: "Live stream of controller sensor events, touch coordinates, and button states",
    telemetry_awaiting: "Touchpad Position: Awaiting input stream...",
    buttons_idle: "Buttons: Trigger: Idle | Back: Idle | Home: Idle",
    timestamp_no_tx: "Controller Timestamp: No active transmission",

    touch_cal_title: "Touchpad Boundary Normalization",
    touch_cal_desc: "Glide your thumb across the extreme edges of the touchpad to calibrate sensor bounds",
    touch_cal_status: "Normalized Domain: [-1.0, 1.0] across horizontal and vertical axes",
    imu_cal_title: "IMU Gyroscope Zero-Point Reference",
    imu_cal_desc: "Place controller completely flat and motionless on a level desk to eliminate rotational drift",
    imu_cal_status: "Sensor Calibration Status: Reference Tare Balanced",
    imu_cal_filter: "Dynamic drift compensation filter is continuously active during runtime.",

    language_card_title: "Display Language",
    language_card_desc: "Choose UI language or automatically follow Windows system settings",
    anti_sleep_title: "Prevent Display Sleep",
    anti_sleep_desc: "Keep Windows displays awake while in Presenter mode to ensure uninterrupted slideshows",
    auto_profile_title: "Context-Aware Auto Switching",
    auto_profile_desc: "Automatically switch profile to Presenter mode when PowerPoint, Keynote, or PDF viewer is focused",
    tray_title: "System Tray Background Execution",
    tray_desc: "Keep background Bluetooth link running in the Windows taskbar notification area",

    imu_diag_title: "9-DOF IMU Raw Sensor Readings",
    imu_diag_desc: "Direct telemetry stream decoded from Gear VR Controller GATT characteristic packets",
    bt_recovery_title: "Windows Bluetooth Subsystem Diagnostics",
    bt_recovery_desc: "System-level troubleshooting actions to unpair stale GATT handles and clear ghost devices",
    bt_troubleshoot_hint: "If Bluetooth discovery fails, check paired state in Windows Settings.",
    open_bt_settings: "Open Windows Bluetooth Settings",
};

// ----------------- 简体中文 (Simplified Chinese) -----------------
pub static ZH_CN_STRINGS: I18nStrings = I18nStrings {
    app_title: "三星 Gear VR 手柄控制器",
    app_subtitle: "通用 Windows 蓝牙输入驱动与姿态动作转换服务",

    nav_pane_title: "Gear VR 控制中心",
    nav_dashboard: "控制面板",
    nav_calibration: "校准中心",
    nav_settings: "系统设置",
    nav_diagnostics: "遥测诊断",

    status_connected: "已连接",
    status_ready: "已就绪，正在接收动作姿态与按键输入",
    status_connecting: "正在连接",
    status_negotiating: "正在协商低功耗蓝牙 (BLE) GATT 通信链路...",
    status_disconnected: "已断开",
    status_no_link: "当前无活跃的控制器连接",
    status_error: "连接异常",

    conn_card_title: "蓝牙控制器无线连接",
    conn_card_desc: "配对并管理与三星 Gear VR 控制器的低延迟无线通信",
    address_placeholder: "蓝牙 MAC 地址 (例如 2C41A1001234)",
    connect_button: "连接控制器",
    disconnect_button: "断开连接",
    scan_button: "搜索设备",
    stop_scan_button: "停止搜索",

    mode_card_title: "工作控制模式",
    mode_card_desc: "选择传感器姿态映射算法与按键映射行为",
    mode_air_mouse: "空中飞鼠",
    mode_trackpad: "笔记本触控板",
    mode_presenter: "演示演讲笔",
    active_prefix: "[ 当前: {} ]",

    telemetry_card_title: "实时输入遥测",
    telemetry_card_desc: "实时监控触控板坐标、物理按键状态及信号数据包序列",
    telemetry_awaiting: "触控板坐标: 等待输入数据流...",
    buttons_idle: "按键状态: 扳机键: 空闲 | 返回键: 空闲 | 主页键: 空闲",
    timestamp_no_tx: "控制器时戳: 无活跃数据流",

    touch_cal_title: "触控板物理边界归一化校准",
    touch_cal_desc: "在触控板四周边缘滑动拇指，重新映射传感器物理有效范围",
    touch_cal_status: "归一化范围: 水平及垂直轴 [-1.0, 1.0]",
    imu_cal_title: "陀螺仪零点校准与漂移补偿",
    imu_cal_desc: "将控制器水平静置于平整桌面上，消除传感器旋转漂移偏差",
    imu_cal_status: "传感器校准状态: 零位基准平衡已就绪",
    imu_cal_filter: "动态漂移补偿滤波器在运行时持续保持激活状态。",

    language_card_title: "界面显示语言",
    language_card_desc: "选择界面显示语言或自动跟随 Windows 系统默认设置",
    anti_sleep_title: "演示防休眠",
    anti_sleep_desc: "在演示模式下保持屏幕常亮，防止演示中断或息屏锁定",
    auto_profile_title: "场景感知自适应切换",
    auto_profile_desc: "检测到 PowerPoint、Keynote 或 PDF 处于前台时自动切换至演示模式",
    tray_title: "系统托盘后台驻留",
    tray_desc: "窗口关闭时最小化到系统托盘，维持蓝牙后台链路不中断",

    imu_diag_title: "9 轴 IMU 原始物理传感器遥测",
    imu_diag_desc: "从控制器低功耗蓝牙特征值中解析出的高精度实时物理向量",
    bt_recovery_title: "Windows 蓝牙子系统诊断与修复",
    bt_recovery_desc: "清理残留的幽灵蓝牙句柄，重新同步 Windows 设备管理器",
    bt_troubleshoot_hint: "如果蓝牙扫描或连接失败，请在 Windows 设置中检查配对状态。",
    open_bt_settings: "打开 Windows 蓝牙设置",
};

// ----------------- 日本語 (Japanese) -----------------
pub static JA_JP_STRINGS: I18nStrings = I18nStrings {
    app_title: "Samsung Gear VR コントローラー",
    app_subtitle: "ユニバーサル Windows Bluetooth 入力ドライバー＆モーション変換サービス",

    nav_pane_title: "Gear VR コントローラー",
    nav_dashboard: "ダッシュボード",
    nav_calibration: "キャリブレーション",
    nav_settings: "設定",
    nav_diagnostics: "診断情報",

    status_connected: "接続完了",
    status_ready: "モーションおよびボタン入力の準備完了",
    status_connecting: "接続中",
    status_negotiating: "Bluetooth Low Energy GATT リンクをネゴシエート中...",
    status_disconnected: "未接続",
    status_no_link: "アクティブなコントローラーリンクがありません",
    status_error: "接続エラー",

    conn_card_title: "Bluetooth コントローラーリンク",
    conn_card_desc: "Samsung Gear VR コントローラーとの低遅延通信をペアリング・管理",
    address_placeholder: "Bluetooth アドレス (例: 2C41A1001234)",
    connect_button: "接続",
    disconnect_button: "切断",
    scan_button: "デバイス検索",
    stop_scan_button: "検索停止",

    mode_card_title: "動作プロファイル",
    mode_card_desc: "モーション変換モデルと入力動作を選択",
    mode_air_mouse: "エアマウス",
    mode_trackpad: "トラックパッド",
    mode_presenter: "プレゼンター",
    active_prefix: "[ 有効: {} ]",

    telemetry_card_title: "リアルタイム入力テレメトリ",
    telemetry_card_desc: "センサー、タッチ座標、ボタン状態のライブストリーム",
    telemetry_awaiting: "タッチパッド座標: 入力待機中...",
    buttons_idle: "ボタン: トリガー: 待機 | 戻る: 待機 | ホーム: 待機",
    timestamp_no_tx: "コントローラータイムスタンプ: 送信データなし",

    touch_cal_title: "タッチパッド境界正規化",
    touch_cal_desc: "タッチパッドの端まで親指を滑らせてセンサー境界を測定します",
    touch_cal_status: "正規化範囲: 水平・垂直軸ともに [-1.0, 1.0]",
    imu_cal_title: "ジャイロスコープ原点キャリブレーション",
    imu_cal_desc: "回転ドリフトを解消するため平らな場所に静止させてください",
    imu_cal_status: "キャリブレーション状態: 基準バランス調整済み",
    imu_cal_filter: "動的ドリフト補正フィルターが常時有効です。",

    language_card_title: "表示言語",
    language_card_desc: "表示言語を選択するかWindowsシステムの既定値に従います",
    anti_sleep_title: "画面スリープ防止",
    anti_sleep_desc: "プレゼンターモード中、画面オフやスリープを防止します",
    auto_profile_title: "コンテキスト自動プロファイル切替",
    auto_profile_desc: "PowerPointやPDFが前面にある場合、自動的にプレゼンターに切り替えます",
    tray_title: "タスクトレイ常駐",
    tray_desc: "ウィンドウを閉じてもタスクトレイでBluetooth通信を維持します",

    imu_diag_title: "9軸 IMU 生センサー測定値",
    imu_diag_desc: "Gear VR コントローラーの GATT パケットからデコードされた生データ",
    bt_recovery_title: "Windows Bluetooth サブシステム診断",
    bt_recovery_desc: "古いGATTハンドルを解除し、ゴーストデバイスをクリアします",
    bt_troubleshoot_hint: "Bluetooth 接続に失敗する場合は、Windows 設定を確認してください。",
    open_bt_settings: "Windows Bluetooth 設定を開く",
};

// ----------------- 한국어 (Korean) -----------------
pub static KO_KR_STRINGS: I18nStrings = I18nStrings {
    app_title: "삼성 기어 VR 컨트롤러",
    app_subtitle: "범용 Windows 블루투스 입력 드라이버 및 모션 변환 서비스",

    nav_pane_title: "기어 VR 제어 센터",
    nav_dashboard: "대시보드",
    nav_calibration: "보정 센터",
    nav_settings: "설정",
    nav_diagnostics: "원격 진단",

    status_connected: "연결됨",
    status_ready: "모션 및 버튼 입력 준비 완료",
    status_connecting: "연결 중",
    status_negotiating: "저전력 블루투스(BLE) GATT 링크 협상 중...",
    status_disconnected: "연결 끊김",
    status_no_link: "활성화된 컨트롤러 연결 없음",
    status_error: "연결 오류",

    conn_card_title: "블루투스 컨트롤러 무선 연결",
    conn_card_desc: "삼성 기어 VR 컨트롤러 저지연 무선 연결 페어링 및 관리",
    address_placeholder: "블루투스 MAC 주소 (예: 2C41A1001234)",
    connect_button: "컨트롤러 연결",
    disconnect_button: "연결 끊기",
    scan_button: "장치 검색",
    stop_scan_button: "검색 중지",

    mode_card_title: "활성 제어 프로필",
    mode_card_desc: "모션 변환 알고리즘 및 입력 동작 선택",
    mode_air_mouse: "에어 마우스",
    mode_trackpad: "트랙패드",
    mode_presenter: "프레젠터",
    active_prefix: "[ 활성: {} ]",

    telemetry_card_title: "실시간 입력 텔레메트리",
    telemetry_card_desc: "컨트롤러 센서 이벤트, 터치 좌표 및 버튼 상태 실시간 모니터링",
    telemetry_awaiting: "터치패드 좌표: 입력 스트림 대기 중...",
    buttons_idle: "버튼 상태: 트리거: 대기 | 뒤로: 대기 | 홈: 대기",
    timestamp_no_tx: "컨트롤러 타임스탬프: 활성 전송 데이터 없음",

    touch_cal_title: "터치패드 물리 경계 정규화 보정",
    touch_cal_desc: "터치패드 가장자리를 따라 엄지손가락을 문질러 센서 범위를 보정합니다",
    touch_cal_status: "정규화 범위: 수평 및 수직 축 [-1.0, 1.0]",
    imu_cal_title: "자이로스코프 영점 보정 및 드리프트 보상",
    imu_cal_desc: "회전 드리프트를 제거하기 위해 컨트롤러를 평평한 바닥에 움직이지 않게 두세요",
    imu_cal_status: "센서 보정 상태: 기준점 영점 균형 완료",
    imu_cal_filter: "동적 드리프트 보정 필터가 런타임 동안 지속적으로 작동합니다.",

    language_card_title: "인터페이스 표시 언어",
    language_card_desc: "UI 표시 언어를 선택하거나 Windows 시스템 기본값을 따릅니다",
    anti_sleep_title: "화면 절전 방지",
    anti_sleep_desc: "프레젠터 모드 중 슬라이드쇼가 중단되지 않도록 디스플레이를 계속 켭니다",
    auto_profile_title: "상황 인식 자동 프로필 전환",
    auto_profile_desc: "PowerPoint 또는 PDF가 포커스될 때 자동으로 프레젠터 모드로 전환",
    tray_title: "시스템 트레이 백그라운드 실행",
    tray_desc: "창을 닫아도 작업 표시줄 알림 영역에서 블루투스 연결 유지",

    imu_diag_title: "9축 IMU 원시 센서 판독값",
    imu_diag_desc: "기어 VR 컨트롤러 GATT 특성 패킷에서 직접 디코딩된 실시간 센서 벡터",
    bt_recovery_title: "Windows 블루투스 하위 시스템 진단 및 복구",
    bt_recovery_desc: "오래된 GATT 핸들 페어링을 해제하고 고스트 장치를 정리합니다",
    bt_troubleshoot_hint: "블루투스 검색이 실패하면 Windows 설정에서 페어링 상태를 확인하세요.",
    open_bt_settings: "Windows 블루투스 설정 열기",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_languages_have_valid_strings() {
        for lang in Language::ALL {
            let s = lang.strings();
            assert!(!s.app_title.is_empty());
            assert!(!s.nav_dashboard.is_empty());
            assert!(!s.nav_calibration.is_empty());
            assert!(!s.nav_settings.is_empty());
            assert!(!s.nav_diagnostics.is_empty());
            assert!(!s.connect_button.is_empty());
            assert!(!s.mode_air_mouse.is_empty());
        }
    }

    #[test]
    fn test_auto_resolution() {
        let detected = detect_system_language();
        let auto_resolved = Language::Auto.resolve();
        assert_eq!(detected, auto_resolved);
        assert_ne!(auto_resolved, Language::Auto);
    }

    #[test]
    fn test_language_display_names() {
        assert!(Language::SimplifiedChinese.display_name().contains("简体中文"));
        assert!(Language::English.display_name().contains("English"));
        assert!(Language::Japanese.display_name().contains("日本語"));
        assert!(Language::Korean.display_name().contains("한국어"));
    }
}
