fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => cfa_flu_simulator::snapshot::update_snapshot(),
        Some("--stdout") => {
            if let Some(arg) = args.next() {
                eprintln!("unexpected argument after --stdout: {arg}");
                std::process::exit(2);
            }
            print!("{}", cfa_flu_simulator::snapshot::snapshot_json());
        }
        Some(arg) => {
            eprintln!("unknown argument: {arg}\nusage: update_snapshot [--stdout]");
            std::process::exit(2);
        }
    }
}
