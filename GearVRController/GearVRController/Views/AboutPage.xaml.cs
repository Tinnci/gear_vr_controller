using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using GearVRController.ViewModels;

namespace GearVRController.Views
{
    public sealed partial class AboutPage : Page
    {
        public MainViewModel? ViewModel { get; set; }

        public AboutPage()
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
    }
}