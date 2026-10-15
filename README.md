# 7zip-rpa-plugin

A high-performance, pure Rust 64-bit plugin for **7-Zip** that adds native support for opening and extracting Ren'Py visual novel archive containers (`.rpa`, `.rpi`).

---

## ✨ Features

- **Format Compatibility**:
  - **RPA-1.0** / `.rpi`: Direct zlib index decompression.
  - **RPA-2.0**: Hex offset indexing.
  - **RPA-3.0**: XOR obfuscation key decoding + variable-length prefixes.
  - **RPA-3.2**: Multi-subkey XOR obfuscation with dummy token handling.
- **Zero C++ Dependencies**:
  - Implements the Windows COM `IInArchive` interface natively in safe/unsafe Rust without MSVC C++ runtime dependencies (`msvcp140.dll` / `vcruntime140.dll`).
- **Low Memory Footprint (O(1))**:
  - Streams archive contents directly in 64 KB chunks to 7-Zip's `ISequentialOutStream`. Large files (background videos, music) extract with constant memory usage.
- **Virtual Folder Trees**:
  - Preserves full internal path hierarchies in 7-Zip File Manager for browsing and selective extraction.

---

## 🚀 Installation

Place `Rpa.64.dll` into your 7-Zip `Formats` folder:

```
C:\Program Files\7-Zip\Formats\Rpa.64.dll
```

### Option A: Using the Installer Script
Right-click `install.cmd` and select **Run as administrator**.

### Option B: Elevated PowerShell
```powershell
Copy-Item ".\Rpa.64.dll" "C:\Program Files\7-Zip\Formats\Rpa.64.dll" -Force
```

Restart 7-Zip File Manager if it is currently open. You can now double-click or right-click any `.rpa` or `.rpi` file to browse and extract its contents!

---

## 🛠️ Building from Source

### Prerequisites
- [Rust toolchain](https://www.rust-lang.org/) (`x86_64-pc-windows-msvc`)

### Build Release DLL
```bash
cargo build --release
```

The compiled 64-bit plugin DLL will be located at:
```
target/release/rpa.dll
```

Copy and rename it to `Rpa.64.dll` in your 7-Zip `Formats` directory:
```powershell
Copy-Item target\release\rpa.dll "C:\Program Files\7-Zip\Formats\Rpa.64.dll" -Force
```

---

## 🧪 Testing

Run the automated integration and COM interface tests:

```bash
cargo test
```

Included tests:
- Header parsing and key extraction across RPA-2.0, RPA-3.0, RPA-3.2
- Full parse, decompression, and file extraction from real Ren'Py archives (`sample.rpa`)
- Verification of binary signatures (e.g. TrueType font and JPEG magic bytes)
- Synthetic archive creation and extraction with XOR key masking and prefixes
- COM interface queries (`IID_IInArchive`, `IID_IUnknown`), reference counting, and 7-Zip metadata queries

---

## 📁 Project Architecture

```
renpy-7zip/
├── Cargo.toml                  # Cargo manifest (cdylib + rlib)
├── Rpa.64.dll                  # Precompiled 64-bit release DLL
├── install.cmd / install.ps1   # Quick installer scripts
├── src/
│   ├── lib.rs                  # 7-Zip DLL exports (CreateObject, GetHandlerProperty, GetHandlerProperty2, GetNumberOfFormats)
│   ├── com/                    # Pure Rust Windows COM implementation
│   │   ├── guid.rs             # GUID types & 7-Zip format constants
│   │   ├── interfaces.rs       # VTables: IInArchive, IInStream, ISequentialOutStream, callbacks
│   │   ├── propvariant.rs      # PROPVARIANT tagged union & OleAut32 BSTR allocators
│   │   └── stream.rs           # StreamReader: wraps IInStream as standard Rust Read + Seek
│   ├── models/                 # Structs & data models
│   │   ├── version.rs          # RpaVersion enum & header parser
│   │   ├── entry.rs            # FileEntry & FileIndex
│   │   ├── archive.rs          # RpaArchive container & metadata
│   │   ├── props.rs            # 7-Zip PropID constants & ArcInfoFlags
│   │   └── error.rs            # RpaError enum
│   ├── rpa/                    # Ren'Py archive parser & reader
│   │   ├── parser.rs           # Header detection, zlib decompression, Python pickle parsing
│   │   └── reader.rs           # Streaming file extractor (O(1) memory)
│   └── archive/                # 7-Zip IInArchive COM handler
│       └── in_archive.rs       # RpaInArchive COM object
└── test/
    ├── test_rpa.rs             # RPA engine integration tests
    └── test_com.rs             # COM interface lifecycle & property tests
```

---

## 📄 License

MIT License
