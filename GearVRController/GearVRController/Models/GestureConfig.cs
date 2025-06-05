using GearVRController.Enums;

namespace GearVRController.Models
{
    public class GestureConfig
    {
        public float Sensitivity { get; set; } = 0.3f;
        public bool ShowGestureHints { get; set; } = true;

        public GestureConfig()
        {
        }
    }
}