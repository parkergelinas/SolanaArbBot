PRIMARY RULE:
Follow docs/system_contract.md as the absolute source of truth.

ARCHITECTURE RULES:
- Do NOT modify crate boundaries
- Do NOT introduce new crates
- Do NOT change data flow
- All logic must conform to defined pipeline

HOT PATH RULES:
(stream, decoder, pricing, graph, routing, execution)

- no Arc<RwLock<>>
- no blocking in async loops
- minimize allocations
- deterministic outputs only

CODE RULES:
- Rust 2021
- full unit tests for all logic
- prefer explicit types over inference in core logic
- use crossbeam for eventing

FORBIDDEN:
- architectural redesigns
- “global refactors”
- merging multiple crates in one task