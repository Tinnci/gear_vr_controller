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
using Microsoft.Extensions.DependencyInjection;
using GearVRController.Services.Interfaces;

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
        private AppWindow _appWindow;
        private readonly IWindowManagerService _windowManagerService;
        private readonly ISettingsService _settingsService;

        public MainWindow(MainViewModel viewModel, IWindowManagerService windowManagerService, ISettingsService settingsService)
        {
            ViewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));
            _windowManagerService = windowManagerService;
            _settingsService = settingsService;

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

            // 初始导航到主页
            MainNavigationView.SelectedItem = MainNavigationView.MenuItems.OfType<NavigationViewItem>().First();
        }

        private async void ConnectButton_Click(object sender, RoutedEventArgs e)
        {
            try
            {
                // 从设置服务获取已知的Gear VR控制器MAC地址
                List<ulong> knownAddresses = _settingsService.KnownBluetoothAddresses;

                // 尝试连接到每个已知地址
                foreach (var address in knownAddresses)
                {
                    try
                    {
                        ViewModel.StatusMessage = $"正在尝试连接到设备 {address.ToString("X")}";
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

        private void MainNavigationView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
        {
            if (args.IsSettingsSelected)
            {
                // 设置页面（如果将来有专门的设置页面）
                // ContentFrame.Navigate(typeof(SettingsPage)); // 目前我们已经在主设置页面中处理了
            }
            else if (args.SelectedItem is NavigationViewItem selectedItem)
            {
                switch (selectedItem.Tag)
                {
                    case "home":
                        ContentFrame.Navigate(typeof(HomePage), ViewModel);
                        break;
                    case "settings":
                        ContentFrame.Navigate(typeof(SettingsPage), ViewModel);
                        break;
                    case "calibrate":
                        ContentFrame.Navigate(typeof(CalibrationPage), ViewModel);
                        break;
                    case "about":
                        ContentFrame.Navigate(typeof(AboutPage), ViewModel);
                        break;
                    case "resetSettings":
                        ViewModel.ResetSettings();
                        break;
                    case "visualizer":
                        _windowManagerService.OpenTouchpadVisualizerWindow();
                        break;
                }
            }
        }
    }
}
