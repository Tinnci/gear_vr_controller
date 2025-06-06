using GearVRController.Enums;

namespace GearVRController.Events
{
    public class GestureExecutedEvent
    {
        public GestureDirection DetectedDirection { get; }
        public GestureAction ExecutedAction { get; }

        public GestureExecutedEvent(GestureDirection detectedDirection, GestureAction executedAction)
        {
            DetectedDirection = detectedDirection;
            ExecutedAction = executedAction;
        }
    }
}