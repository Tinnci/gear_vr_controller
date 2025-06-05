using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using GearVRController.Services.Interfaces;
using Microsoft.UI.Dispatching; // For DispatcherQueue

namespace GearVRController.Services
{
    /// <summary>
    /// EventAggregator 实现了发布/订阅模式，允许不同组件之间进行松散耦合的通信。
    /// 它负责管理事件订阅和发布，并确保事件在正确的（UI）线程上被处理。
    /// </summary>
    public class EventAggregator : IEventAggregator
    {
        /// <summary>
        /// 存储事件类型到其订阅处理程序列表的映射。
        /// ConcurrentDictionary 用于线程安全地管理订阅。
        /// </summary>
        private readonly ConcurrentDictionary<Type, List<object>> _subscriptions = new();
        /// <summary>
        /// 用于确保事件在 UI 线程上发布。
        /// </summary>
        private readonly DispatcherQueue _dispatcherQueue;

        /// <summary>
        /// EventAggregator 的构造函数。
        /// </summary>
        /// <param name="dispatcherQueue">DispatcherQueue 实例，用于在 UI 线程上调度事件发布。</param>
        public EventAggregator(DispatcherQueue dispatcherQueue)
        {
            _dispatcherQueue = dispatcherQueue;
        }

        /// <summary>
        /// 发布一个指定类型的事件。
        /// 所有订阅了该事件类型的处理程序都将在 UI 线程上被调用。
        /// </summary>
        /// <typeparam name="TEvent">要发布的事件类型。</typeparam>
        /// <param name="@event">要发布（传递给订阅者）的事件实例。</param>
        public void Publish<TEvent>(TEvent @event)
        {
            if (_subscriptions.TryGetValue(typeof(TEvent), out var handlers))
            {
                // Publish on the UI thread for UI-related events
                _dispatcherQueue.TryEnqueue(() =>
                {
                    // Create a copy to prevent issues if a handler unsubscribes during iteration
                    foreach (var handler in handlers.ToList())
                    {
                        if (handler is Action<TEvent> typedHandler)
                        {
                            typedHandler.Invoke(@event);
                        }
                    }
                });
            }
        }

        /// <summary>
        /// 订阅一个指定类型的事件。
        /// 当该类型的事件被发布时，提供的 `action` 将被执行。
        /// 返回一个 `IDisposable` 对象，可用于取消订阅。
        /// </summary>
        /// <typeparam name="TEvent">要订阅的事件类型。</typeparam>
        /// <param name="action">事件发生时要执行的动作。</param>
        /// <returns>一个 `IDisposable` 对象，调用其 `Dispose()` 方法可取消订阅。</returns>
        public IDisposable Subscribe<TEvent>(Action<TEvent> action)
        {
            var handlers = _subscriptions.GetOrAdd(typeof(TEvent), _ => new List<object>());
            lock (handlers)
            {
                handlers.Add(action);
            }
            return new Unsubscriber<TEvent>(_subscriptions, action);
        }

        /// <summary>
        /// 内部类，用于提供事件订阅的 `IDisposable` 实现。
        /// 当其 `Dispose()` 方法被调用时，它将从 EventAggregator 中取消对应的订阅。
        /// </summary>
        private class Unsubscriber<TEvent> : IDisposable
        {
            /// <summary>
            /// 对 EventAggregator 订阅字典的引用。
            /// </summary>
            private readonly ConcurrentDictionary<Type, List<object>> _subscriptions;
            /// <summary>
            /// 要取消订阅的动作。
            /// </summary>
            private readonly Action<TEvent> _action;

            /// <summary>
            /// Unsubscriber 的构造函数。
            /// </summary>
            /// <param name="subscriptions">EventAggregator 的订阅字典。</param>
            /// <param name="action">要取消订阅的动作。</param>
            public Unsubscriber(ConcurrentDictionary<Type, List<object>> subscriptions, Action<TEvent> action)
            {
                _subscriptions = subscriptions;
                _action = action;
            }

            /// <summary>
            /// 取消对事件的订阅。
            /// 从 EventAggregator 的订阅列表中移除对应的处理程序。
            /// </summary>
            public void Dispose()
            {
                if (_subscriptions.TryGetValue(typeof(TEvent), out var handlers))
                {
                    lock (handlers)
                    {
                        handlers.Remove(_action);
                    }
                }
            }
        }
    }
}