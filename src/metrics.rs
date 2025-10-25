use std::time::Duration;
use sysinfo::{System, Pid, Networks};
use std::thread;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    pub duration_ms: u64,
    pub cpu_avg: f32,
    pub cpu_peak: f32,
    pub memory_mb: f64,
    pub network_sent: u64,
    pub network_received: u64,
}

pub struct MetricsCollector {
    pid: u32,
    cpu_samples: Arc<Mutex<Vec<f32>>>,
    memory_peak: Arc<Mutex<u64>>,
    network_start: (u64, u64),
    monitoring: Arc<Mutex<bool>>,
}

impl MetricsCollector {
    pub fn new(pid: u32) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let (sent, received) = Self::get_network_stats();
        
        Self {
            pid,
            cpu_samples: Arc::new(Mutex::new(Vec::new())),
            memory_peak: Arc::new(Mutex::new(0)),
            network_start: (sent, received),
            monitoring: Arc::new(Mutex::new(false)),
        }
    }
    
    pub fn start_monitoring(&mut self) {
        let pid = self.pid;
        let cpu_samples = Arc::clone(&self.cpu_samples);
        let memory_peak = Arc::clone(&self.memory_peak);
        let monitoring = Arc::clone(&self.monitoring);
        
        *monitoring.lock().unwrap() = true;
        
        thread::spawn(move || {
            let mut sys = System::new_all();
            
            while *monitoring.lock().unwrap() {
                sys.refresh_all();
                
                let pid_obj = Pid::from_u32(pid);
                
                if let Some(process) = sys.process(pid_obj) {
                    let cpu = process.cpu_usage();
                    cpu_samples.lock().unwrap().push(cpu);
                    let mem = process.memory();
                    let mut peak = memory_peak.lock().unwrap();
                    if mem > *peak {
                        *peak = mem;
                    }
                }
                
                thread::sleep(Duration::from_millis(100));
            }
        });
    }
    
    pub fn stop_monitoring(&self) {
        *self.monitoring.lock().unwrap() = false;
        thread::sleep(Duration::from_millis(150));
    }
    
    pub fn get_metrics(&self, duration: Duration) -> ExecutionMetrics {
        let samples = self.cpu_samples.lock().unwrap();
        
        let cpu_avg = if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f32>() / samples.len() as f32
        };
        
        let cpu_peak = samples.iter().cloned().fold(0.0f32, f32::max);
        let memory_mb = *self.memory_peak.lock().unwrap() as f64 / 1_024_000.0;
        
        let (sent_end, received_end) = Self::get_network_stats();
        
        ExecutionMetrics {
            duration_ms: duration.as_millis() as u64,
            cpu_avg,
            cpu_peak,
            memory_mb,
            network_sent: sent_end.saturating_sub(self.network_start.0),
            network_received: received_end.saturating_sub(self.network_start.1),
        }
    }
    
    fn get_network_stats() -> (u64, u64) {
        let networks = Networks::new_with_refreshed_list();
        let mut total_sent = 0;
        let mut total_received = 0;
        
        for (_name, network) in &networks {
            total_sent += network.total_transmitted();
            total_received += network.total_received();
        }
        
        (total_sent, total_received)
    }
}