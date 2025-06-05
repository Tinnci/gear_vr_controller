using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using GearVRController.ViewModels;

namespace GearVRController.Views
{
    public sealed partial class SettingsPage : Page
    {
        public MainViewModel? ViewModel { get; set; }

        public SettingsPage()
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
        }

        private void ResetSettingsButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel?.ResetSettings();
        }
    }
}