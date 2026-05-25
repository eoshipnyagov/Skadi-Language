pub fn run(args: &[String]) -> Result<(), String> {
    let mut target = "host".to_string();
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == "--target" {
            if i + 1 >= args.len() {
                return Err("--target requires value".to_string());
            }
            target = args[i + 1].clone();
            i += 2;
            continue;
        }
        i += 1;
    }
    println!("skadi build: planned (target={target})");
    Ok(())
}
