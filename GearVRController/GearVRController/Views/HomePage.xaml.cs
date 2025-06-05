using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using GearVRController.ViewModels;
using GearVRController.Services.Interfaces;
using Microsoft.Extensions.DependencyInjection;

namespace GearVRController.Views
{
    public sealed partial class HomePage : Page
    {
        public MainViewModel? ViewModel { get; set; }
        private IWindowManagerService? _windowManagerService;

        public HomePage()
        {
            this.InitializeComponent();
            // ViewModel 应该通过 OnNavigatedTo 传递，而不是在构造函数中直接获取
            // _windowManagerService 也应通过依赖注入获取，或在 OnNavigatedTo 中设置
        }

        protected override void OnNavigatedTo(NavigationEventArgs e)
        {
            base.OnNavigatedTo(e);
            if (e.Parameter is MainViewModel viewModel)
            {
                ViewModel = viewModel;
                this.DataContext = ViewModel;
            }
            _windowManagerService = App.ServiceProvider!.GetRequiredService<IWindowManagerService>();
        }

        private void ToggleControlButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel?.ToggleControl();
        }
    }
}