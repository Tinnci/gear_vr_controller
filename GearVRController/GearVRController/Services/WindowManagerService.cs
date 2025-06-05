using System;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.UI.Xaml;
using GearVRController.ViewModels;
using GearVRController.Views;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class WindowManagerService : IWindowManagerService
    {
        private readonly IServiceProvider _serviceProvider;
        private TouchpadCalibrationWindow? _calibrationWindow;
        private TouchpadVisualizerWindow? _visualizerWindow;

        public WindowManagerService(IServiceProvider serviceProvider)
        {
            _serviceProvider = serviceProvider;
        }

        public void OpenTouchpadCalibrationWindow()
        {
            if (_calibrationWindow == null)
            {
                // Resolve TouchpadCalibrationViewModel and the window from DI
                var calibrationViewModel = _serviceProvider.GetRequiredService<TouchpadCalibrationViewModel>();
                _calibrationWindow = new TouchpadCalibrationWindow(calibrationViewModel);
                _calibrationWindow.Closed += (s, e) => _calibrationWindow = null;
                _calibrationWindow.Activate();
            }
            else
            {
                _calibrationWindow.Activate();
            }
        }

        public void CloseTouchpadCalibrationWindow()
        {
            _calibrationWindow?.Close();
        }

        public void OpenTouchpadVisualizerWindow()
        {
            if (_visualizerWindow == null)
            {
                // Pass MainViewModel to the visualizer window as it needs direct access to processed data
                var mainViewModel = _serviceProvider.GetRequiredService<MainViewModel>();
                _visualizerWindow = new TouchpadVisualizerWindow(mainViewModel);
                _visualizerWindow.Closed += (s, e) => _visualizerWindow = null;
                _visualizerWindow.Activate();
            }
            else
            {
                _visualizerWindow.Activate();
            }
        }

        public void CloseTouchpadVisualizerWindow()
        {
            _visualizerWindow?.Close();
        }
    }
}