using System;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.UI.Xaml;
using GearVRController.ViewModels;
using GearVRController.Views;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    /// <summary>
    /// 管理应用程序中的窗口打开和关闭。
    /// </summary>
    public class WindowManagerService : IWindowManagerService
    {
        /// <summary>
        /// 用于解析服务和视图模型的服务提供程序。
        /// </summary>
        private readonly IServiceProvider _serviceProvider;
        /// <summary>
        /// 触控板校准窗口实例。如果窗口未打开，则为 null。
        /// </summary>
        private TouchpadCalibrationWindow? _calibrationWindow;
        /// <summary>
        /// 触控板可视化工具窗口实例。如果窗口未打开，则为 null。
        /// </summary>
        private TouchpadVisualizerWindow? _visualizerWindow;

        /// <summary>
        /// 初始化 WindowManagerService 类的新实例。
        /// </summary>
        /// <param name="serviceProvider">用于解析服务和视图模型的服务提供程序。</param>
        public WindowManagerService(IServiceProvider serviceProvider)
        {
            _serviceProvider = serviceProvider;
        }

        /// <summary>
        /// 打开触控板校准窗口。如果窗口已打开，则激活现有窗口。
        /// </summary>
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

        /// <summary>
        /// 关闭触控板校准窗口。
        /// </summary>
        public void CloseTouchpadCalibrationWindow()
        {
            _calibrationWindow?.Close();
        }

        /// <summary>
        /// 打开触控板可视化工具窗口。如果窗口已打开，则激活现有窗口。
        /// </summary>
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

        /// <summary>
        /// 关闭触控板可视化工具窗口。
        /// </summary>
        public void CloseTouchpadVisualizerWindow()
        {
            _visualizerWindow?.Close();
        }
    }
}