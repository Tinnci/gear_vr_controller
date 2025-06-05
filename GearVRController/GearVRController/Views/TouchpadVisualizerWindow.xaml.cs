using System;
using System.Collections.Generic;
using System.Linq;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;
using GearVRController.Models;
using Microsoft.UI;
using Microsoft.UI.Dispatching;
using System.Diagnostics;
using Microsoft.UI.Xaml.Media.Animation;
using Windows.UI;
using EnumsNS = GearVRController.Enums;
using GearVRController.ViewModels;

namespace GearVRController.Views
{
    public sealed partial class TouchpadVisualizerWindow : Window
    {
        private readonly MainViewModel _viewModel;
        private bool _showTrail = true;
        private double _canvasWidth;
        private double _canvasHeight;
        private double _radius;
        private Point _center;
        private readonly DispatcherQueue _dispatcherQueue;

        // 用于绘制的元素
        private Ellipse _touchPoint;
        private Line _touchLine;
        private TextBlock _coordsText;
        private readonly List<UIElement> _gridElements = new List<UIElement>();
        private readonly List<UIElement> _trailElements = new List<UIElement>();
        private UIElement? _gestureIndicator = null;

        // 历史轨迹点集合
        private readonly List<Ellipse> _historyPoints = new();

        // 历史轨迹的最大数量
        private const int MaxHistoryPoints = 50;

        // 触摸板区域的尺寸
        private double _touchpadSize;

        // 触摸点的位置
        private double _currentX;
        private double _currentY;

        // 触摸状态
        private bool _isTouching;

        // 添加一个属性来初始化和获取手势指示器
        private UIElement? GestureIndicator => _gestureIndicator ??= CreateGestureIndicator(EnumsNS.TouchpadGesture.None);

        public TouchpadVisualizerWindow(MainViewModel viewModel)
        {
            this.InitializeComponent();

            _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));

            // 获取当前线程的调度器队列
            _dispatcherQueue = DispatcherQueue.GetForCurrentThread();

            // 设置窗口大小
            this.AppWindow.Resize(new Windows.Graphics.SizeInt32(400, 500));

            // 初始化触摸点圆形
            _touchPoint = new Ellipse
            {
                Width = 20,
                Height = 20,
                Fill = new SolidColorBrush(Colors.Blue),
                Visibility = Visibility.Collapsed
            };

            // 初始化从中心到触摸点的线
            _touchLine = new Line
            {
                Stroke = new SolidColorBrush(Colors.Gray),
                StrokeThickness = 1,
                Opacity = 0.5,
                Visibility = Visibility.Collapsed
            };

            // 初始化坐标文本
            _coordsText = new TextBlock
            {
                Foreground = new SolidColorBrush(Colors.Black),
                FontSize = 12,
                Visibility = Visibility.Collapsed
            };

            TouchpadCanvas.Children.Add(_touchLine);
            TouchpadCanvas.Children.Add(_touchPoint);
            TouchpadCanvas.Children.Add(_coordsText);

            // InitializeGrid() will be called from UpdateVisualizationLayout()

            Debug.WriteLine("触摸板可视化窗口已初始化");
            Debug.WriteLine("Attempting to add Closed event handler."); // TEST LINE

            // 窗口加载完成后初始化UI - 使用正确的事件类型
            this.Activated += TouchpadVisualizerWindow_Activated;

            // 窗口大小改变时更新UI - 使用Window对应的事件类型
            this.AppWindow.Changed += AppWindow_Changed;

            // Subscribe to ViewModel's PropertyChanged event for visualization updates
            _viewModel.PropertyChanged += ViewModel_PropertyChanged;

            // Handle window closing for proper cleanup
            // this.Closed += TouchpadVisualizerWindow_Closed; // This will be uncommented after testing

