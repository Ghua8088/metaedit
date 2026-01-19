# MetaEdit
 
> **Surgical Binary Metadata Editing. Powered by Rust.**

![Python](https://img.shields.io/badge/python-3670A0?style=for-the-badge&logo=python&logoColor=ffdd54)
![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![PyPI](https://img.shields.io/pypi/v/metaedit.svg)

MetaEdit is a high-performance, cross-platform library and CLI tool designed for **surgical binary manipulation**. It allows you to patch executable metadata (icons, version strings, manifests) with 1:1 precision, without re-compiling your source.

Originally built for the **CactusCat** framework, MetaEdit is now available as a standalone power-tool for any Python or Rust project that needs to brand and identify binaries post-build.

---

## Features

- **Rust-Native Performance:** Zero-overhead metadata patching. No reliance on legacy `.exe` tools like `rcedit`.
- **1:1 Binary Surgery:** Directly manipulates PE resources (Windows), Info.plist (macOS), and Desktop entries (Linux).
- **Icon Injection:** High-fidelity icon patching with multi-resolution support.
- **Fluent API:** Modern, chainable Python API for complex branding sequences.
- **Unified CLI:** A single command to rule all platforms.

---

## 💻 Python Usage

MetaEdit provides three distinct ways to brand your binaries:

### 1. Dictionary-based (Clean & Pythonic)
Best for dynamic configuration files or settings blocks.
```python
import metaedit

metaedit.edit("app.exe", {
    "icon": "app.ico",
    "version": "1.2.3.4",
    "CompanyName": "CactusCat Industries",
    "FileDescription": "The Next Big Thing",
    "LegalCopyright": "© 2026"
}).apply()
```

### 2. Direct Update (Fast & Simple)
One-liner for quick patches.
```python
metaedit.update("app.exe", icon="logo.ico", version="1.0.0.1")
```

### 3. Fluent API (Chainable)
Maximum control and readability.
```python
metaedit.edit("app.exe") \
    .set_icon("app.ico") \
    .set_version("1.2.3.4") \
    .set_string("ProductName", "CactusCat Engine") \
    .apply()
```

---

## 📟 CLI Usage

MetaEdit is its own standalone power-tool. Install globally and brand anything.

```bash
metaedit app.exe --icon logo.ico --version 2.0.0.0 --company "Acme Corp"
```

---

## 🌍 Platform Specifics

| Platform | Technique | Result |
| :--- | :--- | :--- |
| **Windows** | Direct PE Resource Patching | Built-in Icons & Version Info (Task Manager ready) |
| **macOS** | Bundle Synthesis | Full `.app` structure with `Info.plist` and `.icns` |
| **Linux** | Desktop Integration | `.desktop` entry generation and Icon distribution |

---

## 📦 Installation

```bash
pip install metaedit
```

MetaEdit ships with **pre-compiled Rust wheels**. You do not need a Rust compiler installed to use it.

---

## ⚖️ License
MetaEdit is open-source software licensed under the MIT License.
