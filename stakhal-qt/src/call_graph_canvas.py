import math
import os
from PyQt6.QtWidgets import QGraphicsView, QGraphicsScene, QGraphicsItem, QGraphicsTextItem
from PyQt6.QtGui import (
    QPainter, QPainterPath, QPen, QColor, QFont, QBrush, QPolygonF, QImage
)
from PyQt6.QtCore import Qt, QRectF, QPointF

# Color Constants matching GTK4 draw.rs
COLOR_CANVAS_BG = QColor("#060608")
COLOR_HEADER_BG = QColor("#141414")
COLOR_HEADER_BORDER = QColor("#333333")
COLOR_HEADER_ICON = QColor("#d99178")
COLOR_HEADER_TEXT = QColor("#d9d9d9")

COLOR_NODE_NORMAL_BG = QColor("#17171c")
COLOR_NODE_NORMAL_BORDER = QColor("#33333d")
COLOR_NODE_HOVER_BG = QColor("#26262e")
COLOR_NODE_HOVER_BORDER = QColor("#e6e6e6")
COLOR_NODE_SELECT_BG = QColor("#242429")
COLOR_NODE_SELECT_BORDER = QColor("#ffffff")
COLOR_NODE_DIMMED_BG = QColor("#0d0d0f")
COLOR_NODE_DIMMED_BORDER = QColor("#242424")

COLOR_EDGE_NORMAL = QColor("#6e6e6e")
COLOR_EDGE_HIGHLIGHT = QColor("#ffffff")
COLOR_EDGE_DIMMED = QColor("#292929")
COLOR_SOCKET_DOT = QColor("#858585")


def get_rect_ray_intersection(rect_x, rect_y, rect_w, rect_h, target_x, target_y):
    """Compute the intersection point of a ray from rectangle center to target_point."""
    cx = rect_x + rect_w / 2.0
    cy = rect_y + rect_h / 2.0
    dx = target_x - cx
    dy = target_y - cy

    if dx == 0.0 and dy == 0.0:
        return (cx, cy)

    scale_x = (rect_w / 2.0) / dx if dx > 0.0 else ((-rect_w / 2.0) / dx if dx < 0.0 else float('inf'))
    scale_y = (rect_h / 2.0) / dy if dy > 0.0 else ((-rect_h / 2.0) / dy if dy < 0.0 else float('inf'))
    scale = min(scale_x, scale_y)

    return (cx + dx * scale, cy + dy * scale)


def get_node_status_color(node_id, edges):
    """Port of stakhal-ui's node status coloring rule."""
    outgoing_count = sum(1 for e in edges if e.from_node == node_id)
    incoming_count = sum(1 for e in edges if e.to_node == node_id)

    if node_id.endswith("_IRQHandler") and not node_id.startswith("HAL_"):
        if outgoing_count == 0:
            return QColor("#ef4444")  # Red: Unlinked IRQ handler
        elif outgoing_count > 2:
            return QColor("#f59e0b")  # Yellow: Shared vector IRQ handler chain

    if "Callback" in node_id or node_id.startswith("HAL_"):
        has_user_override = any(
            e.from_node == node_id and not e.to_node.startswith("HAL_") and not e.to_node.endswith("_IRQHandler")
            for e in edges
        )
        if has_user_override:
            return QColor("#22c55e")  # Green: User-implemented callback override
        elif outgoing_count == 0 and incoming_count > 0:
            return QColor("#f59e0b")  # Yellow: Unhandled weak callback

    return QColor("#bfbfbf")  # Neutral monochrome default


class ChainHeaderGraphicsItem(QGraphicsItem):
    """Header bar representing a peripheral swimlane / execution chain."""

    def __init__(self, header_layout, parent=None):
        super().__init__(parent)
        self.h_id = header_layout.handler_id
        self.label = header_layout.label
        self.w = header_layout.w
        self.h = header_layout.h
        self.is_collapsed = header_layout.is_collapsed
        self.setPos(header_layout.x, header_layout.y)

    def boundingRect(self):
        return QRectF(0, 0, self.w, self.h)

    def paint(self, painter, option, widget=None):
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        path = QPainterPath()
        path.addRoundedRect(0, 0, self.w, self.h, 5.0, 5.0)

        painter.fillPath(path, QBrush(COLOR_HEADER_BG))
        painter.setPen(QPen(COLOR_HEADER_BORDER, 1.0))
        painter.drawPath(path)

        # Draw icon and text
        font = QFont("Monospace", 9, QFont.Weight.Bold)
        painter.setFont(font)

        # Icon "▾" or "▸"
        icon = "▸" if self.is_collapsed else "▾"
        painter.setPen(QPen(COLOR_HEADER_ICON))
        painter.drawText(QRectF(10, 0, 20, self.h), Qt.AlignmentFlag.AlignVCenter, icon)

        # Label
        painter.setPen(QPen(COLOR_HEADER_TEXT))
        painter.drawText(QRectF(26, 0, self.w - 30, self.h), Qt.AlignmentFlag.AlignVCenter, self.label)


