#[cfg(test)]
mod tests;

use galfus_image::ModuleImage;
use galfus_target::TargetCapabilityProvider;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ModuleRegistry {
    modules: HashMap<String, Arc<ModuleImage>>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn register(&mut self, image: ModuleImage) -> Arc<ModuleImage> {
        let name = image.name.clone();
        let arc = Arc::new(image);
        self.modules.insert(name, arc.clone());
        arc
    }

    pub fn get(&self, name: &str) -> Option<Arc<ModuleImage>> {
        self.modules.get(name).cloned()
    }
}

pub struct RuntimeLoader {
    registry: Arc<Mutex<ModuleRegistry>>,
}

impl RuntimeLoader {
    pub fn new(registry: Arc<Mutex<ModuleRegistry>>) -> Self {
        Self { registry }
    }

    pub fn load(&self, image: ModuleImage) -> Arc<ModuleImage> {
        self.registry.lock().unwrap().register(image)
    }
}

pub struct LogicalThread {
    id: usize,
    state: ThreadState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Running,
    Suspended,
    Terminated,
}

impl LogicalThread {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            state: ThreadState::Running,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn state(&self) -> ThreadState {
        self.state
    }
}

pub struct Runtime {
    registry: Arc<Mutex<ModuleRegistry>>,
    threads: Vec<LogicalThread>,
    _capabilities: Box<dyn TargetCapabilityProvider>,
}

impl Runtime {
    pub fn new(capabilities: Box<dyn TargetCapabilityProvider>) -> Self {
        Self {
            registry: Arc::new(Mutex::new(ModuleRegistry::new())),
            threads: Vec::new(),
            _capabilities: capabilities,
        }
    }

    pub fn spawn_thread(&mut self) -> usize {
        let id = self.threads.len();
        self.threads.push(LogicalThread::new(id));
        id
    }

    pub fn threads(&self) -> &[LogicalThread] {
        &self.threads
    }

    pub fn registry(&self) -> Arc<Mutex<ModuleRegistry>> {
        self.registry.clone()
    }
}
