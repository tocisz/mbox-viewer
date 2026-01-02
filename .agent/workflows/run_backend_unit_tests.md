---
description: Run backend unit tests
---

1. Run the Python unit tests using unittest module.
   ```bash
   # Use the virtual environment Python to run unittest discovery
   // turbo
   ./.venv/bin/python3 -m unittest discover -v -s ./backend/tests -p "test_*.py"
   ```