            // Ensure layout is initialized after all setup
            UpdateVisualizationLayout();
        }

        private void ViewModel_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
        {
            // Only update visualization if relevant properties change
            if (e.PropertyName == nameof(MainViewModel.ProcessedTouchpadX) ||
                e.PropertyName == nameof(MainViewModel.ProcessedTouchpadY) ||
                e.PropertyName == nameof(MainViewModel.LastControllerData) || // For touchpad button state
                e.PropertyName == nameof(MainViewModel.LastGesture) ||
                e.PropertyName == nameof(MainViewModel.TouchpadHistory)) // For trail drawing
            {
                _dispatcherQueue.TryEnqueue(() =>
                {
                    UpdateTouchpadVisualization(
                        _viewModel.ProcessedTouchpadX,
                        _viewModel.ProcessedTouchpadY,
                        _viewModel.LastControllerData.TouchpadButton, // Use LastControllerData for button state
                        _viewModel.LastGesture
                    );
                });
            }
        }

        private void TouchpadVisualizerWindow_Activated(object sender, WindowActivatedEventArgs args)
        {
            // InitializeVisualization(); // This method is now redundant, UpdateVisualizationLayout already handles init
        }

        private void AppWindow_Changed(Microsoft.UI.Windowing.AppWindow sender, Microsoft.UI.Windowing.AppWindowChangedEventArgs args)
        {
            if (args.DidSizeChange)
            {
                _dispatcherQueue.TryEnqueue(() =>
                {
                    UpdateVisualizationLayout();
                });
            }
        }

        private void TouchpadVisualizerWindow_Closed(object sender, WindowEventArgs args)
        {
            // Unsubscribe from ViewModel events
            if (_viewModel != null)
            {
                _viewModel.PropertyChanged -= ViewModel_PropertyChanged;
                Debug.WriteLine("[TouchpadVisualizerWindow] Unsubscribed from MainViewModel.PropertyChanged.");
            }

            // Unsubscribe from window events
            this.Activated -= TouchpadVisualizerWindow_Activated;
            this.AppWindow.Changed -= AppWindow_Changed;
            this.Closed -= TouchpadVisualizerWindow_Closed; // Unsubscribe itself
            Debug.WriteLine("[TouchpadVisualizerWindow] Unsubscribed from window events.");

            // Clear visual elements
            TouchpadCanvas.Children.Clear();
            _gridElements.Clear();
            _trailElements.Clear();
            _historyPoints.Clear(); // Ensure this is also cleared if used separately

            Debug.WriteLine("[TouchpadVisualizerWindow] Window resources cleaned up.");
        }

        private void UpdateVisualizationLayout()
        {
            // 确保Canvas有合理的尺寸
            if (TouchpadCanvas.ActualWidth <= 20 || TouchpadCanvas.ActualHeight <= 20)
            {
                Debug.WriteLine("画布尺寸太小，暂不更新布局");
                return; // 画布尺寸太小，不进行更新
            }

            // Update internal canvas dimensions and center for grid/object positioning
            _canvasWidth = TouchpadCanvas.ActualWidth;
            _canvasHeight = TouchpadCanvas.ActualHeight;
            _center = new Point(_canvasWidth / 2, _canvasHeight / 2);

            // 计算触摸板显示区域的尺寸（取宽度和高度的较小值）
            // 考虑到实际触摸板范围是0~315，我们设置一个合理的尺寸
            _touchpadSize = Math.Min(_canvasWidth, _canvasHeight) - 20;

            // 确保尺寸是正数且合理
            _touchpadSize = Math.Max(100, Math.Min(500, _touchpadSize));
            _radius = _touchpadSize / 2; // Update radius based on calculated touchpadSize

            // Update coordinate axes
            HorizontalLine.X1 = 0;
            HorizontalLine.Y1 = _canvasHeight / 2;
            HorizontalLine.X2 = _canvasWidth;
            HorizontalLine.Y2 = _canvasHeight / 2;

            VerticalLine.X1 = _canvasWidth / 2;
            VerticalLine.Y1 = 0;
            VerticalLine.X2 = _canvasWidth / 2;
            VerticalLine.Y2 = _canvasHeight;

            try
            {
                // Update touchpad boundary circle
                TouchpadBoundary.Width = _touchpadSize;
                TouchpadBoundary.Height = _touchpadSize;
                TouchpadBoundary.Margin = new Thickness(
                    (_canvasWidth - _touchpadSize) / 2,
                    (_canvasHeight - _touchpadSize) / 2,
                    0, 0);
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"设置触摸板边界尺寸时出错: {ex.Message}");
                // 使用更安全的默认值
                TouchpadBoundary.Width = 300;
                TouchpadBoundary.Height = 300;
                TouchpadBoundary.Margin = new Thickness(
                    Math.Max(0, (_canvasWidth - 300) / 2),
                    Math.Max(0, (_canvasHeight - 300) / 2),
                    0, 0);
            }

            // Re-initialize grid as its size depends on _radius and _center
            InitializeGrid();

            // Clear history after layout updates, if desired. Otherwise, history points might be misplaced.
            ClearHistory();
        }

        /// <summary>
        /// 初始化网格
        /// </summary>
        private void InitializeGrid()
        {
            // 移除旧的网格元素
            foreach (var element in _gridElements)
            {
                TouchpadCanvas.Children.Remove(element);
            }
            _gridElements.Clear();

            // Check if canvas dimensions are valid BEFORE drawing grid elements
            if (_canvasWidth <= 0 || _canvasHeight <= 0 || _radius <= 0)
                return;

            // 创建同心圆
            for (int i = 1; i <= 3; i++)
            {
                var circle = new Ellipse
                {
                    Width = _radius * 2 * i / 3, // Scale circles relative to _radius
                    Height = _radius * 2 * i / 3,
                    Stroke = new SolidColorBrush(Colors.Gray),
                    StrokeThickness = 1,
                    Opacity = 0.2
                };

                Canvas.SetLeft(circle, _center.X - circle.Width / 2);
                Canvas.SetTop(circle, _center.Y - circle.Height / 2);

                TouchpadCanvas.Children.Add(circle);
                _gridElements.Add(circle);
            }

            // 创建十字轴线
            var horizontalLine = new Line
            {
                X1 = _center.X - _radius,
                Y1 = _center.Y,
                X2 = _center.X + _radius,
                Y2 = _center.Y,
                Stroke = new SolidColorBrush(Colors.Gray),
                StrokeThickness = 2,
                Opacity = 0.5
            };

            var verticalLine = new Line
            {
                X1 = _center.X,
                Y1 = _center.Y - _radius,
                X2 = _center.X,
                Y2 = _center.Y + _radius,
                Stroke = new SolidColorBrush(Colors.Gray),
                StrokeThickness = 2,
                Opacity = 0.5
            };

            TouchpadCanvas.Children.Add(horizontalLine);
            TouchpadCanvas.Children.Add(verticalLine);
            _gridElements.Add(horizontalLine);
            _gridElements.Add(verticalLine);

            // 绘制径向线
            for (int i = 0; i < 8; i++)
            {
                double angle = i * Math.PI / 4;
                double dx = Math.Cos(angle) * _radius;
                double dy = Math.Sin(angle) * _radius;

                var line = new Line
                {
                    X1 = _center.X,
                    Y1 = _center.Y,
                    X2 = _center.X + dx,
                    Y2 = _center.Y + dy,
                    Stroke = new SolidColorBrush(Colors.Gray),
                    StrokeThickness = 1,
                    Opacity = 0.2
                };

                TouchpadCanvas.Children.Add(line);
                _gridElements.Add(line);
            }
        }

        /// <summary>
        /// 处理触摸板数据
        /// </summary>
        public void UpdateTouchpadVisualization(double normalizedX, double normalizedY, bool isPressed, EnumsNS.TouchpadGesture gesture)
        {
            _currentX = normalizedX; // Store normalized X
            _currentY = normalizedY; // Store normalized Y
            _isTouching = isPressed;

            // 将归一化坐标 (-1 到 1) 映射到画布坐标
            // (normalizedX + 1) / 2 将范围从 [-1, 1] 变为 [0, 1]
            // 然后乘以 _touchpadSize 得到在触摸板区域内的坐标
            // 最后加上偏移量将坐标放置在画布中心
            double displayX = (_currentX + 1) / 2 * _touchpadSize + (TouchpadCanvas.ActualWidth - _touchpadSize) / 2;
            // Y 轴通常在UI中是反向的（正Y向下），所以 (1 - normalizedY) / 2
            double displayY = (1 - _currentY) / 2 * _touchpadSize + (TouchpadCanvas.ActualHeight - _touchpadSize) / 2;

            // Update touch point position
            Canvas.SetLeft(_touchPoint, displayX - _touchPoint.Width / 2);
            Canvas.SetTop(_touchPoint, displayY - _touchPoint.Height / 2);

            // Update line from center to touch point
            _touchLine.X1 = TouchpadCanvas.ActualWidth / 2;
            _touchLine.Y1 = TouchpadCanvas.ActualHeight / 2;
            _touchLine.X2 = displayX;
            _touchLine.Y2 = displayY;

            // Update coordinates text
            Canvas.SetLeft(_coordsText, displayX + _touchPoint.Width / 2 + 5);
            Canvas.SetTop(_coordsText, displayY - _coordsText.Height / 2);
            _coordsText.Text = $"({_currentX:F2}, {_currentY:F2})";

            // Set visibility based on whether the touchpad is touched
            _touchPoint.Visibility = isPressed ? Visibility.Visible : Visibility.Collapsed;
            _touchLine.Visibility = isPressed ? Visibility.Visible : Visibility.Collapsed;
            _coordsText.Visibility = isPressed ? Visibility.Visible : Visibility.Collapsed;

            // Draw trail if enabled
            if (_showTrail)
            {
                DrawTrail();
            }
            else
            {
                ClearHistory(); // Clear trail if it's disabled now
            }

            // Update gesture indicator
            UpdateGestureIndicator(gesture);
        }

        private void DrawTrail()
        {
            // 移除旧的轨迹点
            foreach (var point in _trailElements)
            {
                TouchpadCanvas.Children.Remove(point);
            }
            _trailElements.Clear();

            // 绘制新轨迹点
            int historyCount = _viewModel.TouchpadHistory.Count;
            for (int i = 0; i < historyCount; i++)
            {
                var point = _viewModel.TouchpadHistory[i];
                // Apply the same coordinate transformation as for the current touch point
                double displayX = (point.X + 1) / 2 * _touchpadSize + (TouchpadCanvas.ActualWidth - _touchpadSize) / 2;
                double displayY = (1 - point.Y) / 2 * _touchpadSize + (TouchpadCanvas.ActualHeight - _touchpadSize) / 2;

                // Calculate opacity based on age (older points are more transparent)
                double opacity = (double)(i + 1) / historyCount * 0.8; // Max opacity 0.8

                // Color based on touch state (if available in history point)
                Color color = point.IsTouched ? Colors.DodgerBlue : Colors.LightGray; // Example colors

                var ellipse = new Ellipse
                {
                    Width = 6,
                    Height = 6,
                    Fill = new SolidColorBrush(color),
                    Opacity = opacity
                };

                Canvas.SetLeft(ellipse, displayX - ellipse.Width / 2);
                Canvas.SetTop(ellipse, displayY - ellipse.Height / 2);

                TouchpadCanvas.Children.Add(ellipse);
                _trailElements.Add(ellipse);
            }
        }

        private void UpdateGestureIndicator(EnumsNS.TouchpadGesture gesture)
        {
            // Remove previous gesture indicator if it exists and a new one is needed or old one is not None
            if (_gestureIndicator != null && (_gestureIndicator.Visibility == Visibility.Visible || gesture != EnumsNS.TouchpadGesture.None))
            {
                TouchpadCanvas.Children.Remove(_gestureIndicator);
                _gestureIndicator = null;
            }

            if (gesture != EnumsNS.TouchpadGesture.None)
            {
                _gestureIndicator = CreateGestureIndicator(gesture);
                TouchpadCanvas.Children.Add(_gestureIndicator);

                // Position the gesture indicator at the center of the canvas
                if (_gestureIndicator != null) // Add null check here
                {
                    double indicatorWidth = _gestureIndicator.DesiredSize.Width == 0 ? ((FrameworkElement)_gestureIndicator).ActualWidth : _gestureIndicator.DesiredSize.Width; // Handle initial size
                    double indicatorHeight = _gestureIndicator.DesiredSize.Height == 0 ? ((FrameworkElement)_gestureIndicator).ActualHeight : _gestureIndicator.DesiredSize.Height;

                    Canvas.SetLeft(_gestureIndicator, _center.X - indicatorWidth / 2);
                    Canvas.SetTop(_gestureIndicator, _center.Y - indicatorHeight / 2);
                    _gestureIndicator.Visibility = Visibility.Visible;
                }
            }
            else
            {
                if (_gestureIndicator != null)
                {
                    _gestureIndicator.Visibility = Visibility.Collapsed;
                }
            }
            UpdateGestureInfo(gesture);
        }

        private UIElement? CreateGestureIndicator(EnumsNS.TouchpadGesture gesture)
        {
            // Create a Grid to hold the text and arrow/shape
            var indicatorGrid = new Grid();
            indicatorGrid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            indicatorGrid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });

            var gestureText = new TextBlock
            {
                Text = GetGestureText(gesture),
                FontSize = 18,
                FontWeight = Microsoft.UI.Text.FontWeights.Bold,
                Foreground = new SolidColorBrush(Colors.DarkBlue),
                HorizontalAlignment = HorizontalAlignment.Center
            };
            Grid.SetRow(gestureText, 0);
            indicatorGrid.Children.Add(gestureText);

            // Create a Path for the arrow/shape based on gesture direction
            Path arrowPath = new Path
            {
                Fill = new SolidColorBrush(Colors.DarkBlue),
                Stroke = new SolidColorBrush(Colors.DarkBlue),
                StrokeThickness = 1
            };

            TransformGroup transformGroup = new TransformGroup();
            RotateTransform rotateTransform = new RotateTransform();
            transformGroup.Children.Add(rotateTransform);
            arrowPath.RenderTransform = transformGroup;
            arrowPath.RenderTransformOrigin = new Point(0.5, 0.5); // Rotate around center

            // Define arrow geometry and rotation based on gesture
            PathGeometry? arrowGeometry = null; // Initialize as null and set type to PathGeometry
            double angle = 0;

            switch (gesture)
            {
                case EnumsNS.TouchpadGesture.SwipeUp:
                    arrowGeometry = new PathGeometry();
                    PathFigure figureUp = new PathFigure { StartPoint = new Point(0, 10) };
                    figureUp.Segments.Add(new LineSegment { Point = new Point(5, 0) });
                    figureUp.Segments.Add(new LineSegment { Point = new Point(10, 10) });
                    figureUp.IsClosed = true;
                    arrowGeometry.Figures.Add(figureUp);
                    angle = 0;
                    break;
                case EnumsNS.TouchpadGesture.SwipeDown:
                    arrowGeometry = new PathGeometry();
                    PathFigure figureDown = new PathFigure { StartPoint = new Point(0, 0) };
                    figureDown.Segments.Add(new LineSegment { Point = new Point(5, 10) });
                    figureDown.Segments.Add(new LineSegment { Point = new Point(10, 0) });
                    figureDown.IsClosed = true;
                    arrowGeometry.Figures.Add(figureDown);
                    angle = 0;
                    break;
                case EnumsNS.TouchpadGesture.SwipeLeft:
                    arrowGeometry = new PathGeometry();
                    PathFigure figureLeft = new PathFigure { StartPoint = new Point(10, 0) };
                    figureLeft.Segments.Add(new LineSegment { Point = new Point(0, 5) });
                    figureLeft.Segments.Add(new LineSegment { Point = new Point(10, 10) });
                    figureLeft.IsClosed = true;
                    arrowGeometry.Figures.Add(figureLeft);
                    angle = 0; // Handled by geometry
                    break;
                case EnumsNS.TouchpadGesture.SwipeRight:
                    arrowGeometry = new PathGeometry();
                    PathFigure figureRight = new PathFigure { StartPoint = new Point(0, 0) };
                    figureRight.Segments.Add(new LineSegment { Point = new Point(10, 5) });
                    figureRight.Segments.Add(new LineSegment { Point = new Point(0, 10) });
                    figureRight.IsClosed = true;
                    arrowGeometry.Figures.Add(figureRight);
                    angle = 0; // Handled by geometry
                    break;
                // For other gestures, you might use different shapes or hide the arrow
                default:
                    arrowPath.Visibility = Visibility.Collapsed; // Hide arrow for None or other gestures
                    break;
            }

            arrowPath.Data = arrowGeometry ?? Geometry.Empty;
            rotateTransform.Angle = angle;

            Grid.SetRow(arrowPath, 1);
            indicatorGrid.Children.Add(arrowPath);

            return indicatorGrid;
        }

        private string GetGestureText(EnumsNS.TouchpadGesture gesture)
        {
            return gesture switch
            {
                EnumsNS.TouchpadGesture.None => "无",
                EnumsNS.TouchpadGesture.SwipeUp => "向上滑动",
                EnumsNS.TouchpadGesture.SwipeDown => "向下滑动",
                EnumsNS.TouchpadGesture.SwipeLeft => "向左滑动",
                EnumsNS.TouchpadGesture.SwipeRight => "向右滑动",
                _ => "未知"
            };
        }

        /// <summary>
        /// 清除历史轨迹
        /// </summary>
        public void ClearHistory()
        {
            // Clear history in ViewModel
            _viewModel.ClearTouchpadHistory();
            // Clear visual trail
            foreach (var element in _trailElements)
            {
                TouchpadCanvas.Children.Remove(element);
            }
            _trailElements.Clear();
            UpdateGestureInfo(EnumsNS.TouchpadGesture.None);
        }

        /// <summary>
        /// 设置是否显示轨迹
        /// </summary>
        public void SetShowTrail(bool show)
        {
            _showTrail = show;

            _dispatcherQueue.TryEnqueue(() =>
            {
                if (_showTrail)
                {
                    DrawTrail();
                }
                else
                {
                    // 清除轨迹
                    foreach (var element in _trailElements)
                    {
                        TouchpadCanvas.Children.Remove(element);
                    }
                    _trailElements.Clear();
                }
            });
        }

        private void ShowTrailToggle_Checked(object sender, RoutedEventArgs e)
        {
            SetShowTrail(true);
        }

        private void ShowTrailToggle_Unchecked(object sender, RoutedEventArgs e)
        {
            SetShowTrail(false);
        }

        private void ClearTrailButton_Click(object sender, RoutedEventArgs e)
        {
            ClearHistory();
        }

        private void CloseButton_Click(object sender, RoutedEventArgs e)
        {
            this.Close();
        }

        /// <summary>
        /// 更新手势信息文本
        /// </summary>
        private void UpdateGestureInfo(EnumsNS.TouchpadGesture gesture)
        {
            string gestureText = gesture switch
            {
                EnumsNS.TouchpadGesture.None => "无手势",
                EnumsNS.TouchpadGesture.SwipeUp => "向上滑动",
                EnumsNS.TouchpadGesture.SwipeDown => "向下滑动",
                EnumsNS.TouchpadGesture.SwipeLeft => "向左滑动",
                EnumsNS.TouchpadGesture.SwipeRight => "向右滑动",
                _ => "未知手势"
            };

            GestureInfoText.Text = $"检测到手势: {gestureText}";
        }

        private void ClearHistoryButton_Click(object sender, RoutedEventArgs e)
        {
            ClearHistory();
        }
    }
}