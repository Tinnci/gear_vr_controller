using Microsoft.UI.Xaml.Data;
using System;

namespace GearVRController.Converters
{
    public class BoolToControlStateConverter : IValueConverter
    {
        public object Convert(object value, Type targetType, object parameter, string language)
        {
            if (value is bool isEnabled)
            {
                return isEnabled ? "禁用控制" : "启用控制";
            }
            return "启用控制";
        }

        public object ConvertBack(object value, Type targetType, object parameter, string language)
        {
            throw new NotImplementedException();
        }
    }
} 