#!/bin/bash
# Test the Kyco server endpoint

echo "Testing POST to http://localhost:9876/selection..."
echo ""

curl -X POST http://localhost:9876/selection \
  -H "Content-Type: application/json" \
  -d '{
    "file_path": "/test/file.rs",
    "selected_text": "fn main() { }",
    "line_start": 1,
    "line_end": 1,
    "workspace": "/test"
  }' \
  -v

echo ""
echo ""
echo "If the server is running, you should see a 2xx response above."
