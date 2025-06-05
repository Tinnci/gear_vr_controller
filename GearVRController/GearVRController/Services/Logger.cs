using System;
using System.Diagnostics;
using GearVRController.Services.Interfaces;

namespace GearVRController.Services
{
    public class Logger : ILogger
    {
        public void LogInfo(string message, string source = "")
        {
            string logMessage = !string.IsNullOrEmpty(source) ? $"[INFO] [{source}] {message}" : $"[INFO] {message}";
            Debug.WriteLine(logMessage);
        }

        public void LogWarning(string message, string source = "")
        {
            string logMessage = !string.IsNullOrEmpty(source) ? $"[WARNING] [{source}] {message}" : $"[WARNING] {message}";
            Debug.WriteLine(logMessage);
        }

        public void LogError(string message, string source = "", Exception? ex = null)
        {
            string logMessage = !string.IsNullOrEmpty(source) ? $"[ERROR] [{source}] {message}" : $"[ERROR] {message}";
            if (ex != null)
            {
                logMessage += $"\nException: {ex.Message}\nStackTrace: {ex.StackTrace}";
            }
            Debug.WriteLine(logMessage);
        }
    }
}