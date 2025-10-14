# WGPU Fractals

A real-time Mandelbrot set visualizer built with Rust and WebGPU (WGPU). Explore the beauty of fractals with hardware-accelerated rendering, running both natively on desktop and in web browsers via WebAssembly.

![WGPU Fractals](https://img.shields.io/badge/Rust-2024-orange) ![WebGPU](https://img.shields.io/badge/WebGPU-WGPU-blue)

## Features

- **Real-time Mandelbrot Set Rendering**: GPU-accelerated fractal generation using compute shaders
- **Interactive Exploration**: 
  - Zoom in/out with smooth scaling
  - Pan across the complex plane by dragging
  - Adjust iteration depth for more detail
- **Cross-Platform**: Runs natively on Windows, macOS, and Linux
- **Web Support**: Deployable as a WebAssembly application in modern browsers
- **Colorful Visualization**: HSV-based coloring scheme that highlights fractal structure

## Controls

| Action | Control |
|--------|---------|
| **Zoom In** | Scroll wheel up or `+` key |
| **Zoom Out** | Scroll wheel down or `-` key |
| **Pan** | Click and drag |
| **Increase Iterations** | Up Arrow (↑) |
| **Decrease Iterations** | Down Arrow (↓) |
| **Close Application** | ESC key (desktop only) |

## Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **GPU**: A graphics card with WebGPU support
  - Most modern GPUs (2016+) support Vulkan, Metal, or DirectX 12

For web builds, you'll also need:
- **wasm-pack**: Install with `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`

## Building and Running

### Native Desktop Application

1. Clone the repository:
   ```bash
   git clone https://github.com/Abhiram-Kasu/wgpu_fractals.git
   cd wgpu_fractals
   ```

2. Build and run:
   ```bash
   cargo run --release
   ```

### Web Application

1. Build for WebAssembly:
   ```bash
   chmod +x build_web.sh
   ./build_web.sh
   ```

2. Serve the application:
   ```bash
   # Using Python
   python3 -m http.server 8000
   
   # Or using npx
   npx serve .
   ```

3. Open your browser and navigate to:
   ```
   http://localhost:8000
   ```

**Note**: The web application must be served over HTTP/HTTPS due to WebAssembly security requirements. Opening `index.html` directly won't work.

## How It Works

### Architecture

The application uses a compute shader pipeline to generate the Mandelbrot set:

1. **Compute Shader** (`src/compute.wgsl`): Calculates Mandelbrot iterations for each pixel in parallel on the GPU
2. **Render Pipeline** (`src/shader.wgsl`): Displays the computed fractal texture on a fullscreen quad
3. **State Management** (`src/state.rs`): Tracks zoom level, center position, and iteration count
4. **Event Handling** (`src/app.rs`): Processes user input for navigation and control

### The Mandelbrot Set

The Mandelbrot set is defined by the iterative formula:
```
z(n+1) = z(n)² + c
```

Where:
- `c` is a complex number representing a point in the complex plane
- `z(0) = 0`
- A point is in the Mandelbrot set if the sequence remains bounded

The visualization colors points based on how quickly they escape to infinity, creating the characteristic fractal patterns.

## Project Structure

```
wgpu_fractals/
├── src/
│   ├── main.rs          # Desktop entry point
│   ├── lib.rs           # Library exports and web entry point
│   ├── app.rs           # Application lifecycle and event handling
│   ├── state.rs         # GPU state and Mandelbrot parameters
│   ├── shader.rs        # Vertex definitions
│   ├── shader.wgsl      # Render shader (WGSL)
│   └── compute.wgsl     # Mandelbrot compute shader (WGSL)
├── index.html           # Web application HTML
├── build_web.sh         # WebAssembly build script
└── Cargo.toml           # Rust dependencies
```

## Technical Details

- **Language**: Rust (Edition 2024)
- **Graphics API**: WebGPU via the `wgpu` crate
- **Windowing**: `winit` for cross-platform window management
- **Compute Shaders**: WGSL (WebGPU Shading Language)
- **Web Target**: WebAssembly with `wasm-bindgen`

## License

This project is open source. Please check the repository for license details.

## Contributing

Contributions are welcome! Feel free to submit issues or pull requests.

## Credits

Built with [wgpu](https://github.com/gfx-rs/wgpu) - A safe and portable GPU API for Rust.
