use pyo3::prelude::*;
use std::path::Path;
use std::collections::HashMap;
use std::fs;
use pyo3::create_exception;

#[cfg(target_os = "macos")]
use plist::Value;

// Define custom exceptions
create_exception!(_metaedit, MetaEditError, pyo3::exceptions::PyException);
create_exception!(_metaedit, PEParseError, MetaEditError);
create_exception!(_metaedit, IconError, MetaEditError);
// create_exception!(_metaedit, SigningError, MetaEditError);

#[pyclass]
#[derive(Clone)]
pub struct MetadataEditor {
    file_path: String,
    icon_path: Option<String>,
    version: Option<String>,
    strings: HashMap<String, String>,
}

#[cfg(target_os = "windows")]
use editpe::{Image, VersionStringTable};
#[cfg(target_os = "windows")]
use image::{ImageReader, imageops::FilterType, ExtendedColorType};
#[cfg(target_os = "windows")]
use image::codecs::ico::{IcoEncoder, IcoFrame};
#[cfg(target_os = "windows")]
use std::io::Cursor;

#[pymethods]
impl MetadataEditor {
    #[new]
    pub fn new(file_path: String) -> Self {
        MetadataEditor {
            file_path,
            icon_path: None,
            version: None,
            strings: HashMap::new(),
        }
    }

    pub fn set_icon(mut sli: PyRefMut<'_, Self>, icon_path: String) -> PyRefMut<'_, Self> {
        sli.icon_path = Some(icon_path);
        sli
    }

    pub fn set_version(mut sli: PyRefMut<'_, Self>, version: String) -> PyRefMut<'_, Self> {
        sli.version = Some(version);
        sli
    }

    pub fn set_string(mut sli: PyRefMut<'_, Self>, key: String, value: String) -> PyRefMut<'_, Self> {
        sli.strings.insert(key, value);
        sli
    }

    #[cfg(target_os = "windows")]
    pub fn remove_signature(&self) -> PyResult<()> {
        let path = Path::new(&self.file_path);
        if !path.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyFileNotFoundError, _>(
                format!("File not found: {}", self.file_path),
            ));
        }

        let mut data = fs::read(path)?;
        if strip_pe_signature(&mut data) {
            fs::write(path, data)?;
        }
        Ok(())
    }

    pub fn apply(&self) -> PyResult<()> {
        let path = Path::new(&self.file_path);
        if !path.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyFileNotFoundError, _>(
                format!("File not found: {}", self.file_path),
            ));
        }

        #[cfg(target_os = "windows")]
        {
            self.apply_windows()?;
        }

        #[cfg(target_os = "macos")]
        {
            self.apply_macos()?;
        }

        #[cfg(target_os = "linux")]
        {
            self.apply_linux()?;
        }

        Ok(())
    }
}

impl MetadataEditor {
    #[cfg(target_os = "windows")]
    fn process_icon_windows(&self, icon_path: &str) -> PyResult<Vec<u8>> {
        let path = Path::new(icon_path);

        // Try decoding as image to see if we can generate a better ICO
        if let Ok(reader) = ImageReader::open(path) {
            if let Ok(img) = reader.decode() {
                // Generate multi-size ICO
                // Windows prefers 256x256 PNG, others using BMP format for crispness at low res.
                // Standard sizes: 256, 128, 64, 48, 32, 24, 16.
                let sizes = vec![256, 128, 64, 48, 32, 24, 16];
                let mut frames = Vec::new();
                
                for size in sizes {
                    let resized = img.resize(size, size, FilterType::Lanczos3);
                    let width = resized.width();
                    let height = resized.height();
                    
                    if size >= 128 {
                        // Use PNG for large icons (Vista+ support)
                        let buf = resized.clone().into_rgba8().into_vec();
                        if let Ok(frame) = IcoFrame::as_png(&buf, width, height, ExtendedColorType::Rgba8) {
                            frames.push(frame);
                        }
                    } else {
                        // Use manually constructed BMP for smaller icons to avoid artifacting
                        if let Ok(bmp_data) = create_ico_bmp_data(&resized, width, height) {
                             if let Ok(frame) = IcoFrame::with_encoded(bmp_data, width, height, ExtendedColorType::Rgba8) {
                                frames.push(frame);
                            }
                        }
                    }
                }
                
                if !frames.is_empty() {
                    let mut out_buffer = Vec::new();
                    let mut cursor = Cursor::new(&mut out_buffer);
                    let encoder = IcoEncoder::new(&mut cursor);
                    if encoder.encode_images(&frames).is_ok() {
                        return Ok(out_buffer);
                    }
                }
            }
        }

        // Fallback: read file directly
        fs::read(path).map_err(|e| PyErr::new::<IconError, _>(format!("Failed to read icon file: {:?}", e)))
    }

