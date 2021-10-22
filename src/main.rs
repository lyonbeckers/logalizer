use anyhow::Result;
use cli_table::{print_stdout, Cell, CellStruct, Style, Table};
use rayon::prelude::*;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::Instant,
};

#[derive(Deserialize)]
struct Log {
    #[serde(rename = "type")]
    log_type: String,
}

struct TypeData {
    total_byte_size: usize,
    num_instances: usize,
}

#[derive(Default)]
struct TypeTable {
    types: HashMap<String, TypeData>,
    lines_excluded: Vec<ExcludedLine>,
}

struct ExcludedLine {
    index: usize,
    error: String,
}

impl TypeTable {
    fn from_file(path: &str) -> Result<Self> {
        let input_file = File::open(path)?;

        let lines = BufReader::new(input_file).lines();

        let mut type_table = Self::default();

        lines
            .into_iter()
            .enumerate()
            .try_for_each(|(i, line)| -> Result<()> {
                let line = line?;

                match serde_json::from_str::<Log>(&line) {
                    Ok(log) => match type_table.types.get_mut(&log.log_type) {
                        Some(type_data) => {
                            type_data.num_instances += 1;
                            type_data.total_byte_size += line.len()
                        }
                        None => {
                            type_table.types.insert(
                                log.log_type,
                                TypeData {
                                    num_instances: 1,
                                    total_byte_size: line.len(),
                                },
                            );
                        }
                    },
                    Err(err) => type_table.lines_excluded.push(ExcludedLine {
                        index: i,
                        error: err.to_string(),
                    }),
                };

                Ok(())
            })?;

        Ok(type_table)
    }
}

fn main() {
    let start = Instant::now();

    let args: Vec<String> = env::args().collect();

    let input_arg = args.get(1).cloned();
    match input_arg {
        Some(path) => match TypeTable::from_file(&path) {
            Ok(type_table) => {
                render_table(&type_table);

                if !type_table.lines_excluded.is_empty() {
                    println!("The following lines were excluded because of errors:");
                    for excluded in type_table.lines_excluded {
                        println!("- line {}: {}", excluded.index + 1, excluded.error)
                    }
                }

                let elapsed = Instant::now() - start;
                print!(
                    "Task succesfully completed in {} microseconds",
                    elapsed.as_micros()
                );
            }
            Err(err) => println!("Error reading file {}: {}", path, err),
        },
        None => {
            let exe_name = Path::new(&args[0]).iter().last().unwrap().to_str().unwrap();
            println!(
                "No input provided as an argument. Expected usage is: \"{} [filename]\"",
                exe_name
            );
        }
    }
}

fn render_table(type_table: &TypeTable) {
    let mut table: Vec<Vec<CellStruct>> = Vec::with_capacity(type_table.types.len());
    for (
        type_name,
        TypeData {
            total_byte_size,
            num_instances,
        },
    ) in &type_table.types
    {
        table.push(vec![
            type_name.cell(),
            num_instances.cell(),
            total_byte_size.cell(),
        ]);
    }

    let table = table
        .table()
        .title(vec![
            "type".cell().bold(true),
            "instances".cell().bold(true),
            "total byte size".cell().bold(true),
        ])
        .bold(true);

    print_stdout(table).ok();
}
