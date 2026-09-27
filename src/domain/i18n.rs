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
    pub fn action_text(&self, action: &str) -> &'static str {
        match (self.resolve(), action) {
            (Language::SimplifiedChinese, "imu_done") => "陀螺仪校准完成",
            (Language::Japanese, "imu_done") => "ジャイロ校正完了",
            (Language::Korean, "imu_done") => "자이로 보정 완료",
            (_, "imu_done") => "Gyroscope calibration complete",
            (Language::SimplifiedChinese, "touch_start") => "开始触控校准（沿边缘移动）",
            (Language::SimplifiedChinese, "touch_save") => "保存触控校准",
            (Language::SimplifiedChinese, "imu_start") => "校准陀螺仪（保持静止）",
            (Language::SimplifiedChinese, "recover") => "重启蓝牙服务（需要管理员权限）",
            (Language::Japanese, "touch_start") => "タッチ校正を開始（縁をなぞる）",
            (Language::Japanese, "touch_save") => "タッチ校正を保存",
            (Language::Japanese, "imu_start") => "ジャイロを校正（静止してください）",
            (Language::Japanese, "recover") => "Bluetooth サービス再起動（管理者）",
            (Language::Korean, "touch_start") => "터치 보정 시작 (가장자리를 따라 이동)",
            (Language::Korean, "touch_save") => "터치 보정 저장",
            (Language::Korean, "imu_start") => "자이로 보정 (움직이지 마세요)",
            (Language::Korean, "recover") => "Bluetooth 서비스 재시작 (관리자)",
            (_, "touch_start") => "Start touch calibration (trace the edge)",
            (_, "touch_save") => "Save touch calibration",
            (_, "imu_start") => "Calibrate gyroscope (keep still)",
            _ => "Restart Bluetooth service (administrator)",
        }
    }
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    app_subtitle: "Bluetooth Input Driver and Motion Control for Windows",

    nav_pane_title: "Gear VR Controller",
    nav_dashboard: "Dashboard",
    nav_calibration: "Calibration",
    nav_settings: "Settings",
    nav_diagnostics: "Diagnostics",

    status_connected: "Connected",
    status_ready: "Controller is ready for input.",
    status_connecting: "Connecting",
    status_negotiating: "Connecting to Bluetooth Low Energy (BLE) device...",
    status_disconnected: "Disconnected",
    status_no_link: "No controller connected.",
    status_error: "Connection Error",

    conn_card_title: "Bluetooth Connection",
    conn_card_desc: "Connect and manage the Samsung Gear VR controller.",
    address_placeholder: "Bluetooth Address (e.g. 2C41A1001234)",
    connect_button: "Connect",
    disconnect_button: "Disconnect",
    scan_button: "Scan for Devices",
    stop_scan_button: "Stop Scan",

    mode_card_title: "Control Mode",
    mode_card_desc: "Select the motion control mode and input behavior.",
    mode_air_mouse: "Air Mouse",
    mode_trackpad: "Touchpad",
    mode_presenter: "Presenter",
    active_prefix: "[ Active: {} ]",

    telemetry_card_title: "Input Telemetry",
    telemetry_card_desc: "Shows sensor data, touch coordinates, and button states.",
    telemetry_awaiting: "Touchpad: Waiting for input...",
    buttons_idle: "Buttons: Trigger: Inactive | Back: Inactive | Home: Inactive",
    timestamp_no_tx: "Timestamp: No data",

    touch_cal_title: "Touchpad Boundary Calibration",
    touch_cal_desc: "Move your thumb across the outer edges of the touchpad to calibrate limits.",
    touch_cal_status: "Normalized range: [-1.0, 1.0] for X and Y axes.",
    imu_cal_title: "Gyroscope Zero-Point Calibration",
    imu_cal_desc: "Put the controller flat on a horizontal surface to remove drift.",
    imu_cal_status: "Start calibration while the controller is still.",
    imu_cal_filter: "Collects 50 samples; offsets apply to this session.",

    language_card_title: "Display Language",
    language_card_desc: "Select the interface language or use Windows system settings.",
    anti_sleep_title: "Prevent Display Sleep",
    anti_sleep_desc: "Keep displays on during Presentation mode.",
    auto_profile_title: "Automatic Mode Switching",
    auto_profile_desc:
        "Switch to Presenter mode when PowerPoint, Keynote, or PDF viewer is active.",
    tray_title: "Minimize to System Tray",
    tray_desc: "Keep Bluetooth connection active in the Windows notification area.",

    imu_diag_title: "Raw IMU Sensor Data",
    imu_diag_desc: "Raw sensor data received from Bluetooth GATT packets.",
    bt_recovery_title: "Bluetooth Diagnostics and Recovery",
    bt_recovery_desc: "Troubleshoot Bluetooth connections and remove inactive device handles.",
    bt_troubleshoot_hint: "If device scan fails, verify pairing status in Windows Settings.",
    open_bt_settings: "Open Windows Bluetooth Settings",
};

