using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Windows.ApplicationModel;
using Windows.ApplicationModel.Activation;
using Windows.Foundation;
using Windows.Foundation.Collections;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;
using Microsoft.UI.Xaml.Shapes;
using Microsoft.UI.Dispatching;
using GearVRController.Services;
using GearVRController.Services.Interfaces;
using GearVRController.ViewModels;
using Microsoft.Extensions.DependencyInjection;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace GearVRController
{
    /// <summary>
    /// Provides application-specific behavior to supplement the default Application class.
    /// </summary>
    public partial class App : Application
    {
        public static IServiceProvider? ServiceProvider { get; private set; }
        /// <summary>
        /// Initializes the singleton application object.  This is the first line of authored code
        /// executed, and as such is the logical equivalent of main() or WinMain().
        /// </summary>
        public App()
        {
            this.InitializeComponent();
            ConfigureServices();
        }

        private void ConfigureServices()
        {
            var services = new ServiceCollection();

            // Register DispatcherQueue as a singleton
            services.AddSingleton(DispatcherQueue.GetForCurrentThread());

            // Register Services
            services.AddSingleton<IBluetoothService, BluetoothService>();
            services.AddSingleton<IInputSimulator, WindowsInputSimulator>();
            services.AddSingleton<ISettingsService, LocalSettingsService>();
            services.AddSingleton<IControllerService, ControllerService>();
            services.AddSingleton<TouchpadProcessor>();
            services.AddSingleton<GestureRecognizer>();
            services.AddSingleton<IInputStateMonitorService, InputStateMonitorService>();
            services.AddSingleton<IEventAggregator, EventAggregator>();
            services.AddSingleton<IWindowManagerService, WindowManagerService>();
            services.AddSingleton<IActionExecutionService, ActionExecutionService>();

            // Register ViewModels as transient (or singleton if their state needs to persist globally)
            services.AddSingleton<MainViewModel>();
            services.AddTransient<TouchpadCalibrationViewModel>();

            // Register Views (MainWindow, etc.) as transient or singleton as needed
            services.AddSingleton<MainWindow>();

            ServiceProvider = services.BuildServiceProvider();
        }

        /// <summary>
        /// Invoked when the application is launched.
        /// </summary>
        /// <param name="args">Details about the launch request and process.</param>
        protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
        {
            m_window = ServiceProvider!.GetRequiredService<MainWindow>();
            m_window.Activate();
        }

        private Window? m_window;
    }
}
