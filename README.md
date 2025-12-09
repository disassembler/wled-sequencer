# WLED FSEQ Sequencer Player

This project provides a reliable Rust-based player for streaming Financial Sequence (FSEQ) show files over the Distributed Display Protocol (DDP) to WLED controllers. It is designed to pause automatically if the WLED device goes offline (via ICMP ping monitoring) and resume when connectivity returns.

## ✨ Features

- **DDP Streaming**: Efficiently sends pixel data using the DDP protocol.
- **FSEQ Playback**: Reads and interprets FSEQ v2.x files.
- **Resilience**: Automatically pauses and resumes playback based on ICMP (ping) connectivity to the WLED device.
- **Cross-Platform Build**: Simple compilation using cargo or reproducible builds via Nix.
- **NixOS Integration**: Includes a dedicated NixOS module for easy deployment as a background service.

## 🛠️ Build Instructions

You have two primary options for building the `wled-sequencer` executable.

### 1. Build with Nix (Recommended for Reproducibility)

Using Nix ensures a completely reproducible build environment, automatically fetching the correct Rust toolchain and the required C dependency, `libtinyfseq`.

```bash
# Clone the repository
git clone https://github.com/disassembler/wled-sequencer.git
cd wled-sequencer

# Build the project using the Nix package defined in packages.nix
# This command will place the final executable in your result path.
nix build .#wled-sequencer
```

### 2. Build with Cargo

If you do not use Nix, you can build the project directly with Cargo. You must ensure the C dependency, `libtinyfseq`, is installed on your system and available to the build script.

**Install C Library**: Install `libtinyfseq` and its development headers via your system's package manager (e.g., `apt`, `pacman`, `brew`).

**Set Environment Variable (Important)**: Your `build.rs` script requires the include path.

```bash
# Replace /path/to/libtinyfseq/include with the actual path on your system
export TINYFSEQ_INCLUDE_DIR="/usr/include/libtinyfseq"
export TINYFSEQ_LIB_DIR="/usr/lib"

# Build the project
cargo build --release
```

The executable will be located at `./target/release/wled-sequencer`.

## 🏃 Running the Sequencer

The player requires a WLED IP address, a DDP port, and the path to an FSEQ file.

### FSEQ File Requirements

FSEQ files are typically created by lighting sequencing software like xLights.

⚠️ **Note on Compression**: You must disable frame data compression when generating FSEQ files in xLights. Compressed frame data support is not yet implemented in `wled-sequencer` but may be added as a future feature.

### Command Line Execution

Run the player, specifying the required host IP address and FSEQ file path:

| Flag | Description | Default |
|------|-------------|---------|
| `-h`, `--host` | WLED controller IP address (e.g., 192.168.1.50) | (Required) |
| `-f`, `--file` | Path to the FSEQ sequence file | (Required) |
| `-p`, `--port` | UDP port for DDP | 4048 |
| `--loop-enabled` | Enables continuous sequence looping | true |

```bash
# Example: Run sequence, loop continuously (default behavior)
./target/release/wled-sequencer \
  --host 10.40.8.61 \
  --file /path/to/your/show/tree.fseq
```

## ☁️ NixOS Service Deployment

The project includes a NixOS module for deploying `wled-sequencer` as a resilient background service.

### 1. Add the Flake Input

Add this repository as a flake input in your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    wled-sequencer = {
      url = "github:disassembler/wled-sequencer";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, wled-sequencer, ... }: {
    nixosConfigurations.your-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        wled-sequencer.nixosModules.wled-sequencer
        ./configuration.nix
      ];
    };
  };
}
```

### 2. Configuration (configuration.nix)

Set the `enable` flag and configure the mandatory `host` and `file` settings.

```nix
{ config, system, inputs, ... }:
{
  services.wled-sequencer = {
    enable = true;
    # Reference the package from the flake input
    package = inputs.wled-sequencer.packages.${system}.wled-sequencer;
    settings = {
      # MANDATORY: IP address of your WLED controller
      host = "192.168.1.100";

      # MANDATORY: Path to the FSEQ file on the NixOS system
      file = "/etc/sequences/my-show.fseq";

      # OPTIONAL: DDP port (defaults to 4048)
      # port = 4048;

      # OPTIONAL: Looping (defaults to true)
      # loop-enabled = false;
    };
  };
}
```

### 3. Deploy the Service

After updating your `configuration.nix`, rebuild and switch the system:

```bash
sudo nixos-rebuild switch
```

The service will start and monitor and stream to the specified WLED IP address.
