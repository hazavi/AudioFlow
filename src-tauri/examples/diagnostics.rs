fn main() {
    match audioflow_lib::native_diagnostics() {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(e) => {
            eprintln!("Native probe failed: {e}");
            std::process::exit(1);
        }
    }
}
