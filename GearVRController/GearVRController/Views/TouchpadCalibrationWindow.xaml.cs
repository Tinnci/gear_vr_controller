using System;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using GearVRController.ViewModels;
using GearVRController.Models;
using Microsoft.UI.Dispatching;
using GearVRController.Services;

namespace GearVRController.Views
{
    public sealed partial class TouchpadCalibrationWindow : Window
    {
        private readonly TouchpadCalibrationViewModel _viewModel;
        private readonly DispatcherQueue _dispatcherQueue;
        private readonly MainViewModel _mainViewModel;

        public TouchpadCalibrationWindow()
        {
            this.InitializeComponent();
            _viewModel = new TouchpadCalibrationViewModel();
            _dispatcherQueue = DispatcherQueue.GetForCurrentThread();
            _mainViewModel = ServiceLocator.GetService<MainViewModel>();

            if (Content is FrameworkElement rootElement)
            {
                rootElement.DataContext = _viewModel;
            }

            _viewModel.StartCalibration();
        }

        public void ProcessControllerData(ControllerData data)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                _viewModel.ProcessTouchpadData(data);
            });
        }

        private void FinishCalibration_Click(object sender, RoutedEventArgs e)
        {
            _viewModel.FinishCalibration();
            this.Close();
        }

        private void Cancel_Click(object sender, RoutedEventArgs e)
        {
            _viewModel.CancelCalibration();
            this.Close();
        }

        private void NextStep_Click(object sender, RoutedEventArgs e)
        {
            _viewModel.ProceedToNextStep();
        }

        public event EventHandler<TouchpadCalibrationData>? CalibrationCompleted
        {
            add => _viewModel.CalibrationCompleted += value;
            remove => _viewModel.CalibrationCompleted -= value;
        }
    }
}