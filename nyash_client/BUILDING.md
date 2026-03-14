# Building nyash_client

Cross-compilation guide for **Ubuntu 24.04 LTS** — produces native Linux binary and Windows `.exe` from a single build server.

## Requirements

- Ubuntu 24.04 LTS (Noble Numbat)
- Internet access on the build server
- No GPU required — OpenCL CPU runtime is provided by the user at runtime

---

## Step 1 — System packages

```bash
sudo apt update && sudo apt install -y \
    curl wget git unzip build-essential pkg-config \
    gcc-mingw-w64-x86-64 mingw-w64 mingw-w64-tools \
    ocl-icd-opencl-dev opencl-headers pocl-opencl-icd \
    protobuf-compiler
```

---

## Step 2 — Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Select: 1 (default)

source $HOME/.cargo/env

rustup target add x86_64-pc-windows-gnu
```

Verify:

```bash
rustc --version
protoc --version
```

---

## Step 3 — OpenCL Windows stub

The build server does not need a GPU or OpenCL runtime. Only a linker stub (`libOpenCL.a`) is required at compile time. The actual OpenCL runtime is provided by the user's OS/drivers at runtime.

```bash
cd ~
wget https://github.com/KhronosGroup/OpenCL-SDK/releases/download/v2023.12.14/OpenCL-SDK-v2023.12.14-Win-x64.zip
unzip OpenCL-SDK-v2023.12.14-Win-x64.zip -d opencl-sdk

SDK=~/opencl-sdk/OpenCL-SDK-v2023.12.14-Win-x64

# Generate .def from DLL
cd $SDK/bin
gendef OpenCL.dll

# Generate libOpenCL.a from .def
x86_64-w64-mingw32-dlltool \
    -D OpenCL.dll \
    -d OpenCL.def \
    -l libOpenCL.a

# Install into MinGW sysroot
sudo cp libOpenCL.a /usr/x86_64-w64-mingw32/lib/
sudo cp -r $SDK/include/CL /usr/x86_64-w64-mingw32/include/
```

Verify:

```bash
ls /usr/x86_64-w64-mingw32/lib/libOpenCL.a
ls /usr/x86_64-w64-mingw32/include/CL/cl.h
```

---

## Step 4 — Clone the repository

```bash
cd ~
git clone https://github.com/Nyanraltotlapun/nyash-aes-xts256-plain64.git
cd nyash-aes-xts256-plain64/nyash_client
```

---

## Step 5 — Cargo config

Create `.cargo/config.toml` in the `nyash_client` directory:

```bash
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"
rustflags = ["-L", "/usr/x86_64-w64-mingw32/lib"]

[target.x86_64-unknown-linux-gnu]
linker = "gcc"

[env]
PROTOC = "/usr/bin/protoc"
EOF
```

---

## Step 6 — Build

```bash
rustup target add x86_64-pc-windows-gnu

# Linux
cargo build --release

# Windows
cargo build --release --target x86_64-pc-windows-gnu
```

---

## Output

| Platform | Path |
|----------|------|
| Linux    | `target/release/nyash-client` |
| Windows  | `target/x86_64-pc-windows-gnu/release/nyash-client.exe` |

---

## Runtime requirements for end users

The `.exe` does **not** bundle an OpenCL runtime — it is loaded dynamically from the user's system.

| OS      | OpenCL source                                      |
|---------|----------------------------------------------------|
| Windows | Intel CPU Runtime for OpenCL — installed automatically with Intel drivers, or download from [intel.com](https://www.intel.com/content/www/us/en/developer/articles/tool/opencl-drivers.html) |
| Linux   | `sudo apt install pocl-opencl-icd` (CPU) or vendor GPU driver |
