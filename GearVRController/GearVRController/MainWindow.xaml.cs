using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Windows.Foundation;
using Windows.Foundation.Collections;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;
using GearVRController.ViewModels;
using GearVRController.Views;
using GearVRController.Models;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace GearVRController
{
    /// <summary>
    /// An empty window that can be used on its own or navigated to within a Frame.
    /// </summary>
    public sealed partial class MainWindow : Window
    {
        public MainViewModel ViewModel { get; }
        private TouchpadCalibrationWindow? _calibrationWindow;

        public MainWindow()
        {
            ViewModel = new MainViewModel();
            this.InitializeComponent();
            // 设置Content的DataContext
            if (Content is FrameworkElement rootElement)
            {
                rootElement.DataContext = ViewModel;
            }

            // 订阅控制器数据更新事件
            ViewModel.ControllerDataReceived += ViewModel_ControllerDataReceived;
            // 订阅自动校准事件
            ViewModel.AutoCalibrationRequired += ViewModel_AutoCalibrationRequired;
        }

        private void ViewModel_ControllerDataReceived(object? sender, ControllerData data)
        {
            // 如果校准窗口打开，发送数据给校准窗口
            _calibrationWindow?.ProcessControllerData(data);
        }

        private void ViewModel_AutoCalibrationRequired(object? sender, EventArgs e)
        {
            // 在UI线程上执行
            DispatcherQueue.TryEnqueue(() =>
            {
                if (_calibrationWindow != null)
                {
                    _calibrationWindow.Close();
                }

                _calibrationWindow = new TouchpadCalibrationWindow();
                _calibrationWindow.CalibrationCompleted += (s, data) =>
                {
                    ViewModel.ApplyCalibrationData(data);
                    ViewModel.EndCalibration();
                    _calibrationWindow = null;
                };
                _calibrationWindow.Closed += (s, e) =>
                {
                    ViewModel.EndCalibration();
                    _calibrationWindow = null;
                };
                ViewModel.StartAutoCalibration();
                _calibrationWindow.Activate();
            });
        }

        private async void ConnectButton_Click(object sender, RoutedEventArgs e)
        {
            // 这里使用硬编码的MAC地址，实际应用中应该通过蓝牙设备扫描获取
            ulong deviceAddress = 49180499202480; // MAC地址：2C:BA:BA:25:6A:A1
            await ViewModel.ConnectAsync(deviceAddress);
        }

        private void DisconnectButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel.Disconnect();
        }

        private void ToggleControlButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel.ToggleControl();
        }

        private void ResetSettingsButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel.ResetSettings();
        }

        private void CalibrateButton_Click(object sender, RoutedEventArgs e)
        {
            if (_calibrationWindow != null)
            {
                _calibrationWindow.Close();
            }

            _calibrationWindow = new TouchpadCalibrationWindow();
            _calibrationWindow.CalibrationCompleted += (s, data) =>
            {
                ViewModel.ApplyCalibrationData(data);
                ViewModel.EndCalibration();
                _calibrationWindow = null;
            };
            _calibrationWindow.Closed += (s, e) =>
            {
                ViewModel.EndCalibration();
                _calibrationWindow = null;
            };
            ViewModel.StartManualCalibration();
            _calibrationWindow.Activate();
        }
    }
}
