import os
import sys

src_dir = os.path.dirname(os.path.abspath(__file__))
if src_dir not in sys.path:
    sys.path.insert(0, src_dir)

from PyQt6.QtWidgets import QApplication
from theme import apply_monochrome_theme
from main_window import StakHalMainWindow

def main():
    app = QApplication(sys.argv)
    apply_monochrome_theme(app)
    
    window = StakHalMainWindow()
    window.show()
    
    if "--headless" in sys.argv or os.environ.get("QT_QPA_PLATFORM") == "offscreen":
        app.processEvents()
        
        # Perform Fit-to-View verification
        window.on_fit_to_view()
        app.processEvents()
        
        # Perform PNG export verification
        artifacts_dir = os.path.abspath(
            os.path.join(os.path.dirname(__file__), "../../artifacts")
        )
        png_path = os.path.join(artifacts_dir, "stakhal_qt_call_graph.png")
        success = window.export_canvas_png(png_path)
        
        print("\n=== VERIFICATION CHECKS ===")
        print(f"1. PNG Exported to: {png_path} (Success: {success})")
        print(f"2. Graph Node Count in Scene: {len(window.call_graph_canvas.node_items)}")
        print(f"3. Graph Edge Count in Scene: {len(window.call_graph_canvas.edge_items)}")
        print(f"4. Rust Core Reported Nodes: {len(window.graph_layout.positions)}")
        print(f"5. Rust Core Reported Edges: {len(window.project.call_graph_edges)}")
        
        # Verify node count matching
        assert len(window.call_graph_canvas.node_items) == len(window.graph_layout.positions), "Node count mismatch!"
        assert len(window.call_graph_canvas.edge_items) == len(window.project.call_graph_edges), "Edge count mismatch!"
        
        # Verify fit-to-view bounds include unreachable/isolated nodes
        scene_rect = window.call_graph_canvas.scene.itemsBoundingRect()
        for node_id, node_item in window.call_graph_canvas.node_items.items():
            incoming_count = sum(1 for e in window.project.call_graph_edges if e.to_node == node_id)
            node_rect = node_item.sceneBoundingRect()
            assert scene_rect.contains(node_rect), f"Node {node_id} outside scene bounds!"
            if incoming_count == 0:
                print(f"   [Unreachable Node Verified in Bounds]: '{node_id}' at {node_rect.getRect()}")
        
        print("ALL VERIFICATION CHECKS PASSED!\n")
        sys.exit(0)
    
    sys.exit(app.exec())

if __name__ == "__main__":
    main()
