using System;
using System.Threading.Tasks;
using Windows.Storage;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class LocalSettingsService : ISettingsService
    {
        private const string MOUSE_SENSITIVITY_KEY = "MouseSensitivity";
        private const string IS_MOUSE_ENABLED_KEY = "IsMouseEnabled";
        private const string IS_KEYBOARD_ENABLED_KEY = "IsKeyboardEnabled";
        private const string IS_CONTROL_ENABLED_KEY = "IsControlEnabled";

        private readonly ApplicationDataContainer _localSettings;

        public LocalSettingsService()
        {
            _localSettings = ApplicationData.Current.LocalSettings;
            LoadDefaultSettings();
        }

        public double MouseSensitivity
        {
            get => GetSetting(MOUSE_SENSITIVITY_KEY, 1.0);
            set => SaveSetting(MOUSE_SENSITIVITY_KEY, value);
        }

        public bool IsMouseEnabled
        {
            get => GetSetting(IS_MOUSE_ENABLED_KEY, true);
            set => SaveSetting(IS_MOUSE_ENABLED_KEY, value);
        }

        public bool IsKeyboardEnabled
        {
            get => GetSetting(IS_KEYBOARD_ENABLED_KEY, true);
            set => SaveSetting(IS_KEYBOARD_ENABLED_KEY, value);
        }

        public bool IsControlEnabled
        {
            get => GetSetting(IS_CONTROL_ENABLED_KEY, true);
            set => SaveSetting(IS_CONTROL_ENABLED_KEY, value);
        }

        public Task LoadSettingsAsync()
        {
            // 设置已经在属性访问器中加载
            return Task.CompletedTask;
        }

        public Task SaveSettingsAsync()
        {
            // 设置已经在属性访问器中保存
            return Task.CompletedTask;
        }

        public void ResetToDefaults()
        {
            MouseSensitivity = 1.0;
            IsMouseEnabled = true;
            IsKeyboardEnabled = true;
            IsControlEnabled = true;
        }

        private void LoadDefaultSettings()
        {
            if (!_localSettings.Values.ContainsKey(MOUSE_SENSITIVITY_KEY))
            {
                ResetToDefaults();
            }
        }

        private T GetSetting<T>(string key, T defaultValue)
        {
            if (_localSettings.Values.TryGetValue(key, out object? value))
            {
                if (value is T typedValue)
                {
                    return typedValue;
                }
            }
            return defaultValue;
        }

        private void SaveSetting<T>(string key, T value)
        {
            _localSettings.Values[key] = value;
        }
    }
}