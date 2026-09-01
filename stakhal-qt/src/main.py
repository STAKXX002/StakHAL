import os
import sys

src_dir = os.path.dirname(os.path.abspath(__file__))
if src_dir not in sys.path:
    sys.path.insert(0, src_dir)

from PyQt6.QtWidgets import QApplication
from main_window import StakHalMainWindow

def main():
    app = QApplication(sys.argv)
    window = StakHalMainWindow()
    window.show()
    
    if "--headless" in sys.argv or os.environ.get("QT_QPA_PLATFORM") == "offscreen":
        app.processEvents()
        print("Application scaffold started and verified successfully in offscreen mode.")
        sys.exit(0)
    
    sys.exit(app.exec())

if __name__ == "__main__":
    main()
