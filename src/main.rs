fn main() {
    if let Err(error) = gh_inbox::run() {
        eprintln!("gh inbox: {error:#}");
        std::process::exit(1);
    }
}
