#!/usr/bin/env bash
# Ignore hangup so it keeps running if your session disconnects
trap '' SIGHUP

set -euo pipefail
shopt -s nullglob

# 1) Define parameter sweeps
REQUEST_COUNTS=(64 128 256 512 1024)
NODE_COUNTS=(2 4 8 16 32)
CLIENT_COUNTS=(2 4 8 16 32)

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$BASE_DIR")"
MASTER_CSV="$BASE_DIR/master_summary.csv"


# 2) Master summary header (no network bytes)
echo "req_count,node_count,client_count,\
total_requests,successful,success_rate,\
avg_latency_ms,min_latency_ms,max_latency_ms,\
wall_time_s,throughput_rps,per_node_throughput_rps,\
total_comm_bytes,total_comm_MiB" > "$MASTER_CSV"

# 3) Build the release binary once
echo "? Building release binary in $PROJECT_ROOT?"
(
  cd "$PROJECT_ROOT"
  cargo build --release --bin event_beacon
)
BINARY="$PROJECT_ROOT/target/release/event_beacon"
strip "$BINARY" &>/dev/null || true
echo "? Done, using $BINARY"

# 4) Sweep all scenarios
for REQ_COUNT in "${REQUEST_COUNTS[@]}"; do
  for NODE_COUNT in "${NODE_COUNTS[@]}"; do
    for CLIENT_COUNT in "${CLIENT_COUNTS[@]}"; do

      echo
      echo "=== Scenario: ${REQ_COUNT} requests, ${NODE_COUNT} nodes, ${CLIENT_COUNT} clients ==="

      # 4a) Paths for this run
      REQ_DIR="$BASE_DIR/requests/${REQ_COUNT}_requests/${CLIENT_COUNT}_${REQ_COUNT}_requests"
      NODELIST_FILE="$BASE_DIR/nodelists/nodelist_${NODE_COUNT}"
      TIMESTAMP=$(date +%Y%m%d_%H%M%S)
      RESULTS_DIR="$BASE_DIR/results/${REQ_COUNT}req_${NODE_COUNT}nodes_${CLIENT_COUNT}clients_${TIMESTAMP}"
      TIMINGS_FILE="$RESULTS_DIR/timings_${REQ_COUNT}req_${NODE_COUNT}nodes_${CLIENT_COUNT}clients.csv"
      SUMMARY_FILE="$RESULTS_DIR/summary_${REQ_COUNT}req_${NODE_COUNT}nodes_${CLIENT_COUNT}clients.csv"

      # 4b) Sanity checks
      [[ -d "$REQ_DIR" ]]   || { echo "ERROR: requests dir not found: $REQ_DIR"; continue; }
      [[ -f "$NODELIST_FILE" ]] || { echo "ERROR: nodelist file not found: $NODELIST_FILE"; continue; }
      mkdir -p "$RESULTS_DIR"

      # 4c) Load requests & nodes
      mapfile -t REQUESTS < <(printf '%s\n' "$REQ_DIR"/*.json)
      (( ${#REQUESTS[@]} )) || { echo "ERROR: no .json in $REQ_DIR"; continue; }
      mapfile -t NODES < "$NODELIST_FILE"

      # 4d) Dispatch jobs
      echo "? Dispatching ${#REQUESTS[@]} requests across ${#NODES[@]} nodes?"
      touch "$RESULTS_DIR/raw_results.csv"
      echo "fn,lat_ms,exit" > "$RESULTS_DIR/raw_results.csv"
      GLOBAL_START=$(date +%s%3N)

      for req in "${REQUESTS[@]}"; do
        fn="$(basename "$req")"
        name="${fn%.json}"
        set_num="${name#*_*_}"     # ?2c_1_1? ? ?1_1?
        set_num="${set_num%%_*}"   # ? ?1?
        idx=$(( (set_num - 1) % ${#NODES[@]} ))
        node="${NODES[$idx]}"

        (
          start=$(date +%s%3N)
          nohup "$BINARY" "$req" "$node" >/dev/null 2>&1
          exit_code=$?
          end=$(date +%s%3N)
          lat=$((end - start))
          printf '%s,%d,%d\n' "$fn" "$lat" "$exit_code" \
            >> "$RESULTS_DIR/raw_results.csv"
        ) &
      done

      echo "? All dispatched; waiting for completion?"
      wait
      GLOBAL_END=$(date +%s%3N)

      # 4e) Build timings CSV
      {
        echo "request,latency_ms,status"
        tail -n +2 "$RESULTS_DIR/raw_results.csv" | while IFS=, read -r fn lat exit; do
          [[ $exit -eq 0 ]] && status=SUCCESS || status=FAILURE
          echo "$fn,$lat,$status"
        done
      } > "$TIMINGS_FILE"

      # 4f) Compute summary metrics
      total=$(( $(wc -l < "$TIMINGS_FILE") - 1 ))
      succ=$(grep -c ',SUCCESS$' "$TIMINGS_FILE")
      minlat=$(awk -F, 'NR>1{print $2}' "$TIMINGS_FILE" | sort -n | head -1)
      maxlat=$(awk -F, 'NR>1{print $2}' "$TIMINGS_FILE" | sort -n | tail -1)
      avg=$(awk -F, 'NR>1{sum+=$2} END{printf("%.2f", sum/(NR-1))}' "$TIMINGS_FILE")
      rate=$(awk -v s="$succ" -v t="$total" 'BEGIN{printf("%.2f", s/t*100)}')
      wall_ms=$((GLOBAL_END - GLOBAL_START))
      wall_s=$(awk "BEGIN{printf(\"%.3f\", $wall_ms/1000)}")
      throughput=$(awk "BEGIN{printf(\"%.2f\", $total/$wall_s)}")
      per_node=$(awk "BEGIN{printf(\"%.2f\", $throughput/$NODE_COUNT)}")
      req_bytes=$(awk '{sum+=$1} END{print sum}' < <(du -b "${REQUESTS[@]}" | awk '{print $1}'))
      comm_mb=$(awk "BEGIN{printf(\"%.2f\", $req_bytes/1024/1024)}")

      # write per?scenario summary (no network fields)
      cat > "$SUMMARY_FILE" <<EOF
total_requests,$total
successful,$succ
success_rate,${rate}%
avg_latency_ms,$avg
min_latency_ms,$minlat
max_latency_ms,$maxlat
wall_time_s,$wall_s
throughput_rps,$throughput
per_node_throughput_rps,$per_node
total_comm_bytes,$req_bytes
total_comm_MiB,$comm_mb
EOF

      # immediate feedback
      echo "? Done ${REQ_COUNT} req, ${NODE_COUNT} nodes, ${CLIENT_COUNT} clients: total=$total, avg=${avg}ms, thrpt=${throughput}rps"

      # 4g) Append summary to master CSV
      declare -A sum
      while IFS=, read -r key value; do sum[$key]=$value; done < "$SUMMARY_FILE"

      printf "%s,%s,%s," \
        "$REQ_COUNT" "$NODE_COUNT" "$CLIENT_COUNT" \
        >> "$MASTER_CSV"
      printf "%s,%s,%s," \
        "${sum[total_requests]}" \
        "${sum[successful]}" \
        "${sum[success_rate]}" \
        >> "$MASTER_CSV"
      printf "%s,%s,%s," \
        "${sum[avg_latency_ms]}" \
        "${sum[min_latency_ms]}" \
        "${sum[max_latency_ms]}" \
        >> "$MASTER_CSV"
      printf "%s,%s,%s," \
        "${sum[wall_time_s]}" \
        "${sum[throughput_rps]}" \
        "${sum[per_node_throughput_rps]}" \
        >> "$MASTER_CSV"
      printf "%s,%s\n" \
        "${sum[total_comm_bytes]}" \
        "${sum[total_comm_MiB]}" \
        >> "$MASTER_CSV"

      # 4h) Cleanup remote JSONs on all nodes with identity file
      echo "? Cleaning up .json request files on nodes: ${NODES[*]}"
      for node_host in "${NODES[@]}"; do
        ssh -i rpi.key -o BatchMode=yes node@"$node_host.local" \
          "rm -f /home/node/Ebeacon/*.json" \
          || echo "?? Failed to clean on $node_host"
      done

    done
  done
done

echo
echo "? All scenarios complete."
echo "? Master summary: $MASTER_CSV"
