# Gitflict 

Visualize git merge conflicts side by side in your terminal.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Show all conflicts in the current repo
gitflict

# Filter on a specific file
gitflict --file app.js

# Plain text output, no colors
gitflict --plain

# Help
gitflict --help
```

## Requirements

- Rust 1.70+
- Git
