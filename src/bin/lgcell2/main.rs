use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::Parser;
use lgcell2_core::io::json::{output_json_to_string, parse_circuit_json, simulate_to_output_json};

/// LGCELL2 回路シミュレータ
#[derive(Debug, Parser)]
#[command(name = "lgcell2")]
struct Cli {
    /// 回路定義 JSON ファイル。省略時は標準入力から読み込み。
    file: Option<PathBuf>,

    /// シミュレーションする tick 数
    #[arg(short, long, default_value_t = 100)]
    ticks: u64,
}

fn read_input(file: Option<PathBuf>) -> Result<String, std::io::Error> {
    if let Some(path) = file {
        fs::read_to_string(path)
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let input = read_input(cli.file)?;
    let circuit = parse_circuit_json(&input)?;
    let output = simulate_to_output_json(circuit, cli.ticks);
    let output_str = output_json_to_string(&output)?;
    println!("{}", output_str);
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
