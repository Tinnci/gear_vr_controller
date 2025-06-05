using System;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using GearVRController.ViewModels;
using GearVRController.Models;
using Microsoft.UI.Dispatching;

namespace GearVRController.Views
{
    public sealed partial class TouchpadCalibrationWindow : Window
    {
        private readonly TouchpadCalibrationViewModel _viewModel;
        private readonly DispatcherQueue _dispatcherQueue;

        // 校准中心点UI常量
        private const double CALIBRATION_CENTER_DOT_SIZE = 10.0;
        // 原始触摸板数据范围
        private const double RAW_TOUCHPAD_RANGE = 1023.0;

        public TouchpadCalibrationWindow(TouchpadCalibrationViewModel viewModel)
        {
            this.InitializeComponent();
            _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));
            _dispatcherQueue = DispatcherQueue.GetForCurrentThread();

            if (Content is FrameworkElement rootElement)
            {
                rootElement.DataContext = _viewModel;
            }

            _viewModel.StartCalibration();

            // Subscribe to PropertyChanged event to update visualization
            _viewModel.PropertyChanged += ViewModel_PropertyChanged;

            // Subscribe to window closed event for cleanup
            this.Closed += TouchpadCalibrationWindow_Closed;

            // Initial visualization update
            UpdateCalibrationVisualization();
        }

        private void TouchpadCalibrationWindow_Closed(object sender, WindowEventArgs args)
        {
            // Unsubscribe from ViewModel events to prevent memory leaks
            if (_viewModel != null)
            {
                _viewModel.PropertyChanged -= ViewModel_PropertyChanged;
                System.Diagnostics.Debug.WriteLine("[TouchpadCalibrationWindow] Unsubscribed from ViewModel.PropertyChanged.");
            }
            // Unsubscribe from window events to prevent double subscription on re-opening
            this.Closed -= TouchpadCalibrationWindow_Closed;
            System.Diagnostics.Debug.WriteLine("[TouchpadCalibrationWindow] Unsubscribed from its own Closed event.");
        }

        private void ViewModel_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
        {
            // Update visualization when relevant properties change
            if (e.PropertyName == nameof(TouchpadCalibrationViewModel.MinX) ||
                e.PropertyName == nameof(TouchpadCalibrationViewModel.MaxX) ||
                e.PropertyName == nameof(TouchpadCalibrationViewModel.MinY) ||
                e.PropertyName == nameof(TouchpadCalibrationViewModel.MaxY) ||
                e.PropertyName == nameof(TouchpadCalibrationViewModel.CenterX) ||
                e.PropertyName == nameof(TouchpadCalibrationViewModel.CenterY))
            {
                UpdateCalibrationVisualization();
            }
        }

        private void UpdateCalibrationVisualization()
        {
            // Ensure the canvas is ready and has valid dimensions
            if (CalibrationCanvas == null || CalibrationCanvas.ActualWidth == 0 || CalibrationCanvas.ActualHeight == 0)
            {
                return;
            }

            // Clear existing elements (if any, though we only have two fixed shapes)
            // CalibrationCanvas.Children.Clear(); // Not needed for fixed shapes

            // Get calibration data from ViewModel
            int minX = _viewModel.MinX;
            int maxX = _viewModel.MaxX;
            int minY = _viewModel.MinY;
            int maxY = _viewModel.MaxY;
            int centerX = _viewModel.CenterX;
            int centerY = _viewModel.CenterY;

            // Determine if calibration data is meaningful (not default int.MaxValue/MinValue)
            bool hasValidBounds = (minX != int.MaxValue && maxX != int.MinValue && minY != int.MaxValue && maxY != int.MinValue);

            if (hasValidBounds)
            {
                // Scale raw data (0-1023) to canvas size (200x200)
                double canvasWidth = CalibrationCanvas.ActualWidth;
                double canvasHeight = CalibrationCanvas.ActualHeight;
                // const double rawRange = 1023.0; // Assuming 0-1023 as the full raw range

                double scaleX = canvasWidth / RAW_TOUCHPAD_RANGE;
                double scaleY = canvasHeight / RAW_TOUCHPAD_RANGE;

                // Update Rectangle (Boundary)
                double rectLeft = minX * scaleX;
                double rectTop = minY * scaleY;
                double rectWidth = (maxX - minX) * scaleX;
                double rectHeight = (maxY - minY) * scaleY;

                // Ensure positive dimensions for the rectangle
                rectWidth = Math.Max(0, rectWidth);
                rectHeight = Math.Max(0, rectHeight);

                Canvas.SetLeft(CalibrationRect, rectLeft);
                Canvas.SetTop(CalibrationRect, rectTop);
                CalibrationRect.Width = rectWidth;
                CalibrationRect.Height = rectHeight;
                CalibrationRect.Visibility = Visibility.Visible;

                // Update Ellipse (Center Dot)
                double dotLeft = centerX * scaleX - CalibrationCenterDot.Width / 2;
                double dotTop = centerY * scaleY - CalibrationCenterDot.Height / 2;

                Canvas.SetLeft(CalibrationCenterDot, dotLeft);
                Canvas.SetTop(CalibrationCenterDot, dotTop);
                // CalibrationCenterDot.Width = CALIBRATION_CENTER_DOT_SIZE; // Set in XAML
                // CalibrationCenterDot.Height = CALIBRATION_CENTER_DOT_SIZE; // Set in XAML
                CalibrationCenterDot.Visibility = Visibility.Visible;
            }
            else
            {
                // Hide visualization if no valid calibration data yet
                CalibrationRect.Visibility = Visibility.Collapsed;
                CalibrationCenterDot.Visibility = Visibility.Collapsed;
            }
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

        private void Reset_Click(object sender, RoutedEventArgs e)
        {
            _viewModel.ResetCalibration();
            // Optionally, you might want to re-open the window or clear the current view to reflect the reset state
            // For now, it just resets the ViewModel's state.
        }
    }
}