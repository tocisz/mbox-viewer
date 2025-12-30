#!/bin/bash
# start_es.sh - Starts Elasticsearch if it's not already running

# Check if ES is already responding on port 9200
if curl -s http://localhost:9200 > /dev/null; then
    echo "Elasticsearch is already running on http://localhost:9200"
else
    echo "Starting Elasticsearch in background..."
    # -d runs it in daemon mode
    ./elasticsearch-8.17.0/bin/elasticsearch -d
    echo "Elasticsearch starting. Use 'curl http://localhost:9200' to check status."
fi
