from PyQt6.QtWidgets import QGraphicsView, QGraphicsScene, QGraphicsTextItem
from PyQt6.QtGui import QFont, QColor

class CallGraphCanvas(QGraphicsView):
    """Placeholder QGraphicsView canvas for StakHAL call graph diagram."""
    def __init__(self, parent=None):
        super().__init__(parent)
        self.scene = QGraphicsScene(self)
        self.setScene(self.scene)
        
        title_item = QGraphicsTextItem("Call Graph Canvas (QGraphicsScene Placeholder)")
        title_item.setFont(QFont("SansSerif", 12, QFont.Weight.Bold))
        title_item.setDefaultTextColor(QColor(100, 100, 100))
        title_item.setPos(20, 20)
        self.scene.addItem(title_item)

    def load_graph_data(self, layout, edges):
        """Placeholder method to receive layout data from stakhal-py."""
        info_item = QGraphicsTextItem(
            f"Loaded {len(layout.positions)} nodes and {len(edges)} edges. Canvas Bounds: {layout.bounds}"
        )
        info_item.setFont(QFont("SansSerif", 10))
        info_item.setDefaultTextColor(QColor(60, 60, 60))
        info_item.setPos(20, 50)
        self.scene.addItem(info_item)
