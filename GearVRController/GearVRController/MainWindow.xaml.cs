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
using GearVRController.Services;
using EnumsNS = GearVRController.Enums; // 添加命名空间别名
using Microsoft.UI.Windowing;
using Microsoft.UI;
using Windows.Graphics;

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
        private Views.TouchpadVisualizerWindow? _touchpadVisualizerWindow;
        private AppWindow _appWindow;

        public MainWindow()
        {
            // 初始化服务定位器
            ServiceLocator.Initialize();

            // 获取ViewModel实例
            ViewModel = ServiceLocator.GetService<MainViewModel>();

            this.InitializeComponent();

            // 设置DataContext
            if (Content is FrameworkElement rootElement)
            {
                rootElement.DataContext = ViewModel;
            }

            // 设置窗口大小
            IntPtr hwnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
            WindowId windowId = Win32Interop.GetWindowIdFromWindow(hwnd);
            _appWindow = AppWindow.GetFromWindowId(windowId);
            
            // 设置默认窗口大小
            _appWindow.Resize(new SizeInt32(900, 700));

            // 订阅控制器数据更新事件
            ViewModel.ControllerDataReceived += ViewModel_ControllerDataReceived;
            // 订阅自动校准事件
            ViewModel.AutoCalibrationRequired += ViewModel_AutoCalibrationRequired;
        }

        private void ViewModel_ControllerDataReceived(object? sender, ControllerData data)
        {
            // 如果校准窗口打开，发送数据给校准窗口
            _calibrationWindow?.ProcessControllerData(data);
            
            // 如果触摸板可视化窗口打开，发送数据给触摸板可视化窗口
            if (_touchpadVisualizerWindow != null)
            {
                // 应用校准（复制MainViewModel中的逻辑）
                double normalizedX = 0;
                double normalizedY = 0;
                
                if (ViewModel.CalibrationData != null)
                {
                    // 计算相对于中心点的偏移
                    double deltaX = data.AxisX - ViewModel.CalibrationData.CenterX;
                    double deltaY = data.AxisY - ViewModel.CalibrationData.CenterY;
                    
                    // 计算归一化系数
                    double xScale = deltaX > 0 ?
                        Math.Max(10, ViewModel.CalibrationData.MaxX - ViewModel.CalibrationData.CenterX) :
                        Math.Max(10, ViewModel.CalibrationData.CenterX - ViewModel.CalibrationData.MinX);

                    double yScale = deltaY > 0 ?
                        Math.Max(10, ViewModel.CalibrationData.MaxY - ViewModel.CalibrationData.CenterY) :
                        Math.Max(10, ViewModel.CalibrationData.CenterY - ViewModel.CalibrationData.MinY);
                        
                    // 归一化坐标
                    normalizedX = Math.Max(-1.0, Math.Min(1.0, deltaX / xScale));
                    normalizedY = Math.Max(-1.0, Math.Min(1.0, deltaY / yScale));
                }
                else
                {
                    // 如果没有校准数据，使用简单的归一化方法
                    normalizedX = Math.Max(-1.0, Math.Min(1.0, (data.AxisX - 500) / 500.0));
                    normalizedY = Math.Max(-1.0, Math.Min(1.0, (data.AxisY - 500) / 500.0));
                }
                
                // 发送数据到可视化窗口
                try
                {
                    _touchpadVisualizerWindow.ProcessTouchpadData(normalizedX, normalizedY, data.TouchpadButton, ViewModel.LastGesture);
                }
                catch (Exception ex)
                {
                    // 如果ProcessTouchpadData方法不存在，尝试使用UpdateTouchpadData方法
                    System.Diagnostics.Debug.WriteLine($"触摸板可视化数据发送错误: {ex.Message}");
                    _touchpadVisualizerWindow.UpdateTouchpadData(normalizedX, normalizedY, data.TouchpadButton, ViewModel.LastGesture);
                }
            }
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
            try
            {
                // 尝试连接到已知的Gear VR控制器MAC地址
                // 这里提供几个常见的Gear VR控制器MAC地址
                ulong[] knownAddresses = new ulong[]
                {
                    49180499202480, // 2C:BA:BA:25:6A:A1
                    49180499202481, // 2C:BA:BA:25:6A:A2 (可能的变体)
                    49180499202482, // 2C:BA:BA:25:6A:A3 (可能的变体)
                    // 可以添加更多已知地址
                };

                // 尝试连接到每个已知地址
                foreach (var address in knownAddresses)
                {
                    try
                    {
                        ViewModel.StatusMessage = $"正在尝试连接到设备 {address.ToString("X")}...";
                        await ViewModel.ConnectAsync(address);
                        
                        // 如果连接成功，跳出循环
                        if (ViewModel.IsConnected)
                        {
                            ViewModel.StatusMessage = $"已连接到设备 {address.ToString("X")}";
                            return;
                        }
                    }
                    catch (Exception ex)
                    {
                        // 记录错误但继续尝试下一个地址
                        System.Diagnostics.Debug.WriteLine($"连接到 {address.ToString("X")} 失败: {ex.Message}");
                    }
                }

                // 如果所有已知地址都连接失败，显示错误消息
                ViewModel.StatusMessage = "无法连接到任何已知设备，请确保设备已开启并在范围内";
            }
            catch (Exception ex)
            {
                ViewModel.StatusMessage = $"连接错误: {ex.Message}";
            }
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

            // 激活窗口并自动开始校准
            _calibrationWindow.Activate();
            ViewModel.StartManualCalibration();
        }

        private void TouchpadVisualizerButton_Click(object sender, RoutedEventArgs e)
        {
            // 如果窗口已经打开，激活它
            if (_touchpadVisualizerWindow != null)
            {
                _touchpadVisualizerWindow.Activate();
                return;
            }

            // 创建新的触摸板可视化窗口
            _touchpadVisualizerWindow = new Views.TouchpadVisualizerWindow();
            _touchpadVisualizerWindow.Closed += (s, e) =>
            {
                _touchpadVisualizerWindow = null;
            };

            // 激活窗口
            _touchpadVisualizerWindow.Activate();
        }
    }
}
