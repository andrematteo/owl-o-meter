use crate::metrics::ExecutionMetrics;

#[derive(Debug, Clone, Copy)]
pub enum AwsRegion {
    UsEast1,      // Norte da Virgínia
    UsWest2,      // Oregon
    EuWest1,      // Irlanda
    ApSoutheast1, // Singapura
    SaEast1,      // São Paulo
}

impl AwsRegion {
    pub fn name(&self) -> &str {
        match self {
            Self::UsEast1 => "us-east-1 (N. Virginia)",
            Self::UsWest2 => "us-west-2 (Oregon)",
            Self::EuWest1 => "eu-west-1 (Ireland)",
            Self::ApSoutheast1 => "ap-southeast-1 (Singapore)",
            Self::SaEast1 => "sa-east-1 (São Paulo)",
        }
    }
}

pub struct CostEstimator {
    pricing: RegionPricing,
}

struct RegionPricing {
    lambda_gb_second: f64,
    lambda_per_million_requests: f64,
  
    fargate_vcpu_hour: f64,
    fargate_gb_hour: f64,
    eks_cluster_hour: f64,
}

impl CostEstimator {
    pub fn new(region: AwsRegion) -> Self {
        let pricing = match region {
            AwsRegion::UsEast1 => RegionPricing {
                lambda_gb_second: 0.0000166667,
                lambda_per_million_requests: 0.20,
                fargate_vcpu_hour: 0.04048,
                fargate_gb_hour: 0.004445,
                eks_cluster_hour: 0.10,
            },
            AwsRegion::UsWest2 => RegionPricing {
                lambda_gb_second: 0.0000166667,
                lambda_per_million_requests: 0.20,
                fargate_vcpu_hour: 0.04048,
                fargate_gb_hour: 0.004445,
                eks_cluster_hour: 0.10,
            },
            AwsRegion::EuWest1 => RegionPricing {
                lambda_gb_second: 0.0000166667,
                lambda_per_million_requests: 0.20,
                fargate_vcpu_hour: 0.04456,
                fargate_gb_hour: 0.004890,
                eks_cluster_hour: 0.10,
            },
            AwsRegion::ApSoutheast1 => RegionPricing {
                lambda_gb_second: 0.0000166667,
                lambda_per_million_requests: 0.20,
                fargate_vcpu_hour: 0.04656,
                fargate_gb_hour: 0.005107,
                eks_cluster_hour: 0.10,
            },
            AwsRegion::SaEast1 => RegionPricing {
                lambda_gb_second: 0.0000208334,
                lambda_per_million_requests: 0.30,
                fargate_vcpu_hour: 0.05664,
                fargate_gb_hour: 0.006218,
                eks_cluster_hour: 0.10,
            },
        };
        
        Self { pricing }
    }
    
    pub fn estimate_lambda(&self, metrics: &ExecutionMetrics) -> LambdaCost {
    
        let memory_mb = metrics.memory_mb.max(128.0).ceil();
        let duration_seconds = metrics.duration_ms as f64 / 1000.0;
        
        let gb_seconds = (memory_mb / 1024.0) * duration_seconds;
      
        let compute_cost = gb_seconds * self.pricing.lambda_gb_second;
        
        let request_cost = self.pricing.lambda_per_million_requests / 1_000_000.0;
        
        let cost_per_execution = compute_cost + request_cost;
        let monthly_cost_1m = cost_per_execution * 1_000_000.0;
        
        LambdaCost {
            cost_per_execution,
            monthly_cost_1m,
        }
    }
    
    pub fn estimate_ecs_fargate(&self, metrics: &ExecutionMetrics) -> FargateCost {
        let vcpu = self.calculate_vcpu(metrics.cpu_avg);
        let memory_gb = self.calculate_memory_gb(metrics.memory_mb, vcpu);
        
        let duration_hours = metrics.duration_ms as f64 / 3_600_000.0;
        
        let vcpu_cost = vcpu * self.pricing.fargate_vcpu_hour * duration_hours;
        let memory_cost = memory_gb * self.pricing.fargate_gb_hour * duration_hours;
        
        let cost_per_execution = vcpu_cost + memory_cost;

        let hours_per_month = 730.0;
        let monthly_cost_continuous = (vcpu * self.pricing.fargate_vcpu_hour + 
                                      memory_gb * self.pricing.fargate_gb_hour) * hours_per_month;
        
        FargateCost {
            cost_per_execution,
            monthly_cost_continuous,
        }
    }
    
    pub fn estimate_eks_fargate(&self, metrics: &ExecutionMetrics) -> EksCost {
        let fargate_cost = self.estimate_ecs_fargate(metrics);
        
        let duration_hours = metrics.duration_ms as f64 / 3_600_000.0;
        let cluster_cost_per_exec = self.pricing.eks_cluster_hour * duration_hours;
        
        let cost_per_execution = fargate_cost.cost_per_execution + cluster_cost_per_exec;
        let monthly_cost_continuous = fargate_cost.monthly_cost_continuous + 
                                     (self.pricing.eks_cluster_hour * 730.0);
        EksCost {
            cost_per_execution,
            monthly_cost_continuous,
        }
    }
    
    fn calculate_vcpu(&self, cpu_avg: f32) -> f64 {
        let vcpu_needed = (cpu_avg / 100.0).max(0.25);
        
        if vcpu_needed <= 0.25 { 0.25 }
        else if vcpu_needed <= 0.5 { 0.5 }
        else if vcpu_needed <= 1.0 { 1.0 }
        else if vcpu_needed <= 2.0 { 2.0 }
        else if vcpu_needed <= 4.0 { 4.0 }
        else if vcpu_needed <= 8.0 { 8.0 }
        else { 16.0 }
    }
    
    fn calculate_memory_gb(&self, memory_mb: f64, vcpu: f64) -> f64 {
        let memory_gb = (memory_mb / 1024.0).max(0.5);

        let max_memory = match vcpu {
            0.25 => 2.0,
            0.5 => 4.0,
            1.0 => 8.0,
            2.0 => 16.0,
            4.0 => 30.0,
            _ => 120.0,
        };
        
        memory_gb.min(max_memory).ceil()
    }
}

#[derive(Debug)]
pub struct LambdaCost {
    pub cost_per_execution: f64,
    pub monthly_cost_1m: f64,
}

#[derive(Debug)]
pub struct FargateCost {
    pub cost_per_execution: f64,
    pub monthly_cost_continuous: f64,
}

#[derive(Debug)]
pub struct EksCost {
    pub cost_per_execution: f64,
    pub monthly_cost_continuous: f64,
}