// ----------------- 简体中文 (Simplified Chinese) -----------------
pub static ZH_CN_STRINGS: I18nStrings = I18nStrings {
    app_title: "三星 Gear VR 手柄控制器",
    app_subtitle: "Windows 蓝牙输入驱动与动作控制服务",

    nav_pane_title: "Gear VR 控制中心",
    nav_dashboard: "控制面板",
    nav_calibration: "校准中心",
    nav_settings: "系统设置",
    nav_diagnostics: "遥测诊断",

    status_connected: "已连接",
    status_ready: "控制器已就绪，正在接收输入。",
    status_connecting: "正在连接",
    status_negotiating: "正在建立低功耗蓝牙 (BLE) 连接...",
    status_disconnected: "已断开",
    status_no_link: "未连接控制器",
    status_error: "连接错误",

    conn_card_title: "蓝牙设备连接",
    conn_card_desc: "配对并管理三星 Gear VR 控制器连接。",
    address_placeholder: "蓝牙 MAC 地址 (例如 2C41A1001234)",
    connect_button: "连接",
    disconnect_button: "断开",
    scan_button: "搜索设备",
    stop_scan_button: "停止搜索",

    mode_card_title: "输入控制模式",
    mode_card_desc: "选择动作控制模式与按键行为。",
    mode_air_mouse: "空中飞鼠",
    mode_trackpad: "触控板",
    mode_presenter: "演示笔",
    active_prefix: "[ 当前: {} ]",

    telemetry_card_title: "输入数据遥测",
    telemetry_card_desc: "显示触控板坐标、按键状态与传感器数据。",
    telemetry_awaiting: "触控板: 等待输入...",
    buttons_idle: "按键: 扳机键: 未按下 | 返回键: 未按下 | 主页键: 未按下",
    timestamp_no_tx: "时间戳: 无数据",

    touch_cal_title: "触控板边界校准",
    touch_cal_desc: "在触控板外边缘滑动手指，校准传感器范围。",
    touch_cal_status: "归一化范围: X 轴与 Y 轴 [-1.0, 1.0]",
    imu_cal_title: "陀螺仪零点校准",
    imu_cal_desc: "将控制器平放在水平桌面上，消除旋转漂移。",
    imu_cal_status: "保持控制器静止后开始校准。",
    imu_cal_filter: "采集 50 个样本，校准偏移在当前会话生效。",

    language_card_title: "界面语言",
    language_card_desc: "选择界面语言或使用 Windows 系统设置。",
    anti_sleep_title: "屏幕常亮",
    anti_sleep_desc: "在演示模式下保持屏幕常亮，防止息屏。",
    auto_profile_title: "自动模式切换",
    auto_profile_desc: "当 PowerPoint 或 PDF 处于前台时，自动切换到演示模式。",
    tray_title: "最小化到系统托盘",
    tray_desc: "最小化时隐藏到托盘，点击图标恢复；关闭窗口退出。",

    imu_diag_title: "原始 IMU 传感器数据",
    imu_diag_desc: "从蓝牙 GATT 数据包解析的传感器原始数据。",
    bt_recovery_title: "蓝牙诊断与恢复",
    bt_recovery_desc: "清理无效蓝牙连接句柄并重新同步设备。",
    bt_troubleshoot_hint: "如果设备搜索失败，请在 Windows 设置中检查配对状态。",
    open_bt_settings: "打开 Windows 蓝牙设置",
};

