use std::collections::{HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 地址前缀，多个前缀用逗号分隔
    #[arg(short, long, use_value_delimiter = true, value_delimiter = ',')]
    prefixes: Vec<String>,

    /// 要保存的非匹配地址的数量
    #[arg(short, long, default_value_t = 0)]
    non_matching_count: usize,

    /// 线程数量
    #[arg(short, long, default_value_t = 0)]
    threads: usize,

    /// 输出文件
    #[arg(short, long, default_value = "data/solana_addresses.csv")]
    output: String,

    /// 匹配地址的输出文件
    #[arg(short, long, default_value = "data/matched_addresses.csv")]
    matched_output: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // 设置线程数，默认使用所有可用线程
    let num_threads = if args.threads == 0 {
        rayon::current_num_threads()
    } else {
        args.threads
    };
    println!("使用 {} 个线程", num_threads);
    
    // 创建本地线程池，而不是使用全局线程池
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();

    // 准备前缀集合
    let prefixes: HashSet<String> = args.prefixes.into_iter().collect();
    println!("查找以下前缀: {:?}", prefixes);

    // 初始化计数器和文件
    let generated = Arc::new(Mutex::new(0u64));
    let matched = Arc::new(Mutex::new(0u64));
    
    // 保存前N个非匹配地址
    let non_matching_addresses = Arc::new(Mutex::new(VecDeque::with_capacity(args.non_matching_count)));

    // 在创建输出文件之前，确保目录存在
    let output_dir = std::path::Path::new(&args.output).parent().unwrap_or_else(|| std::path::Path::new("."));
    let matched_output_dir = std::path::Path::new(&args.matched_output).parent().unwrap_or_else(|| std::path::Path::new("."));

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }
    if !matched_output_dir.exists() && matched_output_dir != output_dir {
        std::fs::create_dir_all(matched_output_dir)?;
    }
    
    // 创建输出文件
    let output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&args.output)?;
    let output_writer = Arc::new(Mutex::new(BufWriter::new(output_file)));
    
    // 匹配地址输出文件
    let matched_output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&args.matched_output)?;
    let matched_output_writer = Arc::new(Mutex::new(BufWriter::new(matched_output_file)));
    
    // 写入CSV标题
    {
        writeln!(output_writer.lock().unwrap(), "address,private_key")?;
        writeln!(matched_output_writer.lock().unwrap(), "address,private_key")?;
    }

    // 设置进度条
    let multi_progress = MultiProgress::new();
    let total_progress = multi_progress.add(ProgressBar::new_spinner());
    total_progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    
    let matched_progress = multi_progress.add(ProgressBar::new_spinner());
    matched_progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} [{elapsed_precise}] {msg}")
            .unwrap(),
    );

    // 启动进度条更新线程
    let generated_clone = Arc::clone(&generated);
    let matched_clone = Arc::clone(&matched);
    let start_time = Instant::now();
    
    std::thread::spawn(move || {
        loop {
            let generation_count = *generated_clone.lock().unwrap();
            let matches = *matched_clone.lock().unwrap();
            let elapsed = start_time.elapsed().as_secs();
            
            if elapsed > 0 {
                let rate = generation_count as f64 / elapsed as f64;
                total_progress.set_message(format!(
                    "已生成: {} | 速率: {:.2}/秒 | 匹配: {}",
                    generation_count, rate, matches
                ));
                
                if matches > 0 {
                    matched_progress.set_message(format!(
                        "找到 {} 个匹配的地址! 当前概率: 1/{}",
                        matches, if matches > 0 { generation_count / matches } else { 0 }
                    ));
                }
            }
            
            std::thread::sleep(Duration::from_millis(200));
        }
    });

    // 使用本地线程池执行并行任务
    thread_pool.install(|| {
        (0..num_threads).into_par_iter().for_each(|_| {
            loop {
                // 生成新的密钥对
                let keypair = Keypair::new();
                let address = keypair.pubkey().to_string();
                let secret_key = bs58::encode(keypair.secret().as_ref()).into_string();
                
                // 更新计数器
                let mut gen_lock = generated.lock().unwrap();
                *gen_lock += 1;
                let current_count = *gen_lock;
                drop(gen_lock);
                
                // 检查是否匹配任何前缀
                let mut is_match = false;
                for prefix in &prefixes {
                    if address.starts_with(prefix) {
                        is_match = true;
                        
                        // 更新匹配计数
                        let mut match_lock = matched.lock().unwrap();
                        *match_lock += 1;
                        drop(match_lock);
                        
                        // 写入匹配的地址
                        let mut writer = matched_output_writer.lock().unwrap();
                        writeln!(writer, "{},{}", address, secret_key).unwrap();
                        writer.flush().unwrap();
                        break;
                    }
                }
                
                // 如果不匹配但在前N个，保存它
                if !is_match {
                    let mut addresses = non_matching_addresses.lock().unwrap();
                    if addresses.len() < args.non_matching_count {
                        addresses.push_back((address.clone(), secret_key.clone()));
                        
                        // 写入非匹配地址
                        let mut writer = output_writer.lock().unwrap();
                        writeln!(writer, "{},{}", address, secret_key).unwrap();
                    }
                }
                
                // 每生成100万个地址刷新一次输出文件
                if current_count % 1_000_000 == 0 {
                    output_writer.lock().unwrap().flush().unwrap();
                    matched_output_writer.lock().unwrap().flush().unwrap();
                }
            }
        });
    });

    Ok(())
}