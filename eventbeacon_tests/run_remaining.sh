#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

# ==== capture network stats before test ====
IFACE=eth0   # adjust to your beacon interface
RX_BEFORE=$(cat /sys/class/net/$IFACE/statistics/rx_bytes)
TX_BEFORE=$(cat /sys/class/net/$IFACE/statistics/tx_bytes)
# ============================================

# ---- Prompt for counts ----
read -p "Enter request count (64/128/256/512/1024): " REQ_COUNT
read -p "Enter node count    (2/4/8/16/32):        " NODE_COUNT
read -p "Enter client count  (2/4/8/16/32):        " CLIENT_COUNT

# ---- Directories and files ----
BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REQ_DIR="$BASE_DIR/requests/${REQ_COUNT}_requests/${CLIENT_COUNT}_${REQ_COUNT}_requests"
NODELIST_FILE="$BASE_DIR/nodelists/nodelist_${NODE_COUNT}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="$BASE_DIR/results/${REQ_COUNT}req_${NODE_COUNT}nodes_${CLIENT_COUNT}clients_${TIMESTAMP}"

TIMINGS_FILE="$RESULTS_DIR/timings_${REQ_COUNT}req_${NODE_COUNT}nodes_${CLIENT_COUNT}clients.csv"
SUMMARY_FILE="$RESULTS_DIR/summary_${REQ_COUNT}req_${NODE_COUNT}nodes_${CLIENT_COUNT}clients.csv"

# Master summary (persistent across runs)
MASTER_CSV="$BASE_DIR/master_summary.csv"
KEY_FILE="$BASE_DIR/rpi.key"

# Initialize master_summary.csv header if it doesn't exist
if [[ ! -f "$MASTER_CSV" ]]; then
  echo "req_count,node_count,client_count,\
total_requests,successful,success_rate,\
avg_latency_ms,min_latency_ms,max_latency_ms,\
wall_time_s,throughput_rps,per_node_throughput_rps,\
total_comm_bytes,total_comm_MiB" > "$MASTER_CSV"
fi

# ---- Sanity checks ----
[[ -d "$REQ_DIR" ]]   || { echo "ERROR: requests dir not found: $REQ_DIR" >&2; exit 1; }
[[ -f "$NODELIST_FILE" ]] || { echo "ERROR: nodelist file not found: $NODELIST_FILE" >&2; exit 1; }
mkdir -p "$RESULTS_DIR"

# ---- Gather JSONs and nodes ----
mapfile -t REQUESTS < <(printf '%s\n' "$REQ_DIR"/*.json)
(( ${#REQUESTS[@]} )) || { echo "ERROR: no .json in $REQ_DIR" >&2; exit 1; }
mapfile -t NODES < "$NODELIST_FILE"

# ------------------------------------------------------------------
# Build single release binary
PROJECT_ROOT="$(dirname "$BASE_DIR")"
echo "? Building release binary in $PROJECT_ROOT?"
(
  cd "$PROJECT_ROOT"
  cargo build --release --bin event_beacon
)
BINARY="$PROJECT_ROOT/target/release/event_beacon"
strip "$BINARY" &>/dev/null || true
echo "? Done, using $BINARY"
# ------------------------------------------------------------------

# ---- Banner ----
echo "? Results dir: $RESULTS_DIR"
echo "? ${#REQUESTS[@]} requests ? ${#NODES[@]} nodes ? $CLIENT_COUNT clients each"

# ---- Dispatch all in background (timing) ----
touch "$RESULTS_DIR/raw_results.csv"
echo "fn,lat_ms,exit" > "$RESULTS_DIR/raw_results.csv"
GLOBAL_START=$(date +%s%3N)

for i in "${!REQUESTS[@]}"; do
  req="${REQUESTS[i]}"
  fn="$(basename "$req")"
  node="${NODES[$(( i % ${#NODES[@]} ))]}"
  echo "?  $fn ? $node"

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

echo
echo "? All dispatched; waiting for completion?"
wait
GLOBAL_END=$(date +%s%3N)

# ---- Collate into timings CSV ----
printf 'request,latency_ms,status\n' > "$TIMINGS_FILE"
tail -n +2 "$RESULTS_DIR/raw_results.csv" | while IFS=, read -r fn lat exit; do
  if [[ $exit -eq 0 ]]; then status=SUCCESS; else status=FAILURE; fi
  echo "? $fn ? $status in ${lat}ms"
  printf '%s,%s,%s\n' "$fn" "$lat" "$status" \
    >> "$TIMINGS_FILE"
done

# ---- Build summary.csv with performance & comm data ----
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

# ==== capture network stats after test ====
RX_AFTER=$(cat /sys/class/net/$IFACE/statistics/rx_bytes)
TX_AFTER=$(cat /sys/class/net/$IFACE/statistics/tx_bytes)
NET_RX=$((RX_AFTER - RX_BEFORE))
NET_TX=$((TX_AFTER - TX_BEFORE))
NET_TOTAL=$((NET_RX + NET_TX))
NET_MiB=$(awk "BEGIN{printf(\"%.2f\", $NET_TOTAL/1024/1024)}")

cat >> "$SUMMARY_FILE" <<EOF
total_network_rx_bytes,$NET_RX
total_network_tx_bytes,$NET_TX
total_network_bytes,$NET_TOTAL
total_network_MiB,$NET_MiB
EOF
# ============================================

# ---- Append this scenario to master summary ----
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

# ---- Cleanup remote JSONs on all nodes ----
echo "? Cleaning up .json request files on nodes: ${NODES[*]}"
for node_host in "${NODES[@]}"; do
  ssh -i "$KEY_FILE" -o BatchMode=yes node@"$node_host.local" \
    "rm -f /home/node/Ebeacon/*.json" \
    || echo "?? Failed to clean on $node_host.local"
done

# 9) Done
echo
echo "? Done!"
echo "  ? Timings: $TIMINGS_FILE"
echo "  ? Summary: $SUMMARY_FILE"


echo "  ? Appended to master summary: $MASTER_CSV"
