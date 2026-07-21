use galfus_contract::{HostProvider, HostResponse, HostValue, MessageInjector};
use std::io::{Read, Write};
use std::sync::Arc;

/// Synchronous terminal I/O for native Galfus hosts.
pub struct NativeIoProvider;

impl HostProvider for NativeIoProvider {
    fn dispatch(
        &mut self,
        thread_id: usize,
        method: &str,
        args: &[HostValue],
        injector: Arc<dyn MessageInjector>,
    ) {
        match method {
            "write" => {
                if let Some(HostValue::Bytes(bytes)) = args.first() {
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();
                    if let Err(e) = handle.write_all(bytes).and_then(|()| handle.flush()) {
                        injector
                            .inject_system_response(thread_id, HostResponse::Error(e.to_string()));
                        return;
                    }
                    injector
                        .inject_system_response(thread_id, HostResponse::Success(HostValue::Null));
                } else {
                    injector.inject_system_response(
                        thread_id,
                        HostResponse::Error("Invalid arguments for write".to_string()),
                    );
                }
            }
            "read" => {
                let terminator = if let Some(HostValue::Bytes(b)) = args.first() {
                    b.clone()
                } else {
                    injector.inject_system_response(
                        thread_id,
                        HostResponse::Error("Invalid arguments for read".to_string()),
                    );
                    return;
                };

                if terminator.is_empty() {
                    injector.inject_system_response(
                        thread_id,
                        HostResponse::Error("input terminator must not be empty".to_string()),
                    );
                    return;
                }

                let mut input = Vec::new();
                let mut byte = [0u8; 1];
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();

                loop {
                    match handle.read(&mut byte) {
                        Ok(0) if input.is_empty() => {
                            injector.inject_system_response(
                                thread_id,
                                HostResponse::Success(HostValue::Bytes(Vec::new())),
                            );
                            return;
                        }
                        Ok(0) => {
                            injector.inject_system_response(
                                thread_id,
                                HostResponse::Success(HostValue::Bytes(input)),
                            );
                            return;
                        }
                        Ok(_) => {
                            input.push(byte[0]);
                            if input.ends_with(&terminator) {
                                input.truncate(input.len() - terminator.len());
                                injector.inject_system_response(
                                    thread_id,
                                    HostResponse::Success(HostValue::Bytes(input)),
                                );
                                return;
                            }
                        }
                        Err(error) => {
                            injector.inject_system_response(
                                thread_id,
                                HostResponse::Error(error.to_string()),
                            );
                            return;
                        }
                    }
                }
            }
            _ => {
                injector.inject_system_response(
                    thread_id,
                    HostResponse::Error(format!("Method {} not found", method)),
                );
            }
        }
    }
}
