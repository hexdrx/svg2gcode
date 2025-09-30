# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

svg2gcode converts SVG vector graphics to G-code for CNC machines (pen plotters, laser engravers, etc.). The project is a Rust workspace with three main components:
- **lib** (`svg2gcode`): Core library that performs SVG parsing and G-code generation
- **cli** (`svg2gcode-cli`): Command-line interface
- **web** (`svg2gcode-web`): WASM-based web interface built with Yew framework

## Common Commands

### Building
```bash
# Build the core library
cargo build -p svg2gcode

# Build CLI
cargo build -p svg2gcode-cli

# Check web (faster than full build)
cargo check -p svg2gcode-web

# Build entire workspace
cargo build
```

### Testing
```bash
# Run tests for the library
cargo test -p svg2gcode

# Run tests with all features enabled
cargo test --all-features -p svg2gcode

# Run specific test
cargo test -p svg2gcode square_produces_expected_gcode
```

### CLI Usage
```bash
# Install CLI locally
cargo install --path cli

# Run CLI directly
cargo run -p svg2gcode-cli -- <args>

# Example conversion
cargo run -p svg2gcode-cli -- examples/Vanderbilt_Commodores_logo.svg --off 'M4' --on 'M5' -o out.gcode
```

### Web Development
The web interface is built with Yew and compiled to WASM. Check the web-deploy workflow for build commands.

## Architecture

### Core Conversion Pipeline

The conversion follows this flow:
1. **SVG Parsing** (`converter/mod.rs`): Parse SVG using `roxmltree`, extract paths and transforms
2. **Turtle Graphics** (`turtle/mod.rs`): Abstract drawing interface that translates SVG path commands to drawing operations
3. **G-code Generation** (`turtle/g_code.rs`): Convert turtle operations to G-code tokens using the `g-code` crate
4. **Machine State** (`machine.rs`): Track machine state (tool on/off, distance mode) to minimize redundant G-code
5. **Postprocessing** (`postprocess.rs`): Add line numbers, checksums, origin adjustments

### Key Abstractions

**Turtle Trait**: The `Turtle` trait provides an abstraction for drawing paths. It has three implementations:
- `PreprocessTurtle`: First pass to calculate bounding boxes
- `GCodeTurtle`: Generates actual G-code tokens
- `DpiConvertingTurtle`: Wrapper that converts between SVG units and machine units

**Terrarium**: Wrapper around a `Turtle` that manages coordinate transforms, position tracking, and SVG path state (like reflected control points for smooth curves).

**Machine**: Simulates CNC machine state to avoid redundant commands. Tracks tool state (on/off) and distance mode (absolute/relative). Also handles circular interpolation support (G2/G3 commands).

### SVG Coordinate Conversion

SVG coordinates are converted to G-code coordinates in two steps:
1. Y-axis is flipped (SVG origin is top-left, G-code is bottom-left) via `Transform2D::scale(1., -1.)`
2. Units are converted from SVG units (pixels, mm, etc.) to machine units using DPI

### Transform Handling

SVG transforms are applied using a stack-based approach:
- Transforms are pushed onto a stack when entering SVG groups
- The current transform is the composition of all transforms on the stack
- Path coordinates are transformed before being passed to the turtle

### Settings System

The `Settings` struct combines three config types:
- `ConversionConfig`: tolerance, feedrate, DPI, origin
- `MachineConfig`: circular interpolation support, tool on/off sequences
- `PostprocessConfig`: line numbers, checksums, comment formatting

Settings support versioning via the `Version` enum to handle breaking changes. The `try_upgrade()` method attempts automatic upgrades between versions.

## Testing

Tests are located in `lib/tests/` with paired `.svg` and `.gcode` files. The test harness:
1. Parses the SVG
2. Converts to G-code tokens
3. Compares against expected `.gcode` file
4. Uses floating-point tolerance comparison (`1E-10`) since values differ between debug/release builds

Key test categories:
- Basic shapes (square, transformations)
- Circular interpolation
- Smooth curves (cubic/quadratic bezier)
- Dimension overrides

## Important Implementation Details

- **Circular Interpolation**: Uses `arc.rs` module to approximate Bézier curves with circular arcs (G2/G3). Only enabled if machine supports it.
- **Curve Flattening**: Curves are flattened to line segments using Lyon geometry library with configurable tolerance
- **Extra Attribute Printing**: The `--extra-attribute-name` CLI option allows printing additional SVG attributes in G-code comments
- **Dimension Overrides**: SVGs without width/height can have dimensions specified via CLI or ConversionOptions
- **Origin Setting**: The origin can be set to position the output on the machine bed

## Dependencies

Key external crates:
- `g-code`: Parsing and emitting G-code
- `lyon_geom`: Geometric primitives and curve operations
- `roxmltree`: Fast XML/SVG parsing
- `svgtypes`: SVG data type parsing
- `uom`: Unit conversion
- `yew`/`yewdux`: Web framework and state management (web only)