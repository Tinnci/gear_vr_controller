using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using GearVRController.ViewModels;
using Microsoft.Extensions.DependencyInjection;

namespace GearVRController.Views
{
    public sealed partial class CalibrationPage : Page
    {
        public MainViewModel? ViewModel { get; set; }
        private TouchpadCalibrationViewModel? _touchpadCalibrationViewModel;

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
            _touchpadCalibrationViewModel = App.ServiceProvider!.GetRequiredService<TouchpadCalibrationViewModel>();
        }

        private void StartCalibrationButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel?.StartManualCalibration();
            if (Frame != null && _touchpadCalibrationViewModel != null)
            {
                Frame.Navigate(typeof(TouchpadCalibrationPage), _touchpadCalibrationViewModel);
            }
        }
    }
}