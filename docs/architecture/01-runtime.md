# Runtime

## Purpose

The Runtime is responsible for executing and coordinating long-lived realtime components.

It owns their lifecycle, connects them together, and provides graceful startup and shutdown.

The Runtime itself contains no provider-specific or business-specific logic.

---

## Responsibilities

Runtime is responsible for:

- starting components;
- stopping components;
- supervising background tasks;
- propagating cancellation;
- connecting components together;
- reporting runtime events.

Runtime is NOT responsible for:

- audio processing;
- translation;
- protocol implementation;
- provider-specific logic.

---

## Runtime Model

```
TranslationBridge
        │
        ▼
    Runtime
        │
 ┌──────┴──────┐
 │             │
 ▼             ▼
TranslationLine
TranslationLine
```

The Runtime owns one or more TranslationLine instances and manages their execution.

---

## Components

A TranslationLine internally consists of several independent tasks.

```
TranslationLine

    AudioCapture

    AudioPlayback

    RealtimeSession

    Audio pipeline

    Background tasks
```

Runtime starts and supervises all of them.

---

## Lifecycle

```
Created

↓

Starting

↓

Running

↓

Stopping

↓

Stopped
```

Errors move the component into the Error state.

---

## Cancellation

Every long-running task must support cooperative cancellation.

Runtime never terminates threads forcefully.

Cancellation propagates from parent components to child components.

---

## Communication

Components communicate only through explicit interfaces.

Examples:

- channels;
- event buses;
- provider interfaces.

Shared mutable state should be avoided whenever possible.

---

## Threading

Realtime audio callbacks never perform network operations.

Network tasks never block audio callbacks.

Realtime and async worlds are connected using lock-free queues or channels.

---

## Goals

The Runtime should:

- be provider-independent;
- be transport-independent;
- support multiple TranslationLine instances;
- support future TranslationBridge implementation;
- allow embedding into desktop and server applications.
