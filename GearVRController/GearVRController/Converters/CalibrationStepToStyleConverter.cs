using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using static Microsoft.UI.Colors;
using GearVRController.ViewModels;
using System;
using Windows.UI;

namespace GearVRController.Converters
{
    public class CalibrationStepToStyleConverter : IValueConverter
    {
        public object Convert(object value, Type targetType, object parameter, string language)
        {
            if (value is CalibrationStep currentStep && parameter is string stepParameter)
            {
                if (Enum.TryParse(stepParameter, out CalibrationStep targetStep))
                {
                    if (currentStep == targetStep)
                    {
                        // 当前步骤：高亮显示 (从资源中获取主题强调色)
                        object accentColorResource = Application.Current.Resources["SystemAccentColor"];
                        if (accentColorResource is SolidColorBrush accentBrush)
                        {
                            return accentBrush;
                        }
                        else if (accentColorResource is Color accentColor)
                        {
                            return new SolidColorBrush(accentColor);
                        }
                        return new SolidColorBrush(Blue); // Fallback color, in case resource is neither
                    }
                    else if (currentStep > targetStep)
                    {
                        // 已完成的步骤：显示成功颜色
                        return new SolidColorBrush(Green);
                    }
                    else
                    {
                        // 未开始的步骤：保持透明或默认颜色
                        return new SolidColorBrush(Transparent);
                    }
                }
            }
            return new SolidColorBrush(Transparent); // 默认值
        }

        public object ConvertBack(object value, Type targetType, object parameter, string language)
        {
            throw new NotImplementedException();
        }
    }

    public class CalibrationStepToForegroundConverter : IValueConverter
    {
        public object Convert(object value, Type targetType, object parameter, string language)
        {
            if (value is CalibrationStep currentStep && parameter is string stepParameter)
            {
                if (Enum.TryParse(stepParameter, out CalibrationStep targetStep))
                {
                    if (currentStep == targetStep)
                    {
                        // 当前步骤：白色文本
                        return new SolidColorBrush(White);
                    }
                    else if (currentStep > targetStep)
                    {
                        // 已完成的步骤：白色文本
                        return new SolidColorBrush(White);
                    }
                    else
                    {
                        // 未开始的步骤：默认文本颜色
                        return (SolidColorBrush)Application.Current.Resources["SystemControlForegroundBaseHighBrush"];
                    }
                }
            }
            return (SolidColorBrush)Application.Current.Resources["SystemControlForegroundBaseHighBrush"];
        }

        public object ConvertBack(object value, Type targetType, object parameter, string language)
        {
            throw new NotImplementedException();
        }
    }
}