use super::*;

impl VirtualMachine {
    pub(super) fn value_to_bytes(
        &self,
        thread: &mut crate::thread::VirtualThread,
        val: Value,
    ) -> Result<Vec<u8>, VmError> {
        let mut bytes = Vec::new();
        match val {
            Value::Object(obj_ref) => {
                let heap_obj = thread.heap.get_object(obj_ref)?;
                if let HeapObject::Array { elements, .. } = heap_obj {
                    for elem in elements {
                        match elem {
                            Value::Int8(b) => bytes.push(*b as u8),
                            Value::Uint8(b) => bytes.push(*b),
                            _ => {
                                let s = format!("{:?}", elem);
                                bytes.extend_from_slice(s.as_bytes());
                            }
                        }
                    }
                } else {
                    let s = format!("{:?}", heap_obj);
                    bytes.extend_from_slice(s.as_bytes());
                }
            }
            other => match other {
                Value::Null => bytes.extend_from_slice(b"null"),
                Value::Bool(b) => bytes.extend_from_slice(if b { b"true" } else { b"false" }),
                Value::Int8(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Int16(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Int32(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Int64(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint8(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint16(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint32(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Uint64(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Float32(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Float64(b) => bytes.extend_from_slice(b.to_string().as_bytes()),
                Value::Function(idx) => {
                    bytes.extend_from_slice(format!("<function {}>", idx.0).as_bytes())
                }
                Value::Object(obj_ref) => {
                    let s = format!("ObjectRef({})", obj_ref.raw());
                    bytes.extend_from_slice(s.as_bytes());
                }
            },
        }
        Ok(bytes)
    }

    pub(super) fn bytes_to_uint8_array(
        &self,
        thread: &mut crate::thread::VirtualThread,
        bytes: Vec<u8>,
    ) -> Value {
        let element_ty = self
            .current_image(thread)
            .unwrap()
            .types
            .iter()
            .position(|ty| matches!(ty, BytecodeType::Uint8))
            .map(|idx| TypeIdx(idx as u16))
            .unwrap_or(TypeIdx(7));
        let obj = HeapObject::Array {
            element_ty,
            elements: bytes.into_iter().map(Value::Uint8).collect(),
        };
        Value::Object(thread.heap.alloc(obj))
    }

    pub(super) fn execute_read(
        &self,
        thread: &mut crate::thread::VirtualThread,
        terminator: Value,
    ) -> Result<Value, VmError> {
        let terminator = self.byte_array_value(thread, terminator)?;
        let mut providers_borrow = self
            .context
            .providers
            .as_ref()
            .ok_or(VmError::IoProviderUnavailable { operation: "read" })?
            .lock()
            .unwrap();

        let input = providers_borrow
            .io_mut()
            .ok_or(VmError::IoProviderUnavailable { operation: "read" })?
            .read(terminator.as_slice())
            .map_err(|error: galfus_contract::IoProviderError| {
                VmError::IoError(error.message().to_string())
            })?;

        let bytes = match input {
            galfus_contract::IoRead::Bytes(bytes) => bytes,
            galfus_contract::IoRead::EndOfInput => Vec::new(),
        };
        Ok(self.bytes_to_uint8_array(thread, bytes))
    }

    fn byte_array_value(
        &self,
        thread: &mut crate::thread::VirtualThread,
        value: Value,
    ) -> Result<Vec<u8>, VmError> {
        let Value::Object(object) = value else {
            return Err(VmError::TypeMismatch {
                expected: "[u8]".to_string(),
                found: format!("{value:?}"),
            });
        };
        let heap_object = thread.heap.get_object(object)?;
        let HeapObject::Array { elements, .. } = heap_object else {
            return Err(VmError::TypeMismatch {
                expected: "[u8]".to_string(),
                found: format!("{heap_object:?}"),
            });
        };

        elements
            .iter()
            .map(|element| match element {
                Value::Uint8(byte) => Ok(*byte),
                _ => Err(VmError::TypeMismatch {
                    expected: "[u8]".to_string(),
                    found: format!("{element:?}"),
                }),
            })
            .collect()
    }

    pub(super) fn execute_write(
        &self,
        thread: &mut crate::thread::VirtualThread,
        val: Value,
    ) -> Result<(), VmError> {
        let bytes = self.value_to_bytes(thread, val)?;
        let mut providers_borrow = self
            .context
            .providers
            .as_ref()
            .ok_or(VmError::IoProviderUnavailable { operation: "write" })?
            .lock()
            .unwrap();

        providers_borrow
            .io_mut()
            .ok_or(VmError::IoProviderUnavailable { operation: "write" })?
            .write(bytes.as_slice())
            .map_err(|error: galfus_contract::IoProviderError| {
                VmError::IoError(error.message().to_string())
            })
    }
}
