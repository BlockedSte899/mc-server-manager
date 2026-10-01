use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ServerMetrics {
    pub id: String,
    pub cpu_percent: Option<f32>,
    pub mem_bytes: Option<u64>,
    pub system_cpu_percent: f32,
    pub system_mem_used: u64,
    pub system_mem_total: u64,
}
