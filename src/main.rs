use std::process::Command;
use std::io::stdout;
use anyhow::{Context, Result};
use crossterm::ExecutableCommand;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::size;
use clap::Parser;

// CLI arguments definition
// clap reads these annotations and builds --help, validation, etc.
#[derive(Parser)]
#[command(
    name = "gitflict",
    about = "Visualize git merge conflicts side by side",
    version = "0.1.0"
)]
struct Cli {
    #[arg(short, long, help = "Show conflicts for a specific file only")]
    file: Option<String>,

    #[arg(short, long, help = "Plain text output, no colors")]
    plain: bool,
}

#[derive(Debug)]
struct Conflict {
    head_lines: Vec<String>,
    incoming_lines: Vec<String>,
    start_line: usize,
}

fn run_git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to run git — is git installed?")?;

    let stdout = String::from_utf8(output.stdout)
        .context("Git output is not valid UTF-8")?;

    Ok(stdout)
}

fn get_conflicted_files() -> Result<Vec<String>> {
    let output = run_git(&["diff", "--name-only", "--diff-filter=U"])?;

    let files = output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(files)
}

fn read_file_content(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .context(format!("Failed to read file: {}", path))
}

fn parse_conflicts(content: &str) -> Vec<Conflict> {
    let mut conflicts = Vec::new();
    let mut head_lines: Vec<String> = Vec::new();
    let mut incoming_lines: Vec<String> = Vec::new();
    let mut in_head = false;
    let mut in_incoming = false;
    let mut start_line = 0;

    for (index, line) in content.lines().enumerate() {
        if line.starts_with("<<<<<<<") {
            in_head = true;
            start_line = index;
        } else if line.starts_with("=======") {
            in_head = false;
            in_incoming = true;
        } else if line.starts_with(">>>>>>>") {
            conflicts.push(Conflict {
                head_lines: head_lines.clone(),
                incoming_lines: incoming_lines.clone(),
                start_line,
            });
            head_lines = Vec::new();
            incoming_lines = Vec::new();
            in_head = false;
            in_incoming = false;
        } else if in_head {
            head_lines.push(line.to_string());
        } else if in_incoming {
            incoming_lines.push(line.to_string());
        }
    }

    conflicts
}

fn pad_or_truncate(s: &str, width: usize) -> String {
    if s.len() >= width {
        format!("{}…", &s[..width - 1])
    } else {
        format!("{:<width$}", s, width = width)
    }
}

// Prints a row with colors
fn print_row_colored(left: &str, right: &str, col_width: usize) -> Result<()> {
    let left_padded = pad_or_truncate(left, col_width);
    let right_padded = pad_or_truncate(right, col_width);

    stdout()
        .execute(SetBackgroundColor(Color::DarkGreen))?
        .execute(SetForegroundColor(Color::White))?
        .execute(Print(format!(" {} ", left_padded)))?
        .execute(ResetColor)?
        .execute(Print(" "))?
        .execute(SetBackgroundColor(Color::DarkRed))?
        .execute(SetForegroundColor(Color::White))?
        .execute(Print(format!(" {} ", right_padded)))?
        .execute(ResetColor)?
        .execute(Print("\n"))?;

    Ok(())
}

// Prints a row as plain text — for --plain mode
fn print_row_plain(left: &str, right: &str, col_width: usize) {
    let left_padded = pad_or_truncate(left, col_width);
    let right_padded = pad_or_truncate(right, col_width);
    println!(" {} | {}", left_padded, right_padded);
}

fn print_conflict(conflict: &Conflict, index: usize, col_width: usize, plain: bool) -> Result<()> {
    println!("\n  Conflict {} — line {}", index + 1, conflict.start_line + 1);

    let separator = "-".repeat(col_width);
    let max_lines = conflict.head_lines.len().max(conflict.incoming_lines.len());

    if plain {
        // Plain text mode — like a fallback renderer in Laravel
        print_row_plain("HEAD (your branch)", "INCOMING (their branch)", col_width);
        print_row_plain(&separator, &separator, col_width);
        for i in 0..max_lines {
            let head_line = conflict.head_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let incoming_line = conflict.incoming_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            print_row_plain(head_line, incoming_line, col_width);
        }
    } else {
        print_row_colored("HEAD (your branch)", "INCOMING (their branch)", col_width)?;
        print_row_colored(&separator, &separator, col_width)?;
        for i in 0..max_lines {
            let head_line = conflict.head_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let incoming_line = conflict.incoming_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            print_row_colored(head_line, incoming_line, col_width)?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    // Parse CLI arguments — clap handles --help and errors automatically
    let cli = Cli::parse();

    let all_files = get_conflicted_files()?;

    if all_files.is_empty() {
        println!("No conflicted files found.");
        println!("Tip: run this tool inside a git repo that has merge conflicts.");
        return Ok(());
    }

    // If --file is passed, filter to that file only
    // Like a Laravel Collection::filter() based on a query param
    let files: Vec<String> = match &cli.file {
        Some(filter) => all_files
            .into_iter()
            .filter(|f| f.contains(filter.as_str()))
            .collect(),
        None => all_files,
    };

    if files.is_empty() {
        println!("No conflicted files matching '{}'.", cli.file.unwrap());
        return Ok(());
    }

    let (terminal_width, _) = size().unwrap_or((120, 40));
    let col_width = (terminal_width as usize / 2) - 3;

    println!("Conflicted files: {}", files.len());

    for file in &files {
        println!("\n=== {} ===", file);

        let content = read_file_content(file)?;
        let conflicts = parse_conflicts(&content);

        println!("  {} conflict(s)", conflicts.len());

        for (i, conflict) in conflicts.iter().enumerate() {
            print_conflict(conflict, i, col_width, cli.plain)?;
        }
    }

    println!();
    Ok(())
}