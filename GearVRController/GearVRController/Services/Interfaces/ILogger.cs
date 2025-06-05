using System;

namespace GearVRController.Services.Interfaces
{
    public interface ILogger
    {
        void LogInfo(string message, string source = "");
        void LogWarning(string message, string source = "");
        void LogError(string message, string source = "", Exception? ex = null);
        void LogDebug(string message, string source = "");
    }
}