using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using GearVRController.ViewModels;
using Microsoft.Extensions.DependencyInjection;

namespace GearVRController.Views
{
    public sealed partial class SettingsPage : Page
    {
        public SettingsViewModel? ViewModel { get; set; }

        public SettingsPage()
        {
            this.InitializeComponent();
            ViewModel = App.ServiceProvider?.GetRequiredService<SettingsViewModel>();
            this.DataContext = ViewModel;
        }

        private void ResetSettingsButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel?.ResetSettings();
        }
    }
}