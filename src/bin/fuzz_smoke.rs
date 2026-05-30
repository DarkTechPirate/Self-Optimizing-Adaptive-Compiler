use nyx::api::NyxCompiler;
use nyx::optimizer::OptimizationPlan;

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn gen_range_i64(&mut self, min: i64, max: i64) -> i64 {
        if max <= min {
            return min;
        }
        let span = (max - min) as u32;
        let val = self.next_u32() % span;
        min + val as i64
    }

    fn gen_bool(&mut self, threshold: u32) -> bool {
        self.next_u32() % 100 < threshold
    }
}

fn gen_expr(rng: &mut Lcg, depth: usize) -> String {
    if depth == 0 || rng.gen_bool(40) {
        return rng.gen_range_i64(-9, 10).to_string();
    }

    let left = gen_expr(rng, depth - 1);
    let right = gen_expr(rng, depth - 1);
    let op = match rng.next_u32() % 4 {
        0 => "+",
        1 => "-",
        2 => "*",
        _ => "/",
    };
    format!("({} {} {})", left, op, right)
}

fn gen_program(rng: &mut Lcg) -> String {
    let mut lines = Vec::new();
    lines.push("fn main() {".to_string());

    let let_count = rng.gen_range_i64(1, 4) as usize;
    for i in 0..let_count {
        let expr = gen_expr(rng, 3);
        lines.push(format!("let v{} = {}", i, expr));
    }

    if rng.gen_bool(50) {
        let loop_end = rng.gen_range_i64(1, 6);
        lines.push("let total = 0".to_string());
        lines.push(format!("for i in 0..{} {{", loop_end));
        lines.push("total = total + i".to_string());
        lines.push("}".to_string());
        lines.push("return total".to_string());
    } else {
        lines.push(format!("return v{}", let_count - 1));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

fn main() {
    let mut iterations = 200u32;
    let mut seed = 0xC0FFEEu64;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iterations" => {
                if let Some(val) = args.next() {
                    iterations = val.parse().unwrap_or(iterations);
                }
            }
            "--seed" => {
                if let Some(val) = args.next() {
                    seed = val.parse().unwrap_or(seed);
                }
            }
            _ => {}
        }
    }

    let mut rng = Lcg::new(seed);

    for iter in 0..iterations {
        let program = gen_program(&mut rng);
        let mut compiler = NyxCompiler::new();
        let compile = compiler.compile(&program);
        if !compile.success {
            eprintln!("compile failed on iteration {}", iter);
            eprintln!("{}", program);
            std::process::exit(1);
        }

        let baseline = compiler.execute();
        let optimize = compiler.optimize_with_plan(OptimizationPlan::aggressive());
        if !optimize.success {
            eprintln!("optimize failed on iteration {}", iter);
            eprintln!("{}", program);
            std::process::exit(1);
        }

        let optimized = compiler.execute();
        if baseline.return_value != optimized.return_value {
            eprintln!("mismatch on iteration {}", iter);
            eprintln!("baseline: {:?}", baseline.return_value);
            eprintln!("optimized: {:?}", optimized.return_value);
            eprintln!("{}", program);
            std::process::exit(1);
        }
    }

    println!("fuzz smoke passed: {} iterations", iterations);
}
