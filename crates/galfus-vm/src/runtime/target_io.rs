use super::*;

impl VirtualMachine {
    pub(super) fn value_to_bytes(&self, val: Value) -> Result<Vec<u8>, VmError> {
        let mut bytes = Vec::new();
        match val {
            Value::Object(obj_ref) => {
                let heap_obj = self.get_object(obj_ref)?;
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

    pub(super) fn bytes_to_uint8_array(&mut self, bytes: Vec<u8>) -> Value {
        let element_ty = self
            .image
            .types
            .iter()
            .position(|ty| matches!(ty, ImageType::Uint8))
            .map(|idx| TypeIdx(idx as u16))
            .unwrap_or(TypeIdx(7));
        let obj = HeapObject::Array {
            element_ty,
            elements: bytes.into_iter().map(Value::Uint8).collect(),
        };
        Value::Object(self.alloc(obj))
    }

    pub(super) fn execute_read(&mut self, terminator: Value) -> Result<Value, VmError> {
        let terminator = self.byte_array_value(terminator)?;
        let input = self
            .context
            .providers
            .as_mut()
            .and_then(|providers| providers.io_mut())
            .ok_or(VmError::IoProviderUnavailable { operation: "read" })?
            .read(terminator.as_slice())
            .map_err(|error| VmError::IoError(error.message().to_string()))?;

        let bytes = match input {
            galfus_host::IoRead::Bytes(bytes) => bytes,
            galfus_host::IoRead::EndOfInput => Vec::new(),
        };
        Ok(self.bytes_to_uint8_array(bytes))
    }

    fn byte_array_value(&self, value: Value) -> Result<Vec<u8>, VmError> {
        let Value::Object(object) = value else {
            return Err(VmError::TypeMismatch {
                expected: "[u8]".to_string(),
                found: format!("{value:?}"),
            });
        };
        let heap_object = self.get_object(object)?;
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

    pub(super) fn execute_write(&mut self, val: Value) -> Result<(), VmError> {
        let bytes = self.value_to_bytes(val)?;
        self.context
            .providers
            .as_mut()
            .and_then(|providers| providers.io_mut())
            .ok_or(VmError::IoProviderUnavailable { operation: "write" })?
            .write(bytes.as_slice())
            .map_err(|error| VmError::IoError(error.message().to_string()))
    }
}
