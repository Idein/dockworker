use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// response type of containers/stats api
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub id: String,
    pub name: String,
    pub read: String,
    pub networks: Option<HashMap<String, Network>>,
    #[serde(with = "format::memory_stats")]
    pub memory_stats: Option<MemoryStats>,
    pub cpu_stats: CpuStats,
    /// The precpu_stats is the CPU statistic of the previous read, and is used to calculate the CPU usage percentage.
    /// It is not an exact copy of the cpu_stats field.
    pub precpu_stats: CpuStats,
    pub blkio_stats: BlkioStats,
    /// The number of pids in the cgroup
    pub pids_stats: PidsStats,
}

impl Stats {
    pub fn used_memory(&self) -> Option<u64> {
        self.memory_stats
            .as_ref()
            .map(|mem| mem.usage - mem.stats.cache)
    }
    pub fn available_memory(&self) -> Option<u64> {
        self.memory_stats.as_ref().map(|mem| mem.limit)
    }
    /// memory usage %
    pub fn memory_usage(&self) -> Option<f64> {
        if let (Some(used_memory), Some(available_memory)) =
            (self.used_memory(), self.available_memory())
        {
            Some((used_memory as f64 / available_memory as f64) * 100.0)
        } else {
            None
        }
    }
    pub fn cpu_delta(&self) -> u64 {
        self.cpu_stats.cpu_usage.total_usage - self.precpu_stats.cpu_usage.total_usage
    }
    pub fn system_cpu_delta(&self) -> Option<u64> {
        if let (Some(cpu), Some(pre)) = (
            self.cpu_stats.system_cpu_usage,
            self.precpu_stats.system_cpu_usage,
        ) {
            Some(cpu - pre)
        } else {
            None
        }
    }
    /// If either `precpu_stats.online_cpus` or `cpu_stats.online_cpus` is nil then for
    /// compatibility with older daemons the length of the corresponding `cpu_usage.percpu_usage` array should be used.
    pub fn number_cpus(&self) -> u64 {
        if let Some(cpus) = self.cpu_stats.online_cpus {
            cpus
        } else {
            let empty = &[];
            self.cpu_stats
                .cpu_usage
                .percpu_usage
                .as_deref()
                .unwrap_or(empty)
                .len() as u64
        }
    }
    /// cpu usage %
    pub fn cpu_usage(&self) -> Option<f64> {
        self.system_cpu_delta().map(|system_cpu_delta| {
            (self.cpu_delta() as f64 / system_cpu_delta as f64) * self.number_cpus() as f64 * 100.0
        })
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct Network {
    pub rx_dropped: u64,
    pub rx_bytes: u64,
    pub rx_errors: u64,
    pub tx_packets: u64,
    pub tx_dropped: u64,
    pub rx_packets: u64,
    pub tx_errors: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct MemoryStats {
    pub max_usage: u64,
    pub usage: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failcnt: Option<u64>,
    pub limit: u64,
    pub stats: MemoryStat,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct MemoryStat {
    pub total_pgmajfault: u64,
    pub cache: u64,
    pub mapped_file: u64,
    pub total_inactive_file: u64,
    pub pgpgout: u64,
    pub rss: u64,
    pub total_mapped_file: u64,
    pub writeback: u64,
    pub unevictable: u64,
    pub pgpgin: u64,
    pub total_unevictable: u64,
    pub pgmajfault: u64,
    pub total_rss: u64,
    pub total_rss_huge: u64,
    pub total_writeback: u64,
    pub total_inactive_anon: u64,
    pub rss_huge: u64,
    pub hierarchical_memory_limit: u64,
    pub total_pgfault: u64,
    pub total_active_file: u64,
    pub active_anon: u64,
    pub total_active_anon: u64,
    pub total_pgpgout: u64,
    pub total_cache: u64,
    pub inactive_anon: u64,
    pub active_file: u64,
    pub pgfault: u64,
    pub inactive_file: u64,
    pub total_pgpgin: u64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct CpuStats {
    pub cpu_usage: CpuUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_cpu_usage: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online_cpus: Option<u64>,
    pub throttling_data: ThrottlingData,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct CpuUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percpu_usage: Option<Vec<u64>>,
    pub usage_in_usermode: u64,
    pub total_usage: u64,
    pub usage_in_kernelmode: u64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct ThrottlingData {
    pub periods: u64,
    pub throttled_periods: u64,
    pub throttled_time: u64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct BlkioStats {
    pub io_service_bytes_recursive: Option<Vec<BlkioStat>>,
    pub io_serviced_recursive: Option<Vec<BlkioStat>>,
    pub io_queue_recursive: Option<Vec<BlkioStat>>,
    pub io_service_time_recursive: Option<Vec<BlkioStat>>,
    pub io_wait_time_recursive: Option<Vec<BlkioStat>>,
    pub io_merged_recursive: Option<Vec<BlkioStat>>,
    pub io_time_recursive: Option<Vec<BlkioStat>>,
    pub sectors_recursive: Option<Vec<BlkioStat>>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct BlkioStat {
    pub major: u64,
    pub minor: u64,
    pub op: String,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PidsStats {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u64>,
}

mod format {
    use super::*;
    use serde::de::{DeserializeOwned, Deserializer};
    use serde::ser::{Serialize, Serializer};

    pub mod memory_stats {
        use super::*;

        #[derive(Debug, Serialize, Deserialize)]
        #[serde(untagged)]
        enum Plus1<T> {
            Item(T),
            Empty {},
        }

        impl<T> From<Plus1<T>> for Option<T> {
            fn from(value: Plus1<T>) -> Option<T> {
                match value {
                    Plus1::Item(t) => Some(t),
                    Plus1::Empty {} => None,
                }
            }
        }

        impl<T> From<Option<T>> for Plus1<T> {
            fn from(value: Option<T>) -> Plus1<T> {
                match value {
                    Option::Some(t) => Plus1::Item(t),
                    Option::None => Plus1::Empty {},
                }
            }
        }

        pub fn deserialize<'de, D, T>(de: D) -> std::result::Result<Option<T>, D::Error>
        where
            D: Deserializer<'de>,
            T: DeserializeOwned,
        {
            Plus1::<T>::deserialize(de).map(Into::into)
        }

        pub fn serialize<T, S>(t: &Option<T>, se: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: Serializer,
            T: Serialize,
        {
            Into::<Plus1<&T>>::into(t.as_ref()).serialize(se)
        }
    }
}
