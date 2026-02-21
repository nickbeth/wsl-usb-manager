# Building instructions

This project is designed to be built on both Windows and Linux platforms. Below are the instructions for building on each platform. The executable produced is intended to run on Windows only, but cross-compilation from Linux is supported.

The following steps will guide you through the process of setting up your environment for building the project on both Windows and Linux. After following these instructions, you'll be able to build the project with the usual Cargo commands (with no target specification needed):

```bash
cargo build --release
```

## Building on Windows

Building on Windows should be straightforward. The only requirement is to have the `x86_64-pc-windows-msvc` target installed, which is typically included in the default Rust installation on Windows.

You can verify this by running:

```powershell
rustup show
```

and looking for the `x86_64-pc-windows-msvc` target in the list of installed targets.

### Troubleshooting

If you get an error about `link.exe` not being found, it means that the Visual Studio Build Tools are not properly installed or configured. If you let `rustup` install the MSVC toolchain, you might need to uninstall it and **reinstall manually** by downloading [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/?q=build+tools), and following the instructions in the installer to install the "Desktop development with C++" workload, making sure to select the "MSVC v143 - VS 2022 C++ x64/x86 build tools" and the "Windows 11 SDK" components.

## Building on Linux

The project is configured to build for the `x86_64-pc-windows-msvc` target on all platforms, including Linux. Follow the instructions below to set up your Linux environment for cross-compilation. Obviously, you won't be able to run the resulting executable on Linux, but this can still be useful for development, and is required for rust-analyzer to work properly.

### Install LLVM for `clang-cl` and `lld-link`

We will leverage the native cross-compilation capabilities of LLVM, which includes drivers for the MSVC compiler and linker, that understand the MSVC command line arguments and can link against MSVC libraries.

Verify that `clang-cl` and `lld-link` are available in your PATH by running:

```bash
clang-cl --version
lld-link --version
```

If your distribution doesn't ship with LLVM pre-installed, you can install it using your package manager:

- Debian/Ubuntu: `sudo apt install llvm clang lld`
- Fedora: `sudo dnf install llvm clang lld`

### Installing the Rust target

To install the Rust target for Windows, run the following command:

```bash
rustup target add x86_64-pc-windows-msvc
```

### Installing the required libraries

You will also need to install the `CRT` and Windows SDK libraries. We will achieve this by leveraging one of the various `xwin` packages available as a Cargo crate.

Install `xwin` locally by running:

```bash
cargo install xwin
```

> [!TIP]
> Cargo will install `xwin` for the current user only, so if you want to call it as sudo, you will need to specify the full path to the executable, which is typically `~/.cargo/bin/xwin`.

Then, run `xwin` in a directory of your choice to download the required libraries.

```bash
xwin --accept-license splat --output <dir>
```

Take note of the `<dir>` you specify, as you will need to set the linker search path to this directory when building the project.

> [!NOTE]
> Please note that `xwin` caches downloaded files in a folder relative to the current working directory, and then moves them to the specified output directory. The move operation might fail if the cache and output dirs are on different mountpoints, so try to `cd` into the parent directory of the output directory before running `xwin` if you're encountering issues.

### Setting up the toolchain

To build the project, you will need to tell cargo to use lld-link as the linker for the `x86_64-pc-windows-msvc` target, and to add the directory containing the libraries downloaded by `xwin` to the linker's search path.

Add the following to `~/.cargo/config.toml` (or create it if it doesn't exist):

```toml
[env]
CC_x86_64_pc_windows_msvc = "clang-cl"
CXX_x86_64_pc_windows_msvc = "clang-cl"
AR_x86_64_pc_windows_msvc = "llvm-lib"

[target.x86_64-pc-windows-msvc]
linker = "lld-link"
rustflags = [
  "-Clink-arg=/libpath:<dir>/crt/lib/x86_64",
  "-Clink-arg=/libpath:<dir>/sdk/lib/ucrt/x86_64",
  "-Clink-arg=/libpath:<dir>/sdk/lib/um/x86_64",
]
```

Make sure to replace `<dir>` with the actual directory you specified when running `xwin`.
