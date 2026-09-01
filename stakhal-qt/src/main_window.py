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
        self.setWindowTitle("StakHAL — Hardware Abstraction Inspector (PyQt6)")
        self.resize(1200, 800)
        
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
        
        export_png_action = QAction("Export Call Graph PNG", self)
        export_png_action.triggered.connect(self.export_canvas_png)
        file_menu.addAction(export_png_action)

        exit_action = QAction("Exit", self)
        exit_action.triggered.connect(self.close)
        file_menu.addAction(exit_action)

        view_menu = menu_bar.addMenu("View")
        fit_action = QAction("Fit to View", self)
        fit_action.triggered.connect(self.on_fit_to_view)
        view_menu.addAction(fit_action)

        help_menu = menu_bar.addMenu("Help")
        about_action = QAction("About StakHAL Qt", self)
        help_menu.addAction(about_action)

        # Toolbar
        toolbar = QToolBar("Main Toolbar", self)
        self.addToolBar(toolbar)
        
        btn_open = QAction("Open Project", self)
        toolbar.addAction(btn_open)
        
        btn_fit = QAction("Fit to View", self)
        btn_fit.triggered.connect(self.on_fit_to_view)
        toolbar.addAction(btn_fit)

        btn_export = QAction("Export PNG", self)
        btn_export.triggered.connect(self.export_canvas_png)
        toolbar.addAction(btn_export)

        # Status Bar
        self.status_bar = QStatusBar(self)
        self.setStatusBar(self.status_bar)

    def _init_ui_layout(self):
        main_splitter = QSplitter(Qt.Orientation.Horizontal, self)
        
        self.pv_editor = PvEditorPanel(self)
        main_splitter.addWidget(self.pv_editor)

        right_splitter = QSplitter(Qt.Orientation.Vertical, self)
        
        self.call_graph_canvas = CallGraphCanvas(self)
        right_splitter.addWidget(self.call_graph_canvas)
        
        self.source_editor = SourceEditorPanel(self)
        right_splitter.addWidget(self.source_editor)
        
        main_splitter.addWidget(right_splitter)
        main_splitter.setSizes([260, 940])
        right_splitter.setSizes([520, 240])

        self.setCentralWidget(main_splitter)

    def load_project_and_analyze(self, project_dir: str):
        if not os.path.exists(project_dir):
            self.status_bar.showMessage(f"Fixture directory not found: {project_dir}")
            return
        
        # Core Analysis via stakhal-py
        self.project = stakhal_py.load_project_from_dir(project_dir)
        self.graph_layout = stakhal_py.compute_graph_layout(self.project.call_graph_edges, [])
        
        # Populate Panels
        self.pv_editor.populate(self.project.pv_declarations)
        self.call_graph_canvas.load_graph_data(self.graph_layout, self.project.call_graph_edges)
        
        main_c_path = self.project.meta.main_c_path
        if os.path.exists(main_c_path):
            with open(main_c_path, "r", encoding="utf-8") as f:
                content = f.read()
            self.source_editor.set_content(main_c_path, content)

        # Status Bar Output
        status_msg = (
            f"Project: {self.project.meta.name} ({self.project.meta.mcu_name}) | "
            f"PV Decls: {len(self.project.pv_declarations)} | "
            f"Graph Nodes: {len(self.graph_layout.positions)} | "
            f"Edges: {len(self.project.call_graph_edges)}"
        )
        self.status_bar.showMessage(status_msg)
        print(f"STATUS BAR OUTPUT: {status_msg}")

    def on_fit_to_view(self):
        self.call_graph_canvas.fit_to_view()

    def export_canvas_png(self, path=None):
        if not path:
            artifacts_dir = os.path.abspath(
                os.path.join(os.path.dirname(__file__), "../../artifacts")
            )
            path = os.path.join(artifacts_dir, "stakhal_qt_call_graph.png")
        return self.call_graph_canvas.export_to_png(path)
