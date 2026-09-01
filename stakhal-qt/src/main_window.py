import os
import sys
from PyQt6.QtWidgets import QMainWindow, QSplitter, QStatusBar, QToolBar
from PyQt6.QtGui import QAction
from PyQt6.QtCore import Qt

from pv_editor import PvEditorPanel
from call_graph_canvas import CallGraphCanvas
from source_editor import SourceEditorPanel

import stakhal_py

class StakHalMainWindow(QMainWindow):
    def __init__(self, fixture_dir=None):
        super().__init__()
        self.setWindowTitle("StakHAL — PyQt6 Frontend Scaffold")
        self.resize(1100, 700)
        
        self.project = None
        self.graph_layout = None
        
        self._init_menu_and_toolbar()
        self._init_ui_layout()
        
        if not fixture_dir:
            fixture_dir = os.path.abspath(
                os.path.join(
                    os.path.dirname(__file__),
                    "../../stakhal-core/tests/fixtures/stm32_03_timers"
                )
            )
            
        self.load_project_and_analyze(fixture_dir)

    def _init_menu_and_toolbar(self):
        # Menu bar
        menu_bar = self.menuBar()
        file_menu = menu_bar.addMenu("File")
        open_action = QAction("Open Project...", self)
        file_menu.addAction(open_action)
        exit_action = QAction("Exit", self)
        exit_action.triggered.connect(self.close)
        file_menu.addAction(exit_action)

        help_menu = menu_bar.addMenu("Help")
        about_action = QAction("About StakHAL Qt", self)
        help_menu.addAction(about_action)

        # Toolbar stub
        toolbar = QToolBar("Main Toolbar", self)
        self.addToolBar(toolbar)
        btn_open = QAction("Open Project", self)
        toolbar.addAction(btn_open)
        btn_refresh = QAction("Run Analysis", self)
        toolbar.addAction(btn_refresh)

        # Status Bar
        self.status_bar = QStatusBar(self)
        self.setStatusBar(self.status_bar)

    def _init_ui_layout(self):
        # Main horizontal splitter
        main_splitter = QSplitter(Qt.Orientation.Horizontal, self)
        
        # Left panel: PV Editor Stub
        self.pv_editor = PvEditorPanel(self)
        main_splitter.addWidget(self.pv_editor)

        # Right vertical splitter (Call Graph + Source View)
        right_splitter = QSplitter(Qt.Orientation.Vertical, self)
        
        # Central canvas: Call Graph View Stub
        self.call_graph_canvas = CallGraphCanvas(self)
        right_splitter.addWidget(self.call_graph_canvas)
        
        # Bottom panel: Source Editor Stub
        self.source_editor = SourceEditorPanel(self)
        right_splitter.addWidget(self.source_editor)
        
        main_splitter.addWidget(right_splitter)
        main_splitter.setSizes([250, 850])
        right_splitter.setSizes([450, 250])

        self.setCentralWidget(main_splitter)

    def load_project_and_analyze(self, project_dir: str):
        if not os.path.exists(project_dir):
            self.status_bar.showMessage(f"Fixture directory not found: {project_dir}")
            return
        
        # Run stakhal-py core analysis
        self.project = stakhal_py.load_project_from_dir(project_dir)
        self.graph_layout = stakhal_py.compute_graph_layout(self.project.call_graph_edges, [])
        
        # Populate left panel
        self.pv_editor.populate(self.project.pv_declarations)
        
        # Populate central canvas stub
        self.call_graph_canvas.load_graph_data(self.graph_layout, self.project.call_graph_edges)
        
        # Populate source editor stub
        main_c_path = self.project.meta.main_c_path
        if os.path.exists(main_c_path):
            with open(main_c_path, "r", encoding="utf-8") as f:
                content = f.read()
            self.source_editor.set_content(main_c_path, content)

        # Update status bar message
        status_msg = (
            f"Project: {self.project.meta.name} ({self.project.meta.mcu_name}) | "
            f"PV Decls: {len(self.project.pv_declarations)} | "
            f"Graph Nodes: {len(self.graph_layout.positions)} | "
            f"Edges: {len(self.project.call_graph_edges)}"
        )
        self.status_bar.showMessage(status_msg)
        print(f"STATUS BAR OUTPUT: {status_msg}")
