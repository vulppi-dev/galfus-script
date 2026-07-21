# galfus-contract

`galfus-contract` defines the optional host integration contracts used by a Galfus
execution. It contains no target selection and no concrete platform adapter.

## Responsibilities

- **Providers**: Owns the optional providers supplied for one execution.
- **HostProvider**: Defines an asynchronous, message-based dispatch contract for executing native host capabilities.
- **HostValue & HostResponse**: Agnostic data representation for payloads passing between the Galfus VM and the Host.
- **MessageInjector**: Trait for injecting responses back into a suspended virtual thread.

Hosts construct `Providers` and pass them to `Workspace::run`. The CLI uses a
native host provider, while the playground uses a buffered host provider. If no
host provider is supplied, only executions that reach native calls (e.g. `std/io`) fail at runtime;
compilation and executions without native calls remain valid.
