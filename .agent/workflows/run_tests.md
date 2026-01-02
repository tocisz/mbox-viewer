---
description: Run backend integration tests to verify system functionality
---

1. Build the Rust backend binary to ensure it's up to date.
   ```bash
   cd backend
   cargo build --release
   ```

2. Run the Python integration tests.
   ```bash
   # Use the virtual environment Python to run pytest
   .venv/bin/pytest tests/integration_tests.py
   ```