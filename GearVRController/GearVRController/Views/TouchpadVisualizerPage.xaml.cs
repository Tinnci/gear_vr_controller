using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Linq;
using GearVRController.ViewModels;
using GearVRController.Models;
using System.ComponentModel;
using Microsoft.UI.Xaml.Navigation;
using System.Diagnostics;
using Windows.Foundation;
using Microsoft.UI;

namespace GearVRController.Views
{
    public sealed partial class TouchpadVisualizerPage : Page
    {
        public MainViewModel? ViewModel { get; set; }

        private const double TOUCHPAD_RADIUS = 100.0; // 假设触摸板是一个半径为100的圆
        private const double RAW_TOUCHPAD_MAX_VALUE = 255.0; // 原始触摸板最大值

        private Microsoft.UI.Xaml.Media.SolidColorBrush _lineColor = new Microsoft.UI.Xaml.Media.SolidColorBrush(Colors.Blue);
        private Microsoft.UI.Xaml.Media.SolidColorBrush _gestureHintColor = new Microsoft.UI.Xaml.Media.SolidColorBrush(Colors.Green);
        private const double NORMALIZED_RANGE = 1.0; // 归一化后的范围，例如 -1.0 到 1.0

        public TouchpadVisualizerPage()
        {
            this.InitializeComponent();
            this.Loaded += TouchpadVisualizerPage_Loaded;
            this.Unloaded += TouchpadVisualizerPage_Unloaded;
        }

        protected override void OnNavigatedTo(NavigationEventArgs e)
        {
            base.OnNavigatedTo(e);
            if (e.Parameter is MainViewModel viewModel)
            {
                ViewModel = viewModel;
                this.DataContext = ViewModel;
                Debug.WriteLine("[TouchpadVisualizerPage] Subscribed to MainViewModel.PropertyChanged.");
                ViewModel.PropertyChanged += ViewModel_PropertyChanged;
            }
        }

        protected override void OnNavigatedFrom(NavigationEventArgs e)
        {
            base.OnNavigatedFrom(e);
            if (ViewModel != null)
            {
                ViewModel.PropertyChanged -= ViewModel_PropertyChanged;
                Debug.WriteLine("[TouchpadVisualizerPage] Unsubscribed from MainViewModel.PropertyChanged.");
            }
            // 在离开页面时清除Canvas内容
            HistoryCanvas.Children.Clear();
            TouchPoint.Visibility = Visibility.Collapsed;
        }

        private void TouchpadVisualizerPage_Loaded(object sender, RoutedEventArgs e)
        {
            UpdateVisualization();
        }

        private void TouchpadVisualizerPage_Unloaded(object sender, RoutedEventArgs e)
        {
            // Unsubscribe from events to prevent memory leaks when the page is unloaded
            this.Loaded -= TouchpadVisualizerPage_Loaded;
            this.Unloaded -= TouchpadVisualizerPage_Unloaded;
        }

        private void ViewModel_PropertyChanged(object? sender, PropertyChangedEventArgs e)
        {
            if (e.PropertyName == nameof(ViewModel.LastControllerData) ||
                e.PropertyName == nameof(ViewModel.TouchpadHistory) ||
                e.PropertyName == nameof(ViewModel.LastGesture) ||
                e.PropertyName == nameof(ViewModel.ShowGestureHints))
            {
                // Ensure UI updates are on the UI thread
                DispatcherQueue.TryEnqueue(() =>
                {
                    UpdateVisualization();
                });
            }
        }

        private void UpdateVisualization()
        {
            if (ViewModel == null || TouchpadCanvas == null || HistoryCanvas == null || TouchPoint == null) return;

            double canvasWidth = TouchpadCanvas.ActualWidth;
            double canvasHeight = TouchpadCanvas.ActualHeight;

            if (canvasWidth == 0 || canvasHeight == 0) return; // 避免在尺寸未确定时绘制

            double centerX = canvasWidth / 2;
            double centerY = canvasHeight / 2;
            double scale = System.Math.Min(canvasWidth, canvasHeight) / (TOUCHPAD_RADIUS * 2); // 确保整个触摸板都能显示

            // 清除之前的绘图
            HistoryCanvas.Children.Clear();

            // 绘制触摸板边界圆圈
            TouchpadBoundary.Width = TOUCHPAD_RADIUS * 2 * scale;
            TouchpadBoundary.Height = TOUCHPAD_RADIUS * 2 * scale;
            Canvas.SetLeft(TouchpadBoundary, centerX - TouchpadBoundary.Width / 2);
            Canvas.SetTop(TouchpadBoundary, centerY - TouchpadBoundary.Height / 2);

            // 绘制中心线
            HorizontalLine.X1 = 0;
            HorizontalLine.Y1 = centerY;
            HorizontalLine.X2 = canvasWidth;
            HorizontalLine.Y2 = centerY;

            VerticalLine.X1 = centerX;
            VerticalLine.Y1 = 0;
            VerticalLine.X2 = centerX;
            VerticalLine.Y2 = canvasHeight;

            // 绘制历史轨迹
            if (ViewModel.TouchpadHistory != null && ViewModel.TouchpadHistory.Any())
            {
                Point? previousPoint = null;
                foreach (var point in ViewModel.TouchpadHistory)
                {
                    double displayX = centerX + (point.X / NORMALIZED_RANGE) * TOUCHPAD_RADIUS * scale;
                    double displayY = centerY - (point.Y / NORMALIZED_RANGE) * TOUCHPAD_RADIUS * scale; // Y轴反转，因为UI坐标系Y向下

                    if (previousPoint.HasValue)
                    {
                        Microsoft.UI.Xaml.Shapes.Line line = new Microsoft.UI.Xaml.Shapes.Line
                        {
                            X1 = previousPoint.Value.X,
                            Y1 = previousPoint.Value.Y,
                            X2 = displayX,
                            Y2 = displayY,
                            Stroke = _lineColor,
                            StrokeThickness = 2
                        };
                        HistoryCanvas.Children.Add(line);
                    }
                    previousPoint = new Point(displayX, displayY);
                }
            }

            // 绘制当前触摸点
            ControllerData? currentData = ViewModel.LastControllerData;
            if (currentData != null && currentData.TouchpadTouched)
            {
                double displayX = centerX + (currentData.ProcessedTouchpadX / NORMALIZED_RANGE) * TOUCHPAD_RADIUS * scale;
                double displayY = centerY - (currentData.ProcessedTouchpadY / NORMALIZED_RANGE) * TOUCHPAD_RADIUS * scale; // Y轴反转

                Canvas.SetLeft(TouchPoint, displayX - TouchPoint.Width / 2);
                Canvas.SetTop(TouchPoint, displayY - TouchPoint.Height / 2);
                TouchPoint.Visibility = Visibility.Visible;

                // 更新坐标和状态文本
                XValueText.Text = $"{currentData.ProcessedTouchpadX:F2}";
                YValueText.Text = $"{currentData.ProcessedTouchpadY:F2}";
                PressedStateText.Text = "按下";
            }
            else
            {
                TouchPoint.Visibility = Visibility.Collapsed;
                XValueText.Text = "0.00";
                YValueText.Text = "0.00";
                PressedStateText.Text = "未按下";
            }

            // 更新手势信息
            GestureText.Text = ViewModel.LastGesture.ToString();

            // 绘制手势提示 (如果启用)
            if (ViewModel.ShowGestureHints && ViewModel.LastGesture != GearVRController.Enums.TouchpadGesture.None)
            {
                DrawGestureHint(ViewModel.LastGesture, centerX, centerY, scale);
            }
        }

