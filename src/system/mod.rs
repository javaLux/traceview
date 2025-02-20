#![allow(dead_code)]
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::utils;

const MAX_VALUE_LENGTH: usize = 20;

/// Represents the specific System details of the underlying machine
#[derive(Debug, Default)]
pub struct SystemDetails {
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub total_space: u64,
    pub used_space: u64,
    pub system_name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub hostname: String,
    pub cpu_cores: usize,
    pub cpu_arch: String,
    pub cpu_usage: f32,
    system: System,
    disks: Disks,
}

impl SystemDetails {
    pub fn default() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        // we update only the cpu and memory based system information
        system.refresh_cpu_all();
        system.refresh_memory();

        // collect the required system information that can be None
        let system_name = match System::name() {
            Some(name) => utils::reduce_string_and_fill_with_dots(&name, MAX_VALUE_LENGTH),
            None => "Unknown".to_string(),
        };
        let kernel_version = match System::kernel_version() {
            Some(version) => utils::reduce_string_and_fill_with_dots(&version, MAX_VALUE_LENGTH),
            None => "Unknown".to_string(),
        };
        let os_version = match System::os_version() {
            Some(version) => utils::reduce_string_and_fill_with_dots(&version, MAX_VALUE_LENGTH),
            None => "Unknown".to_string(),
        };
        let hostname = match System::host_name() {
            Some(name) => utils::reduce_string_and_fill_with_dots(&name, MAX_VALUE_LENGTH),
            None => "Unknown".to_string(),
        };
        let cpu_arch =
            utils::reduce_string_and_fill_with_dots(&System::cpu_arch(), MAX_VALUE_LENGTH);

        let disks = Disks::new_with_refreshed_list();

        let (total_space, available_space) = Self::get_local_disk_space(&disks);
        let used_space = total_space - available_space;

        Self {
            total_memory: system.total_memory(),
            used_memory: system.used_memory(),
            total_swap: system.total_swap(),
            used_swap: system.used_swap(),
            total_space,
            used_space,
            system_name,
            kernel_version,
            os_version,
            hostname,
            cpu_cores: system.cpus().len(),
            cpu_usage: system.global_cpu_usage(),
            cpu_arch,
            system,
            disks,
        }
    }

    /// Refresh the CPU, Memory/Swap and disk usage
    pub fn refresh(&mut self) {
        self.disks.refresh(true);
        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        let (total_space, available_space) = Self::get_local_disk_space(&self.disks);
        let used_space = total_space - available_space;

        self.total_space = total_space;
        self.used_space = used_space;
        self.used_memory = self.system.used_memory();
        self.used_swap = self.system.used_swap();
        self.cpu_usage = self.system.global_cpu_usage();
    }

    /// Get the total and the available disk space of the first local disk
    /// # Returns
    /// - A tuple which contains two [u64] values. First the total disk space, second the available disk space
    /// - If no local disk was found, this function returns a tuple of zero [u64] values
    fn get_local_disk_space(disks: &Disks) -> (u64, u64) {
        disks
            .iter()
            .find(|disk| !disk.is_removable())
            .map_or((0_u64, 0_u64), |disk| {
                (disk.total_space(), disk.available_space())
            })
    }
}
