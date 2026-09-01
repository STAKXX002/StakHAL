from PyQt6.QtWidgets import QWidget, QVBoxLayout, QLabel, QListWidget, QListWidgetItem

class PvEditorPanel(QWidget):
    """Placeholder panel for PV declaration editor."""
    def __init__(self, parent=None):
        super().__init__(parent)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(4, 4, 4, 4)
        
        self.title_label = QLabel("PV Declarations (Editor Stub)")
        layout.addWidget(self.title_label)
        
        self.list_widget = QListWidget()
        layout.addWidget(self.list_widget)

    def populate(self, pv_declarations):
        self.list_widget.clear()
        for decl in pv_declarations:
            init_str = f" = {decl.initial_value}" if decl.initial_value else ""
            item_text = f"{decl.name}: {decl.type_str}{init_str} (line {decl.line})"
            self.list_widget.addItem(QListWidgetItem(item_text))