        private void DrawGestureHint(GearVRController.Enums.TouchpadGesture gesture, double centerX, double centerY, double scale)
        {
            // 绘制一个简单的箭头或指示器来表示手势方向
            // 这里只是一个简化示例，可以根据需要绘制更复杂、更精美的动画
            Microsoft.UI.Xaml.Shapes.Line arrowLine = new Microsoft.UI.Xaml.Shapes.Line { Stroke = _gestureHintColor, StrokeThickness = 3 };
            Microsoft.UI.Xaml.Shapes.Polyline arrowHead = new Microsoft.UI.Xaml.Shapes.Polyline { Stroke = _gestureHintColor, StrokeThickness = 3 };
            arrowHead.Points = new Microsoft.UI.Xaml.Media.PointCollection();

            double arrowLength = TOUCHPAD_RADIUS * 0.5 * scale;
            double headSize = 10 * scale;

            switch (gesture)
            {
                case GearVRController.Enums.TouchpadGesture.SwipeUp:
                    arrowLine.X1 = centerX;
                    arrowLine.Y1 = centerY + arrowLength / 2;
                    arrowLine.X2 = centerX;
                    arrowLine.Y2 = centerY - arrowLength / 2;
                    arrowHead.Points.Add(new Point(centerX - headSize, centerY - arrowLength / 2 + headSize));
                    arrowHead.Points.Add(new Point(centerX, centerY - arrowLength / 2));
                    arrowHead.Points.Add(new Point(centerX + headSize, centerY - arrowLength / 2 + headSize));
                    break;
                case GearVRController.Enums.TouchpadGesture.SwipeDown:
                    arrowLine.X1 = centerX;
                    arrowLine.Y1 = centerY - arrowLength / 2;
                    arrowLine.X2 = centerX;
                    arrowLine.Y2 = centerY + arrowLength / 2;
                    arrowHead.Points.Add(new Point(centerX - headSize, centerY + arrowLength / 2 - headSize));
                    arrowHead.Points.Add(new Point(centerX, centerY + arrowLength / 2));
                    arrowHead.Points.Add(new Point(centerX + headSize, centerY + arrowLength / 2 - headSize));
                    break;
                case GearVRController.Enums.TouchpadGesture.SwipeLeft:
                    arrowLine.X1 = centerX + arrowLength / 2;
                    arrowLine.Y1 = centerY;
                    arrowLine.X2 = centerX - arrowLength / 2;
                    arrowLine.Y2 = centerY;
                    arrowHead.Points.Add(new Point(centerX - arrowLength / 2 + headSize, centerY - headSize));
                    arrowHead.Points.Add(new Point(centerX - arrowLength / 2, centerY));
                    arrowHead.Points.Add(new Point(centerX - arrowLength / 2 + headSize, centerY + headSize));
                    break;
                case GearVRController.Enums.TouchpadGesture.SwipeRight:
                    arrowLine.X1 = centerX - arrowLength / 2;
                    arrowLine.Y1 = centerY;
                    arrowLine.X2 = centerX + arrowLength / 2;
                    arrowLine.Y2 = centerY;
                    arrowHead.Points.Add(new Point(centerX + arrowLength / 2 - headSize, centerY - headSize));
                    arrowHead.Points.Add(new Point(centerX + arrowLength / 2, centerY));
                    arrowHead.Points.Add(new Point(centerX + arrowLength / 2 - headSize, centerY + headSize));
                    break;
            }
            HistoryCanvas.Children.Add(arrowLine);
            HistoryCanvas.Children.Add(arrowHead);
        }

        private void ClearHistoryButton_Click(object sender, RoutedEventArgs e)
        {
            ViewModel?.ClearTouchpadHistory();
            UpdateVisualization();
        }
    }
}