# Galfus Builtins and Standard Library Reference

This document defines the Galfus standard library design, its API surfaces, and the permission/sandbox model.

---

## 1. Design Philosophy

The Galfus standard library is divided into two distinct tiers:

```txt
+---------------------------------------------------------+
|                Tier 2: Rich Utility Modules             |
|  (text, format, json, regex, math, path, http, crypto)  |
+---------------------------------------------------------+
                             |  uses (if needed)
                             v
+---------------------------------------------------------+
|         Tier 1: std/* (Thin Target Standard Surface)     |
|   (std/io, std/fs, std/net, std/time, std/random...)    |
+---------------------------------------------------------+
                             |
                             +---> Host/OS Capabilities (OS, WASM, Web, Embedded)
```

1. **Tier 1: `std/*` (Thin Target Standard Surface)**
   - Low-level, host/target-connected capabilities.
   - Minimal and clean interface matching the target surface.
   - Requires explicit permissions to access. By default, access is blocked under a closed sandbox.
   - Implementations are target-dependent (e.g. native OS calls, WASM imports, mobile bridge, or embedded registers).

2. **Tier 2: Rich Utility Modules**
   - Platform-agnostic utility libraries.
   - Often built on top of `std/*` or providing pure algorithmic tools (e.g. data structure logic, mathematics, regex parsing).
   - Higher-level, developer-friendly interfaces.

---

## 2. Sandbox and Permission Model

By default, any Galfus program runs in a **Closed Sandbox**. Access to low-level host resources through `std/*` is restricted.

### Default Sandbox State

- Attempting to import or use a `std/*` module without explicit permissions in the configuration causes a compilation or link-time capability error, or a runtime panic if loaded dynamically.
- System inputs, outputs, files, networking, process controls, and environment variable accesses are entirely blocked by default.

### Workspace Permissions Configuration

Permissions are explicitly declared in the module's `galfus.toml` file under the `[permissions]` section.

Example configuration:

```toml
[permissions]
# Allow specific directory scopes for reading and writing
"std/fs" = { read = ["/data/public", "./assets"], write = ["/data/temp"] }

# Allow connections only to specified domains/ports
"std/net" = { connect = ["api.example.com:443", "localhost:*"] }

# Allow environment variables read access to specific keys, and passing command-line args
"std/env" = { allow_args = true, env_permitted = ["^APP_.+$", "i"] }

# Allow exit codes and target-level execution controls
"std/process" = { allow_exit = true }
```

### Permission Inheritance & Propagation

- When a Tier 2 module (like `http`) uses a Tier 1 module (like `std/net`), the VM checks the calling context's permissions.
- A library module cannot bypass the sandbox restrictions configured for the main application bundle. The lowest common denominator of permissions applies.

---

## 3. Tier 1: `std/*` (Thin Target Standard Surface)

### `std/io`

Basic console and standard input/output stream interaction.

```galfus
# Write raw bytes to standard output or standard error (specified by file descriptor id)
# Returns the number of bytes written, or a negative error code
fn write(fd: int32, data: [uint8]): int32

# Read raw bytes from standard input (specified by fd) into the target buffer
# Returns the number of bytes read, or a negative error code
fn read(fd: int32, buffer: [uint8]): int32

# Log helper for diagnostic output; logs directly to the target environment's logger/sink
fn log(message: [uint8]): null
```

### `std/fs`

Direct filesystem access, mapped to OS level operations.

```galfus
external struct FileHandle {}

struct FileStat {
  size: uint64,
  is_dir: bool,
  modified: int64,
  created: int64,
}

# Open file path with mode and flags. Returns a FileHandle or null on failure
fn open(path: [uint8], flags: int32, mode: int32): FileHandle

# Read bytes from a specific offset into the buffer. Returns bytes read
fn read(file: FileHandle, offset: int64, buffer: [uint8]): int32

# Write bytes to a specific offset. Returns bytes written
fn write(file: FileHandle, offset: int64, data: [uint8]): int32

# Close the file handle, releasing resources
fn close(file: FileHandle): null

# Query metadata for a given path
fn stat(path: [uint8]): FileStat
```

### `std/net`

Raw TCP/UDP socket networking.

```galfus
external struct SocketHandle {}

# Connect to a target remote host/port. Returns a SocketHandle or null on failure
fn connect(address: [uint8]): SocketHandle

# Send raw bytes over the connection. Returns bytes sent
fn send(socket: SocketHandle, data: [uint8]): int32

# Receive raw bytes into the buffer. Returns bytes received
fn recv(socket: SocketHandle, buffer: [uint8]): int32

# Terminate the socket connection
fn close(socket: SocketHandle): null
```

