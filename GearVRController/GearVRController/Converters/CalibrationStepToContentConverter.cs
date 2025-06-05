using Microsoft.UI.Xaml.Data;
using GearVRController.ViewModels;
using System;

namespace GearVRController.Converters
{
    public class CalibrationStepToContentConverter : IValueConverter
    {
        public object Convert(object value, Type targetType, object parameter, string language)
        {
            if (value is CalibrationStep currentStep && parameter is string stepParameter)
            {
                if (Enum.TryParse(stepParameter, out CalibrationStep targetStep))
                {
                    // 如果当前步骤已经超过目标步骤，说明目标步骤已经完成
                    if (currentStep > targetStep)
                    {
                        return "\u2713"; // Unicode勾号
                    }
                    // 如果当前步骤就是目标步骤，显示数字
                    else if (currentStep == targetStep)
                    {
                        // 根据CalibrationStep的枚举值映射到对应的数字
                        return ((int)targetStep).ToString();
                    }
                    // 否则，显示数字 (未开始的步骤)
                    else
                    {
                        return ((int)targetStep).ToString();
                    }
                }
            }
            return "?"; // 默认值
        }

        public object ConvertBack(object value, Type targetType, object parameter, string language)
        {
            throw new NotImplementedException();
        }
    }
}