class NodeGraphicsItem(QGraphicsItem):
    """Call Graph Node with monochrome body, status header strip, and text label."""

    def __init__(self, node_id, x, y, status_color, canvas, parent=None):
        super().__init__(parent)
        self.node_id = node_id
        self.w = max(110.0, len(node_id) * 8.5 + 28.0)
        self.h = 34.0
        self.radius = 7.0
        self.status_color = status_color
        self.canvas = canvas

        self.setPos(x, y)
        self.setAcceptHoverEvents(True)

        self.is_selected = False
        self.is_hovered = False
        self.is_connected = False
        self.is_dimmed = False

    def boundingRect(self):
        return QRectF(0, 0, self.w, self.h)

    def hoverEnterEvent(self, event):
        self.is_hovered = True
        self.canvas.on_node_hover_changed(self.node_id, True)
        self.update()
        super().hoverEnterEvent(event)

    def hoverLeaveEvent(self, event):
        self.is_hovered = False
        self.canvas.on_node_hover_changed(self.node_id, False)
        self.update()
        super().hoverLeaveEvent(event)

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.canvas.select_node(self.node_id)
        super().mousePressEvent(event)

    def paint(self, painter, option, widget=None):
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        # Determine colors based on interaction state
        if self.is_selected:
            bg_color = COLOR_NODE_SELECT_BG
            border_color = COLOR_NODE_SELECT_BORDER
            pen_width = 2.0
            text_color = QColor("#ffffff")
        elif self.is_hovered:
            bg_color = COLOR_NODE_HOVER_BG
            border_color = COLOR_NODE_HOVER_BORDER
            pen_width = 1.5
            text_color = QColor("#ffffff")
        elif self.is_connected:
            bg_color = QColor("#111113")
            border_color = QColor("#ababa")
            pen_width = 1.5
            text_color = QColor("#ffffff")
        elif self.is_dimmed:
            bg_color = COLOR_NODE_DIMMED_BG
            border_color = COLOR_NODE_DIMMED_BORDER
            pen_width = 1.0
            text_color = QColor("#555555")
        else:
            bg_color = COLOR_NODE_NORMAL_BG
            border_color = COLOR_NODE_NORMAL_BORDER
            pen_width = 1.0
            text_color = QColor("#e0e0e0")

        # 1. Body shape
        body_path = QPainterPath()
        body_path.addRoundedRect(0, 0, self.w, self.h, self.radius, self.radius)

        painter.fillPath(body_path, QBrush(bg_color))
        painter.setPen(QPen(border_color, pen_width))
        painter.drawPath(body_path)

        # 2. Header Strip (top 7px filled with status color)
        painter.save()
        painter.setClipPath(body_path)
        strip_color = QColor(self.status_color)
        if self.is_dimmed:
            strip_color = QColor(
                int(strip_color.red() * 0.4),
                int(strip_color.green() * 0.4),
                int(strip_color.blue() * 0.4)
            )
        painter.fillRect(QRectF(0, 0, self.w, 7.0), strip_color)
        painter.restore()

        # 3. Label Text
        painter.setFont(QFont("Monospace", 9, QFont.Weight.Bold if (self.is_selected or self.is_hovered) else QFont.Weight.Normal))
        painter.setPen(QPen(text_color))
        painter.drawText(QRectF(0, 7.0, self.w, self.h - 7.0), Qt.AlignmentFlag.AlignCenter, self.node_id)


