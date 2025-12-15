# Relay Server Refactoring Plan v2 (Revised)

## 1. Current State Analysis

### Strengths
- **Ports & Adapters Pattern Adoption**: The introduction of `src/ports` and `src/services` (specifically `TradeCopyService`) demonstrates a move towards a clean, testable architecture.
- **Dependency Injection**: `TradeCopyService` uses `Arc<Trait>` which allows for easy mocking and testing.
- **Separation of Concerns (Partial)**: `TradeCopyService` is well-separated from the ZMQ transport layer.

### Identified Issues
1.  **Mixing of Concerns**:
    -   `message_handler` acts as both a controller and a business logic container (e.g., `heartbeat.rs`).
    -   Handlers directly depend on concrete implementations (`db`, `zeromq`), making refactoring difficult.
2.  **Misplaced Adapter Implementations**:
    -   Outbound adapters (Repository implementations) are nested inside inbound handlers.
3.  **Ambiguous Directory Names**:
    -   `engine` vs `services` confusion.
    -   `models` contains logic (`status_engine.rs`) which should be in the domain layer.

## 2. Target Architecture (Hexagonal / Clean Architecture)

We will adopt a strict Hexagonal Architecture. The dependency rule applies: **Adapters -> Application -> Domain**.

```text
relay-server/src/
├── adapters/                 # Interface Adapters (External World)
│   ├── inbound/
│   │   ├── http/             # (From api) Axum handlers
│   │   └── zmq/              # (From message_handler) ZMQ controllers
│   └── outbound/
│       ├── persistence/      # (From db) Database implementations of Repositories
│       ├── messaging/        # (From zeromq) ZMQ Publisher implementations
│       └── notifications/    # Broadcast/Logging adapters
├── application/              # Application Services (Use Cases)
│   ├── trade_copy.rs         # TradeCopyService
│   ├── status.rs             # (New) StatusService (extracted from heartbeat)
│   └── config.rs             # (New) ConfigurationService
├── domain/                   # Enterprise Business Logic & Pure Data
│   ├── models/               # Pure Structs/Enums (No heavy logic)
│   └── services/             # Domain Services (e.g., CopyEngine, StatusEngine logic)
├── ports/                    # Input/Output Ports (Traits/Interfaces)
│   ├── outbound.rs           # Repositories, Publishers, Notifiers Traits
│   └── inbound.rs            # (Optional) Service Interfaces
└── lib.rs                    # Composition Root (Dependency Injection wiring)

```

> **Note on Config**: The global `config` module should be carefully placed to avoid circular dependencies. Ideally, it sits in `domain/config` or a shared `infrastructure` module, but can remain at the root temporarily if strict layering is maintained.

##3. Refactoring Roadmap (Revised)We will prioritize **Logic Extraction** before **File Movement** to minimize compilation errors and "fighting the borrow checker."

###Phase 1: Abstraction & Logic Extraction (Safety First)*Goal: Separate business logic from ZMQ/DB dependencies without changing directory structure yet.*

* [ ] **Step 1: Define Ports (Traits)**
* Create `src/ports/outbound.rs`.
* Define traits for `TradeGroupRepository`, `ConnectionManager`, `ConfigPublisher`, etc.


* [ ] **Step 2: Extract `StatusService**`
* Create `src/services/status_service.rs` (temporary location).
* Move business logic from `heartbeat.rs` to `StatusService`.
* Ensure `StatusService` depends **only** on `Arc<dyn Trait>` (Ports), not concrete DB/ZMQ structs.


* [ ] **Step 3: Refactor `heartbeat.rs` to a Controller**
* Rewrite `heartbeat.rs` to simply deserialize messages and call `StatusService`.
* Inject `StatusService` into the handler.



###Phase 2: Structural Reorganization (The Big Move)*Goal: Move files to the new Hexagonal structure. Since logic is already decoupled, this is mostly file moving and import fixing.*

* [ ] **Move Inbound Adapters**
* `src/api` -> `src/adapters/inbound/http`
* `src/message_handler` -> `src/adapters/inbound/zmq`


* [ ] **Move Outbound Adapters**
* `src/db` -> `src/adapters/outbound/persistence`
* `src/zeromq` -> `src/adapters/outbound/messaging`


* [ ] **Move Application Services**
* `src/services/*` -> `src/application/`



###Phase 3: Domain Purification & Cleanup*Goal: Refine the Domain layer and finalize DI.*

* [ ] **Clean up `models**`
* Move logic (like `status_engine.rs`) to `src/domain/services/`.
* Ensure `src/domain/models` contains only data structures (Entities/Value Objects).


* [ ] **Finalize Composition Root**
* Update `src/lib.rs` or `src/main.rs` to instantiate all Adapters and Services and wire them together.



##4. Implementation Guidelines1. **Dependency Direction**:
* `adapters` depends on `application` and `ports`.
* `application` depends on `domain` and `ports`.
* `domain` depends on nothing (standard library only).


2. **Testing**:
* Write unit tests for `StatusService` immediately after extraction, using Mock implementations of the Ports.
* This proves the benefit of the refactor early on.


3. **Incremental Commits**:
* Commit after defining Ports.
* Commit after extracting `StatusService`.
* Commit after moving each major directory. Do not try to move everything in one giant commit.