// ----------------- 日本語 (Japanese) -----------------
pub static JA_JP_STRINGS: I18nStrings = I18nStrings {
    app_title: "Samsung Gear VR コントローラー",
    app_subtitle: "Windows 用 Bluetooth 入力ドライバーおよびモーション制御サービス",

    nav_pane_title: "Gear VR コントローラー",
    nav_dashboard: "ダッシュボード",
    nav_calibration: "キャリブレーション",
    nav_settings: "設定",
    nav_diagnostics: "診断情報",

    status_connected: "接続完了",
    status_ready: "コントローラーの準備完了。入力を受信しています。",
    status_connecting: "接続中",
    status_negotiating: "Bluetooth Low Energy (BLE) 機器に接続中...",
    status_disconnected: "未接続",
    status_no_link: "コントローラーが接続されていません",
    status_error: "接続エラー",

    conn_card_title: "Bluetooth 接続",
    conn_card_desc: "Samsung Gear VR コントローラーを接続・管理します。",
    address_placeholder: "Bluetooth アドレス (例: 2C41A1001234)",
    connect_button: "接続",
    disconnect_button: "切断",
    scan_button: "デバイス検索",
    stop_scan_button: "検索停止",

    mode_card_title: "制御モード",
    mode_card_desc: "モーション制御モードと入力動作を選択します。",
    mode_air_mouse: "エアマウス",
    mode_trackpad: "タッチパッド",
    mode_presenter: "プレゼンター",
    active_prefix: "[ 有効: {} ]",

    telemetry_card_title: "入力テレメトリ",
    telemetry_card_desc: "センサーデータ、タッチ座標、ボタン状態を表示します。",
    telemetry_awaiting: "タッチパッド: 入力待機中...",
    buttons_idle: "ボタン: トリガー: 未押下 | 戻る: 未押下 | ホーム: 未押下",
    timestamp_no_tx: "タイムスタンプ: データなし",

    touch_cal_title: "タッチパッド境界校正",
    touch_cal_desc: "タッチパッドの外枠を指でなぞり、センサー範囲を校正します。",
    touch_cal_status: "正規化範囲: X 軸・Y 軸ともに [-1.0, 1.0]",
    imu_cal_title: "ジャイロスコープ零点校正",
    imu_cal_desc: "コントローラーを水平な場所に置き、回転ドリフトを除去します。",
    imu_cal_status: "コントローラーを静止して校正を開始してください。",
    imu_cal_filter: "50 サンプルを収集し、このセッションに適用します。",

    language_card_title: "表示言語",
    language_card_desc: "表示言語を選択するか Windows の設定を使用します。",
    anti_sleep_title: "画面スリープ防止",
    anti_sleep_desc: "プレゼンテーションモード中、画面オフを防止します。",
    auto_profile_title: "自動モード切り替え",
    auto_profile_desc: "PowerPoint または PDF がアクティブな時、プレゼンターモードに切り替えます。",
    tray_title: "システムトレイに最小化",
    tray_desc: "最小化でトレイに隠し、クリックで復元。閉じると終了します。",

    imu_diag_title: "IMU 生センサーデータ",
    imu_diag_desc: "Bluetooth GATT パケットからデコードされた生センサーデータ。",
    bt_recovery_title: "Bluetooth 診断と復旧",
    bt_recovery_desc: "無効な Bluetooth 接続を解除し、デバイスを再同期します。",
    bt_troubleshoot_hint:
        "デバイス検索に失敗する場合は、Windows 設定でペアリング状態を確認してください。",
    open_bt_settings: "Windows Bluetooth 設定を開く",
};