    #[cfg(target_os = "windows")]
    fn apply_windows(&self) -> PyResult<()> {
        let data = fs::read(&self.file_path)?;
        let mut image = Image::parse(&data).map_err(|e| PyErr::new::<PEParseError, _>(format!("PE Parse error: {:?}", e)))?;
        
        let mut resources = image.resource_directory().cloned().unwrap_or_default();
        
        println!("Rust (Windows): Patching PE Resources in {}", self.file_path);
        
        // 1. Set Icon
        if let Some(icon_path) = &self.icon_path {
            let icon_data = self.process_icon_windows(icon_path)?;
            resources.set_main_icon(icon_data).map_err(|e| PyErr::new::<PEParseError, _>(format!("Failed to set icon: {:?}", e)))?;
        }

        // 2. Set Version Strings
        if !self.strings.is_empty() || self.version.is_some() {
            let mut version_info = resources.get_version_info().map_err(|e| PyErr::new::<PEParseError, _>(format!("Failed to get version info: {:?}", e)))?.unwrap_or_default();
            
            if let Some(v) = &self.version {
                // FixedFileInfo version is numeric (Major.Minor)
                // We'll attempt to parse if possible, or leave as default for now as editpe uses VersionU32
                // Most users care about the string entries which we handle below
            }
            
            if let Some(table) = version_info.strings.get_mut(0) {
                if let Some(v) = &self.version {
                    table.strings.insert("FileVersion".to_string(), v.clone());
                    table.strings.insert("ProductVersion".to_string(), v.clone());
                }
                for (k, v) in &self.strings {
                    table.strings.insert(k.clone(), v.clone());
                }
            } else {
                // If no table exists, create one (040904b0 is US English)
                let mut strings = indexmap::IndexMap::default();
                if let Some(v) = &self.version {
                    strings.insert("FileVersion".to_string(), v.clone());
                    strings.insert("ProductVersion".to_string(), v.clone());
                }
                for (k, v) in &self.strings {
                    strings.insert(k.clone(), v.clone());
                }
                version_info.strings.push(VersionStringTable {
                    key: "040904b0".to_string(),
                    strings,
                });
            }
            
            resources.set_version_info(&version_info).map_err(|e| PyErr::new::<PEParseError, _>(format!("Failed to set version info: {:?}", e)))?;
        }

        // 3. Re-insert and Write back
        image.set_resource_directory(resources).map_err(|e| PyErr::new::<PEParseError, _>(format!("Failed to set resources: {:?}", e)))?;
        let mut final_data = image.data().to_vec();
        
        // Strip signature to prevent corruption errors (hash mismatch)
        strip_pe_signature(&mut final_data);

        fs::write(&self.file_path, final_data)?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn apply_macos(&self) -> PyResult<()> {
        let path = Path::new(&self.file_path);
        let bundle_path = if self.file_path.ends_with(".app") {
            path.to_path_buf()
        } else {
            let parent = path.parent().unwrap_or(Path::new("."));
            let name = path.file_stem().unwrap().to_str().unwrap();
            parent.join(format!("{}.app", name))
        };

        let contents = bundle_path.join("Contents");
        let macos_dir = contents.join("MacOS");
        let resources_dir = contents.join("Resources");
        
        fs::create_dir_all(&macos_dir)?;
        fs::create_dir_all(&resources_dir)?;

        if path.is_file() {
            let target_bin = macos_dir.join(path.file_name().unwrap());
            fs::copy(path, target_bin)?;
        }

        let mut dict = HashMap::new();
        dict.insert("CFBundleExecutable".to_string(), Value::String(path.file_name().unwrap().to_str().unwrap().to_string()));
        
        if let Some(ver) = &self.version {
            dict.insert("CFBundleShortVersionString".to_string(), Value::String(ver.clone()));
            dict.insert("CFBundleVersion".to_string(), Value::String(ver.clone()));
        }

        if let Some(title) = self.strings.get("ProductName") {
            dict.insert("CFBundleName".to_string(), Value::String(title.clone()));
        }

        let plist_path = contents.join("Info.plist");
        plist::to_file_xml(plist_path, &dict).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        if let Some(icon) = &self.icon_path {
            let icon_source = Path::new(icon);
            if icon_source.exists() {
                let icon_dest = resources_dir.join("app.icns");
                fs::copy(icon_source, icon_dest)?;
            }
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn apply_linux(&self) -> PyResult<()> {
        let path = Path::new(&self.file_path);
        let parent = path.parent().unwrap_or(Path::new("."));
        let name = path.file_stem().unwrap().to_str().unwrap();
        let desktop_path = parent.join(format!("{}.desktop", name));

        let mut content = String::from("[Desktop Entry]\nType=Application\n");
        content.push_str(&format!("Name={}\n", self.strings.get("ProductName").unwrap_or(&name.to_string())));
        
        if let Some(ver) = &self.version {
            content.push_str(&format!("Version={}\n", ver));
        }

        content.push_str(&format!("Exec=./{}\n", path.file_name().unwrap().to_str().unwrap()));
        content.push_str("Terminal=false\n");

        if let Some(icon) = &self.icon_path {
            content.push_str(&format!("Icon={}\n", icon));
        }

        fs::write(desktop_path, content)?;
        Ok(())
    }
}

#[pyfunction]
#[pyo3(signature = (file_path, metadata=None))]
fn edit(file_path: String, metadata: Option<HashMap<String, String>>) -> MetadataEditor {
    let mut editor = MetadataEditor::new(file_path);
    if let Some(meta) = metadata {
        for (k, v) in meta {
            match k.as_str() {
                "icon" => { editor.icon_path = Some(v); },
                "version" => { editor.version = Some(v); },
                _ => { editor.strings.insert(k, v); }
            }
        }
    }
    editor
}

#[pyfunction]
#[pyo3(signature = (file_path, **kwargs))]
fn update(file_path: String, kwargs: Option<HashMap<String, String>>) -> PyResult<()> {
    let mut editor = MetadataEditor::new(file_path);
    if let Some(args) = kwargs {
        for (k, v) in args {
            match k.as_str() {
                "icon" => { editor.icon_path = Some(v); },
                "version" => { editor.version = Some(v); },
                _ => { editor.strings.insert(k, v); }
            }
        }
    }
    editor.apply()
}

#[pymodule]
fn _metaedit(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MetadataEditor>()?;
    m.add_function(wrap_pyfunction!(edit, m)?)?;
    m.add_function(wrap_pyfunction!(update, m)?)?;
    
    m.add("MetaEditError", py.get_type::<MetaEditError>())?;
    m.add("PEParseError", py.get_type::<PEParseError>())?;
    m.add("IconError", py.get_type::<IconError>())?;
    // m.add("SigningError", py.get_type::<SigningError>())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn create_ico_bmp_data(img: &image::DynamicImage, width: u32, height: u32) -> PyResult<Vec<u8>> {
    let rgba = img.to_rgba8();
    
    // Each row in the AND mask must be a multiple of 4 bytes (32 bits)
    // Formula: ((width + 31) / 32) * 4
    let mask_row_size = ((width + 31) / 32) * 4;
    let mask_size = mask_row_size * height;
    
    // Header (40) + XOR data (w*h*4) + AND mask
    let data_size = 40 + (width * height * 4) + mask_size;
    let mut data = Vec::with_capacity(data_size as usize);

    // BITMAPINFOHEADER (40 bytes)
    data.extend_from_slice(&(40u32).to_le_bytes()); // biSize
    data.extend_from_slice(&(width as i32).to_le_bytes()); // biWidth
    // ICO BMPs often use (height * 2) in the header to indicate XOR+AND combination
    data.extend_from_slice(&((height * 2) as i32).to_le_bytes()); // biHeight
    data.extend_from_slice(&(1u16).to_le_bytes()); // biPlanes
    data.extend_from_slice(&(32u16).to_le_bytes()); // biBitCount (BGRA)
    data.extend_from_slice(&(0u32).to_le_bytes()); // biCompression (BI_RGB)
    data.extend_from_slice(&(0u32).to_le_bytes()); // biSizeImage (can be 0 for BI_RGB)
    data.extend_from_slice(&(0u32).to_le_bytes()); // biXPelsPerMeter
    data.extend_from_slice(&(0u32).to_le_bytes()); // biYPelsPerMeter
    data.extend_from_slice(&(0u32).to_le_bytes()); // biClrUsed
    data.extend_from_slice(&(0u32).to_le_bytes()); // biClrImportant

    // XOR Mask (Pixel Data) - Stored Bottom-Up, BGRA format
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            data.push(pixel[2]); // B
            data.push(pixel[1]); // G
            data.push(pixel[0]); // R
            data.push(pixel[3]); // A
        }
    }

    // AND Mask (1-bit transparency) - Stored Bottom-Up
    // 0 = Opaque, 1 = Transparent.
    // Since we used Alpha channel in XOR mask (32-bit), this is technically redundant on modern Windows,
    // but absolutely required for legacy compatibility and valid BMP structure in ICO.
    for y in (0..height).rev() {
        let mut row_bytes = vec![0u8; mask_row_size as usize];
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            // If alpha is 0, we mark it as transparent (1) in the AND mask.
            // Otherwise opaque (0).
            if pixel[3] == 0 {
                let byte_idx = (x / 8) as usize;
                let bit_idx = 7 - (x % 8);
                row_bytes[byte_idx] |= 1 << bit_idx;
            }
        }
        data.extend_from_slice(&row_bytes);
    }
    
