#!/usr/bin/env python3
"""Entrypoint for explicit multi-dataset reproduction stages."""
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE / "reproduction"))

from runner import main

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
