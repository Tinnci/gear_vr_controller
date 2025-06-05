using System;

namespace GearVRController.Services.Interfaces
{
    /// <summary>
    /// 定义一个事件聚合器服务接口，用于发布和订阅事件。
    /// </summary>
    public interface IEventAggregator
    {
        /// <summary>
        /// 发布指定类型的事件。
        /// </summary>
        /// <typeparam name="TEvent">事件的类型。</typeparam>
        /// <param name="event">要发布的事件实例。</param>
        void Publish<TEvent>(TEvent @event);
        /// <summary>
        /// 订阅指定类型的事件。
        /// </summary>
        /// <typeparam name="TEvent">要订阅的事件类型。</typeparam>
        /// <param name="action">事件触发时执行的动作。</param>
        /// <returns>一个 IDisposable 对象，用于取消订阅。</returns>
        IDisposable Subscribe<TEvent>(Action<TEvent> action);
    }
}