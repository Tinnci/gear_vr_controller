using Microsoft.UI.Xaml.Controls;

namespace GearVRController.Models
{
    public class StatusInfo
    {
        public string Message { get; }
        public InfoBarSeverity Severity { get; }

        public StatusInfo(string message, InfoBarSeverity severity)
        {
            Message = message;
            Severity = severity;
        }
    }
}