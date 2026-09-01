from PyQt6.QtWidgets import QApplication
from PyQt6.QtGui import QPalette, QColor
from PyQt6.QtCore import Qt

# Color Palette Constants (Exact values from stakhal-ui CSS)
COLOR_BG_DARK = "#0a0a0a"
COLOR_BG_PANEL = "#121212"
COLOR_BG_HOVER = "#1a1a1a"
COLOR_BG_ACTIVE = "#262626"
COLOR_BORDER_SUBTLE = "#262626"
COLOR_BORDER_MUTED = "#525252"
COLOR_FG_PRIMARY = "#e5e5e5"
COLOR_FG_MUTED = "#737373"
COLOR_FG_HEADING = "#f5f5f5"

# Reserved Status Colors (Exact values from stakhal-ui CSS)
COLOR_STATUS_OK = "#22c55e"       # Green
COLOR_STATUS_WARN = "#f59e0b"     # Yellow
COLOR_STATUS_ERROR = "#ef4444"    # Red

def apply_monochrome_theme(app: QApplication):
    """Apply the strict StakHAL monochrome dark theme to the Qt application."""
    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window, QColor(COLOR_BG_DARK))
    palette.setColor(QPalette.ColorRole.WindowText, QColor(COLOR_FG_PRIMARY))
    palette.setColor(QPalette.ColorRole.Base, QColor(COLOR_BG_PANEL))
    palette.setColor(QPalette.ColorRole.AlternateBase, QColor(COLOR_BG_DARK))
    palette.setColor(QPalette.ColorRole.ToolTipBase, QColor(COLOR_BG_PANEL))
    palette.setColor(QPalette.ColorRole.ToolTipText, QColor(COLOR_FG_PRIMARY))
    palette.setColor(QPalette.ColorRole.Text, QColor(COLOR_FG_PRIMARY))
    palette.setColor(QPalette.ColorRole.Button, QColor(COLOR_BG_PANEL))
    palette.setColor(QPalette.ColorRole.ButtonText, QColor(COLOR_FG_PRIMARY))
    palette.setColor(QPalette.ColorRole.BrightText, QColor("#ffffff"))
    palette.setColor(QPalette.ColorRole.Highlight, QColor(COLOR_FG_PRIMARY))
    palette.setColor(QPalette.ColorRole.HighlightedText, QColor(COLOR_BG_DARK))
    
    app.setPalette(palette)
    
    stylesheet = f"""
    * {{
        font-family: 'DejaVu Sans Mono', 'Liberation Mono', monospace;
        font-size: 13px;
        border-radius: 0px;
    }}
    
    QMainWindow, QDialog {{
        background-color: {COLOR_BG_DARK};
        color: {COLOR_FG_PRIMARY};
    }}
    
    QSplitter::handle {{
        background-color: {COLOR_BORDER_SUBTLE};
    }}
    
    QSplitter::handle:hover {{
        background-color: {COLOR_BORDER_MUTED};
    }}
    
    QToolBar {{
        background-color: {COLOR_BG_DARK};
        border-bottom: 1px solid {COLOR_BORDER_SUBTLE};
        spacing: 6px;
        padding: 4px;
    }}
    
    QStatusBar {{
        background-color: {COLOR_BG_PANEL};
        color: {COLOR_FG_MUTED};
        border-top: 1px solid {COLOR_BORDER_SUBTLE};
    }}
    
    QGraphicsView {{
        background-color: #060608;
        border: 1px solid {COLOR_BORDER_SUBTLE};
    }}
    
    QListWidget, QPlainTextEdit {{
        background-color: {COLOR_BG_PANEL};
        color: {COLOR_FG_PRIMARY};
        border: 1px solid {COLOR_BORDER_SUBTLE};
        selection-background-color: {COLOR_BG_ACTIVE};
        selection-color: #ffffff;
    }}
    
    QListWidget::item:hover {{
        background-color: {COLOR_BG_HOVER};
    }}
    
    QListWidget::item:selected {{
        background-color: {COLOR_BG_ACTIVE};
        color: #ffffff;
    }}
    
    QLabel {{
        color: {COLOR_FG_PRIMARY};
    }}
    
    QLabel[class="heading"] {{
        color: {COLOR_FG_HEADING};
        font-weight: bold;
    }}
    
    QPushButton {{
        border: 1px solid {COLOR_BORDER_SUBTLE};
        background-color: {COLOR_BG_PANEL};
        color: {COLOR_FG_PRIMARY};
        padding: 4px 12px;
        border-radius: 0px;
    }}
    
    QPushButton:hover {{
        border-color: {COLOR_BORDER_MUTED};
        background-color: {COLOR_BG_HOVER};
        color: #ffffff;
    }}
    
    QPushButton:pressed {{
        background-color: {COLOR_BG_ACTIVE};
    }}
    """
    app.setStyleSheet(stylesheet)