    Ok(data)
}

#[cfg(target_os = "windows")]
fn strip_pe_signature(data: &mut Vec<u8>) -> bool {
    // Minimum size for DOS header + PE Sig + File Header
    if data.len() < 0x40 { return false; }
    
    // Read e_lfanew (offset to PE header)
    let e_lfanew = u32::from_le_bytes(data[0x3c..0x40].try_into().unwrap()) as usize;
    if data.len() < e_lfanew + 4 + 20 + 2 { return false; }
    
    // Validate PE signature "PE\0\0"
    if &data[e_lfanew..e_lfanew+4] != b"PE\0\0" { return false; }
    
    // Optional Header Magic is at e_lfanew + 4 (Sig) + 20 (FileHeader)
    let opt_header_offset = e_lfanew + 24;
    let magic = u16::from_le_bytes(data[opt_header_offset..opt_header_offset+2].try_into().unwrap());
    
    // Locate Security Directory Entry (Index 4 in Data Directories)
    // PE32 (0x10b): Data Dirs start at offset 96 (0x60) in Optional Header
    // PE32+ (0x20b): Data Dirs start at offset 112 (0x70) in Optional Header
    // Security entry is 4th (index 4), so + 4 * 8 bytes
    let rva_offset = match magic {
        0x10b => opt_header_offset + 96 + 32,
        0x20b => opt_header_offset + 112 + 32,
        _ => { println!("DEBUG: Unknown magic: {:x}, opt_header_offset: {}", magic, opt_header_offset); return false; },
    };
    
    if data.len() < rva_offset + 8 { println!("DEBUG: File too short for rva"); return false; }
    
    let virt_addr = u32::from_le_bytes(data[rva_offset..rva_offset+4].try_into().unwrap());
    let size = u32::from_le_bytes(data[rva_offset+4..rva_offset+8].try_into().unwrap());
    
    println!("DEBUG: Found Security Dir at offset {}: VA={:x}, Size={}", rva_offset, virt_addr, size);

    if virt_addr == 0 || size == 0 {
        return false; // No signature present
    }
    
    // Zero out the Security Directory entry
    data[rva_offset..rva_offset+8].fill(0);
    
    // Truncate the file if the certificate table is at the very end
    let start = virt_addr as usize;
    let end = start + size as usize;
    
    // Safety check: ensure start is within bounds
    if start <= data.len() && end <= data.len() {
        // If the table ends exactly at the file end, we can safely truncate
        if end == data.len() {
            data.truncate(start);
        }
    }
    
    true
}