// ----------------- 한국어 (Korean) -----------------
pub static KO_KR_STRINGS: I18nStrings = I18nStrings {
    app_title: "삼성 기어 VR 컨트롤러",
    app_subtitle: "Windows용 블루투스 입력 드라이버 및 모션 제어 서비스",

    nav_pane_title: "기어 VR 제어 센터",
    nav_dashboard: "대시보드",
    nav_calibration: "보정 센터",
    nav_settings: "설정",
    nav_diagnostics: "원격 진단",

    status_connected: "연결됨",
    status_ready: "컨트롤러 준비 완료. 입력을 수신 중입니다.",
    status_connecting: "연결 중",
    status_negotiating: "저전력 블루투스(BLE) 장치에 연결 중...",
    status_disconnected: "연결 끊김",
    status_no_link: "연결된 컨트롤러 없음",
    status_error: "연결 오류",

    conn_card_title: "블루투스 연결",
    conn_card_desc: "삼성 기어 VR 컨트롤러 연결 및 관리.",
    address_placeholder: "블루투스 MAC 주소 (예: 2C41A1001234)",
    connect_button: "연결",
    disconnect_button: "연결 끊기",
    scan_button: "장치 검색",
    stop_scan_button: "검색 중지",

    mode_card_title: "제어 모드",
    mode_card_desc: "모션 제어 모드 및 입력 동작 선택.",
    mode_air_mouse: "에어 마우스",
    mode_trackpad: "터치패드",
    mode_presenter: "프레젠터",
    active_prefix: "[ 활성: {} ]",

    telemetry_card_title: "입력 텔레메트리",
    telemetry_card_desc: "센서 데이터, 터치 좌표 및 버튼 상태 표시.",
    telemetry_awaiting: "터치패드: 입력 대기 중...",
    buttons_idle: "버튼: 트리거: 해제됨 | 뒤로: 해제됨 | 홈: 해제됨",
    timestamp_no_tx: "타임스탬프: 데이터 없음",

    touch_cal_title: "터치패드 경계 보정",
    touch_cal_desc: "터치패드 바깥 가장자리를 문질러 센서 범위를 보정합니다.",
    touch_cal_status: "정규화 범위: X 축 및 Y 축 [-1.0, 1.0]",
    imu_cal_title: "자이로스코프 영점 보정",
    imu_cal_desc: "회전 드리프트를 제거하기 위해 컨트롤러를 수평 바닥에 평평하게 놓으십시오.",
    imu_cal_status: "컨트롤러를 움직이지 않고 보정을 시작하세요.",
    imu_cal_filter: "50개 샘플을 수집하여 현재 세션에 적용합니다.",

    language_card_title: "표시 언어",
    language_card_desc: "인터페이스 언어를 선택하거나 Windows 시스템 설정을 사용합니다.",
    anti_sleep_title: "화면 절전 방지",
    anti_sleep_desc: "프레젠테이션 모드 중 화면을 계속 켭니다.",
    auto_profile_title: "자동 모드 전환",
    auto_profile_desc: "PowerPoint 또는 PDF가 활성화되면 프레젠터 모드로 자동 전환합니다.",
    tray_title: "시스템 트레이로 최소화",
    tray_desc: "최소화 시 트레이에 숨기고 클릭하면 복원합니다. 닫으면 종료합니다.",

    imu_diag_title: "원시 IMU 센서 데이터",
    imu_diag_desc: "블루투스 GATT 패킷에서 디코딩된 원시 센서 데이터.",
    bt_recovery_title: "블루투스 진단 및 복구",
    bt_recovery_desc: "비활성 블루투스 핸들을 해제하고 장치를 다시 동기화합니다.",
    bt_troubleshoot_hint: "장치 검색에 실패하면 Windows 설정에서 페어링 상태를 확인하십시오.",
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
        assert!(Language::SimplifiedChinese
            .display_name()
            .contains("简体中文"));
        assert!(Language::English.display_name().contains("English"));
        assert!(Language::Japanese.display_name().contains("日本語"));
        assert!(Language::Korean.display_name().contains("한국어"));
    }
}
