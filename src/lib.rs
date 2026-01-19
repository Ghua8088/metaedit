use pyo3::prelude::*;
use std::path::Path;
use std::collections::HashMap;
use std::fs;

#[cfg(target_os = "macos")]
use plist::Value;

#[pyclass]
#[derive(Clone)]
pub struct MetadataEditor {
    file_path: String,
    icon_path: Option<String>,
    version: Option<String>,
    strings: HashMap<String, String>,
}

#[cfg(target_os = "windows")]
use editpe::{Image, ResourceDirectory, VersionInfo, VersionStringTable};

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
    fn apply_windows(&self) -> PyResult<()> {
        let data = fs::read(&self.file_path)?;
        let mut image = Image::parse(&data).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("PE Parse error: {:?}", e)))?;
        
        let mut resources = image.resource_directory().cloned().unwrap_or_default();
        
        println!("Rust (Windows): Patching PE Resources in {}", self.file_path);
        
        // 1. Set Icon
        if let Some(icon_path) = &self.icon_path {
            let icon_data = fs::read(icon_path)?;
            resources.set_main_icon(icon_data).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to set icon: {:?}", e)))?;
        }

        // 2. Set Version Strings
        if !self.strings.is_empty() || self.version.is_some() {
            let mut version_info = resources.get_version_info().map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to get version info: {:?}", e)))?.unwrap_or_default();
            
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
            
            resources.set_version_info(&version_info).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to set version info: {:?}", e)))?;
        }

        // 3. Re-insert and Write back
        image.set_resource_directory(resources).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to set resources: {:?}", e)))?;
        let final_data = image.data();
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
fn _metaedit(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MetadataEditor>()?;
    m.add_function(wrap_pyfunction!(edit, m)?)?;
    m.add_function(wrap_pyfunction!(update, m)?)?;
    Ok(())
}
