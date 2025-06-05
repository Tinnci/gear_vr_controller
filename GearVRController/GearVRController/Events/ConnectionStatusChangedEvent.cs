using System;

namespace GearVRController.Events
{
    /// <summary>
    /// 当蓝牙连接状态改变时发布的事件。
    /// </summary>
    public class ConnectionStatusChangedEvent : EventArgs
    {
        public bool IsConnected { get; }

        public ConnectionStatusChangedEvent(bool isConnected)
        {
            IsConnected = isConnected;
        }
    }
}