using System.Threading.Tasks;

namespace GearVRController.Services.Interfaces
{
    public interface ISettingsService
    {
        double MouseSensitivity { get; set; }
        bool IsMouseEnabled { get; set; }
        bool IsKeyboardEnabled { get; set; }
        bool IsControlEnabled { get; set; }
        bool UseNaturalScrolling { get; set; }
        bool InvertYAxis { get; set; }
        bool EnableAutoCalibration { get; set; }
        void ResetToDefaults();
        Task SaveSettingsAsync();
        Task LoadSettingsAsync();
    }
}