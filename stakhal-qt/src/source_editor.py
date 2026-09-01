from PyQt6.QtWidgets import QWidget, QVBoxLayout, QLabel, QPlainTextEdit
from PyQt6.QtGui import QFont

class SourceEditorPanel(QWidget):
    """Placeholder panel for source code editor (QPlainTextEdit stub)."""
    def __init__(self, parent=None):
        super().__init__(parent)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(4, 4, 4, 4)
        
        self.header_label = QLabel("Source View (QPlainTextEdit Stub)")
        layout.addWidget(self.header_label)
        
        self.text_edit = QPlainTextEdit()
        self.text_edit.setFont(QFont("Monospace", 10))
        self.text_edit.setPlaceholderText("Source file content will be rendered here...")
        layout.addWidget(self.text_edit)

    def set_content(self, file_path, content):
        self.header_label.setText(f"Source View — {file_path}")
        self.text_edit.setPlainText(content)
