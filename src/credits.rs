use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead};

pub fn print_credits() {
    let license: &str = include_str!("../LICENSE.txt");
    println!("{}", env!("CARGO_PKG_NAME"));
    println!("{}", env!("CARGO_PKG_HOMEPAGE"));
    println!("\n{}", license);
    println!("Licenses for third party dependencies which may be included in this binary:\n");
    print_deps().expect("expected to print dependencies");
}
fn print_deps() -> std::io::Result<()> {
    // Read the static file
    let content = std::fs::read_to_string("licenses.txt")?;

    // Regular expression to match the format: name, version, "license", repository
    let re = regex::Regex::new(r#"^(\S+)\s+(\S+)\s+"([^"]+)"\s+(\S+)$"#).unwrap();

    // Parse lines into structured data
    let mut licenses_map: std::collections::BTreeMap<
        String,
        std::collections::HashMap<String, (Vec<String>, String)>,
    > = std::collections::BTreeMap::new();

    for line in content.lines() {
        if let Some(captures) = re.captures(line) {
            let name = captures[1].to_string();
            let version = captures[2].to_string();
            let license = captures[3].to_string();
            let repository = captures[4].to_string();

            // Group by license, then by package name
            let entry = licenses_map.entry(license).or_default();
            let package = entry
                .entry(name.clone())
                .or_insert((Vec::new(), repository.clone()));

            // Aggregate versions
            if !package.0.contains(&version) {
                package.0.push(version);
            }
        } else {
            //eprintln!("Skipping malformed line: {}", line);
        }
    }

    // Print the sorted output
    for (license, packages) in &licenses_map {
        println!("{license}");

        for (name, (versions, repository)) in packages {
            let versions_str = versions.join(", ");
            println!("  {:<30} {}", name, repository,);
        }

        println!(); // Blank line between license groups
    }

    Ok(())
}
