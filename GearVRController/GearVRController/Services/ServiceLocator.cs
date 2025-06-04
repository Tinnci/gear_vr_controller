using System;
using Microsoft.Extensions.DependencyInjection;
using GearVRController.Services.Interfaces;
using GearVRController.ViewModels;
using Microsoft.UI.Dispatching;

namespace GearVRController.Services
{
    public static class ServiceLocator
    {
        private static IServiceProvider? _serviceProvider;

        public static void Initialize()
        {
            var services = new ServiceCollection();

            // 注册服务
            services.AddSingleton<IBluetoothService, BluetoothService>();
            services.AddSingleton<IControllerService, ControllerService>();
            services.AddSingleton<IInputSimulator, WindowsInputSimulator>();
            services.AddSingleton<ISettingsService, LocalSettingsService>();
            services.AddSingleton<TouchpadProcessor>();
            services.AddSingleton<RotationProcessor>();

            // 注册 DispatcherQueue
            services.AddSingleton(DispatcherQueue.GetForCurrentThread());

            // 注册 ViewModel
            services.AddTransient<MainViewModel>();

            _serviceProvider = services.BuildServiceProvider();
        }

        public static T GetService<T>() where T : class
        {
            if (_serviceProvider == null)
            {
                throw new InvalidOperationException("ServiceLocator has not been initialized.");
            }

            var service = _serviceProvider.GetRequiredService<T>();
            return service;
        }
    }
}