use std::process::Command;
use std::time::Instant;
use std::env;
use std::io;
use colored::*;

mod metrics;
mod aws_cost;
use metrics::{ExecutionMetrics, MetricsCollector};
use aws_cost::{AwsRegion, CostEstimator};

fn main() -> io::Result<()> {
    banner();

    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: <command> <args...>");
        eprintln!("Example: python3 simple_script.py");
        eprintln!("Example: uv run --python /path/to/.venv/bin/python --env-file /path/to/.env -- python script.py arg1 arg2");
        std::process::exit(1);
    }
        
    match execute_and_monitor(args) {
        Ok(metrics) => {
            println!("** Execução concluída!\n");
            display_metrics(&metrics);
            
            estimate_aws_costs(&metrics);
        }
        Err(e) => {
            eprintln!("** Erro na execução: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())    
}

fn execute_and_monitor(args: Vec<String>) -> Result<ExecutionMetrics, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let executable = &args[0];
    let exec_args = &args[1..];

    let mut child = Command::new(executable)
        .args(exec_args)
        .spawn()?;    
    
    let pid = child.id();
    
    let mut collector = MetricsCollector::new(pid);
    collector.start_monitoring();
    
    let status = child.wait()?;
    
    let duration = start.elapsed();
    collector.stop_monitoring();
    
    if !status.success() {
        return Err(format!("Script retornou código de erro: {:?}", status.code()).into());
    }
    
    Ok(collector.get_metrics(duration))
}

fn display_metrics(metrics: &ExecutionMetrics) {
    println!("MÉTRICAS COLETADAS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Duração: {} ms", metrics.duration_ms);
    println!("  CPU Média: {:.2}%", metrics.cpu_avg);
    println!(" CPU Pico: {:.2}%", metrics.cpu_peak);
    println!(" Memória Usada: {:.2} MB", metrics.memory_mb);
    println!(" Bytes Enviados: {} bytes", metrics.network_sent);
    println!(" Bytes Recebidos: {} bytes", metrics.network_received);
    println!();
}

fn estimate_aws_costs(metrics: &ExecutionMetrics) {
    let region = AwsRegion::UsEast1;
    let estimator = CostEstimator::new(region);
    
    println!("ESTIMATIVA DE CUSTOS AWS ({})", region.name());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Lambda
    let lambda_cost = estimator.estimate_lambda(metrics);
    println!("AWS Lambda:");
    println!("   • Custo por execução: ${:.6}", lambda_cost.cost_per_execution);
    println!("   • Custo mensal (1M exec): ${:.2}", lambda_cost.monthly_cost_1m);
    println!();
    
    // ECS Fargate
    let ecs_cost = estimator.estimate_ecs_fargate(metrics);
    println!("ECS Fargate:");
    println!("   • Custo por execução: ${:.6}", ecs_cost.cost_per_execution);
    println!("   • Custo mensal (contínuo): ${:.2}", ecs_cost.monthly_cost_continuous);
    println!();
    
    // EKS Fargate
    let eks_cost = estimator.estimate_eks_fargate(metrics);
    println!("EKS Fargate:");
    println!("   • Custo por execução: ${:.6}", eks_cost.cost_per_execution);
    println!("   • Custo mensal (contínuo): ${:.2}", eks_cost.monthly_cost_continuous);
    println!();
}





fn banner() {
    let banner = r#"
      
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣀⣄⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠐⣶⣾⣿⣿⣿⣿⣿⣶⡆⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⢰⡏⢤⡎⣿⣿⢡⣶⢹⣧⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⣶⣶⣇⣸⣷⣶⣾⣿⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⢨⣿⣿⣿⢟⣿⣿⣿⣿⣿⣧⡀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⣿⡏⣿⣿⣿⣿⣿⣿⣿⣿⡄⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⣿⣿⣿⣜⠿⣿⣿⣿⣿⣿⣿⣿⡄⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠐⣷⣿⡿⣷⣮⣙⠿⣿⣿⣿⣿⣿⡄⠀
        ⠀⠀⠀⠀⠈⠫⡯⢿⣿⣿⣿⣶⣯⣿⣻⣿⣿⠀
            ⠀⠀⠀⠙⢻⣿⣿⠿⠿⠿⢻⣿⠙⠇
                ⣠⡶⠿⣟⠀⠀⠀⠀⠻⡀⠀
  ___           _           __  __      _            
 / _ \__      _| |   ___   |  \/  | ___| |_ ___ _ __ 
| | | \ \ /\ / / |  / _ \  | |\/| |/ _ \ __/ _ \ '__|
| |_| |\ V  V /| | | (_) | | |  | |  __/ ||  __/ |   
 \___/  \_/\_/ |_|  \___/  |_|  |_|\___|\__\___|_|   

    "#;

    println!("{}", banner.purple());
}