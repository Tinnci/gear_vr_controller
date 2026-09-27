//! Language preferences and automatic Windows UI language detection.

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

#[cfg(test)]
mod tests {
    use super::*;

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