### `std/time`

System-level and high-resolution timer access.

```galfus
# Return UTC UNIX timestamp in milliseconds
fn now(): int64

# Return monotonic time in nanoseconds/microseconds (for performance tracking)
fn monotonic(): int64

# Return system-specific timer ticks
fn ticks(): int64
```

### `std/env`

Process environment and runtime arguments.

```galfus
# Return list of command line arguments
fn args(): [[uint8]]

# Return value of environment variable key, or null if unset
fn get(key: [uint8]): [uint8]

# Return current working directory path
fn cwd(): [uint8]
```

### `std/random`

Secure target entropy access.

```galfus
# Fill target buffer with cryptographically secure random bytes from host entropy
fn randomBytes(buffer: [uint8]): null
```

### `std/process`

Process termination and control. (Available only on desktop/server targets).

```galfus
# Exit current process execution with the specified exit code status
fn exit(code: int32): null
```

---

## 4. Tier 2: Rich Utility Modules

These modules do not interact with the host OS directly unless using a configured and permitted `std/*` surface. They represent the main application programming API.

### `text`

Comprehensive text and string utility operations. (Since Galfus strings are UTF-8 `[uint8]` arrays, `text` provides UTF-8 validation and parsing).

- `fn length(s: [uint8]): int32` - Returns the UTF-8 character length (as opposed to raw byte count).
- `fn substring(s: [uint8], start: int32, len: int32): [uint8]` - Extract slice based on character offsets.
- `fn split(s: [uint8], separator: [uint8]): [[uint8]]` - Split a byte array string by a separator.
- `fn join(parts: [[uint8]], separator: [uint8]): [uint8]` - Join elements with a separator.
- `fn toUpper(s: [uint8]): [uint8]` / `fn toLower(s: [uint8]): [uint8]` - Case mapping.

### `format`

String templates, printf formatting, and base-level string conversion.

- `fn sprint(template: [uint8], args: [Any]): [uint8]` - Returns a formatted string.
- `fn parseInt(s: [uint8]): int64` / `fn parseFloat(s: [uint8]): float64` - Convert text representation to numeric types.
- `fn toString(val: Any): [uint8]` - Convert standard types to their string representation.

### `json`

Highly optimized JSON parsing and serialization.

- `fn parse(jsonBytes: [uint8]): Any` - Deserialize JSON bytes into dynamic structure or native types.
- `fn stringify(val: Any): [uint8]` - Serialize any structured data back into JSON UTF-8 bytes.

### `regex`

Regular expression pattern matching.

- `fn match(pattern: [uint8], text: [uint8]): bool` - Test if text matches regex.
- `fn find(pattern: [uint8], text: [uint8]): [[uint8]]` - Find capture groups of regex match.
- `fn replace(pattern: [uint8], text: [uint8], replacement: [uint8]): [uint8]` - Replace matches in text.

### `math`

Standard mathematical functions.

- Constants: `PI` (3.14159...), `E` (2.71828...).
- Functions: `sin(x)`, `cos(x)`, `tan(x)`, `log(x)`, `pow(base, exp)`, `sqrt(x)`, `ceil(x)`, `floor(x)`, `round(x)`.

### `path`

Algorithmic file path manipulation (purely textual).

- `fn join(parts: [[uint8]]): [uint8]` - Join path components safely according to target path separators.
- `fn dirname(path: [uint8]): [uint8]` - Get parent directory path segment.
- `fn basename(path: [uint8]): [uint8]` - Get filename portion.
- `fn extname(path: [uint8]): [uint8]` - Get file extension portion.

### `http`

High-level HTTP client and listener capabilities (built on `std/net` and `std/time`).

- `fn get(url: [uint8]): HttpResponse`
- `fn post(url: [uint8], body: [uint8], headers: Map): HttpResponse`
- `fn createServer(port: int32, handler: fn(HttpRequest): HttpResponse): HttpServer`

### `collections`

Standard utility structures and generic operations.

- `Map` / `Set` implementation wrappers and utilities.
- List operations: `filter`, `map`, `reduce`, `sort`, `reverse`.

### `crypto`

Cryptographic primitives and utilities (built on `std/random`).

- Hash functions: `sha256(data: [uint8]): [uint8]`, `md5(data: [uint8]): [uint8]`.
- Cipher helpers: `encrypt(data: [uint8], key: [uint8]): [uint8]`, `decrypt(data: [uint8], key: [uint8]): [uint8]`.
- Signatures: `sign(data: [uint8], privateKey: [uint8]): [uint8]`, `verify(data: [uint8], signature: [uint8], publicKey: [uint8]): bool`.
