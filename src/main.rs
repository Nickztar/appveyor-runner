extern crate yaml_rust;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use yaml_rust::YamlLoader;

use inquire::Text;
fn read_file_to_string(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn get_captures<'a>(re: &'a Regex, hay: &'a str) -> impl Iterator<Item = &'a str> {
    re.captures_iter(hay).map(|c| c.extract::<1>().1[0])
}

fn main() -> anyhow::Result<()> {
    let all_env_re = Regex::new(r"(\$env:\w+)").unwrap();
    let set_env_re = Regex::new(r"(\$env:\w+=)").unwrap();
    let av_path = Text::new("Enter appveyor file")
        .with_initial_value("./appveyor.yml")
        .prompt()?;
    let av_str = read_file_to_string(&av_path)?;
    let yml_docs = YamlLoader::load_from_str(&av_str)?;
    let av_file = yml_docs.get(0).unwrap();
    let ps_script = av_file["build_script"][0]["ps"].as_str().unwrap();

    let set_env_iter: Vec<&str> = get_captures(&set_env_re, ps_script)
        .map(|e| e.trim_end_matches('='))
        .collect();
    let envs_to_request: Vec<&str> = get_captures(&all_env_re, ps_script)
        .filter(|env| !set_env_iter.contains(env))
        .collect();

    let mut variable_map = HashMap::new();
    for variable in envs_to_request {
        let new_key = variable.replace("$env:", "$AV_");
        // prevent dupes
        if variable_map.contains_key(&new_key) {
            continue;
        }
        let value = Text::new(&format!("Enter value for \"{}\"=", variable)).prompt()?;
        variable_map.insert(new_key, value);
    }

    let formatted = ps_script.replace("$env:", "$AV_");
    // Replace enviroment varaibles in the script to make sure that we don't read from env
    let script_to_run = variable_map
        .iter()
        .map(|(key, value)| format!("{key}=\"{value}\""))
        .fold(String::new(), |acc, line| acc + &line + "\n")
        + "\n"
        + &formatted;

    let output = powershell_script::run(&script_to_run)?;
    println!("{}", output);
    return Ok(());
}