class EdgeGraphicsItem(QGraphicsItem):
    """Cubic Bezier connection edge with arrowhead and socket dots."""

    def __init__(self, from_node, to_node, sx, sy, ex, ey, parent=None):
        super().__init__(parent)
        self.from_node = from_node
        self.to_node = to_node
        self.sx = sx
        self.sy = sy
        self.ex = ex
        self.ey = ey

        # Compute cubic bezier control points
        dx = ex - sx
        dy = ey - sy

        if abs(dy) >= abs(dx):
            offset_y = max(40.0, abs(dy) * 0.55)
            sign_y = 1.0 if dy >= 0.0 else -1.0
            self.cp1 = QPointF(sx, sy + offset_y * sign_y)
            self.cp2 = QPointF(ex, ey - offset_y * sign_y)
        else:
            offset_x = max(40.0, abs(dx) * 0.55)
            sign_x = 1.0 if dx >= 0.0 else -1.0
            self.cp1 = QPointF(sx + offset_x * sign_x, sy)
            self.cp2 = QPointF(ex - offset_x * sign_x, ey)

        self.is_highlighted = False
        self.is_dimmed = False

    def boundingRect(self):
        min_x = min(self.sx, self.ex, self.cp1.x(), self.cp2.x()) - 15
        max_x = max(self.sx, self.ex, self.cp1.x(), self.cp2.x()) + 15
        min_y = min(self.sy, self.ey, self.cp1.y(), self.cp2.y()) - 15
        max_y = max(self.sy, self.ey, self.cp1.y(), self.cp2.y()) + 15
        return QRectF(min_x, min_y, max_x - min_x, max_y - min_y)

    def paint(self, painter, option, widget=None):
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        if self.is_highlighted:
            pen_color = COLOR_EDGE_HIGHLIGHT
            pen_width = 2.0
            socket_color = COLOR_EDGE_HIGHLIGHT
        elif self.is_dimmed:
            pen_color = COLOR_EDGE_DIMMED
            pen_width = 1.0
            socket_color = COLOR_EDGE_DIMMED
        else:
            pen_color = COLOR_EDGE_NORMAL
            pen_width = 1.5
            socket_color = COLOR_SOCKET_DOT

        # 1. Draw Cubic Bezier Path
        path = QPainterPath()
        path.moveTo(self.sx, self.sy)
        path.cubicTo(self.cp1, self.cp2, QPointF(self.ex, self.ey))

        painter.setPen(QPen(pen_color, pen_width))
        painter.drawPath(path)

        # 2. Draw Arrowhead at Target (ex, ey)
        angle = math.atan2(self.ey - self.cp2.y(), self.ex - self.cp2.x())
        arrow_len = 10.0 if self.is_highlighted else 8.0
        arrow_angle = 0.45

        p1 = QPointF(
            self.ex - arrow_len * math.cos(angle - arrow_angle),
            self.ey - arrow_len * math.sin(angle - arrow_angle),
        )
        p2 = QPointF(
            self.ex - arrow_len * math.cos(angle + arrow_angle),
            self.ey - arrow_len * math.sin(angle + arrow_angle),
        )

        arrow_poly = QPolygonF([QPointF(self.ex, self.ey), p1, p2])
        painter.setBrush(QBrush(pen_color))
        painter.drawPolygon(arrow_poly)

        # 3. Draw Connection Sockets (Dots)
        if not self.is_dimmed:
            r = 3.5
            painter.setBrush(QBrush(socket_color))
            painter.setPen(Qt.PenStyle.NoPen)
            painter.drawEllipse(QPointF(self.sx, self.sy), r, r)
            painter.drawEllipse(QPointF(self.ex, self.ey), r, r)


