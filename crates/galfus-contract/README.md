# galfus-contract

`galfus-contract` defines the optional host integration contracts used by a Galfus
execution. It contains no target selection and no concrete platform adapter.

## Responsibilities

- **Providers**: Owns the optional providers supplied for one execution.
- **IoProvider**: Defines synchronous byte-stream reads and writes for `std/io`.
- **IoRead**: Distinguishes bytes read from end of input.
- **IoProviderError**: Reports an I/O operation failure to the runtime.

Hosts construct `Providers` and pass them to `Workspace::run`. The CLI uses a
native stream provider, while the playground uses a buffered provider. If no
I/O provider is supplied, only executions that reach `std/io` fail at runtime;
compilation and executions without I/O remain valid.
