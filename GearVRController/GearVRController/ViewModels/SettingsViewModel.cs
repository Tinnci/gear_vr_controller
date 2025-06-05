using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Collections.Generic;
using GearVRController.Services.Interfaces;
using GearVRController.Events;

namespace GearVRController.ViewModels
{
    public class SettingsViewModel : INotifyPropertyChanged
    {
        private readonly ISettingsService _settingsService;
        private readonly IEventAggregator _eventAggregator;

        public SettingsViewModel(ISettingsService settingsService, IEventAggregator eventAggregator)
        {
            _settingsService = settingsService;
            _eventAggregator = eventAggregator;
            LoadSettings();
        }

        // Properties for settings will go here (e.g., MouseSensitivity, IsGestureMode, etc.)
        public double MouseSensitivity
        {
            get => _settingsService.MouseSensitivity;
            set
            {
                if (_settingsService.MouseSensitivity != value)
                {
                    _settingsService.MouseSensitivity = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool IsMouseEnabled
        {
            get => _settingsService.IsMouseEnabled;
            set
            {
                if (_settingsService.IsMouseEnabled != value)
                {
                    _settingsService.IsMouseEnabled = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool IsKeyboardEnabled
        {
            get => _settingsService.IsKeyboardEnabled;
            set
            {
                if (_settingsService.IsKeyboardEnabled != value)
                {
                    _settingsService.IsKeyboardEnabled = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool IsControlEnabled
        {
            get => _settingsService.IsControlEnabled;
            set
            {
                if (_settingsService.IsControlEnabled != value)
                {
                    _settingsService.IsControlEnabled = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool UseNaturalScrolling
        {
            get => _settingsService.UseNaturalScrolling;
            set
            {
                if (_settingsService.UseNaturalScrolling != value)
                {
                    _settingsService.UseNaturalScrolling = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool InvertYAxis
        {
            get => _settingsService.InvertYAxis;
            set
            {
                if (_settingsService.InvertYAxis != value)
                {
                    _settingsService.InvertYAxis = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool EnableSmoothing
        {
            get => _settingsService.EnableSmoothing;
            set
            {
                if (_settingsService.EnableSmoothing != value)
                {
                    _settingsService.EnableSmoothing = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public int SmoothingLevel
        {
            get => _settingsService.SmoothingLevel;
            set
            {
                if (_settingsService.SmoothingLevel != value)
                {
                    _settingsService.SmoothingLevel = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool EnableNonLinearCurve
        {
            get => _settingsService.EnableNonLinearCurve;
            set
            {
                if (_settingsService.EnableNonLinearCurve != value)
                {
                    _settingsService.EnableNonLinearCurve = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public double NonLinearCurvePower
        {
            get => _settingsService.NonLinearCurvePower;
            set
            {
                if (_settingsService.NonLinearCurvePower != value)
                {
                    _settingsService.NonLinearCurvePower = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public double DeadZone
        {
            get => _settingsService.DeadZone;
            set
            {
                if (_settingsService.DeadZone != value)
                {
                    _settingsService.DeadZone = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool IsGestureMode
        {
            get => _settingsService.IsGestureMode;
            set
            {
                if (_settingsService.IsGestureMode != value)
                {
                    _settingsService.IsGestureMode = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public float GestureSensitivity
        {
            get => _settingsService.GestureSensitivity;
            set
            {
                if (_settingsService.GestureSensitivity != value)
                {
                    _settingsService.GestureSensitivity = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool ShowGestureHints
        {
            get => _settingsService.ShowGestureHints;
            set
            {
                if (_settingsService.ShowGestureHints != value)
                {
                    _settingsService.ShowGestureHints = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public bool ShowTouchpadVisualizer
        {
            get => _settingsService.ShowTouchpadVisualizer;
            set
            {
                if (_settingsService.ShowTouchpadVisualizer != value)
                {
                    _settingsService.ShowTouchpadVisualizer = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public int ButtonDebounceThreshold
        {
            get => _settingsService.ButtonDebounceThreshold;
            set
            {
                if (_settingsService.ButtonDebounceThreshold != value)
                {
                    _settingsService.ButtonDebounceThreshold = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public Enums.GestureAction SwipeUpAction
        {
            get => _settingsService.SwipeUpAction;
            set
            {
                if (_settingsService.SwipeUpAction != value)
                {
                    _settingsService.SwipeUpAction = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public Enums.GestureAction SwipeDownAction
        {
            get => _settingsService.SwipeDownAction;
            set
            {
                if (_settingsService.SwipeDownAction != value)
                {
                    _settingsService.SwipeDownAction = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public Enums.GestureAction SwipeLeftAction
        {
            get => _settingsService.SwipeLeftAction;
            set
            {
                if (_settingsService.SwipeLeftAction != value)
                {
                    _settingsService.SwipeLeftAction = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public Enums.GestureAction SwipeRightAction
        {
            get => _settingsService.SwipeRightAction;
            set
            {
                if (_settingsService.SwipeRightAction != value)
                {
                    _settingsService.SwipeRightAction = value;
                    OnPropertyChanged();
                    _eventAggregator.Publish(new SettingsChangedEvent());
                }
            }
        }

        public IEnumerable<Enums.GestureAction> AvailableGestureActions => Enum.GetValues<Enums.GestureAction>();

        public void ResetSettings()
        {
            _settingsService.ResetToDefaults();
            LoadSettings(); // Reload settings to update UI
            _eventAggregator.Publish(new SettingsChangedEvent());
        }

        private void LoadSettings()
        {
            // Trigger all property changes to update UI with current settings values
            OnPropertyChanged(nameof(MouseSensitivity));
            OnPropertyChanged(nameof(IsMouseEnabled));
            OnPropertyChanged(nameof(IsKeyboardEnabled));
            OnPropertyChanged(nameof(IsControlEnabled));
            OnPropertyChanged(nameof(UseNaturalScrolling));
            OnPropertyChanged(nameof(InvertYAxis));
            OnPropertyChanged(nameof(EnableSmoothing));
            OnPropertyChanged(nameof(SmoothingLevel));
            OnPropertyChanged(nameof(EnableNonLinearCurve));
            OnPropertyChanged(nameof(NonLinearCurvePower));
            OnPropertyChanged(nameof(DeadZone));
            OnPropertyChanged(nameof(IsGestureMode));
            OnPropertyChanged(nameof(GestureSensitivity));
            OnPropertyChanged(nameof(ShowGestureHints));
            OnPropertyChanged(nameof(ShowTouchpadVisualizer));
            OnPropertyChanged(nameof(ButtonDebounceThreshold));
            OnPropertyChanged(nameof(SwipeUpAction));
            OnPropertyChanged(nameof(SwipeDownAction));
            OnPropertyChanged(nameof(SwipeLeftAction));
            OnPropertyChanged(nameof(SwipeRightAction));
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}