class CallGraphCanvas(QGraphicsView):
    """Swimlane Call Graph Canvas in PyQt6 with pan/zoom and fitInView."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.scene = QGraphicsScene(self)
        self.setScene(self.scene)

        self.setRenderHint(QPainter.RenderHint.Antialiasing)
        self.setRenderHint(QPainter.RenderHint.TextAntialiasing)
        self.setDragMode(QGraphicsView.DragMode.ScrollHandDrag)
        self.setTransformationAnchor(QGraphicsView.ViewportAnchor.AnchorUnderMouse)
        self.setResizeAnchor(QGraphicsView.ViewportAnchor.AnchorViewCenter)

        self.node_items = {}
        self.edge_items = []
        self.header_items = []
        self.selected_node_id = None

    def load_graph_data(self, layout, edges):
        """Clear canvas and construct all swimlanes, nodes, and bezier edges."""
        self.scene.clear()
        self.node_items.clear()
        self.edge_items.clear()
        self.header_items.clear()
        self.selected_node_id = None

        # Title Header Item
        title_item = QGraphicsTextItem("[ STAKHAL CALL GRAPH DIAGRAM (DRAGGABLE) ]")
        title_item.setFont(QFont("Monospace", 10, QFont.Weight.Bold))
        title_item.setDefaultTextColor(QColor(110, 110, 110))
        title_item.setPos(20, 20)
        self.scene.addItem(title_item)

        # 1. Add Swimlane Chain Headers
        for h in layout.headers:
            header_item = ChainHeaderGraphicsItem(h)
            self.scene.addItem(header_item)
            self.header_items.append(header_item)

        # 2. Compute Node Status Colors
        node_colors = {}
        for n_id in layout.positions.keys():
            node_colors[n_id] = get_node_status_color(n_id, edges)

        # 3. Add Node Items
        for n_id, (n_x, n_y) in layout.positions.items():
            status_color = node_colors[n_id]
            node_item = NodeGraphicsItem(n_id, n_x, n_y, status_color, self)
            self.scene.addItem(node_item)
            self.node_items[n_id] = node_item

        # 4. Add Edge Items with Ray Intersections
        for e in edges:
            if e.from_node in layout.positions and e.to_node in layout.positions:
                fx, fy = layout.positions[e.from_node]
                tx, ty = layout.positions[e.to_node]

                fw = self.node_items[e.from_node].w
                fh = self.node_items[e.from_node].h
                tw = self.node_items[e.to_node].w
                th = self.node_items[e.to_node].h

                tc_x, tc_y = tx + tw / 2.0, ty + th / 2.0
                fc_x, fc_y = fx + fw / 2.0, fy + fh / 2.0

                sx, sy = get_rect_ray_intersection(fx, fy, fw, fh, tc_x, tc_y)
                ex, ey = get_rect_ray_intersection(tx, ty, tw, th, fc_x, fc_y)

                edge_item = EdgeGraphicsItem(e.from_node, e.to_node, sx, sy, ex, ey)
                self.scene.addItem(edge_item)
                self.edge_items.append(edge_item)

        # Auto-fit initial view
        self.fit_to_view()

    def wheelEvent(self, event):
        """Zoom in/out smoothly relative to mouse cursor position."""
        zoom_factor = 1.15 if event.angleDelta().y() > 0 else (1.0 / 1.15)
        self.scale(zoom_factor, zoom_factor)

    def fit_to_view(self):
        """Fit all graph items (including unreachable/isolated nodes) within viewport."""
        items_rect = self.scene.itemsBoundingRect()
        if not items_rect.isEmpty():
            padded_rect = items_rect.adjusted(-60, -60, 60, 60)
            self.fitInView(padded_rect, Qt.AspectRatioMode.KeepAspectRatio)

    def select_node(self, node_id):
        """Select a node and highlight its connected edges."""
        if self.selected_node_id == node_id:
            self.selected_node_id = None
        else:
            self.selected_node_id = node_id

        self.update_highlights()

    def on_node_hover_changed(self, node_id, is_hovered):
        """Hover state update handler."""
        if not self.selected_node_id:
            self.update_highlights(hovered_id=node_id if is_hovered else None)

    def update_highlights(self, hovered_id=None):
        active_id = self.selected_node_id or hovered_id

        if not active_id:
            # Reset all to normal
            for n_item in self.node_items.values():
                n_item.is_selected = False
                n_item.is_connected = False
                n_item.is_dimmed = False
                n_item.update()

            for e_item in self.edge_items:
                e_item.is_highlighted = False
                e_item.is_dimmed = False
                e_item.update()
            return

        connected_nodes = {active_id}
        connected_edges = set()

        for idx, e_item in enumerate(self.edge_items):
            if e_item.from_node == active_id or e_item.to_node == active_id:
                connected_edges.add(idx)
                connected_nodes.add(e_item.from_node)
                connected_nodes.add(e_item.to_node)

        for n_id, n_item in self.node_items.items():
            n_item.is_selected = (n_id == self.selected_node_id)
            n_item.is_connected = (n_id in connected_nodes)
            n_item.is_dimmed = (n_id not in connected_nodes)
            n_item.update()

        for idx, e_item in enumerate(self.edge_items):
            e_item.is_highlighted = (idx in connected_edges)
            e_item.is_dimmed = (idx not in connected_edges)
            e_item.update()

    def export_to_png(self, file_path: str):
        """Render the complete scene to a PNG image file."""
        items_rect = self.scene.itemsBoundingRect()
        if items_rect.isEmpty():
            print("Canvas is empty, skipping PNG export.")
            return False

        padded_rect = items_rect.adjusted(-40, -40, 40, 40)
        img_w = int(padded_rect.width())
        img_h = int(padded_rect.height())

        image = QImage(img_w, img_h, QImage.Format.Format_ARGB32)
        image.fill(COLOR_CANVAS_BG)

        painter = QPainter(image)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setRenderHint(QPainter.RenderHint.TextAntialiasing)
        
        target_rect = QRectF(0, 0, img_w, img_h)
        self.scene.render(painter, target_rect, padded_rect)
        painter.end()

        os.makedirs(os.path.dirname(os.path.abspath(file_path)), exist_ok=True)
        success = image.save(file_path)
        print(f"Exported Call Graph PNG to: {file_path} (Success: {success})")
        return success
