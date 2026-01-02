#!/bin/sh
set -e

# Ensure directories exist
mkdir -p "$INDEX_PATH"
mkdir -p "$ATTACHMENTS_DIR"

# Check if index exists (look for meta.json which is standard for Tantivy)
if [ ! -f "$INDEX_PATH/meta.json" ]; then
    echo "Index not found at $INDEX_PATH"
    
    if [ -f "$MBOX_FILE" ]; then
        echo "Found MBOX at $MBOX_FILE"
        echo "Starting indexing process... This may take a while depending on the size."
        
        # Run indexer
        /app/backend index --mbox "$MBOX_FILE" --attachments-dir "$ATTACHMENTS_DIR"
        
        echo "Indexing complete."
    else
        echo "WARNING: No MBOX file found at $MBOX_FILE"
        echo "To auto-index, mount your mbox file: -v /path/to/my.mbox:/data/mail.mbox"
        echo "Or mount an existing index: -v /path/to/index:/data/index"
    fi
else
    echo "Index found at $INDEX_PATH"
fi

echo "--------------------------------------------------------"
echo "Starting Email Server..."
echo "Listening on port $PORT"
echo "Make sure to map the port: -p 8080:$PORT"
echo "--------------------------------------------------------"

# Execute Server
exec /app/backend serve --port "$PORT" --attachments-dir "$ATTACHMENTS_DIR"
