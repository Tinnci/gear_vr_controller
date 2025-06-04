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

namespace GearVRController.Views
{
    public sealed partial class TouchpadVisualizerWindow : Window
    {
        private readonly List<TouchpadPoint> _touchpadHistory = new List<TouchpadPoint>();
        private const int MAX_HISTORY_POINTS = 100;
        private bool _showTrail = true;
        private double _canvasWidth;
        private double _canvasHeight;
        private double _radius;
        private Point _center;
        private readonly Queue<TouchpadPoint> _gestureBuffer = new Queue<TouchpadPoint>();
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

        public TouchpadVisualizerWindow()
        {
            this.InitializeComponent();

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

            // 初始化网格
            InitializeGrid();

            Debug.WriteLine("触摸板可视化窗口已初始化");

            // 窗口加载完成后初始化UI - 使用正确的事件类型
            this.Activated += TouchpadVisualizerWindow_Activated;

            // 窗口大小改变时更新UI - 使用Window对应的事件类型
            this.AppWindow.Changed += AppWindow_Changed;
        }

        private void TouchpadVisualizerWindow_Activated(object sender, WindowActivatedEventArgs args)
        {
            InitializeVisualization();
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

        // 保留此方法以兼容其他调用
        private void TouchpadVisualizerWindow_SizeChanged(object sender, SizeChangedEventArgs e)
        {
            UpdateVisualizationLayout();
        }

        private void InitializeVisualization()
        {
            UpdateVisualizationLayout();
        }

        private void UpdateVisualizationLayout()
        {
            // 确保Canvas有合理的尺寸
            if (TouchpadCanvas.ActualWidth <= 20 || TouchpadCanvas.ActualHeight <= 20)
            {
                Debug.WriteLine("画布尺寸太小，暂不更新布局");
                return; // 画布尺寸太小，不进行更新
            }

            // 计算触摸板显示区域的尺寸（取宽度和高度的较小值）
            // 考虑到实际触摸板范围是0~315，我们设置一个合理的尺寸
            _touchpadSize = Math.Min(TouchpadCanvas.ActualWidth, TouchpadCanvas.ActualHeight) - 20;

            // 确保尺寸是正数且合理
            _touchpadSize = Math.Max(100, Math.Min(500, _touchpadSize));

            // 更新坐标轴
            HorizontalLine.X1 = 0;
            HorizontalLine.Y1 = TouchpadCanvas.ActualHeight / 2;
            HorizontalLine.X2 = TouchpadCanvas.ActualWidth;
            HorizontalLine.Y2 = TouchpadCanvas.ActualHeight / 2;

            VerticalLine.X1 = TouchpadCanvas.ActualWidth / 2;
            VerticalLine.Y1 = 0;
            VerticalLine.X2 = TouchpadCanvas.ActualWidth / 2;
            VerticalLine.Y2 = TouchpadCanvas.ActualHeight;

            try
            {
                // 更新触摸板边界圆圈
                TouchpadBoundary.Width = _touchpadSize;
                TouchpadBoundary.Height = _touchpadSize;
                TouchpadBoundary.Margin = new Thickness(
                    (TouchpadCanvas.ActualWidth - _touchpadSize) / 2,
                    (TouchpadCanvas.ActualHeight - _touchpadSize) / 2,
                    0, 0);
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"设置触摸板边界尺寸时出错: {ex.Message}");
                // 使用更安全的默认值
                TouchpadBoundary.Width = 300;
                TouchpadBoundary.Height = 300;
                TouchpadBoundary.Margin = new Thickness(
                    Math.Max(0, (TouchpadCanvas.ActualWidth - 300) / 2),
                    Math.Max(0, (TouchpadCanvas.ActualHeight - 300) / 2),
                    0, 0);
            }

            // 清除历史轨迹
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

            if (_canvasWidth <= 0 || _canvasHeight <= 0)
                return;

            // 创建同心圆
            for (int i = 1; i <= 3; i++)
            {
                var circle = new Ellipse
                {
                    Width = _radius * 2 * i / 3,
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
        public void ProcessTouchpadData(double normalizedX, double normalizedY, bool isPressed, EnumsNS.TouchpadGesture gesture)
        {
            _dispatcherQueue.TryEnqueue(() =>
            {
                double processedX = normalizedX;
                double processedY = normalizedY;

                // 记录历史
                var point = new TouchpadPoint((float)processedX, (float)processedY, isPressed);
                _touchpadHistory.Add(point);

                // 维持最大历史点数
                while (_touchpadHistory.Count > MAX_HISTORY_POINTS)
                {
                    _touchpadHistory.RemoveAt(0);
                }

                // 更新UI
                UpdateTouchpadVisualization(processedX, processedY, isPressed, gesture);

                // 更新状态文本
                XValueText.Text = processedX.ToString("F2");
                YValueText.Text = processedY.ToString("F2");
                PressedStateText.Text = isPressed ? "已按下" : "未按下";
                GestureText.Text = GetGestureText(gesture);

                Debug.WriteLine($"触摸板数据更新: X={processedX:F2}, Y={processedY:F2}, 按下={isPressed}, 手势={gesture}");
            });
        }

        /// <summary>
        /// 更新触摸板可视化
        /// </summary>
        private void UpdateTouchpadVisualization(double normalizedX, double normalizedY, bool isPressed, EnumsNS.TouchpadGesture gesture)
        {
            if (_canvasWidth <= 0 || _canvasHeight <= 0)
                return;

            // 转换为画布坐标
            double x = _center.X + normalizedX * _radius;
            double y = _center.Y + normalizedY * _radius;

            // 更新触摸点
            _touchPoint.Fill = new SolidColorBrush(isPressed ? Colors.Red : Colors.Blue);
            _touchPoint.Width = isPressed ? 20 : 16;
            _touchPoint.Height = isPressed ? 20 : 16;

            Canvas.SetLeft(_touchPoint, x - _touchPoint.Width / 2);
            Canvas.SetTop(_touchPoint, y - _touchPoint.Height / 2);
            _touchPoint.Visibility = Visibility.Visible;

            // 更新连线
            _touchLine.X1 = _center.X;
            _touchLine.Y1 = _center.Y;
            _touchLine.X2 = x;
            _touchLine.Y2 = y;
            _touchLine.Visibility = (normalizedX != 0 || normalizedY != 0) ? Visibility.Visible : Visibility.Collapsed;

            // 更新坐标文本
            _coordsText.Text = $"({normalizedX:F2}, {normalizedY:F2})";
            Canvas.SetLeft(_coordsText, x - _coordsText.ActualWidth / 2);
            Canvas.SetTop(_coordsText, y + 15);
            _coordsText.Visibility = (normalizedX != 0 || normalizedY != 0) ? Visibility.Visible : Visibility.Collapsed;

            // 更新轨迹
            if (_showTrail)
            {
                DrawTrail();
            }

            // 更新手势指示器
            UpdateGestureIndicator(gesture);
        }

        /// <summary>
        /// 绘制轨迹
        /// </summary>
        private void DrawTrail()
        {
            // 清除旧的轨迹
            foreach (var element in _trailElements)
            {
                TouchpadCanvas.Children.Remove(element);
            }
            _trailElements.Clear();

            if (_touchpadHistory.Count < 2)
                return;

            // 绘制新轨迹
            for (int i = 1; i < _touchpadHistory.Count; i++)
            {
                var prev = _touchpadHistory[i - 1];
                var curr = _touchpadHistory[i];

                // 如果中间有断点，不连接
                if ((DateTime.Now - curr.Timestamp).TotalSeconds > 1)
                    continue;

                double prevX = _center.X + prev.X * _radius;
                double prevY = _center.Y + prev.Y * _radius;
                double currX = _center.X + curr.X * _radius;
                double currY = _center.Y + curr.Y * _radius;

                var line = new Line
                {
                    X1 = prevX,
                    Y1 = prevY,
                    X2 = currX,
                    Y2 = currY,
                    Stroke = new SolidColorBrush(Colors.Blue),
                    StrokeThickness = 2,
                    Opacity = 0.5
                };

                TouchpadCanvas.Children.Add(line);
                _trailElements.Add(line);
            }
        }

        /// <summary>
        /// 更新手势指示器
        /// </summary>
        private void UpdateGestureIndicator(EnumsNS.TouchpadGesture gesture)
        {
            // 移除旧的手势指示器
            if (_gestureIndicator != null)
            {
                TouchpadCanvas.Children.Remove(_gestureIndicator);
                _gestureIndicator = null;
            }

            if (gesture == EnumsNS.TouchpadGesture.None)
                return;

            // 根据手势类型创建指示器
            _gestureIndicator = CreateGestureIndicator(gesture);
            if (_gestureIndicator != null)
            {
                TouchpadCanvas.Children.Add(_gestureIndicator);
            }
        }

        /// <summary>
        /// 创建手势指示器
        /// </summary>
        private UIElement? CreateGestureIndicator(EnumsNS.TouchpadGesture gesture)
        {
            double angle = 0;
            Windows.UI.Color color = Colors.Gray;

            switch (gesture)
            {
                case EnumsNS.TouchpadGesture.SwipeRight:
                    angle = 0;
                    color = Colors.Indigo;
                    break;
                case EnumsNS.TouchpadGesture.SwipeDown:
                    angle = 90;
                    color = Colors.Orange;
                    break;
                case EnumsNS.TouchpadGesture.SwipeLeft:
                    angle = 180;
                    color = Colors.Teal;
                    break;
                case EnumsNS.TouchpadGesture.SwipeUp:
                    angle = 270;
                    color = Colors.Green;
                    break;
                default:
                    // 为None手势创建一个空指示器而不是返回null
                    var emptyShape = new Ellipse
                    {
                        Width = 10,
                        Height = 10,
                        Fill = new SolidColorBrush(Colors.Transparent)
                    };
                    return emptyShape;
            }

            // 转换为弧度
            double radians = angle * Math.PI / 180;

            // 创建扇形背景
            var path = new Path
            {
                Fill = new SolidColorBrush(color) { Opacity = 0.3 }
            };

            var geometry = new PathGeometry();
            var figure = new PathFigure();

            figure.StartPoint = _center;

            // 添加弧线
            figure.Segments.Add(new ArcSegment
            {
                Point = new Point(
                    _center.X + Math.Cos(radians + Math.PI / 4) * _radius,
                    _center.Y + Math.Sin(radians + Math.PI / 4) * _radius),
                Size = new Size(_radius, _radius),
                SweepDirection = SweepDirection.Clockwise,
                IsLargeArc = false
            });

            // 添加弧线
            figure.Segments.Add(new ArcSegment
            {
                Point = new Point(
                    _center.X + Math.Cos(radians - Math.PI / 4) * _radius,
                    _center.Y + Math.Sin(radians - Math.PI / 4) * _radius),
                Size = new Size(_radius, _radius),
                SweepDirection = SweepDirection.Counterclockwise,
                IsLargeArc = false
            });

            figure.IsClosed = true;
            geometry.Figures.Add(figure);
            path.Data = geometry;

            // 创建箭头
            var arrowLine = new Line
            {
                X1 = _center.X,
                Y1 = _center.Y,
                X2 = _center.X + Math.Cos(radians) * _radius * 0.7,
                Y2 = _center.Y + Math.Sin(radians) * _radius * 0.7,
                Stroke = new SolidColorBrush(color),
                StrokeThickness = 3,
                StrokeEndLineCap = PenLineCap.Triangle
            };

            // 创建容器
            var container = new Canvas();
            container.Children.Add(path);
            container.Children.Add(arrowLine);

            return container;
        }

        /// <summary>
        /// 获取手势文本
        /// </summary>
        private string GetGestureText(EnumsNS.TouchpadGesture gesture)
        {
            return gesture switch
            {
                EnumsNS.TouchpadGesture.None => "无",
                EnumsNS.TouchpadGesture.SwipeUp => "向上滑动",
                EnumsNS.TouchpadGesture.SwipeDown => "向下滑动",
                EnumsNS.TouchpadGesture.SwipeLeft => "向左滑动",
                EnumsNS.TouchpadGesture.SwipeRight => "向右滑动",
                EnumsNS.TouchpadGesture.Tap => "点击",
                EnumsNS.TouchpadGesture.LongPress => "长按",
                _ => "未知"
            };
        }

        /// <summary>
        /// 清除历史轨迹
        /// </summary>
        public void ClearHistory()
        {
            _touchpadHistory.Clear();

            // 更新UI
            _dispatcherQueue.TryEnqueue(() =>
            {
                // 清除轨迹
                foreach (var element in _trailElements)
                {
                    TouchpadCanvas.Children.Remove(element);
                }
                _trailElements.Clear();

                // 清除手势指示器
                if (_gestureIndicator != null)
                {
                    TouchpadCanvas.Children.Remove(_gestureIndicator);
                    _gestureIndicator = null;
                }

                // 重置状态文本
                GestureText.Text = "无";
            });
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

        private void TouchpadCanvas_SizeChanged(object sender, SizeChangedEventArgs e)
        {
            _canvasWidth = e.NewSize.Width;
            _canvasHeight = e.NewSize.Height;
            _radius = Math.Min(_canvasWidth, _canvasHeight) / 2 - 20;
            _center = new Point(_canvasWidth / 2, _canvasHeight / 2);

            // 重新初始化网格
            InitializeGrid();

            // 重新绘制轨迹
            if (_showTrail)
            {
                DrawTrail();
            }

            Debug.WriteLine($"画布大小变化: 宽={_canvasWidth}, 高={_canvasHeight}, 半径={_radius}");
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
        /// 更新触摸板数据并刷新显示
        /// </summary>
        public void UpdateTouchpadData(double normalizedX, double normalizedY, bool isTouching, EnumsNS.TouchpadGesture gesture)
        {
            DispatcherQueue.TryEnqueue(() =>
            {
                // 检查是否需要转换原始值
                double processedX = normalizedX;
                double processedY = normalizedY;

                _currentX = processedX;
                _currentY = processedY;
                _isTouching = isTouching;

                UpdateVisualization(gesture);
            });
        }

        private void UpdateVisualization(EnumsNS.TouchpadGesture gesture)
        {
            // 检查尺寸是否有效
            if (_touchpadSize <= 0 || TouchpadCanvas.ActualWidth <= 0 || TouchpadCanvas.ActualHeight <= 0)
                return;

            // 计算触摸点在画布上的位置
            double canvasCenterX = TouchpadCanvas.ActualWidth / 2;
            double canvasCenterY = TouchpadCanvas.ActualHeight / 2;

            double pointRadius = _touchpadSize / 2;
            double pointX = canvasCenterX + (_currentX * pointRadius);
            double pointY = canvasCenterY - (_currentY * pointRadius); // Y轴向上为正

            // 更新触摸点位置
            TouchPoint.Margin = new Thickness(pointX - TouchPoint.Width / 2, pointY - TouchPoint.Height / 2, 0, 0);

            // 更新触摸点可见性
            TouchPoint.Visibility = _isTouching ? Visibility.Visible : Visibility.Collapsed;

            // 如果正在触摸，添加历史轨迹点
            if (_isTouching)
            {
                AddHistoryPoint(pointX, pointY);
            }

            // 更新信息文本
            TouchpadInfoText.Text = _isTouching
                ? $"触摸位置: X={_currentX:F2}, Y={_currentY:F2}"
                : "未检测到触摸";

            // 更新手势文本
            UpdateGestureInfo(gesture);
        }

        private void AddHistoryPoint(double x, double y)
        {
            // 创建一个新的历史点
            var historyPoint = new Ellipse
            {
                Width = 8,
                Height = 8,
                Fill = new SolidColorBrush(Colors.Blue),
                Opacity = 0.5,
                Margin = new Thickness(x - 4, y - 4, 0, 0)
            };

            // 添加到历史轨迹集合
            _historyPoints.Add(historyPoint);
            HistoryCanvas.Children.Add(historyPoint);

            // 创建淡出动画
            var fadeAnimation = new DoubleAnimation
            {
                From = 0.5,
                To = 0.1,
                Duration = new Duration(TimeSpan.FromSeconds(1))
            };

            Storyboard.SetTarget(fadeAnimation, historyPoint);
            Storyboard.SetTargetProperty(fadeAnimation, "Opacity");

            var storyboard = new Storyboard();
            storyboard.Children.Add(fadeAnimation);
            storyboard.Begin();

            // 限制历史轨迹点的数量
            if (_historyPoints.Count > MaxHistoryPoints)
            {
                var oldestPoint = _historyPoints[0];
                _historyPoints.RemoveAt(0);
                HistoryCanvas.Children.Remove(oldestPoint);
            }
        }

        /// <summary>
        /// 更新手势信息文本
        /// </summary>
        private void UpdateGestureInfo(EnumsNS.TouchpadGesture gesture)
        {
            string gestureText = gesture switch
            {
                EnumsNS.TouchpadGesture.None => "无手势",
                EnumsNS.TouchpadGesture.Tap => "点击",
                EnumsNS.TouchpadGesture.DoubleTap => "双击",
                EnumsNS.TouchpadGesture.SwipeUp => "向上滑动",
                EnumsNS.TouchpadGesture.SwipeDown => "向下滑动",
                EnumsNS.TouchpadGesture.SwipeLeft => "向左滑动",
                EnumsNS.TouchpadGesture.SwipeRight => "向右滑动",
                EnumsNS.TouchpadGesture.Circle => "画圈",
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