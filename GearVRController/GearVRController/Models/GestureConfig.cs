using System.Collections.Generic;
using GearVRController.Enums;

namespace GearVRController.Models
{
    public class GestureConfig
    {
        public float Sensitivity { get; set; } = 0.3f;
        public bool ShowGestureHints { get; set; } = true;
        public Dictionary<GestureDirection, GestureAction> GestureActions { get; set; }

        public GestureConfig()
        {
            GestureActions = new Dictionary<GestureDirection, GestureAction>
            {
                { GestureDirection.Up, GestureAction.PageUp },
                { GestureDirection.Down, GestureAction.PageDown },
                { GestureDirection.Left, GestureAction.BrowserBack },
                { GestureDirection.Right, GestureAction.BrowserForward }
            };
        }

        public void SetGestureAction(GestureDirection direction, GestureAction action)
        {
            if (direction != GestureDirection.None)
            {
                GestureActions[direction] = action;
            }
        }

        public GestureAction GetGestureAction(GestureDirection direction)
        {
            return GestureActions.TryGetValue(direction, out var action) ? action : GestureAction.None;
        }
    }
} 