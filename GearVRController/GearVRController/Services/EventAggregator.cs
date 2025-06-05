using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using GearVRController.Services.Interfaces;
using Microsoft.UI.Dispatching; // For DispatcherQueue

namespace GearVRController.Services
{
    public class EventAggregator : IEventAggregator
    {
        private readonly ConcurrentDictionary<Type, List<object>> _subscriptions = new();
        private readonly DispatcherQueue _dispatcherQueue;

        public EventAggregator(DispatcherQueue dispatcherQueue)
        {
            _dispatcherQueue = dispatcherQueue;
        }

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

        public IDisposable Subscribe<TEvent>(Action<TEvent> action)
        {
            var handlers = _subscriptions.GetOrAdd(typeof(TEvent), _ => new List<object>());
            lock (handlers)
            {
                handlers.Add(action);
            }
            return new Unsubscriber<TEvent>(_subscriptions, action);
        }

        private class Unsubscriber<TEvent> : IDisposable
        {
            private readonly ConcurrentDictionary<Type, List<object>> _subscriptions;
            private readonly Action<TEvent> _action;

            public Unsubscriber(ConcurrentDictionary<Type, List<object>> subscriptions, Action<TEvent> action)
            {
                _subscriptions = subscriptions;
                _action = action;
            }

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