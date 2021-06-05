use crate::Scoop;
use std::collections::HashMap;
use sysinfo::{Process, ProcessExt, System, SystemExt};

#[derive(Debug)]
pub struct SysTool(System);

impl SysTool {
    pub fn new() -> SysTool {
        SysTool(System::default())
    }

    pub fn running_apps(&mut self, scoop: &Scoop) -> HashMap<&usize, &Process> {
        // Find all running processes of installed Scoop apps.
        let root_path = scoop.config.root_path.as_path();
        self.0.refresh_processes();
        let processes = self
            .0
            .get_processes()
            .iter()
            .filter(|(_, p)| p.exe().starts_with(root_path))
            .collect::<HashMap<_, _>>();
        processes
    }
}
