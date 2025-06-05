using System;
using Microsoft.UI.Xaml;
using Microsoft.UI.Dispatching;
using GearVRController.Services;
using GearVRController.Services.Interfaces;
using GearVRController.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using System.Diagnostics;

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
            this.UnhandledException += App_UnhandledException;
            ConfigureServices();
        }

        private void App_UnhandledException(object sender, Microsoft.UI.Xaml.UnhandledExceptionEventArgs e)
        {
            // Log the exception details
            Debug.WriteLine($"Unhandled Exception: {e.Message}");
            if (e.Exception != null)
            {
                Debug.WriteLine($"Exception Type: {e.Exception.GetType().FullName}");
                Debug.WriteLine($"Stack Trace: {e.Exception.StackTrace}");
                if (e.Exception.InnerException != null)
                {
                    Debug.WriteLine($"Inner Exception: {e.Exception.InnerException.Message}");
                    Debug.WriteLine($"Inner Exception Stack Trace: {e.Exception.InnerException.StackTrace}");
                }
            }
            e.Handled = true; // Mark the exception as handled to prevent app crash
        }

        private void ConfigureServices()
        {
            var services = new ServiceCollection();

            // Register DispatcherQueue as a singleton
            services.AddSingleton(DispatcherQueue.GetForCurrentThread());

            // Register Services
            services.AddSingleton<IBluetoothService, BluetoothService>();
            services.AddSingleton<WindowsInputSimulator>();
            services.AddSingleton<IInputStateMonitorService>(provider =>
                new InputStateMonitorService(
                    provider.GetRequiredService<WindowsInputSimulator>(),
                    provider.GetRequiredService<DispatcherQueue>(),
                    provider.GetRequiredService<ILogger>()
                )
            );
            services.AddSingleton<IInputSimulator>(provider =>
                new MonitoredInputSimulator(
                    provider.GetRequiredService<WindowsInputSimulator>(),
                    provider.GetRequiredService<IInputStateMonitorService>()
                )
            );
            services.AddSingleton<ISettingsService, LocalSettingsService>();
            services.AddSingleton<IControllerService, ControllerService>();
            services.AddSingleton<TouchpadProcessor>();
            services.AddSingleton<GestureRecognizer>();
            services.AddSingleton<IEventAggregator, EventAggregator>();
            services.AddSingleton<IWindowManagerService, WindowManagerService>();
            services.AddSingleton<IActionExecutionService, ActionExecutionService>();
            services.AddSingleton<ILogger, Logger>();
            services.AddSingleton<IInputHandlerService, InputHandlerService>();

            // Register ViewModels as transient (or singleton if their state needs to persist globally)
            services.AddSingleton<MainViewModel>();
            services.AddTransient<TouchpadCalibrationViewModel>();

            // Register Views (MainWindow, etc.) as transient or singleton as needed
            services.AddSingleton<MainWindow>();
            services.AddTransient<Views.HomePage>();
            services.AddTransient<Views.SettingsPage>();
            services.AddTransient<Views.CalibrationPage>();
            services.AddTransient<Views.AboutPage>();
            services.AddTransient<Views.TouchpadVisualizerPage>();

            ServiceProvider = services.BuildServiceProvider();
        }

        /// <summary>
        /// Invoked when the application is launched.
        /// </summary>
        /// <param name="args">Details about the launch request and process.</param>
        protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
        {
            try
            {
                Debug.WriteLine("App.OnLaunched: Creating MainWindow...");
                m_window = ServiceProvider!.GetRequiredService<MainWindow>();
                Debug.WriteLine("App.OnLaunched: Activating MainWindow...");
                m_window.Activate();
                Debug.WriteLine("App.OnLaunched: MainWindow activated.");
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"[FATAL ERROR] Failed to create or activate MainWindow. Exception: {ex.ToString()}");
                // You might want to set a breakpoint here in a real debugging scenario
                // Debugger.Break();
            }
        }

        private Window? m_window;
    }
}
