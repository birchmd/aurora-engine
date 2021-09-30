# Profiling the Engine

## Background

The purpose of profiling the engine is to measure how much NEAR gas is used by various parts of the code, with the ultimate goal being to minimize that gas usage.

The purpose of this document is to describe how the profiling is done, and anything we learn as a result of this work.

### Previous work and motivation

The `nearcore` runtime itself has a [profiling mechanism](https://github.com/near/nearcore/blob/e9c1cf1e00dcae033e46d2a34cfc048a2299078b/core/primitives-core/src/profile.rs#L49) at the level of whole transactions.
This allows us to see how much gas an entire operation will use. It can also break down that gas usage by each host function the runtime exposes.
This is already being used [in our code](https://github.com/aurora-is-near/aurora-engine/blob/0fe4f0506866bd8813b270760864d22723925962/engine-tests/src/test_utils/mod.rs#L239-L247) to get high level gas usage from transactions interacting with important [contracts like 1inch](https://github.com/aurora-is-near/aurora-engine/blob/0fe4f0506866bd8813b270760864d22723925962/engine-tests/src/tests/one_inch.rs#L17).
However, this profiling is insufficient because [70% or more of the gas usage is in running wasm instructions](https://github.com/aurora-is-near/aurora-engine/blob/0fe4f0506866bd8813b270760864d22723925962/engine-tests/src/tests/one_inch.rs#L111). Since most of our code is not interacting with host functions, the breakdown provided by `nearcore` is not very helpful in identifying hot-spots of gas usage in our code.
Therefore, part of the current work is to extend this profiling mechanism to allow us to measure what parts of our code contribute to gas usage, even when most of it as result of wasm instructions.

## Methodology

In order to enable gas profiling over custom parts of the code we introduce two new host functions to the NEAR runtime.
The first function enters a "scope" with a particular ID, and the second function exits the last entered scope.
These functions are added to our contract at whatever points we choose in order to learn more information about the gas usage by the code between these two calls.
Gas used in each scope is tracked in its own gas counter and can be viewed as part of the profile after the execution is complete.
Scopes can be nested by calling `enter_scope` multiple times before calling `exit_scope`. This property means that the sum of gas used in all scopes can exceed the total gas of the transaction. However, it can be useful to nest scopes when breaking down the cost of a particular operation.
If `enter_scope(ID)` is called, then this ID cannot be used in another enter call until after it has been exited.
This is to prevent possibly double-counting gas in a given scope.
It also helps to identify recursive code paths because you may accidentally enter the same scope twice if there is a recursive call.

### API

- `fn enter_scope(id: u32)`
- `fn exit_scope()`

### Implementation

This is implemented on @birchmd's fork of `nearcore` on the [`wasm-profiling-improved` branch](https://github.com/birchmd/nearcore/tree/wasm-profiling-improved).

### Usage

To use these host functions, we need to change the NEAR dependencies in the Aurora engine to this custom fork.
Since we also use `near-sdk-sim` in our tests, it too needs to be modified with the custom fork, and in turn the engine's dependency on `near-sdk-sim` needs to point to that change.
Moreover, the bulk of the logic for the engine is actually in the SputnikVM repo, and so a custom version of this library with the profiling functions included is also needed.
All these changes have been made on the following branches:

- https://github.com/aurora-is-near/near-sdk-rs/tree/wasm-profiling-improved
- https://github.com/aurora-is-near/sputnikvm/tree/wasm-profiling
- https://github.com/birchmd/aurora-engine/tree/scoped-profiling

## Results

### Breakdown of gas usage in 1inch pool deposit

- 91% of total gas usage happens in `transact_call` (function in SputnikVM)
- 15% of gas usage comes from setup done in `call_inner` (A function which is called recursively, so this 15% is over multiple `call_inner` invocations. It is called recursively because it is used for call started by the transaction, and all calls to other contracts made internally by the transaction)
- 5% of total gas usage is spent loading the 1inch smart contract code into memory (this cost is included in `call_inner` above, but I thought it was worth calling out explicitly since it is a pretty significant proportion for one operation)
- 23% of total gas usage is spent validating EVM opcodes (i.e. checking we have not yet run out of gas, and the opcode makes sense given the current state of the stack)
- 31% of total gas usage is spent evaluating EVM "internal" opcodes (codes that modify the stack but do not need to reach out to the backend/state)
- 15% of total gas usage is spent evaluating EVM "external" opcodes (codes that require state access)

Based on these numbers, I wonder if there is opportunity for performance optimization in loading the contract code, and in validating the opcodes (prior to execution).