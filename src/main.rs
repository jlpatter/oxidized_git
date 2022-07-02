use git2::Repository;

fn main() {
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    let mut references = match repo.references() {
        Ok(references) => references,
        Err(e) => panic!("failed to load references: {}", e),
    };
    let ref_names = references.names();
    for name in ref_names {
        let str_name = match name {
            Ok(str_name) => str_name,
            Err(e) => panic!("failed to get ref name: {}", e),
        };
        println!("{}", str_name);
    }
}
