---
description: Run backend integration tests to verify system functionality
---

1. Build the Rust email-server binary to ensure it's up to date.
   ```bash
   cd email-server
   cargo build --release
   ```

2. Run the Python integration tests.
   ```bash
   # Use the virtual environment Python to run pytest
   ./.venv/bin/pytest backend/tests/integration_tests.py
   ```
