using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using GearVRController.ViewModels;
using GearVRController.Services.Interfaces;
using Microsoft.Extensions.DependencyInjection;

namespace GearVRController.Views
{
    public sealed partial class CalibrationPage : Page
    {
        public MainViewModel? ViewModel { get; set; }
        private IWindowManagerService? _windowManagerService;

        public CalibrationPage()
        {
            this.InitializeComponent();
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

        private void StartCalibrationButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel?.StartManualCalibration();
            _windowManagerService?.OpenTouchpadCalibrationWindow();
        }
    }
}