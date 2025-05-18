# Component Relationships

This document outlines the relationships between key components in the Gini application architecture.

## System Architecture

```mermaid
graph TD
    A[Gini Main Application] --> B[Kernel]
    B --> C[Plugin System]
    B --> D[Event System]
    B --> E[Storage System]
    B --> F[UI Bridge]
    B --> G[Stage Manager]
    C --> H[Plugin Registry]
    C --> I[Plugin Loader]
    C --> J[Plugin Adapter]
    C --> K[Dependency Resolver]
    C --> L[Conflict Detector]
    D --> M[Event Dispatcher]
    D --> N[Event Manager]
    E --> O[Config Manager]
    E --> P[Local Storage]
    E --> Q[Storage Provider]
    G --> R[Pipeline Executor]
    G --> S[Stage Registry]
    G --> T[Stage Context]
    H -.-> K
    I -.-> K
    I -.-> L
    M -.-> N
    R -.-> S
    R -.-> T
```

## Key Interaction Flows

### Plugin Lifecycle

```mermaid
sequenceDiagram
    participant Main as Gini Application
    participant Kernel as Kernel
    participant PM as Plugin Manager
    participant Loader as Plugin Loader
    participant Registry as Plugin Registry
    participant SM as Stage Manager
    
    Main->>Kernel: Initialize
    Kernel->>PM: Initialize plugin system
    PM->>Loader: Discover plugins
    Loader->>Registry: Register discovered plugins
    Registry->>Registry: Resolve dependencies
    Registry->>Registry: Detect conflicts
    Registry->>PM: Return valid plugins
    PM->>SM: Execute plugin initialization stages
    SM->>PM: Plugins initialized
    PM->>Kernel: Plugin system ready
    Kernel->>Main: Initialization complete
```

### Event Handling

```mermaid
sequenceDiagram
    participant Component as Any Component
    participant Dispatcher as Event Dispatcher
    participant Manager as Event Manager
    participant Handlers as Event Handlers
    
    Component->>Dispatcher: Dispatch event
    Dispatcher->>Manager: Forward event
    Manager->>Handlers: Notify all registered handlers
    Handlers-->>Component: Handle event (possible callback)
```

## Module Dependencies

| Module | Depends On |
|--------|------------|
| Kernel | Event System, Plugin System, Storage System, UI Bridge |
| Plugin System | Event System, Storage System |
| Stage Manager | Plugin System, Event System |
| Storage System | - |
| Event System | - |
| UI Bridge | Event System |