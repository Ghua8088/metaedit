from ._metaedit import MetadataEditor as _MetadataEditor
from pathlib import Path
from typing import Optional, Union

class MetadataEditor:
    """
    High-level Python wrapper for the Rust-powered Metadata Editor.
    """
    def __init__(self, file_path: Union[str, Path]):
        self.file_path = str(Path(file_path).absolute())
        self._editor = _MetadataEditor(self.file_path)

    def set_icon(self, icon_path: Union[str, Path]):
        """Sets the executable icon (.ico)."""
        self._editor.set_icon(str(Path(icon_path).absolute()))
        return self

    def set_version(self, version: str):
        """Sets both File and Product version (e.g., '1.2.3.4')."""
        self._editor.set_version(version)
        return self

    def set_string(self, key: str, value: str):
        """Sets a version string (e.g., 'CompanyName', 'FileDescription')."""
        self._editor.set_string(key, value)
        return self

    def update(self, metadata: dict):
        """Updates multiple metadata fields from a dictionary."""
        for key, value in metadata.items():
            if key == "icon":
                self.set_icon(value)
            elif key == "version":
                self.set_version(value)
            else:
                self.set_string(key, value)
        return self

    def apply(self):
        """Saves changes to the file."""
        self._editor.apply()
        return self

def edit(file_path: Union[str, Path], metadata: Optional[dict] = None) -> MetadataEditor:
    """Quick helper to start editing. Optionally apply a dictionary of metadata."""
    editor = MetadataEditor(file_path)
    if metadata:
        editor.update(metadata)
    return editor

def update(file_path: Union[str, Path], **metadata):
    """One-shot function to update metadata and apply immediately."""
    return edit(file_path, metadata).apply()
